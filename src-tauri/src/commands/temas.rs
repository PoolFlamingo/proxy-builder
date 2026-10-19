//! Los temas de tweakcn que no trae la app (`temas.rs`).

use tauri::State;

use crate::error::AppResult;
use crate::state::AppState;
use crate::temas::{self, TemaCatalogo, TemaInstalado};

/// El registro oficial de tweakcn.
#[tauri::command]
pub async fn themes_catalog(state: State<'_, AppState>) -> AppResult<Vec<TemaCatalogo>> {
    temas::catalogo(&state.externo).await
}

/// Un tema cualquiera por su id o su enlace, para verlo antes de instalarlo.
#[tauri::command]
pub async fn theme_fetch(state: State<'_, AppState>, id: String) -> AppResult<TemaCatalogo> {
    temas::traer(&state.externo, &id).await
}

/// Lo baja con sus letras y lo guarda.
#[tauri::command]
pub async fn theme_install(state: State<'_, AppState>, id: String) -> AppResult<TemaInstalado> {
    temas::instalar(&state.externo, &state.paths.data, &id).await
}

#[tauri::command]
pub fn themes_installed(state: State<'_, AppState>) -> Vec<TemaInstalado> {
    temas::instalados(&state.paths.data)
}

#[tauri::command]
pub fn theme_remove(state: State<'_, AppState>, id: String) -> AppResult<()> {
    temas::quitar(&state.paths.data, &id)
}

/// Abre la galería de la comunidad de tweakcn en el navegador (para copiar el
/// enlace de un tema).
#[tauri::command]
pub fn themes_open_community(app: tauri::AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(temas::COMUNIDAD, None::<&str>)
        .map_err(|e| crate::error::AppError::new(crate::error::ErrorCode::Io).with("detalle", e))
}
