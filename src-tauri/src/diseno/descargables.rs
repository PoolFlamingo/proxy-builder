//! **Packs de marcos descargables** con la definición en el repositorio de
//! packs (`PoolFlamingo/proxy-builder-packs`): el de Balatro. Como los marcos
//! Original (`original.rs`), no van en la app ni en los instaladores: se bajan
//! de un commit fijo y se comprueba el SHA-256 de cada fichero. A diferencia de
//! Original, todo el pack (también `pack.json`) viene del repositorio, junto
//! con sus letras (que se importan como letras propias, con su licencia) y su
//! contenido extra (reversos, fichas, cartas hechas), que va aparte en
//! `app_data_dir/packs-extras/<id>/` para no cargarlo con el pack.
//!
//! El manifiesto (`assets/packs-descargables.json`) lo genera
//! `fuente/manifiesto.py` del repositorio de packs.
//!
//! En desarrollo, `PB_PACKS_LOCAL=<carpeta del repositorio>` lee los ficheros
//! de esa carpeta en vez de la red (comprobando igual cada huella).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use super::recursos::PackInfo;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;

const MANIFIESTO: &str = include_str!("../../assets/packs-descargables.json");

#[derive(Debug, Clone, Deserialize)]
struct Manifiesto {
    repo: String,
    packs: Vec<Descargable>,
}

#[derive(Debug, Clone, Deserialize)]
struct Descargable {
    id: String,
    nombre: String,
    commit: String,
    /// La carpeta del pack dentro del repositorio (`balatro/`).
    carpeta: String,
    /// Los ficheros del pack (ruta dentro de la carpeta → SHA-256).
    ficheros: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    letras: Vec<LetraDescargable>,
    #[serde(default)]
    extras: Vec<Extra>,
}

#[derive(Debug, Clone, Deserialize)]
struct LetraDescargable {
    ruta: String,
    sha256: String,
    licencia: String,
    licencia_sha256: String,
}

/// Una imagen extra de un pack (reverso, ficha, tierra, carta hecha).
#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Extra {
    pub ruta: String,
    #[serde(skip_serializing)]
    #[ts(skip)]
    pub sha256: String,
    pub tipo: TipoExtra,
    pub nombre: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TipoExtra {
    Reverso,
    Ficha,
    Tierra,
    Carta,
}

/// Cómo está un pack descargable.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DescargableInfo {
    pub id: String,
    pub nombre: String,
    /// Bajado entero y cargado.
    pub listo: bool,
    pub faltan: u32,
    pub total: u32,
    /// Lo que pesa todo (bytes, aproximado).
    #[ts(type = "number")]
    pub bytes: u64,
    /// Lo extra que trae (reversos, fichas…), si está bajado.
    pub extras: Vec<Extra>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DescargaProgressEvent {
    pub id: String,
    pub done: u32,
    pub total: u32,
}

fn manifiesto() -> AppResult<Manifiesto> {
    serde_json::from_str(MANIFIESTO)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

fn uno(id: &str) -> AppResult<(Manifiesto, Descargable)> {
    let m = manifiesto()?;
    let d = m
        .packs
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("pack", id))?;
    Ok((m, d))
}

fn huella(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn bueno(ruta: &Path, sha: &str) -> bool {
    std::fs::read(ruta).is_ok_and(|b| huella(&b) == sha)
}

fn dir_pack(datos: &Path, id: &str) -> PathBuf {
    datos.join("marcos").join(id)
}

fn dir_extras(datos: &Path, id: &str) -> PathBuf {
    datos.join("packs-extras").join(id)
}

/// Lo que hay que bajar: (ruta en el repositorio, destino, huella).
fn tareas(datos: &Path, d: &Descargable) -> Vec<(String, PathBuf, String)> {
    let mut v: Vec<(String, PathBuf, String)> = d
        .ficheros
        .iter()
        .map(|(r, sha)| (r.clone(), dir_pack(datos, &d.id).join(r), sha.clone()))
        .collect();
    v.extend(d.extras.iter().map(|e| {
        (
            e.ruta.clone(),
            dir_extras(datos, &d.id).join(&e.ruta),
            e.sha256.clone(),
        )
    }));
    v
}

/// Quita de las carpetas del pack lo que su manifiesto ya no tiene (lo de
/// una versión anterior con otras rutas, un `.part` que quedó a medias).
/// Esas carpetas solo las escribe la descarga.
fn quitar_lo_que_sobra(datos: &Path, d: &Descargable) {
    let queda: std::collections::HashSet<PathBuf> = tareas(datos, d)
        .into_iter()
        .map(|(_, ruta, _)| ruta)
        .collect();
    for raiz in [dir_pack(datos, &d.id), dir_extras(datos, &d.id)] {
        quitar_de(&raiz, &queda);
    }
}

/// Borra lo que no esté en `queda` bajo `dir` (y las carpetas que se queden
/// vacías). Devuelve si `dir` se ha quedado vacía.
fn quitar_de(dir: &Path, queda: &std::collections::HashSet<PathBuf>) -> bool {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut vacia = true;
    for e in entradas.flatten() {
        let p = e.path();
        let es_carpeta = e.file_type().is_ok_and(|t| t.is_dir());
        if es_carpeta {
            if quitar_de(&p, queda) {
                let _ = std::fs::remove_dir(&p);
            } else {
                vacia = false;
            }
        } else if queda.contains(&p) || std::fs::remove_file(&p).is_err() {
            // Lo que sigue en el manifiesto (o no se ha podido borrar) se queda.
            vacia = false;
        }
    }
    vacia
}

fn letras_que_faltan(state: &AppState, d: &Descargable) -> AppResult<Vec<LetraDescargable>> {
    let tengo: std::collections::HashSet<String> =
        state.diseno.letras()?.into_iter().map(|l| l.id).collect();
    Ok(d.letras
        .iter()
        .filter(|l| !tengo.contains(&l.sha256))
        .cloned()
        .collect())
}

/// ¿Tiene un commit de verdad? Uno pendiente (el repositorio aún no está
/// publicado) solo vale en desarrollo, con `PB_PACKS_LOCAL`.
fn publicado(d: &Descargable) -> bool {
    d.commit.len() == 40 && d.commit.chars().all(|c| c.is_ascii_hexdigit())
}

fn disponible(d: &Descargable) -> bool {
    publicado(d) || (cfg!(debug_assertions) && std::env::var_os("PB_PACKS_LOCAL").is_some())
}

/// Los packs descargables y cómo están.
pub fn lista(state: &AppState) -> AppResult<Vec<DescargableInfo>> {
    let mut m = manifiesto()?;
    m.packs.retain(disponible);
    let datos = state.diseno.dir().to_path_buf();
    let cargados: Vec<String> = state.diseno.packs()?.into_iter().map(|p| p.id).collect();
    m.packs
        .iter()
        .map(|d| {
            let t = tareas(&datos, d);
            let faltan = t.iter().filter(|(_, ruta, sha)| !bueno(ruta, sha)).count()
                + letras_que_faltan(state, d)?.len();
            let total = t.len() + d.letras.len();
            let listo = faltan == 0 && cargados.contains(&d.id);
            let bytes = t
                .iter()
                .filter_map(|(_, ruta, _)| std::fs::metadata(ruta).ok())
                .map(|m| m.len())
                .sum();
            Ok(DescargableInfo {
                id: d.id.clone(),
                nombre: d.nombre.clone(),
                listo,
                faltan: faltan as u32,
                total: total as u32,
                bytes,
                extras: if listo { d.extras.clone() } else { Vec::new() },
            })
        })
        .collect()
}

/// Un fichero del repositorio de packs: de la red (lista cerrada) o, en
/// desarrollo con `PB_PACKS_LOCAL`, de esa carpeta.
async fn traer(
    state: &AppState,
    m: &Manifiesto,
    d: &Descargable,
    ruta: &str,
    sha: &str,
) -> AppResult<Vec<u8>> {
    #[cfg(debug_assertions)]
    if let Ok(local) = std::env::var("PB_PACKS_LOCAL") {
        let b = std::fs::read(Path::new(&local).join(&d.carpeta).join(ruta))?;
        return comprobar(b, ruta, sha);
    }
    let url = format!("{}/{}/{}{}", m.repo, d.commit, d.carpeta, ruta);
    let b = state
        .externo
        .bajar(&url, crate::fuentes::MAX_IMAGEN)
        .await?;
    comprobar(b, ruta, sha)
}

fn comprobar(b: Vec<u8>, ruta: &str, sha: &str) -> AppResult<Vec<u8>> {
    // Lo que llega tiene que ser justo lo esperado.
    if huella(&b) != sha {
        return Err(AppError::new(ErrorCode::Network).with("fichero", ruta));
    }
    Ok(b)
}

/// Baja lo que falte del pack `id`, importa sus letras y lo carga.
/// `progreso(hechas, total)` tras cada fichero.
pub async fn descargar(
    state: &AppState,
    id: &str,
    progreso: &(dyn Fn(u32, u32) + Sync),
) -> AppResult<PackInfo> {
    let (m, d) = uno(id)?;
    if !disponible(&d) {
        return Err(AppError::new(ErrorCode::NotFound).with("pack", id));
    }
    let datos = state.diseno.dir().to_path_buf();
    let pendientes: Vec<_> = tareas(&datos, &d)
        .into_iter()
        .filter(|(_, ruta, sha)| !bueno(ruta, sha))
        .collect();
    let letras = letras_que_faltan(state, &d)?;
    let total = (pendientes.len() + letras.len()) as u32;
    let mut hechas = 0;
    progreso(hechas, total);
    for (ruta, destino, sha) in pendientes {
        let b = traer(state, &m, &d, &ruta, &sha).await?;
        escribir_atomico(&destino, &b)?;
        hechas += 1;
        progreso(hechas, total);
    }
    for l in letras {
        let b = traer(state, &m, &d, &l.ruta, &l.sha256).await?;
        let ext = if l.ruta.ends_with(".otf") {
            "otf"
        } else {
            "ttf"
        };
        state.diseno.importar_letra_datos(b, ext)?;
        let licencia = traer(state, &m, &d, &l.licencia, &l.licencia_sha256).await?;
        let nombre = Path::new(&l.ruta)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| id.to_string());
        escribir_atomico(
            &datos
                .join("fuentes")
                .join("licencias")
                .join(format!("{nombre}-LICENCIA.txt")),
            &licencia,
        )?;
        hechas += 1;
        progreso(hechas, total);
    }
    quitar_lo_que_sobra(&datos, &d);
    state.diseno.recargar_pack(id)
}

/// Los bytes del extra número `indice` del pack (para enseñarlo), si está
/// bajado y su huella cuadra.
pub fn extra_por_indice(state: &AppState, id: &str, indice: usize) -> AppResult<Vec<u8>> {
    let (_, d) = uno(id)?;
    let e = d
        .extras
        .get(indice)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("extra", indice))?;
    let b = std::fs::read(dir_extras(state.diseno.dir(), id).join(&e.ruta))?;
    if huella(&b) != e.sha256 {
        return Err(AppError::new(ErrorCode::NotFound).with("extra", indice));
    }
    Ok(b)
}

/// Un extra bajado, al almacén de imágenes (para ponerlo de reverso o
/// añadirlo como carta propia).
pub fn usar_extra(
    state: &AppState,
    id: &str,
    ruta: &str,
) -> AppResult<proxy_core::model::FaceImage> {
    let p = ruta_extra(state, id, ruta)?;
    let b = std::fs::read(p)?;
    state
        .blobs
        .put_bytes(&state.db, &b, proxy_core::model::ImageOrigin::Upload)
}

/// La ruta en disco de un extra bajado (para usarlo en el deck).
pub fn ruta_extra(state: &AppState, id: &str, ruta: &str) -> AppResult<PathBuf> {
    let (_, d) = uno(id)?;
    let e = d
        .extras
        .iter()
        .find(|e| e.ruta == ruta)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("extra", ruta))?;
    let p = dir_extras(state.diseno.dir(), id).join(&e.ruta);
    if !bueno(&p, &e.sha256) {
        return Err(AppError::new(ErrorCode::NotFound).with("extra", ruta));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_manifiesto_se_entiende() {
        let m = manifiesto().unwrap();
        assert!(m.repo.starts_with("https://raw.githubusercontent.com/"));
        for d in &m.packs {
            assert!(d.ficheros.contains_key("pack.json"), "{}", d.id);
            for sha in d.ficheros.values() {
                assert_eq!(sha.len(), 64);
            }
            // La URL de cada fichero pasa la lista cerrada.
            let url = format!("{}/{}/{}pack.json", m.repo, d.commit, d.carpeta);
            assert!(crate::fuentes::url_permitida(&url).is_ok(), "{url}");
        }
    }

    #[test]
    fn una_huella_que_no_cuadra_no_vale() {
        let e = comprobar(b"hola".to_vec(), "x.png", &"0".repeat(64)).unwrap_err();
        assert_eq!(e.code, ErrorCode::Network);
        assert!(comprobar(b"hola".to_vec(), "x.png", &huella(b"hola")).is_ok());
    }

    #[test]
    fn lo_de_una_version_anterior_se_quita() {
        let dir = tempfile::tempdir().unwrap();
        let datos = dir.path();
        let (_, mut d) = uno("balatro").unwrap();
        d.ficheros = [("pack.json".to_string(), "x".to_string())].into();
        d.extras.truncate(1);
        let pack = dir_pack(datos, "balatro");
        let extras = dir_extras(datos, "balatro");
        let extra = extras.join(&d.extras[0].ruta);
        for f in [
            pack.join("pack.json"),
            pack.join("capas/viejo.png"),
            pack.join("layers/joker/a.png.part"),
            extra.clone(),
            extras.join("extras/reversos/viejo.png"),
        ] {
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(&f, b"x").unwrap();
        }
        quitar_lo_que_sobra(datos, &d);
        assert!(pack.join("pack.json").exists());
        assert!(extra.exists());
        assert!(!pack.join("capas").exists());
        assert!(!pack.join("layers").exists());
        assert!(!extras.join("extras/reversos").exists());
    }

    /// Baja de verdad el Balatro del repositorio de packs, con sus letras y
    /// extras, y comprueba que carga con sus estilos propios:
    /// `cargo test --lib descargables::tests::balatro_en_vivo -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn balatro_en_vivo() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(crate::state::Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let info = descargar(&s, "balatro", &|hechas, total| {
            if hechas % 50 == 0 || hechas == total {
                println!("{hechas}/{total}");
            }
        })
        .await
        .unwrap();
        assert_eq!(info.estilos_propios.len(), 7, "{:?}", info.estilos_propios);
        // Una carta con un estilo de serie se pinta con un marco del pack,
        // nunca con el de la casa.
        assert_eq!(info.sustitutos["classic"], "joker");
        assert_eq!(info.sustitutos["borderless"], "joker");
        assert_eq!(info.sustitutos["fullArt"], "tierra");
        let (_, d) = uno("balatro").unwrap();
        for e in &d.extras {
            assert!(ruta_extra(&s, "balatro", &e.ruta).is_ok(), "{}", e.ruta);
        }
        assert!(s
            .diseno
            .letras()
            .unwrap()
            .iter()
            .any(|l| l.familias.iter().any(|f| f.contains("m6x11"))));
        // Una segunda vez no baja nada.
        let total = std::sync::atomic::AtomicU32::new(u32::MAX);
        descargar(&s, "balatro", &|_, t| {
            total.store(t, std::sync::atomic::Ordering::Relaxed)
        })
        .await
        .unwrap();
        assert_eq!(total.into_inner(), 0);
    }
}
