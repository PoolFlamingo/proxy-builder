//! **Marcos «Original»**: los marcos de imagen de Card Conjurer (M15 normal
//! y despejado, coronas y cajas de F/R por color). No van en la app ni en el
//! repositorio: se descargan la primera vez que se piden, de una versión fija
//! del repositorio público, y se comprueba la huella de cada fichero. Quedan
//! en `app_data_dir/marcos/original/` como un pack más.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proxy_core::diseno::pack::{ORIGINAL, ORIGINAL_IMAGENES};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use super::recursos::PackInfo;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;

/// El id del pack.
pub const ID: &str = "original";
const REPO: &str = "https://raw.githubusercontent.com/Investigamer/cardconjurer";

#[derive(Debug, Deserialize)]
struct Lista {
    commit: String,
    base: String,
    imagenes: BTreeMap<String, String>,
}

fn lista() -> AppResult<Lista> {
    serde_json::from_str(ORIGINAL_IMAGENES)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

/// Cómo están los marcos originales.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OriginalInfo {
    /// Están todos y el pack se ha cargado.
    pub listo: bool,
    pub faltan: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OriginalProgressEvent {
    pub done: u32,
    pub total: u32,
}

fn carpeta(datos: &Path) -> PathBuf {
    datos.join("marcos").join(ID)
}

/// ¿El fichero está y es el que tiene que ser?
fn bueno(ruta: &Path, sha: &str) -> bool {
    std::fs::read(ruta).is_ok_and(|b| huella(&b) == sha)
}

fn huella(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Las imágenes que aún no están bien en disco.
fn pendientes(datos: &Path) -> AppResult<Vec<(String, String)>> {
    let dir = carpeta(datos);
    Ok(lista()?
        .imagenes
        .into_iter()
        .filter(|(ruta, sha)| !bueno(&dir.join(ruta), sha))
        .collect())
}

pub fn info(state: &AppState) -> AppResult<OriginalInfo> {
    let total = lista()?.imagenes.len() as u32;
    let faltan = pendientes(state.diseno.dir())?.len() as u32;
    let cargado = state.diseno.packs()?.iter().any(|p| p.id == ID);
    Ok(OriginalInfo {
        listo: faltan == 0 && cargado,
        faltan,
        total,
    })
}

/// Escribe la definición del pack y descarga lo que falte. `progreso(hechas,
/// total)` tras cada imagen. Al terminar, el pack queda cargado.
pub async fn descargar(
    state: &AppState,
    progreso: &(dyn Fn(u32, u32) + Sync),
) -> AppResult<PackInfo> {
    let datos = state.diseno.dir().to_path_buf();
    let dir = carpeta(&datos);
    // La definición: siempre la de esta versión de la app.
    for (rel, texto) in ORIGINAL {
        escribir_atomico(&dir.join(rel), texto.as_bytes())?;
    }
    let l = lista()?;
    let total = l.imagenes.len() as u32;
    let faltan = pendientes(&datos)?;
    let mut hechas = total - faltan.len() as u32;
    progreso(hechas, total);
    let http = reqwest::Client::builder()
        .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
    for (ruta, sha) in faltan {
        let url = format!("{REPO}/{}/{}{ruta}", l.commit, l.base);
        let r = http
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
        tracing::info!(
            host = "raw.githubusercontent.com",
            fichero = ruta.as_str(),
            estado = r.status().as_u16(),
            "red"
        );
        if !r.status().is_success() {
            return Err(AppError::new(ErrorCode::Network).with("estado", r.status().as_u16()));
        }
        let b = r
            .bytes()
            .await
            .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
        // Lo que llega tiene que ser justo lo esperado.
        if huella(&b) != sha {
            return Err(AppError::new(ErrorCode::Network).with("fichero", ruta));
        }
        escribir_atomico(&dir.join(&ruta), &b)?;
        hechas += 1;
        progreso(hechas, total);
    }
    state.diseno.recargar_pack(ID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_lista_esta_bien_escrita() {
        let l = lista().unwrap();
        assert_eq!(l.commit.len(), 40);
        assert!(l.base.ends_with('/'));
        assert!(l.imagenes.len() > 40);
        for (ruta, sha) in &l.imagenes {
            assert!(ruta.ends_with(".png") && !ruta.contains(".."), "{ruta}");
            assert_eq!(sha.len(), 64, "{ruta}");
        }
    }

    #[test]
    fn un_fichero_con_otra_huella_no_vale() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("a.png");
        std::fs::write(&f, b"hola").unwrap();
        assert!(bueno(&f, &huella(b"hola")));
        assert!(!bueno(&f, &huella(b"adios")));
        assert!(!bueno(&dir.path().join("no.png"), &huella(b"hola")));
    }

    #[test]
    fn sin_descargar_faltan_todas() {
        let dir = tempfile::tempdir().unwrap();
        let n = lista().unwrap().imagenes.len();
        assert_eq!(pendientes(dir.path()).unwrap().len(), n);
    }

    /// Descarga de verdad y carga el pack: `cargo test --lib original::en_vivo
    /// -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn en_vivo() {
        use crate::state::Paths;
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let info = descargar(&s, &|h, t| println!("{h}/{t}")).await.unwrap();
        assert_eq!(info.id, ID);
        assert!(super::info(&s).unwrap().listo);
    }
}
