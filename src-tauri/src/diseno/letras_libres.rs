//! **Letras libres para descargar**: un catálogo de letras OFL de Google
//! Fonts (de una versión fija de su repositorio, con la huella SHA-256 de
//! cada fichero) que se bajan desde la app con un botón. No van en el
//! repositorio ni en los instaladores. Al bajarlas se importan como cualquier
//! letra propia (`fuentes/<huella>.ttf`) y su licencia se guarda al lado,
//! en `fuentes/licencias/`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;

const CATALOGO: &str = include_str!("../../assets/letras-libres.json");
const REPO: &str = "https://raw.githubusercontent.com/google/fonts";

/// Para qué va mejor una letra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UsoLetra {
    Titulo,
    Reglas,
    Ambos,
}

#[derive(Debug, Clone, Deserialize)]
struct Fichero {
    ruta: String,
    sha256: String,
    bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct Familia {
    id: String,
    familia: String,
    uso: UsoLetra,
    cursiva: bool,
    ficheros: Vec<Fichero>,
    licencia: Fichero2,
}

#[derive(Debug, Clone, Deserialize)]
struct Fichero2 {
    ruta: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalogo {
    commit: String,
    base: String,
    familias: Vec<Familia>,
}

fn catalogo() -> AppResult<Catalogo> {
    serde_json::from_str(CATALOGO)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

/// Una letra del catálogo y si ya está.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LetraLibre {
    pub id: String,
    /// El nombre de familia con el que se elige en colecciones y packs.
    pub familia: String,
    pub uso: UsoLetra,
    /// Trae cursiva (para el texto de recordatorio y el de ambientación).
    pub cursiva: bool,
    pub descargada: bool,
    #[ts(type = "number")]
    pub bytes: u64,
}

fn huella(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

pub fn lista(state: &AppState) -> AppResult<Vec<LetraLibre>> {
    let tengo: HashSet<String> = state.diseno.letras()?.into_iter().map(|l| l.id).collect();
    Ok(catalogo()?
        .familias
        .into_iter()
        .map(|f| LetraLibre {
            descargada: f.ficheros.iter().all(|x| tengo.contains(&x.sha256)),
            bytes: f.ficheros.iter().map(|x| x.bytes).sum(),
            id: f.id,
            familia: f.familia,
            uso: f.uso,
            cursiva: f.cursiva,
        })
        .collect())
}

/// Baja (si falta) e importa cada fichero de la familia `id`.
pub async fn descargar(state: &AppState, id: &str) -> AppResult<LetraLibre> {
    let c = catalogo()?;
    let f = c
        .familias
        .iter()
        .find(|f| f.id == id)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("letra", id))?;
    let tengo: HashSet<String> = state.diseno.letras()?.into_iter().map(|l| l.id).collect();
    let http = reqwest::Client::builder()
        .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(crate::error::red)?;
    let bajar = |ruta: String, sha: String| {
        let http = http.clone();
        let url = format!("{REPO}/{}/{}{ruta}", c.commit, c.base);
        async move {
            let r = http.get(&url).send().await.map_err(crate::error::red)?;
            tracing::info!(
                host = "raw.githubusercontent.com",
                fichero = ruta.as_str(),
                estado = r.status().as_u16(),
                "red"
            );
            if !r.status().is_success() {
                return Err(crate::error::estado_http(r.status()));
            }
            let b = r.bytes().await.map_err(crate::error::red)?;
            // Lo que llega tiene que ser justo lo esperado.
            if huella(&b) != sha {
                return Err(AppError::new(ErrorCode::Network).with("fichero", ruta));
            }
            Ok::<Vec<u8>, AppError>(b.to_vec())
        }
    };
    for x in &f.ficheros {
        if tengo.contains(&x.sha256) {
            continue;
        }
        let datos = bajar(x.ruta.clone(), x.sha256.clone()).await?;
        state.diseno.importar_letra_datos(datos, "ttf")?;
    }
    let licencia = bajar(f.licencia.ruta.clone(), f.licencia.sha256.clone()).await?;
    escribir_atomico(
        &state
            .diseno
            .dir()
            .join("fuentes")
            .join("licencias")
            .join(format!("{}-OFL.txt", f.id)),
        &licencia,
    )?;
    lista(state)?
        .into_iter()
        .find(|l| l.id == id)
        .ok_or_else(|| AppError::new(ErrorCode::Internal))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxy_core::diseno::letras::Letras;

    #[test]
    fn el_catalogo_esta_bien_escrito() {
        let c = catalogo().unwrap();
        assert_eq!(c.commit.len(), 40);
        assert!(c.familias.len() >= 10);
        let mut ids = HashSet::new();
        for f in &c.familias {
            assert!(ids.insert(f.id.clone()), "{} repetida", f.id);
            assert!(!f.ficheros.is_empty());
            for x in &f.ficheros {
                assert!(x.ruta.starts_with(&format!("{}/", f.id)), "{}", x.ruta);
                assert!(x.ruta.ends_with(".ttf") && !x.ruta.contains(".."));
                assert_eq!(x.sha256.len(), 64);
            }
            assert!(f.licencia.ruta.ends_with("OFL.txt"));
        }
    }

    #[test]
    fn sin_descargar_ninguna_esta() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(crate::state::Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let l = lista(&s).unwrap();
        assert!(!l.is_empty());
        assert!(l.iter().all(|x| !x.descargada && x.bytes > 0));
    }

    /// Que cada fichero (de una copia local del repositorio de Google Fonts,
    /// `PB_GF=<…/ofl>`) tenga la huella apuntada y se llame como dice el
    /// catálogo: `cargo test --lib letras_libres -- --ignored`.
    #[test]
    #[ignore]
    fn los_ficheros_son_los_del_catalogo() {
        let base = std::path::PathBuf::from(std::env::var("PB_GF").unwrap());
        for f in catalogo().unwrap().familias {
            for x in &f.ficheros {
                let b = std::fs::read(base.join(&x.ruta)).unwrap();
                assert_eq!(huella(&b), x.sha256, "{}", x.ruta);
                let familias = Letras::nuevas().cargar(b).unwrap();
                assert!(familias.contains(&f.familia), "{}: {familias:?}", x.ruta);
            }
        }
    }

    /// Descarga de verdad una familia y la deja importada.
    #[tokio::test]
    #[ignore]
    async fn en_vivo() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(crate::state::Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let l = descargar(&s, "domine").await.unwrap();
        assert!(l.descargada);
        assert!(s
            .diseno
            .letras()
            .unwrap()
            .iter()
            .any(|x| x.familias.contains(&"Domine".to_string())));
        assert!(dir
            .path()
            .join("data/fuentes/licencias/domine-OFL.txt")
            .exists());
    }
}
