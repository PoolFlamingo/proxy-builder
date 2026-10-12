//! **Proxy Builder** — arranque de la aplicación.
//!
//! Este fichero solo monta Tauri y registra los comandos de cada área
//! (`commands/`). La lógica vive en su módulo o, si es determinista, en el
//! crate `proxy-core`. 🛑 Que no crezca: en judlex el `lib.rs` llegó a las
//! 11.865 líneas.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod commands;
pub mod error;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let constructor = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(desktop)]
    let constructor = constructor.plugin(tauri_plugin_window_state::Builder::new().build());

    let resultado = constructor
        .invoke_handler(tauri::generate_handler![
            commands::sistema::ping,
            commands::sistema::ping_error,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = resultado {
        // Sin ventana no hay dónde enseñarlo: a la salida de error y fuera.
        eprintln!("Proxy Builder no ha podido arrancar: {e}");
        std::process::exit(1);
    }
}
