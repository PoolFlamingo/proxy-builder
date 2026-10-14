//! **Editor de cartas** (la 2.0): borradores desde Scryfall y colecciones.
//! El motor de render y la vista previa llegan en las fases siguientes.

use proxy_core::diseno::modelo::{Collection, DesignDraft};
use proxy_core::model::DeckCard;
use tauri::State;

use crate::diseno::borrador;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::colecciones;

/// El borrador de una carta del deck: la instantánea mecánica (inglés y
/// español), los nombres originales como punto de partida y el siguiente
/// número libre del pie (`usados`: los números que ya hay en el deck).
#[tauri::command]
pub async fn design_draft(
    state: State<'_, AppState>,
    card: DeckCard,
    usados: Vec<String>,
    collection: Option<String>,
) -> AppResult<DesignDraft> {
    let id = borrador::id_scryfall(&card)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "sin_vincular"))?
        .to_string();
    let api = state.scryfall.clone();
    let en = borrador::carta_completa(&state.db, api.as_ref(), &id).await?;
    let es = borrador::impresion_es(&state.db, api.as_ref(), &en).await?;
    let datos = borrador::datos(&en, es.as_ref(), crate::store::ahora_ms());
    let coleccion = match &collection {
        Some(c) => colecciones::get(&state.db, c)?,
        None => None,
    };
    Ok(borrador::montar(&card, datos, coleccion.as_ref(), &usados))
}

#[tauri::command]
pub fn collection_list(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    colecciones::list(&state.db)
}

#[tauri::command]
pub fn collection_save(
    state: State<'_, AppState>,
    collection: Collection,
) -> AppResult<Collection> {
    colecciones::save(&state.db, &collection)
}

#[tauri::command]
pub fn collection_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    colecciones::delete(&state.db, &id)
}
