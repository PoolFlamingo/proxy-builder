//! Leer la lista de cartas que pega el usuario.

use proxy_core::decklist::{self, ParsedDecklist};

/// Lee una lista entera y dice qué es cada línea. Síncrono y barato: se llama
/// mientras se escribe (con un pequeño retardo en la pantalla).
#[tauri::command]
pub fn parse_decklist(text: String) -> ParsedDecklist {
    decklist::parse(&text)
}
