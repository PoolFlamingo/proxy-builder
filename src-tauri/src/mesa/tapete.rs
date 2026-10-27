//! **El tapete de la mesa**: una imagen de fondo por deck (o una general de
//! reserva), gratis y del usuario. Va al almacén de blobs como cualquier
//! imagen subida y se protege del borrado en `blob_refs` con
//! `owner_kind = 'mesa'` (no `deck`: `store/decks.rs` reescribe las
//! referencias `deck` de un deck cada vez que se guarda). Sus ajustes de
//! pantalla (velo y desenfoque) van con él en `app_settings`
//! (`mesa.tapete.<deck>` y `mesa.tapete`).

use mesa::modelo::RefImagen;
use proxy_core::model::ImageOrigin;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::imagenes;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::state::AppState;
use crate::store::settings;

/// El ancho máximo de un tapete preparado.
pub const ANCHO_TAPETE: u32 = 1920;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Tapete {
    pub blob: String,
    /// 0–90: cuánto se oscurece para que se lean las cartas.
    pub velo: u8,
    /// 0–20 px.
    pub desenfoque: u8,
}

/// De dónde sale un tapete nuevo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum OrigenTapete {
    /// Una imagen del equipo.
    Fichero { ruta: String },
    /// La ilustración (`art_crop`) de una carta de Scryfall.
    #[serde(rename_all = "camelCase")]
    Scryfall { scryfall_id: String },
    /// Una imagen que ya está en el almacén.
    Blob { blob: String },
}

fn clave(deck: Option<&str>) -> String {
    match deck {
        Some(d) => format!("mesa.tapete.{d}"),
        None => "mesa.tapete".into(),
    }
}

fn dueno(deck: Option<&str>) -> &str {
    deck.unwrap_or("general")
}

/// El tapete propio de un deck (sin el general de reserva).
pub fn propio(state: &AppState, deck: Option<&str>) -> AppResult<Option<Tapete>> {
    Ok(settings::get(&state.db, &clave(deck))?.and_then(|j| serde_json::from_str(&j).ok()))
}

/// El tapete que sale con este deck: el suyo o el general.
pub fn de_deck(state: &AppState, deck: &str) -> AppResult<Option<Tapete>> {
    match propio(state, Some(deck))? {
        Some(t) => Ok(Some(t)),
        None => propio(state, None),
    }
}

pub fn guardar(state: &AppState, deck: Option<&str>, t: Option<&Tapete>) -> AppResult<()> {
    let c = state.db.conn()?;
    c.execute(
        "DELETE FROM blob_refs WHERE owner_kind = 'mesa' AND owner_id = ?1",
        [dueno(deck)],
    )?;
    match t {
        Some(t) => {
            c.execute(
                "INSERT OR IGNORE INTO blob_refs (owner_kind, owner_id, blob) VALUES ('mesa', ?1, ?2)",
                (dueno(deck), &t.blob),
            )?;
            let j = serde_json::to_string(t)
                .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
            // La conexión se suelta antes de `settings::set`, que la vuelve a pedir.
            drop(c);
            settings::set(&state.db, &clave(deck), &j)
        }
        None => {
            c.execute("DELETE FROM app_settings WHERE key = ?1", [clave(deck)])?;
            Ok(())
        }
    }
}

/// Mete la imagen elegida en el almacén y devuelve su huella.
pub async fn importar<A: CardApi>(
    state: &AppState,
    api: &A,
    origen: &OrigenTapete,
) -> AppResult<String> {
    match origen {
        OrigenTapete::Fichero { ruta } => {
            let imgs = state
                .blobs
                .import_files(&state.db, &[std::path::PathBuf::from(ruta)])?;
            imgs.into_iter()
                .next()
                .map(|i| i.blob)
                .ok_or_else(|| AppError::new(ErrorCode::Image))
        }
        OrigenTapete::Scryfall { scryfall_id } => {
            let carta = match crate::scryfall::cache::leer(&state.db, scryfall_id)? {
                Some((c, _)) => c,
                None => api
                    .card(scryfall_id)
                    .await?
                    .ok_or_else(|| AppError::new(ErrorCode::NotFound))?,
            };
            let url = carta
                .face_uris(0)
                .and_then(|u| u.art_crop.clone())
                .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_arte"))?;
            let bytes = api.download(&url).await?;
            Ok(state
                .blobs
                .put_bytes(&state.db, &bytes, ImageOrigin::Upload)?
                .blob)
        }
        OrigenTapete::Blob { blob } => {
            state.leer_blob(blob)?;
            Ok(blob.clone())
        }
    }
}

/// El tapete preparado para la mesa (reducido a `ANCHO_TAPETE`).
pub fn para_mesa(state: &AppState, blob: &str) -> AppResult<RefImagen> {
    let clave = format!("t-{blob}");
    if let Some(r) = imagenes::de_indice(state, &clave) {
        return Ok(r);
    }
    let (bytes, _) = state.leer_blob(blob)?;
    let img = proxy_core::imaging::decode::decode(&bytes)?;
    let img = if img.width() > ANCHO_TAPETE {
        let alto =
            (u64::from(img.height()) * u64::from(ANCHO_TAPETE) / u64::from(img.width())) as u32;
        image::imageops::resize(
            &img,
            ANCHO_TAPETE,
            alto.max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let r = imagenes::guardar_propia(state, &img)?;
    imagenes::al_indice(state, &clave, &r);
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;

    #[test]
    fn guardar_y_quitar_un_tapete() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let t = Tapete {
            blob: "a".repeat(64),
            velo: 40,
            desenfoque: 2,
        };
        // Antes se quedaba esperando la conexión que ya tenía cogida.
        guardar(&s, Some("d1"), Some(&t)).unwrap();
        assert_eq!(propio(&s, Some("d1")).unwrap(), Some(t.clone()));
        assert_eq!(de_deck(&s, "d1").unwrap(), Some(t));
        guardar(&s, Some("d1"), None).unwrap();
        assert_eq!(propio(&s, Some("d1")).unwrap(), None);
    }
}
