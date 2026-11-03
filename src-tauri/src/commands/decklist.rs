//! Leer la lista de cartas que pega el usuario y sacarla del deck en texto.

use std::path::PathBuf;

use proxy_core::decklist::{self, FormatoLista, ParsedDecklist};
use proxy_core::model::Deck;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::fichero;
use crate::store::blobs::escribir_atomico;

/// Lee una lista entera y dice qué es cada línea. Síncrono y barato: se llama
/// mientras se escribe (con un pequeño retardo en la pantalla).
#[tauri::command]
pub fn parse_decklist(text: String) -> ParsedDecklist {
    decklist::parse(&text)
}

/// La lista del deck en un formato (para copiarla).
#[tauri::command]
pub fn deck_list_text(deck: Deck, format: FormatoLista) -> String {
    decklist::exportar(&deck, format)
}

/// Guarda la lista del deck en un `.txt` (el que se elige en el diálogo). En
/// Android el diálogo da un `content://` sin ruta ni extensión: se escribe tal
/// cual (el nombre lo puso la persona al elegirlo).
#[tauri::command]
pub fn deck_list_save(
    app: tauri::AppHandle,
    deck: Deck,
    format: FormatoLista,
    path: String,
) -> AppResult<()> {
    if fichero::es_uri(&path) {
        return fichero::escribir_bytes(&app, &path, decklist::exportar(&deck, format).as_bytes());
    }
    let p = PathBuf::from(&path);
    let es_txt = p
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("txt"));
    if !p.is_absolute() || !es_txt {
        return Err(AppError::new(ErrorCode::InvalidInput).with("detalle", path));
    }
    escribir_atomico(&p, decklist::exportar(&deck, format).as_bytes())
}
