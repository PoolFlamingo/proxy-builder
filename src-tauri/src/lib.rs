//! **Proxy Builder** — arranque de la aplicación.
//!
//! Este fichero solo monta Tauri y registra los comandos de cada área
//! (`commands/`). La lógica vive en su módulo o, si es determinista, en el
//! crate `proxy-core`. 🛑 Que no crezca: en judlex el `lib.rs` llegó a las
//! 11.865 líneas.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod archidekt;
pub mod commands;
pub mod error;
pub mod jobs;
pub mod ocr;
pub mod package;
pub mod protocol;
pub mod registro;
pub mod render;
pub mod scryfall;
pub mod state;
pub mod store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let constructor = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init());

    #[cfg(desktop)]
    let constructor = constructor.plugin(tauri_plugin_window_state::Builder::new().build());

    // El protocolo de imágenes: la vista previa pide aquí cada miniatura.
    let constructor = constructor
        .register_asynchronous_uri_scheme_protocol(protocol::ESQUEMA, protocol::manejador);

    let resultado = constructor
        .setup(|app| {
            use tauri::Manager;
            let rutas = app.path();
            let paths = state::Paths {
                data: rutas.app_data_dir()?,
                cache: rutas.app_cache_dir()?,
            };
            registro::iniciar(&rutas.app_log_dir()?);
            let estado = state::AppState::open(paths).map_err(|e| e.to_string())?;
            app.manage(estado);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sistema::ping,
            commands::sistema::ping_error,
            commands::decklist::parse_decklist,
            commands::layout::compute_page_plan,
            commands::library::deck_list,
            commands::library::deck_create,
            commands::library::deck_open,
            commands::library::deck_save,
            commands::library::deck_rename,
            commands::library::deck_duplicate,
            commands::library::deck_delete,
            commands::library::blob_import_files,
            commands::library::blob_generic_back,
            commands::library::blob_gc,
            commands::library::app_setting_get,
            commands::library::app_setting_set,
            commands::cards::resolve_decklist,
            commands::cards::job_cancel,
            commands::cards::card_autocomplete,
            commands::cards::card_search,
            commands::cards::card_prints,
            commands::cards::card_fetch,
            commands::cards::sets_list,
            commands::cards::set_cards,
            commands::cards::import_archidekt,
            commands::export::export_pdf,
            commands::export::export_estimate,
            commands::export::export_calibration,
            commands::export::export_cancel,
            commands::export::export_open,
            commands::export::export_reveal,
            commands::library::cache_stats,
            commands::library::cache_clear,
            commands::library::deck_export_package,
            commands::library::deck_import_package,
            commands::library::preset_list,
            commands::library::preset_save,
            commands::library::preset_delete,
            commands::layout::arrange_cards,
            commands::stats::deck_stats,
            commands::identify::ocr_ready,
            commands::identify::card_identify,
            commands::identify::card_link,
            commands::identify::identify_batch,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = resultado {
        // Sin ventana no hay dónde enseñarlo: a la salida de error y fuera.
        eprintln!("Proxy Builder no ha podido arrancar: {e}");
        std::process::exit(1);
    }
}
