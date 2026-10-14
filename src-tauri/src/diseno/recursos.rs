//! **Recursos del editor de cartas** en la app: los símbolos de Scryfall
//! (descargados y guardados), las letras y los packs importados por el
//! usuario, el motor de cada hilo y las imágenes del almacén para el render.
//!
//! Todo en `app_data_dir`: `simbolos/` (con `indice.json`), `fuentes/`
//! (`<sha256>.<ext>`) y `marcos/<id>/`.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use image::RgbaImage;
use proxy_core::diseno::letras::Letras;
use proxy_core::diseno::pack::{Pack, PackError};
use proxy_core::diseno::render::{Imagenes, Motor};
use proxy_core::diseno::simbolos::Simbolos;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::{CardApi, Scryfall};
use crate::store::blobs::{escribir_atomico, BlobStore};
use crate::store::db::Db;

/// Los símbolos se renuevan si el índice tiene más de 30 días.
const CADUCA_SIMBOLOS: Duration = Duration::from_secs(30 * 24 * 60 * 60);

fn huella(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Una letra importada.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LetraInfo {
    /// SHA-256 del fichero.
    pub id: String,
    pub familias: Vec<String>,
    pub bytes: u64,
}

/// Un pack de marcos disponible.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PackInfo {
    pub id: String,
    pub nombre: String,
    /// `false` para el de la casa (no se puede quitar).
    pub importado: bool,
    pub estilos: Vec<String>,
}

/// Cómo están los símbolos.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SimbolosInfo {
    pub cargados: u32,
    /// Cuándo se descargaron (ms), si se descargaron.
    #[ts(type = "number | null")]
    pub fecha: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Indice {
    fecha: i64,
    /// código → fichero
    simbolos: BTreeMap<String, String>,
}

struct Estado {
    simbolos: Arc<Simbolos>,
    huella_simbolos: String,
    fecha_simbolos: Option<i64>,
    packs: BTreeMap<String, Arc<Pack>>,
    letras: Vec<(LetraInfo, Arc<Vec<u8>>)>,
}

/// Los recursos compartidos por todos los renders.
pub struct Recursos {
    dir: PathBuf,
    estado: RwLock<Estado>,
    /// Sube cada vez que cambian las letras: los motores de cada hilo se
    /// rehacen.
    generacion: AtomicU64,
}

/// Lo que necesita un render, ya resuelto.
pub struct Instantanea {
    pub simbolos: Arc<Simbolos>,
    pub pack: Arc<Pack>,
    /// Huella de letras + pack + símbolos (entra en la huella del render).
    pub huella: String,
    pub generacion: u64,
}

fn leer_letras(dir: &Path) -> Vec<(LetraInfo, Arc<Vec<u8>>)> {
    let mut v = Vec::new();
    let Ok(entradas) = std::fs::read_dir(dir.join("fuentes")) else {
        return v;
    };
    let mut rutas: Vec<PathBuf> = entradas.flatten().map(|e| e.path()).collect();
    rutas.sort();
    for p in rutas {
        let Ok(datos) = std::fs::read(&p) else {
            continue;
        };
        let mut l = Letras::nuevas();
        if let Ok(familias) = l.cargar(datos.clone()) {
            v.push((
                LetraInfo {
                    id: huella(&datos),
                    familias,
                    bytes: datos.len() as u64,
                },
                Arc::new(datos),
            ));
        }
    }
    v
}

/// Lee los ficheros de texto de una carpeta de pack (ruta relativa → texto).
fn leer_carpeta(base: &Path) -> AppResult<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    let mut pila = vec![base.to_path_buf()];
    while let Some(d) = pila.pop() {
        for e in std::fs::read_dir(&d)?.flatten() {
            let p = e.path();
            if p.is_dir() {
                pila.push(p);
            } else if p
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| matches!(x, "json" | "svg"))
            {
                let rel = p
                    .strip_prefix(base)
                    .map_err(|_| AppError::new(ErrorCode::Io))?
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(rel, std::fs::read_to_string(&p)?);
            }
        }
    }
    Ok(out)
}

fn leer_packs(dir: &Path) -> BTreeMap<String, Arc<Pack>> {
    let mut m = BTreeMap::new();
    if let Ok(p) = Pack::de_fabrica() {
        m.insert("builtin".to_string(), Arc::new(p));
    }
    if let Ok(entradas) = std::fs::read_dir(dir.join("marcos")) {
        for e in entradas.flatten() {
            if let Ok(f) = leer_carpeta(&e.path()) {
                if let Ok(p) = Pack::cargar(f) {
                    m.insert(p.def.id.clone(), Arc::new(p));
                }
            }
        }
    }
    m
}

fn leer_simbolos(dir: &Path) -> (Simbolos, String, Option<i64>) {
    let mut s = Simbolos::nuevos();
    let base = dir.join("simbolos");
    let indice: Option<Indice> = std::fs::read(base.join("indice.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    let Some(indice) = indice else {
        return (s, "sin-simbolos".into(), None);
    };
    let mut h = Sha256::new();
    for (codigo, fichero) in &indice.simbolos {
        if let Ok(svg) = std::fs::read(base.join(fichero)) {
            if s.cargar(codigo, &svg).is_ok() {
                h.update(codigo.as_bytes());
                h.update(&svg);
            }
        }
    }
    let hu = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    (s, hu, Some(indice.fecha))
}

fn nombre_fichero(codigo: &str) -> String {
    let limpio: String = codigo
        .trim_matches(|c| c == '{' || c == '}')
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{limpio}-{}.svg", &huella(codigo.as_bytes())[..8])
}

fn de_pack(e: PackError) -> AppError {
    let a = AppError::new(ErrorCode::InvalidInput).with("motivo", "pack_invalido");
    match e {
        PackError::FaltaFichero(f) => a.with("falta", f),
        PackError::FaltaHueco { estilo, hueco } => a.with("estilo", estilo).with("hueco", hueco),
        PackError::FaltaEstilo(s) => a.with("estilo", s),
        PackError::Color(c) => a.with("color", c),
        PackError::Version(v) => a.with("version", v),
        PackError::Json(j) => a.with("detalle", j),
        PackError::Expresion(x) => a.with("expresion", x),
    }
}

impl Recursos {
    /// Lee lo que haya en disco (sin red).
    pub fn abrir(datos: &Path) -> Recursos {
        let (simbolos, huella_simbolos, fecha) = leer_simbolos(datos);
        Recursos {
            dir: datos.to_path_buf(),
            estado: RwLock::new(Estado {
                simbolos: Arc::new(simbolos),
                huella_simbolos,
                fecha_simbolos: fecha,
                packs: leer_packs(datos),
                letras: leer_letras(datos),
            }),
            generacion: AtomicU64::new(1),
        }
    }

    fn leer(&self) -> AppResult<std::sync::RwLockReadGuard<'_, Estado>> {
        self.estado
            .read()
            .map_err(|_| AppError::new(ErrorCode::Internal).with("detalle", "recursos"))
    }

    fn escribir(&self) -> AppResult<std::sync::RwLockWriteGuard<'_, Estado>> {
        self.estado
            .write()
            .map_err(|_| AppError::new(ErrorCode::Internal).with("detalle", "recursos"))
    }

    /// Lo que necesita un render con el pack pedido (si no existe, el de la
    /// casa; quien llama avisa).
    pub fn instantanea(&self, pack: &str) -> AppResult<Instantanea> {
        let e = self.leer()?;
        let p = e
            .packs
            .get(pack)
            .or_else(|| e.packs.get("builtin"))
            .cloned()
            .ok_or_else(|| AppError::new(ErrorCode::Internal).with("detalle", "sin pack"))?;
        let letras: Vec<&str> = e.letras.iter().map(|(i, _)| i.id.as_str()).collect();
        Ok(Instantanea {
            simbolos: e.simbolos.clone(),
            huella: format!("{}|{}|{}", p.huella, e.huella_simbolos, letras.join(",")),
            pack: p,
            generacion: self.generacion.load(Ordering::SeqCst),
        })
    }

    // ── Símbolos ───────────────────────────────────────────────────────────

    pub fn simbolos_info(&self) -> AppResult<SimbolosInfo> {
        let e = self.leer()?;
        Ok(SimbolosInfo {
            cargados: e.simbolos.de_scryfall() as u32,
            fecha: e.fecha_simbolos,
        })
    }

    /// Descarga los símbolos si faltan o tienen más de 30 días. Sin red, se
    /// queda con lo que haya (no es un error).
    pub async fn asegurar_simbolos(&self, api: &Scryfall, forzar: bool) -> AppResult<()> {
        let frescos = {
            let e = self.leer()?;
            e.fecha_simbolos.is_some_and(|f| {
                let edad = crate::store::ahora_ms() - f;
                edad >= 0 && (edad as u128) < CADUCA_SIMBOLOS.as_millis()
            }) && e.simbolos.de_scryfall() > 0
        };
        if frescos && !forzar {
            return Ok(());
        }
        let lista = match api.symbology().await {
            Ok(l) if !l.is_empty() => l,
            Ok(_) => return Ok(()),
            Err(e) => {
                tracing::warn!(?e, "símbolos: sin red, se usan los que haya");
                return Ok(());
            }
        };
        let base = self.dir.join("simbolos");
        let mut indice = Indice {
            fecha: crate::store::ahora_ms(),
            simbolos: BTreeMap::new(),
        };
        for s in lista {
            let Some(url) = s.svg_uri else { continue };
            let fichero = nombre_fichero(&s.symbol);
            let ruta = base.join(&fichero);
            if !ruta.exists() || forzar {
                match api.download(&url).await {
                    Ok(b) => escribir_atomico(&ruta, &b)?,
                    Err(e) => {
                        tracing::warn!(?e, simbolo = s.symbol, "símbolo sin descargar");
                        continue;
                    }
                }
            }
            indice.simbolos.insert(s.symbol, fichero);
        }
        let json = serde_json::to_vec(&indice)
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
        escribir_atomico(&base.join("indice.json"), &json)?;
        let (s, h, f) = leer_simbolos(&self.dir);
        let mut e = self.escribir()?;
        e.simbolos = Arc::new(s);
        e.huella_simbolos = h;
        e.fecha_simbolos = f;
        Ok(())
    }

    // ── Letras ─────────────────────────────────────────────────────────────

    pub fn letras(&self) -> AppResult<Vec<LetraInfo>> {
        Ok(self.leer()?.letras.iter().map(|(i, _)| i.clone()).collect())
    }

    pub fn importar_letra(&self, ruta: &Path) -> AppResult<LetraInfo> {
        let datos = std::fs::read(ruta)?;
        let ext = ruta
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .filter(|e| matches!(e.as_str(), "ttf" | "otf"))
            .ok_or_else(|| AppError::new(ErrorCode::Unsupported))?;
        let mut l = Letras::nuevas();
        let familias = l
            .cargar(datos.clone())
            .map_err(|_| AppError::new(ErrorCode::Unsupported))?;
        let id = huella(&datos);
        escribir_atomico(
            &self.dir.join("fuentes").join(format!("{id}.{ext}")),
            &datos,
        )?;
        let info = LetraInfo {
            id: id.clone(),
            familias,
            bytes: datos.len() as u64,
        };
        let mut e = self.escribir()?;
        if !e.letras.iter().any(|(i, _)| i.id == id) {
            e.letras.push((info.clone(), Arc::new(datos)));
        }
        self.generacion.fetch_add(1, Ordering::SeqCst);
        Ok(info)
    }

    pub fn quitar_letra(&self, id: &str) -> AppResult<()> {
        if let Ok(entradas) = std::fs::read_dir(self.dir.join("fuentes")) {
            for e in entradas.flatten() {
                if e.file_name().to_string_lossy().starts_with(id) {
                    std::fs::remove_file(e.path())?;
                }
            }
        }
        self.escribir()?.letras.retain(|(i, _)| i.id != id);
        self.generacion.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    // ── Packs ──────────────────────────────────────────────────────────────

    pub fn packs(&self) -> AppResult<Vec<PackInfo>> {
        Ok(self
            .leer()?
            .packs
            .values()
            .map(|p| PackInfo {
                id: p.def.id.clone(),
                nombre: p.def.nombre.clone(),
                importado: p.def.id != "builtin",
                estilos: p.def.estilos.keys().cloned().collect(),
            })
            .collect())
    }

    /// Importa un pack desde una carpeta o un `.zip`.
    pub fn importar_pack(&self, ruta: &Path) -> AppResult<PackInfo> {
        let ficheros = if ruta.is_dir() {
            leer_carpeta(ruta)?
        } else {
            leer_zip(ruta)?
        };
        let p = Pack::cargar(ficheros.clone()).map_err(de_pack)?;
        let id: String = p
            .def
            .id
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if id.is_empty() || id == "builtin" {
            return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "pack_id"));
        }
        let destino = self.dir.join("marcos").join(&id);
        if destino.exists() {
            std::fs::remove_dir_all(&destino)?;
        }
        for (rel, texto) in &ficheros {
            if rel.contains("..") {
                continue;
            }
            escribir_atomico(&destino.join(rel), texto.as_bytes())?;
        }
        let info = PackInfo {
            id: id.clone(),
            nombre: p.def.nombre.clone(),
            importado: true,
            estilos: p.def.estilos.keys().cloned().collect(),
        };
        self.escribir()?.packs.insert(id, Arc::new(p));
        Ok(info)
    }

    pub fn quitar_pack(&self, id: &str) -> AppResult<()> {
        if id == "builtin" || id.contains(['/', '\\', '.']) {
            return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "pack_id"));
        }
        let d = self.dir.join("marcos").join(id);
        if d.exists() {
            std::fs::remove_dir_all(d)?;
        }
        self.escribir()?.packs.remove(id);
        Ok(())
    }

    /// Ejecuta `f` con el motor de este hilo, rehecho si cambiaron las letras.
    pub fn con_motor<R>(&self, f: impl FnOnce(&mut Motor) -> R) -> AppResult<R> {
        let generacion = self.generacion.load(Ordering::SeqCst);
        let letras: Vec<Arc<Vec<u8>>> =
            self.leer()?.letras.iter().map(|(_, d)| d.clone()).collect();
        MOTOR.with(|celda| {
            let mut c = celda.borrow_mut();
            let rehacer = c.as_ref().is_none_or(|(g, _)| *g != generacion);
            if rehacer {
                let mut m = Motor::de_fabrica()
                    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
                for d in &letras {
                    let _ = m.letras.cargar(d.as_ref().clone());
                }
                *c = Some((generacion, m));
            }
            match c.as_mut() {
                Some((_, m)) => Ok(f(m)),
                None => Err(AppError::new(ErrorCode::Internal)),
            }
        })
    }
}

thread_local! {
    static MOTOR: RefCell<Option<(u64, Motor)>> = const { RefCell::new(None) };
}

fn leer_zip(ruta: &Path) -> AppResult<BTreeMap<String, String>> {
    use std::io::Read;
    let f = std::fs::File::open(ruta)?;
    let mut z = zip::ZipArchive::new(f).map_err(|_| AppError::new(ErrorCode::Unsupported))?;
    let mut out = BTreeMap::new();
    // Si todo va dentro de una carpeta, se quita ese prefijo.
    let mut nombres = Vec::new();
    for i in 0..z.len() {
        let mut e = z
            .by_index(i)
            .map_err(|_| AppError::new(ErrorCode::Unsupported))?;
        if e.is_dir() {
            continue;
        }
        let n = e
            .name()
            .map_err(|_| AppError::new(ErrorCode::Unsupported))?
            .replace('\\', "/");
        if !(n.ends_with(".json") || n.ends_with(".svg")) || n.contains("..") {
            continue;
        }
        if e.size() > 8 * 1024 * 1024 {
            return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "pack_grande"));
        }
        let mut s = String::new();
        e.read_to_string(&mut s)
            .map_err(|_| AppError::new(ErrorCode::Unsupported))?;
        nombres.push(n.clone());
        out.insert(n, s);
    }
    let prefijo = nombres
        .iter()
        .find(|n| n.ends_with("pack.json"))
        .map(|n| n.trim_end_matches("pack.json").to_string())
        .unwrap_or_default();
    Ok(out
        .into_iter()
        .filter_map(|(k, v)| k.strip_prefix(&prefijo).map(|k| (k.to_string(), v)))
        .collect())
}

/// Las imágenes del almacén para el render, con una caché pequeña (las
/// ilustraciones pesan: se guardan las últimas 12).
pub struct ImagenesDelAlmacen<'a> {
    pub db: &'a Db,
    pub blobs: &'a BlobStore,
    pub cache: &'a CacheImagenes,
}

type Decodificadas = HashMap<String, Arc<RgbaImage>>;

#[derive(Default)]
pub struct CacheImagenes {
    m: Mutex<(Decodificadas, VecDeque<String>)>,
}

const MAX_CACHE: usize = 12;

impl Imagenes for ImagenesDelAlmacen<'_> {
    fn imagen(&self, blob: &str) -> Option<Arc<RgbaImage>> {
        if let Ok(g) = self.cache.m.lock() {
            if let Some(i) = g.0.get(blob) {
                return Some(i.clone());
            }
        }
        let (ruta, _) = self.blobs.locate(self.db, blob).ok()??;
        let bytes = std::fs::read(ruta).ok()?;
        let img = Arc::new(proxy_core::imaging::decode::decode(&bytes).ok()?);
        if let Ok(mut g) = self.cache.m.lock() {
            g.0.insert(blob.to_string(), img.clone());
            g.1.push_back(blob.to_string());
            while g.1.len() > MAX_CACHE {
                if let Some(v) = g.1.pop_front() {
                    g.0.remove(&v);
                }
            }
        }
        Some(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letras_y_packs_importados() {
        let dir = tempfile::tempdir().unwrap();
        let r = Recursos::abrir(dir.path());
        assert_eq!(r.packs().unwrap().len(), 1);
        assert_eq!(r.simbolos_info().unwrap().cargados, 0);

        // Una letra OFL de la casa como si fuera importada.
        let ttf = dir.path().join("prueba.ttf");
        std::fs::write(
            &ttf,
            include_bytes!("../../crates/proxy-core/assets/fuentes/Caladea-Regular.ttf"),
        )
        .unwrap();
        let g0 = r.instantanea("builtin").unwrap().generacion;
        let info = r.importar_letra(&ttf).unwrap();
        assert_eq!(info.familias, vec!["Caladea".to_string()]);
        assert!(r.instantanea("builtin").unwrap().generacion > g0);
        assert!(r.con_motor(|m| m.letras.tiene("Caladea")).unwrap());
        // Un texto con extensión de letra no vale.
        let falsa = dir.path().join("falsa.ttf");
        std::fs::write(&falsa, b"hola").unwrap();
        assert_eq!(
            r.importar_letra(&falsa).unwrap_err().code,
            ErrorCode::Unsupported
        );
        // Sobrevive a reabrir.
        let r2 = Recursos::abrir(dir.path());
        assert_eq!(r2.letras().unwrap().len(), 1);
        r2.quitar_letra(&info.id).unwrap();
        assert!(Recursos::abrir(dir.path()).letras().unwrap().is_empty());

        // Un pack: el de la casa con otro id, desde una carpeta.
        let carpeta = dir.path().join("mipack");
        let casa = Pack::de_fabrica().unwrap();
        for (k, v) in &casa.ficheros {
            let v = if k == "pack.json" {
                v.replacen("\"id\": \"builtin\"", "\"id\": \"mio\"", 1)
            } else {
                v.clone()
            };
            let p = carpeta.join(k);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, v).unwrap();
        }
        let p = r.importar_pack(&carpeta).unwrap();
        assert_eq!(p.id, "mio");
        assert_eq!(Recursos::abrir(dir.path()).packs().unwrap().len(), 2);
        assert!(r.instantanea("mio").unwrap().huella != r.instantanea("builtin").unwrap().huella);
        r.quitar_pack("mio").unwrap();
        assert_eq!(
            r.quitar_pack("builtin").unwrap_err().code,
            ErrorCode::InvalidInput
        );
        assert_eq!(Recursos::abrir(dir.path()).packs().unwrap().len(), 1);
    }
}
