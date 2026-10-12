//! La geometría de página: el `PagePlan` que pintan la vista previa y el PDF.

use proxy_core::layout::{self, PagePlan};
use proxy_core::model::Deck;

use crate::error::AppResult;

/// Calcula dónde va cada carta en cada cara de cada hoja.
#[tauri::command]
pub fn compute_page_plan(deck: Deck) -> AppResult<PagePlan> {
    Ok(layout::plan(&deck)?)
}
