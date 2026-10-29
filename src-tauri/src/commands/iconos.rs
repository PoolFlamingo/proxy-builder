//! El selector de iconos de Iconify (`iconos.rs`).

use std::collections::BTreeMap;

use tauri::State;

use crate::error::AppResult;
use crate::iconos::{self, ColeccionIconos, DatosIcono};
use crate::state::AppState;

#[tauri::command]
pub async fn iconos_colecciones(state: State<'_, AppState>) -> AppResult<Vec<ColeccionIconos>> {
    iconos::colecciones(&state.externo, &state.paths.cache).await
}

#[tauri::command]
pub async fn iconos_nombres(state: State<'_, AppState>, prefijo: String) -> AppResult<Vec<String>> {
    iconos::nombres(&state.externo, &state.paths.cache, &prefijo).await
}

#[tauri::command]
pub async fn iconos_buscar(state: State<'_, AppState>, texto: String) -> AppResult<Vec<String>> {
    iconos::buscar(&state.externo, &texto).await
}

#[tauri::command]
pub async fn iconos_datos(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> AppResult<BTreeMap<String, DatosIcono>> {
    iconos::datos(&state.externo, &state.paths.data, &ids).await
}

/// Guarda el icono elegido en `app_data_dir/iconos/` para verlo sin red.
#[tauri::command]
pub async fn iconos_elegir(state: State<'_, AppState>, id: String) -> AppResult<DatosIcono> {
    iconos::elegir(&state.externo, &state.paths.data, &id).await
}
