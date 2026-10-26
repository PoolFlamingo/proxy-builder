//! Comandos de la partida en red: la identidad de la instalación.

use tauri::State;

use crate::error::AppResult;
use crate::red::identidad::{self, Identidad};
use crate::state::AppState;

/// La identidad con la que se juega en red (clave pública, apodo, avatar).
#[tauri::command]
pub fn red_identidad(state: State<'_, AppState>) -> AppResult<Identidad> {
    identidad::leer(&state.db, state.red.clave())
}

/// Cambia el apodo y el avatar (una huella de blob o ninguno).
#[tauri::command]
pub fn red_identidad_set(
    state: State<'_, AppState>,
    apodo: String,
    avatar: Option<String>,
) -> AppResult<Identidad> {
    identidad::guardar(&state.db, &apodo, avatar.as_deref())?;
    identidad::leer(&state.db, state.red.clave())
}
