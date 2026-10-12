//! La biblioteca: decks guardados, imágenes subidas y ajustes sueltos.

use std::path::PathBuf;

use proxy_core::model::{Deck, FaceImage};
use serde::Serialize;
use tauri::State;
use ts_rs::TS;

use crate::error::AppResult;
use crate::state::AppState;
use crate::store::blobs::GcReport;
use crate::store::decks::{self, DeckSummary};
use crate::store::settings;

#[tauri::command]
pub fn deck_list(state: State<'_, AppState>) -> AppResult<Vec<DeckSummary>> {
    decks::list(&state.db)
}

/// Crea un deck vacío con el reverso genérico de la app.
#[tauri::command]
pub fn deck_create(state: State<'_, AppState>, name: String) -> AppResult<Deck> {
    let reverso = state.blobs.generic_back(&state.db)?;
    decks::create(&state.db, &name, Some(reverso))
}

#[tauri::command]
pub fn deck_open(state: State<'_, AppState>, id: String) -> AppResult<Deck> {
    decks::open(&state.db, &id)
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveResult {
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[tauri::command]
pub fn deck_save(state: State<'_, AppState>, deck: Deck) -> AppResult<SaveResult> {
    Ok(SaveResult {
        updated_at: decks::save(&state.db, &deck)?,
    })
}

#[tauri::command]
pub fn deck_rename(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    decks::rename(&state.db, &id, &name)
}

#[tauri::command]
pub fn deck_duplicate(state: State<'_, AppState>, id: String, name: String) -> AppResult<Deck> {
    decks::duplicate(&state.db, &id, &name)
}

#[tauri::command]
pub fn deck_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    decks::delete(&state.db, &id)
}

/// Copia al almacén las imágenes que el usuario ha elegido en el diálogo.
#[tauri::command]
pub async fn blob_import_files(
    state: State<'_, AppState>,
    paths: Vec<PathBuf>,
) -> AppResult<Vec<FaceImage>> {
    state.blobs.import_files(&state.db, &paths)
}

/// El reverso genérico de la app (para devolverlo a un deck).
#[tauri::command]
pub fn blob_generic_back(state: State<'_, AppState>) -> AppResult<FaceImage> {
    state.blobs.generic_back(&state.db)
}

/// Borra las imágenes que ya no usa ningún deck.
#[tauri::command]
pub fn blob_gc(state: State<'_, AppState>) -> AppResult<GcReport> {
    state.blobs.gc(&state.db, state.ocupado())
}

#[tauri::command]
pub fn app_setting_get(state: State<'_, AppState>, key: String) -> AppResult<Option<String>> {
    settings::get(&state.db, &key)
}

#[tauri::command]
pub fn app_setting_set(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    settings::set(&state.db, &key, &value)
}
