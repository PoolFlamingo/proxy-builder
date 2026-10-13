//! **Vincular una carta propia con la real de Scryfall**: identificarla (por
//! su nombre de fichero y, si se pide, por OCR del título) y traer los datos
//! de la elegida para la info de carta y las estadísticas.

use std::collections::HashSet;

use proxy_core::model::CardMeta;
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use super::cards::{resumir, CardSummary};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::ocr;
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;
use crate::scryfall::resolve::meta_de;
use crate::state::AppState;

/// Cuántos textos se prueban como mucho (pista + líneas del OCR).
const MAX_TEXTOS: usize = 4;
/// Cuántas cartas se proponen.
const MAX_CANDIDATAS: usize = 12;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardIdentify {
    /// Lo que se ha leído en el título (vacío sin OCR).
    pub lines: Vec<String>,
    /// Cartas propuestas, la más probable primero.
    pub candidates: Vec<CardSummary>,
}

/// ¿Están descargados los modelos del OCR? (Para avisar de la primera vez.)
#[tauri::command]
pub fn ocr_ready(state: State<'_, AppState>) -> bool {
    ocr::modelos_listos(&state.paths.data)
}

/// Busca un texto como nombre en inglés y, si no, como nombre impreso en
/// cualquier idioma.
async fn buscar(state: &AppState, texto: &str) -> AppResult<Vec<ScryCard>> {
    let limpio = texto.replace('"', "");
    let exacta = state
        .scryfall
        .search(&format!("!\"{limpio}\""), false, 4)
        .await
        .unwrap_or_default();
    if !exacta.is_empty() {
        return Ok(exacta);
    }
    Ok(state
        .scryfall
        .search(&format!("lang:any \"{limpio}\""), true, 6)
        .await
        .unwrap_or_default())
}

#[tauri::command]
pub async fn card_identify(
    state: State<'_, AppState>,
    blob: String,
    hint: Option<String>,
    ocr: bool,
) -> AppResult<CardIdentify> {
    let mut textos: Vec<String> = Vec::new();
    if let Some(h) = hint.as_deref().map(ocr::limpiar).filter(|h| h.len() >= 3) {
        textos.push(h);
    }
    let mut lineas = Vec::new();
    if ocr {
        let datos = state.paths.data.clone();
        if !ocr::modelos_listos(&datos) {
            ocr::descargar_modelos(&datos).await?;
        }
        let (bytes, _) = state.leer_blob(&blob)?;
        let img = proxy_core::imaging::decode::decode(&bytes)?;
        lineas = tauri::async_runtime::spawn_blocking(move || ocr::lineas_del_titulo(&datos, &img))
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))??;
        textos.extend(lineas.iter().cloned());
    }

    let mut vistas = HashSet::new();
    let mut probados = HashSet::new();
    let mut cartas = Vec::new();
    for t in textos {
        if probados.len() >= MAX_TEXTOS || !probados.insert(t.to_lowercase()) {
            continue;
        }
        for c in buscar(&state, &t).await? {
            if vistas.insert(c.id.clone()) {
                cartas.push(c);
            }
        }
        if cartas.len() >= MAX_CANDIDATAS {
            break;
        }
    }
    cartas.truncate(MAX_CANDIDATAS);
    let candidates = resumir(&state, cartas);
    Ok(CardIdentify {
        lines: lineas,
        candidates,
    })
}

/// Los datos de la carta elegida (de la caché si están; si no, de Scryfall,
/// y se guardan para que las estadísticas funcionen sin red).
#[tauri::command]
pub async fn card_link(state: State<'_, AppState>, scryfall_id: String) -> AppResult<CardMeta> {
    if let Some((c, _)) = cache::leer(&state.db, &scryfall_id)? {
        return Ok(meta_de(&c, false));
    }
    let c = state
        .scryfall
        .card(&scryfall_id)
        .await?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("carta", &scryfall_id))?;
    cache::guardar(&state.db, &c)?;
    Ok(meta_de(&c, false))
}
