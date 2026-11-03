//! **Ficheros por ruta o por `content://`.** En Android los diálogos de
//! ficheros no dan rutas sino URI `content://` (un permiso concedido por el
//! sistema a ese fichero, sin carpeta ni extensión). Los comandos que leen o
//! escriben lo que elige la persona pasan por aquí, y en escritorio todo se
//! comporta exactamente como antes: una ruta es una ruta.
//!
//! - **Leer** un `content://`: se copia a la caché y se trabaja con esa copia
//!   ([`leer`]). Un proveedor en la nube puede dar una tubería sin `Seek`, y el
//!   ZIP del `.pbdeck` lo necesita. La copia se borra al soltar la [`Copia`].
//! - **Escribir** un `content://`: se escribe en la caché y se copia al destino
//!   ([`escribir`], [`escribir_bytes`]), porque un `content://` no se puede
//!   renombrar (`escribir_atomico`).
//! - **Nombre y formato.** Sin extensión en la URI, el nombre sale de Android
//!   (`DISPLAY_NAME`, vía `android::nombre_de_uri`) o del último trozo de la
//!   URI; los comandos que dependen del formato (imagen, `.ttf`, `.txt`) lo
//!   sacan de ese nombre, no de la ruta.
//!
//! La apertura de una URI es de `tauri-plugin-fs` (`app.fs().open(..)`, que
//! da un `std::fs::File` de verdad desde el descriptor del sistema), solo
//! desde Rust y sin permisos `fs:` en las capacidades. Para probarlo sin
//! Android, la lógica va contra el trait [`Acceso`].

use std::fs::File;
use std::io::{Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;

/// Carpeta de la caché donde viven las copias y los temporales.
const CARPETA: &str = "fichero";
/// Lo que lleve más tiempo que esto en [`CARPETA`] es de una sesión anterior.
const VIDA_MAXIMA: Duration = Duration::from_secs(24 * 60 * 60);

/// ¿Es una URI de Android y no una ruta?
pub fn es_uri(ruta: &str) -> bool {
    ruta.starts_with("content://")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    Leer,
    /// Escribir truncando.
    Escribir,
}

/// Cómo se abre una URI y cómo se llama. En la app lo hace Android; en los
/// tests, un mapa a ficheros locales.
pub trait Acceso {
    fn abrir(&self, uri: &str, modo: Modo) -> std::io::Result<File>;
    fn nombre(&self, uri: &str) -> Option<String>;
}

struct DeLaApp<'a>(&'a AppHandle);

impl Acceso for DeLaApp<'_> {
    fn abrir(&self, uri: &str, modo: Modo) -> std::io::Result<File> {
        use tauri_plugin_fs::{FilePath, FsExt, OpenOptions};
        let ruta = FilePath::from_str(uri).map_err(std::io::Error::other)?;
        let mut o = OpenOptions::new();
        match modo {
            Modo::Leer => o.read(true),
            Modo::Escribir => o.read(false).write(true).truncate(true),
        };
        self.0.fs().open(ruta, o)
    }

    fn nombre(&self, uri: &str) -> Option<String> {
        crate::android::nombre_de_uri(self.0, uri)
    }
}

// ── Nombres ──────────────────────────────────────────────────────────────

/// Un nombre de fichero que se puede crear en cualquier carpeta: sin
/// separadores ni caracteres de control, y sin ser vacío ni `..`.
pub fn nombre_seguro(nombre: &str) -> String {
    let limpio: String = nombre
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let limpio = limpio.trim().trim_matches('.').to_string();
    if limpio.is_empty() {
        return "archivo".into();
    }
    // Tampoco enorme: los sistemas de ficheros paran en 255 bytes.
    limpio.chars().take(120).collect()
}

fn decodificar(texto: &str) -> String {
    let b = texto.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let par = std::str::from_utf8(&b[i + 1..i + 3])
                .ok()
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = par {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// El último trozo de una URI, ya decodificado: a veces es el nombre
/// (`…/document/primary%3ADownload%2Fdeck.pbdeck`), a veces un número.
fn nombre_en_la_uri(uri: &str) -> Option<String> {
    let sin_consulta = uri.split(['?', '#']).next()?;
    let ultimo = decodificar(sin_consulta.rsplit('/').next()?);
    let nombre = ultimo.rsplit(['/', ':']).next()?.to_string();
    (!nombre.is_empty()).then_some(nombre)
}

/// El nombre de lo que se va a leer o escribir: el que se da, el de Android o
/// el de la URI, ya saneado.
fn nombre_de(acceso: &dyn Acceso, ruta: &str, dado: Option<&str>) -> String {
    let n = dado
        .map(str::to_string)
        .or_else(|| acceso.nombre(ruta))
        .or_else(|| nombre_en_la_uri(ruta))
        .unwrap_or_default();
    nombre_seguro(&n)
}

/// La extensión (en minúsculas) de un nombre.
pub fn extension(nombre: &str) -> Option<String> {
    let (base, ext) = nombre.rsplit_once('.')?;
    (!base.is_empty() && !ext.is_empty() && ext.len() <= 8).then(|| ext.to_ascii_lowercase())
}

/// El tipo MIME de un fichero por su extensión (para compartirlo).
pub fn mime_de(nombre: &str) -> &'static str {
    match extension(nombre).as_deref() {
        Some("pdf") => "application/pdf",
        Some("zip") => "application/zip",
        Some("txt") => "text/plain",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("tif" | "tiff") => "image/tiff",
        Some("bmp") => "image/bmp",
        Some("apk") => "application/vnd.android.package-archive",
        // `.pbdeck` no tiene tipo propio: es un ZIP, pero ofrecerlo como binario
        // genérico evita que algunas apps lo abran o lo renombren como ZIP.
        Some("pbdeck") => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

/// El nombre que tendrá lo que se escriba en `ruta`: el de Android si es una
/// URI, el de la propia ruta si no. Para decidir el formato de lo que se va a
/// escribir.
pub fn nombre_destino(app: &AppHandle, ruta: &str) -> Option<String> {
    nombre_destino_con(&DeLaApp(app), ruta)
}

fn nombre_destino_con(acceso: &dyn Acceso, ruta: &str) -> Option<String> {
    if es_uri(ruta) {
        acceso.nombre(ruta).or_else(|| nombre_en_la_uri(ruta))
    } else {
        Path::new(ruta)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
    }
}

// ── La caché ────────────────────────────────────────────────────────────

fn carpeta_cache(cache: &Path) -> PathBuf {
    cache.join(CARPETA)
}

/// La caché de la app (con el perfil de `PB_PERFIL`, si lo hay).
fn cache_de(app: &AppHandle) -> AppResult<PathBuf> {
    app.try_state::<AppState>()
        .map(|s| s.paths.cache.clone())
        .or_else(|| app.path().app_cache_dir().ok())
        .ok_or_else(|| AppError::new(ErrorCode::Io).with("detalle", "sin carpeta de caché"))
}

/// Borra lo de sesiones anteriores (copias que no se llegaron a soltar).
fn barrer(cache: &Path) {
    let Ok(entradas) = std::fs::read_dir(carpeta_cache(cache)) else {
        return;
    };
    let ahora = SystemTime::now();
    for e in entradas.flatten() {
        let viejo = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| ahora.duration_since(t).ok())
            .is_some_and(|d| d > VIDA_MAXIMA);
        if viejo {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// Un fichero que se puede leer como ruta: el original (una ruta) o una copia
/// en la caché (una URI), que se borra al soltarla.
#[derive(Debug)]
pub struct Copia {
    ruta: PathBuf,
    /// La carpeta temporal a borrar; `None` si es el fichero original.
    carpeta: Option<PathBuf>,
    /// El nombre del fichero (el de Android si era una URI).
    pub nombre: String,
}

impl Copia {
    pub fn ruta(&self) -> &Path {
        &self.ruta
    }

    /// ¿Es una copia en la caché y no el fichero de la persona?
    pub fn es_copia(&self) -> bool {
        self.carpeta.is_some()
    }

    /// Que no se borre al soltarla (el barrido lo hará con el tiempo) y su
    /// camino. Para lo que otra app tiene que leer después.
    pub fn conservar(mut self) -> PathBuf {
        self.carpeta = None;
        std::mem::take(&mut self.ruta)
    }
}

impl Deref for Copia {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.ruta
    }
}

impl AsRef<Path> for Copia {
    fn as_ref(&self) -> &Path {
        &self.ruta
    }
}

impl Drop for Copia {
    fn drop(&mut self) {
        if let Some(c) = &self.carpeta {
            let _ = std::fs::remove_dir_all(c);
        }
    }
}

/// Una carpeta nueva en la caché y el camino de un fichero llamado `nombre`
/// dentro de ella.
fn temporal_en(cache: &Path, nombre: &str) -> AppResult<Copia> {
    barrer(cache);
    let carpeta = carpeta_cache(cache).join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&carpeta)?;
    let nombre = nombre_seguro(nombre);
    Ok(Copia {
        ruta: carpeta.join(&nombre),
        carpeta: Some(carpeta),
        nombre,
    })
}

/// Un fichero vacío por escribir en la caché (se borra al soltarlo).
pub fn temporal(app: &AppHandle, nombre: &str) -> AppResult<Copia> {
    temporal_en(&cache_de(app)?, nombre)
}

// ── Leer ────────────────────────────────────────────────────────────────

/// Pone `ruta` al alcance de `std::fs`: una ruta tal cual, una URI copiada a
/// la caché. `nombre` fija el de la copia (si no, el de Android).
pub fn leer(app: &AppHandle, ruta: &str, nombre: Option<&str>) -> AppResult<Copia> {
    leer_con(&DeLaApp(app), &cache_de(app)?, ruta, nombre)
}

fn leer_con(
    acceso: &dyn Acceso,
    cache: &Path,
    ruta: &str,
    nombre: Option<&str>,
) -> AppResult<Copia> {
    if !es_uri(ruta) {
        let p = PathBuf::from(ruta);
        let nombre = p
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        return Ok(Copia {
            ruta: p,
            carpeta: None,
            nombre,
        });
    }
    let nombre = nombre_de(acceso, ruta, nombre);
    let copia = temporal_en(cache, &nombre)?;
    let mut origen = acceso.abrir(ruta, Modo::Leer).map_err(io_uri)?;
    let mut destino = File::create(copia.ruta())?;
    std::io::copy(&mut origen, &mut destino)?;
    destino.flush()?;
    Ok(copia)
}

/// El contenido entero de `ruta`. Si es una URI, hasta `tope` bytes (más es
/// un error).
pub fn leer_bytes(app: &AppHandle, ruta: &str, tope: u64) -> AppResult<Vec<u8>> {
    leer_bytes_con(&DeLaApp(app), ruta, tope)
}

fn leer_bytes_con(acceso: &dyn Acceso, ruta: &str, tope: u64) -> AppResult<Vec<u8>> {
    // Una ruta se lee entera como siempre; el tope es para lo que da el sistema.
    if !es_uri(ruta) {
        return Ok(std::fs::read(ruta)?);
    }
    let f = acceso.abrir(ruta, Modo::Leer).map_err(io_uri)?;
    let mut bytes = Vec::new();
    f.take(tope.saturating_add(1)).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > tope {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "fichero_grande"));
    }
    Ok(bytes)
}

fn io_uri(e: std::io::Error) -> AppError {
    tracing::warn!("no se pudo abrir un content://: {e}");
    AppError::new(ErrorCode::Io).with("detalle", e)
}

// ── Escribir ────────────────────────────────────────────────────────────

/// Escribe en `ruta` lo que haga `f` con un camino local: la propia ruta si lo
/// es, o un fichero de la caché que luego se copia a la URI. `nombre` es el
/// del fichero temporal (si no, el de la URI): `f` puede sacar de él la
/// extensión.
pub fn escribir<T>(
    app: &AppHandle,
    ruta: &str,
    nombre: Option<&str>,
    f: impl FnOnce(&Path) -> AppResult<T>,
) -> AppResult<T> {
    if !es_uri(ruta) {
        return f(Path::new(ruta));
    }
    escribir_con(&DeLaApp(app), &cache_de(app)?, ruta, nombre, f)
}

fn escribir_con<T>(
    acceso: &dyn Acceso,
    cache: &Path,
    ruta: &str,
    nombre: Option<&str>,
    f: impl FnOnce(&Path) -> AppResult<T>,
) -> AppResult<T> {
    if !es_uri(ruta) {
        return f(Path::new(ruta));
    }
    let tmp = temporal_en(cache, &nombre_de(acceso, ruta, nombre))?;
    let r = f(tmp.ruta())?;
    copiar_a_con(acceso, ruta, tmp.ruta())?;
    Ok(r)
}

/// Escribe `bytes` en `ruta` (una ruta: de una vez, sin dejar medio fichero).
pub fn escribir_bytes(app: &AppHandle, ruta: &str, bytes: &[u8]) -> AppResult<()> {
    escribir_bytes_con(&DeLaApp(app), ruta, bytes)
}

fn escribir_bytes_con(acceso: &dyn Acceso, ruta: &str, bytes: &[u8]) -> AppResult<()> {
    if !es_uri(ruta) {
        return escribir_atomico(Path::new(ruta), bytes);
    }
    let mut f = acceso.abrir(ruta, Modo::Escribir).map_err(io_uri)?;
    f.write_all(bytes)?;
    f.flush()?;
    Ok(())
}

/// Copia un fichero local a `ruta` (una ruta o una URI).
pub fn copiar_a(app: &AppHandle, ruta: &str, origen: &Path) -> AppResult<()> {
    copiar_a_con(&DeLaApp(app), ruta, origen)
}

fn copiar_a_con(acceso: &dyn Acceso, ruta: &str, origen: &Path) -> AppResult<()> {
    if !es_uri(ruta) {
        // A trozos y a un nombre provisional: un ZIP o un PDF grande no cabe
        // siempre en memoria, y no se deja medio fichero con el nombre bueno.
        let destino = Path::new(ruta);
        if let Some(dir) = destino.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = destino.with_extension(format!(
            "{}.{}.part",
            destino
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin"),
            std::process::id()
        ));
        let copiado = std::fs::copy(origen, &tmp).and_then(|_| std::fs::rename(&tmp, destino));
        if let Err(e) = copiado {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
        return Ok(());
    }
    let mut dentro = File::open(origen)?;
    let mut fuera = acceso.abrir(ruta, Modo::Escribir).map_err(io_uri)?;
    std::io::copy(&mut dentro, &mut fuera)?;
    fuera.flush()?;
    Ok(())
}

// ── ZIP ─────────────────────────────────────────────────────────────────

fn de_zip(e: zip::result::ZipError) -> AppError {
    match e {
        zip::result::ZipError::Io(e) => e.into(),
        otro => AppError::new(ErrorCode::Io).with("detalle", otro),
    }
}

fn recorrer(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    let mut entradas: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
    entradas.sort_by_key(|e| e.file_name());
    for e in entradas {
        let p = e.path();
        if p.is_dir() {
            recorrer(base, &p, out)?;
        } else if let Ok(rel) = p.strip_prefix(base) {
            let nombre: Vec<_> = rel.iter().map(|c| c.to_string_lossy()).collect();
            out.push((nombre.join("/"), p));
        }
    }
    Ok(())
}

/// Mete en un ZIP los ficheros de `carpeta` (con sus subcarpetas, como
/// `es/0001 - Sol Ring.png`). Las imágenes y los PDF ya van comprimidos: se
/// guardan tal cual. Devuelve cuántos ficheros metió.
pub fn zip_de_carpeta(carpeta: &Path, zip: &Path) -> AppResult<u32> {
    let mut lista = Vec::new();
    recorrer(carpeta, carpeta, &mut lista)?;
    zip_de_lista(&lista, zip)
}

/// Mete en un ZIP estos ficheros con el nombre que lleven dentro.
pub fn zip_de_lista(lista: &[(String, PathBuf)], zip: &Path) -> AppResult<u32> {
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};
    let mut z = ZipWriter::new(std::io::BufWriter::new(File::create(zip)?));
    let fecha = zip::DateTime::default();
    let guardado = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(fecha)
        .large_file(true);
    let deflate = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(fecha);
    for (nombre, ruta) in lista {
        let ya_comprimido = matches!(
            extension(nombre).as_deref(),
            Some("pdf" | "png" | "jpg" | "jpeg" | "webp" | "zip")
        );
        z.start_file(nombre, if ya_comprimido { guardado } else { deflate })
            .map_err(de_zip)?;
        let mut f = File::open(ruta)?;
        std::io::copy(&mut f, &mut z)?;
    }
    z.finish().map_err(de_zip)?.flush()?;
    Ok(lista.len() as u32)
}

// ── Compartir ───────────────────────────────────────────────────────────

/// Pone a disposición de otra app el fichero `ruta` (una URI o un fichero de
/// la caché de la app): lo deja en la caché, donde el `FileProvider` lo
/// expone, y devuelve el camino y el tipo MIME. La copia se queda hasta que el
/// barrido la borre (Android la lee cuando la otra app quiere).
///
/// 🛑 Una ruta fuera de la caché se rechaza (`InvalidInput`): la pantalla no
/// puede pedir que se comparta la base de datos, la clave del nodo ni nada de
/// `app_data_dir`. Lo que exporta la app para compartir se escribe antes en la
/// caché (`fichero_temporal`) y lo que elige la persona llega como URI.
pub fn para_compartir(app: &AppHandle, ruta: &str) -> AppResult<(PathBuf, &'static str)> {
    para_compartir_con(&DeLaApp(app), &cache_de(app)?, ruta)
}

fn para_compartir_con(
    acceso: &dyn Acceso,
    cache: &Path,
    ruta: &str,
) -> AppResult<(PathBuf, &'static str)> {
    let en_cache = carpeta_cache(cache);
    let origen = Path::new(ruta);
    // Sin `..` ni enlaces que saquen de la caché.
    let dentro = |de: &Path| -> bool {
        match (origen.canonicalize(), de.canonicalize()) {
            (Ok(o), Ok(d)) => o.starts_with(d),
            _ => false,
        }
    };
    let (camino, nombre) = if es_uri(ruta) {
        // Esta copia no se borra al terminar: la otra app aún no la ha leído.
        let copia = leer_con(acceso, cache, ruta, None)?;
        let nombre = copia.nombre.clone();
        (copia.conservar(), nombre)
    } else if !origen.is_file() {
        return Err(AppError::new(ErrorCode::NotFound).with("motivo", "compartir"));
    } else if !dentro(cache) {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "compartir_fuera"));
    } else if dentro(&en_cache) {
        let nombre = origen
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        (origen.to_path_buf(), nombre)
    } else {
        // De otra carpeta de la caché: se copia a la de compartir.
        let nombre = origen
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let copia = temporal_en(cache, &nombre)?;
        std::fs::copy(origen, copia.ruta())?;
        let nombre = copia.nombre.clone();
        (copia.conservar(), nombre)
    };
    let tipo = mime_de(&nombre);
    Ok((camino, tipo))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Un «Android» de mentira: cada URI es un fichero local.
    struct Falso {
        ficheros: HashMap<String, PathBuf>,
        nombres: HashMap<String, String>,
    }

    impl Acceso for Falso {
        fn abrir(&self, uri: &str, modo: Modo) -> std::io::Result<File> {
            let p = self
                .ficheros
                .get(uri)
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            match modo {
                Modo::Leer => File::open(p),
                Modo::Escribir => std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(p),
            }
        }
        fn nombre(&self, uri: &str) -> Option<String> {
            self.nombres.get(uri).cloned()
        }
    }

    fn montar() -> (tempfile::TempDir, Falso, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let f = Falso {
            ficheros: HashMap::new(),
            nombres: HashMap::new(),
        };
        (dir, f, cache)
    }

    const URI: &str = "content://com.android.providers.downloads.documents/document/msf%3A1234";

    #[test]
    fn solo_content_es_una_uri() {
        assert!(es_uri(URI));
        assert!(!es_uri("/home/x/deck.pbdeck"));
        assert!(!es_uri("C:\\deck.pbdeck"));
        assert!(!es_uri("file:///x"));
    }

    #[test]
    fn nombres_seguros_y_extensiones() {
        assert_eq!(nombre_seguro("a/b\\c:d.pdf"), "a_b_c_d.pdf");
        assert_eq!(nombre_seguro(".."), "archivo");
        assert_eq!(nombre_seguro("   "), "archivo");
        assert_eq!(extension("Deck.PBDECK").as_deref(), Some("pbdeck"));
        assert_eq!(extension("sin"), None);
        assert_eq!(extension(".oculto"), None);
        assert_eq!(mime_de("x.pdf"), "application/pdf");
        assert_eq!(mime_de("x.pbdeck"), "application/octet-stream");
        assert_eq!(
            nombre_en_la_uri("content://p/document/primary%3ADownload%2Fmi%20deck.pbdeck")
                .as_deref(),
            Some("mi deck.pbdeck")
        );
        assert_eq!(nombre_en_la_uri(URI).as_deref(), Some("1234"));
        // Un `%` suelto al final no rompe nada.
        assert_eq!(decodificar("100%"), "100%");
        assert_eq!(decodificar("a%2"), "a%2");
    }

    #[test]
    fn una_ruta_se_lee_tal_cual_y_no_se_borra() {
        let (dir, f, cache) = montar();
        let p = dir.path().join("deck.pbdeck");
        std::fs::write(&p, b"hola").unwrap();
        let c = leer_con(&f, &cache, p.to_str().unwrap(), None).unwrap();
        assert_eq!(c.ruta(), p);
        assert_eq!(c.nombre, "deck.pbdeck");
        assert!(!c.es_copia());
        drop(c);
        assert!(p.exists());
    }

    #[test]
    fn una_uri_se_copia_a_la_cache_con_su_nombre_y_se_borra_al_soltar() {
        let (dir, mut f, cache) = montar();
        let origen = dir.path().join("origen.bin");
        std::fs::write(&origen, b"contenido").unwrap();
        f.ficheros.insert(URI.into(), origen.clone());
        f.nombres.insert(URI.into(), "Mi deck.pbdeck".into());
        let c = leer_con(&f, &cache, URI, None).unwrap();
        assert!(c.es_copia());
        assert_eq!(c.nombre, "Mi deck.pbdeck");
        assert!(c.ruta().starts_with(cache.join(CARPETA)));
        assert_eq!(std::fs::read(c.ruta()).unwrap(), b"contenido");
        let carpeta = c.ruta().parent().unwrap().to_path_buf();
        drop(c);
        assert!(!carpeta.exists());
        assert!(origen.exists(), "el original no se toca");
        // Con nombre dado manda ese.
        let c = leer_con(&f, &cache, URI, Some("otra.ttf")).unwrap();
        assert_eq!(c.nombre, "otra.ttf");
    }

    #[test]
    fn sin_nombre_de_android_sale_el_de_la_uri_y_si_no_hay_archivo() {
        let (dir, mut f, cache) = montar();
        let origen = dir.path().join("o");
        std::fs::write(&origen, b"x").unwrap();
        let uri = "content://p/document/primary%3ADownload%2Fcarta.webp";
        f.ficheros.insert(uri.into(), origen);
        assert_eq!(
            leer_con(&f, &cache, uri, None).unwrap().nombre,
            "carta.webp"
        );
        assert_eq!(nombre_destino_con(&f, uri).as_deref(), Some("carta.webp"));
        assert_eq!(
            nombre_destino_con(&f, "/x/y/carta.png").as_deref(),
            Some("carta.png")
        );
        // Una URI que no abre da un error de disco, no un pánico.
        let e = leer_con(&f, &cache, "content://nada/1", None).unwrap_err();
        assert_eq!(e.code, ErrorCode::Io);
    }

    #[test]
    fn leer_bytes_respeta_el_tope() {
        let (dir, mut f, _cache) = montar();
        let origen = dir.path().join("o");
        std::fs::write(&origen, vec![1u8; 100]).unwrap();
        f.ficheros.insert(URI.into(), origen.clone());
        assert_eq!(leer_bytes_con(&f, URI, 100).unwrap().len(), 100);
        let e = leer_bytes_con(&f, URI, 99).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
        assert_eq!(
            leer_bytes_con(&f, origen.to_str().unwrap(), 100)
                .unwrap()
                .len(),
            100
        );
    }

    #[test]
    fn escribir_bytes_a_una_ruta_es_atomico_y_a_una_uri_trunca() {
        let (dir, mut f, _cache) = montar();
        let ruta = dir.path().join("sub/lista.txt");
        escribir_bytes_con(&f, ruta.to_str().unwrap(), b"uno").unwrap();
        assert_eq!(std::fs::read(&ruta).unwrap(), b"uno");
        // A una URI: sobrescribe entero, no deja cola del contenido anterior.
        let destino = dir.path().join("destino");
        std::fs::write(&destino, b"contenido viejo muy largo").unwrap();
        f.ficheros.insert(URI.into(), destino.clone());
        escribir_bytes_con(&f, URI, b"nuevo").unwrap();
        assert_eq!(std::fs::read(&destino).unwrap(), b"nuevo");
    }

    #[test]
    fn escribir_con_una_ruta_va_directo_y_con_una_uri_pasa_por_la_cache() {
        let (dir, mut f, cache) = montar();
        let ruta = dir.path().join("d.pbdeck");
        let r = escribir_con(&f, &cache, ruta.to_str().unwrap(), None, |p| {
            assert_eq!(p, ruta);
            std::fs::write(p, b"A")?;
            Ok(7u64)
        })
        .unwrap();
        assert_eq!(r, 7);
        assert_eq!(std::fs::read(&ruta).unwrap(), b"A");

        let destino = dir.path().join("fuera");
        f.ficheros.insert(URI.into(), destino.clone());
        f.nombres.insert(URI.into(), "deck.pbdeck".into());
        let mut visto = None;
        escribir_con(&f, &cache, URI, None, |p| {
            visto = Some(p.to_path_buf());
            assert_eq!(p.file_name().unwrap(), "deck.pbdeck");
            assert!(p.starts_with(cache.join(CARPETA)));
            std::fs::write(p, b"B")?;
            Ok(())
        })
        .unwrap();
        assert_eq!(std::fs::read(&destino).unwrap(), b"B");
        // El temporal ya no está.
        assert!(!visto.unwrap().exists());
        // Si `f` falla, no se escribe nada en el destino.
        std::fs::write(&destino, b"intacto").unwrap();
        let e = escribir_con(&f, &cache, URI, None, |_| -> AppResult<()> {
            Err(AppError::new(ErrorCode::Pdf))
        })
        .unwrap_err();
        assert_eq!(e.code, ErrorCode::Pdf);
        assert_eq!(std::fs::read(&destino).unwrap(), b"intacto");
    }

    #[test]
    fn el_zip_lleva_las_carpetas_por_idioma() {
        let dir = tempfile::tempdir().unwrap();
        let png = dir.path().join("png");
        std::fs::create_dir_all(png.join("es")).unwrap();
        std::fs::create_dir_all(png.join("en")).unwrap();
        std::fs::write(png.join("es/0001 - Sol Ring.png"), b"es").unwrap();
        std::fs::write(png.join("en/0001 - Sol Ring.png"), b"en").unwrap();
        let z = dir.path().join("salida.zip");
        assert_eq!(zip_de_carpeta(&png, &z).unwrap(), 2);
        let mut a = zip::ZipArchive::new(File::open(&z).unwrap()).unwrap();
        let mut nombres: Vec<_> = (0..a.len())
            .map(|i| a.by_index(i).unwrap().name().unwrap().to_string())
            .collect();
        nombres.sort();
        assert_eq!(
            nombres,
            vec!["en/0001 - Sol Ring.png", "es/0001 - Sol Ring.png"]
        );
        let mut s = String::new();
        a.by_name("es/0001 - Sol Ring.png")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "es");
    }

    #[test]
    fn compartir_deja_el_fichero_en_la_cache_con_su_tipo() {
        let (dir, mut f, cache) = montar();
        // Uno de otra carpeta de la caché se copia a la de compartir.
        std::fs::create_dir_all(cache.join("renders")).unwrap();
        let p = cache.join("renders").join("deck.pdf");
        std::fs::write(&p, b"%PDF").unwrap();
        let (c, tipo) = para_compartir_con(&f, &cache, p.to_str().unwrap()).unwrap();
        assert!(c.starts_with(cache.join(CARPETA)));
        assert_eq!(tipo, "application/pdf");
        assert_eq!(std::fs::read(&c).unwrap(), b"%PDF");
        // Uno que ya está en la caché se comparte donde está.
        let (c2, _) = para_compartir_con(&f, &cache, c.to_str().unwrap()).unwrap();
        assert_eq!(c2, c);
        // Una URI se copia y la copia sobrevive (la otra app aún no la ha leído).
        let origen = dir.path().join("o");
        std::fs::write(&origen, b"zip").unwrap();
        f.ficheros.insert(URI.into(), origen);
        f.nombres.insert(URI.into(), "png.zip".into());
        let (c3, tipo3) = para_compartir_con(&f, &cache, URI).unwrap();
        assert_eq!(tipo3, "application/zip");
        assert!(c3.exists());
        // Lo que no existe no se comparte.
        let e = para_compartir_con(&f, &cache, "/no/existe.pdf").unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
    }

    #[test]
    fn compartir_no_saca_nada_de_fuera_de_la_cache() {
        let (dir, f, cache) = montar();
        // Lo de la app (la base, la clave del nodo) o de la persona: no.
        let datos = dir.path().join("data").join("identidad");
        std::fs::create_dir_all(&datos).unwrap();
        let clave = datos.join("clave");
        std::fs::write(&clave, b"secreto").unwrap();
        let e = para_compartir_con(&f, &cache, clave.to_str().unwrap()).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
        // Ni con `..` desde la caché.
        std::fs::create_dir_all(cache.join(CARPETA)).unwrap();
        let rodeo = cache
            .join(CARPETA)
            .join("..")
            .join("..")
            .join("data")
            .join("identidad")
            .join("clave");
        let e = para_compartir_con(&f, &cache, rodeo.to_str().unwrap()).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
        // Ni por un enlace dentro de la caché que apunte fuera.
        #[cfg(unix)]
        {
            let enlace = cache.join(CARPETA).join("enlace");
            std::os::unix::fs::symlink(&clave, &enlace).unwrap();
            let e = para_compartir_con(&f, &cache, enlace.to_str().unwrap()).unwrap_err();
            assert_eq!(e.code, ErrorCode::InvalidInput);
        }
        assert!(std::fs::read_dir(cache.join(CARPETA)).unwrap().count() <= 1);
    }

    #[test]
    fn el_barrido_solo_borra_lo_viejo() {
        let (_dir, _f, cache) = montar();
        let nueva = temporal_en(&cache, "a.txt").unwrap();
        let vieja = carpeta_cache(&cache).join("vieja");
        std::fs::create_dir_all(&vieja).unwrap();
        let hace_dos_dias = SystemTime::now() - Duration::from_secs(2 * 24 * 3600);
        File::open(&vieja)
            .unwrap()
            .set_modified(hace_dos_dias)
            .unwrap();
        barrer(&cache);
        assert!(!vieja.exists());
        assert!(nueva.ruta().parent().unwrap().exists());
    }
}
