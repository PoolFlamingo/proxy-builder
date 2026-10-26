//! Actualizaciones de la app (ver `actualizar.rs`): buscar, bajar e instalar.

use tauri::{AppHandle, Emitter, State};

use crate::actualizar::{
    self, ActualizacionEstado, ActualizacionInfo, ActualizacionProgressEvent, Actualizador,
};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

#[tauri::command]
pub fn update_status(app: AppHandle) -> ActualizacionEstado {
    actualizar::estado(&app)
}

#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    estado: State<'_, Actualizador>,
) -> AppResult<Option<ActualizacionInfo>> {
    actualizar::buscar(&app, &estado).await
}

/// Baja la versión encontrada; avisa por `update://progress`.
#[tauri::command]
pub async fn update_download(
    app: AppHandle,
    estado: State<'_, Actualizador>,
    version: String,
) -> AppResult<()> {
    actualizar::bajar(&estado, &version, move |hecho, total| {
        let _ = app.emit(
            "update://progress",
            ActualizacionProgressEvent { hecho, total },
        );
    })
    .await
}

/// Instala la versión bajada y reinicia. No con un trabajo largo en marcha
/// (un PDF a medias se perdería) ni con la mesa abierta: la actualización
/// espera a que se acabe la partida o se salga de ella (`motivo: partida`).
#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    estado: State<'_, Actualizador>,
    state: State<'_, AppState>,
    version: String,
) -> AppResult<()> {
    if state.hay_partida() {
        return Err(AppError::new(ErrorCode::Busy).with("motivo", "partida"));
    }
    if state.ocupado() {
        return Err(AppError::new(ErrorCode::Busy));
    }
    actualizar::instalar(&app, &estado, &version).await
}
