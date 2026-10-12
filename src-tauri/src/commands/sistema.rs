//! Comandos de sistema: comprobar que el puente con la pantalla funciona.

use crate::error::{AppError, AppResult, ErrorCode};

/// Contesta `"pong"`. Sirve para comprobar el puente IPC de punta a punta.
#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}

/// Devuelve siempre un error `InvalidInput`: prueba de que los errores llegan
/// a la pantalla con su código y se traducen.
#[tauri::command]
pub fn ping_error() -> AppResult<()> {
    Err(AppError::new(ErrorCode::InvalidInput).with("detalle", "prueba"))
}
