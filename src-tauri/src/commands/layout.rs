//! La geometría de página: el `PagePlan` que pintan la vista previa y el PDF.

use proxy_core::layout::{self, PagePlan};
use proxy_core::model::{Deck, DeckCard};

use crate::error::AppResult;

/// Calcula dónde va cada carta en cada cara de cada hoja.
#[tauri::command]
pub fn compute_page_plan(deck: Deck) -> AppResult<PagePlan> {
    Ok(layout::plan(&deck)?)
}

/// «Auto ordenar»: el nuevo orden de las cartas (ids). Los huecos no fijados
/// desaparecen; las fijadas se quedan donde están.
#[tauri::command]
pub fn arrange_cards(cards: Vec<DeckCard>) -> Vec<String> {
    layout::arrange(&cards)
}
