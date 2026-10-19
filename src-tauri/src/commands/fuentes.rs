//! Las fuentes de cartas aparte de Scryfall que se gestionan desde Ajustes: el
//! índice de nombres en otros idiomas (MTGJSON).

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use ts_rs::TS;

use crate::error::AppResult;
use crate::fuentes::mtgjson::{self, FaseNombres, NombresIdiomasInfo};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NombresProgressEvent {
    pub fase: FaseNombres,
    #[ts(type = "number")]
    pub hecho: u64,
    #[ts(type = "number")]
    pub total: u64,
}

#[tauri::command]
pub fn names_index_info(state: State<'_, AppState>) -> NombresIdiomasInfo {
    mtgjson::info(&state.paths.cache)
}

/// Baja (o renueva) el índice de nombres en otros idiomas; avisa por
/// `names://progress`.
#[tauri::command]
pub async fn names_index_download(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<NombresIdiomasInfo> {
    let _marca = state.trabajo();
    mtgjson::descargar(
        &state.externo,
        &state.paths.cache,
        move |fase, hecho, total| {
            let _ = app.emit(
                "names://progress",
                NombresProgressEvent { fase, hecho, total },
            );
        },
    )
    .await
}

#[tauri::command]
pub fn names_index_remove(state: State<'_, AppState>) -> AppResult<()> {
    mtgjson::quitar(&state.paths.cache)
}
