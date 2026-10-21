//! **Proxy Builder** — arranque de la aplicación.
//!
//! Este fichero solo monta Tauri y registra los comandos de cada área
//! (`commands/`). La lógica vive en su módulo o, si es determinista, en el
//! crate `proxy-core`. 🛑 Que no crezca: en judlex el `lib.rs` llegó a las
//! 11.865 líneas.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod actualizar;
pub mod archidekt;
pub mod biblioteca;
pub mod commands;
pub mod diseno;
pub mod error;
pub mod fuentes;
pub mod jobs;
pub mod ocr;
pub mod originales;
pub mod package;
pub mod protocol;
pub mod registro;
pub mod render;
pub mod scryfall;
pub mod state;
pub mod store;
pub mod temas;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let constructor = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(actualizar::Actualizador::default());

    #[cfg(desktop)]
    // La ventana arranca oculta (sin el destello blanco del primer
    // fotograma) y la enseña la pantalla cuando ya está pintada: el estado
    // guardado no debe enseñarla antes.
    let constructor = constructor.plugin(
        tauri_plugin_window_state::Builder::new()
            .with_state_flags(
                tauri_plugin_window_state::StateFlags::all()
                    & !tauri_plugin_window_state::StateFlags::VISIBLE,
            )
            .build(),
    );

    // En cuanto la página ha cargado, la ventana se enseña (con su fondo
    // oscuro, sin destello blanco). No se espera a que pinte la pantalla:
    // WebKitGTK no da fotogramas (`requestAnimationFrame`) a una ventana
    // oculta, y entonces solo salía a los 3 segundos.
    #[cfg(desktop)]
    let constructor = constructor.on_page_load(|webview, payload| {
        if payload.event() == tauri::webview::PageLoadEvent::Finished {
            let _ = webview.window().show();
        }
    });

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
            // Los marcos «Original» que pida esta versión y aún no estén.
            let app1 = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                diseno::original::completar(&app1.state::<state::AppState>()).await;
            });
            // Por si la página no llega a cargar: a los 3 segundos, se enseña
            // igual.
            let app2 = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                if let Some(v) = app2.get_webview_window("main") {
                    if !v.is_visible().unwrap_or(true) {
                        let _ = v.show();
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sistema::ping,
            commands::carpetas::carpeta_recordar,
            commands::carpetas::carpeta_inicial,
            commands::sistema::ping_error,
            commands::actualizar::update_status,
            commands::actualizar::update_check,
            commands::actualizar::update_download,
            commands::actualizar::update_install,
            commands::decklist::parse_decklist,
            commands::decklist::deck_list_text,
            commands::decklist::deck_list_save,
            commands::layout::compute_page_plan,
            commands::library::deck_list,
            commands::library::deck_create,
            commands::library::deck_open,
            commands::library::deck_details,
            commands::library::deck_peek,
            commands::library::tag_list,
            commands::library::tag_create,
            commands::library::tag_update,
            commands::library::tag_delete,
            commands::library::deck_tags_set,
            commands::library::deck_pin,
            commands::library::deck_notes_set,
            commands::library::deck_cover_set,
            commands::library::deck_lang_set,
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
            commands::cards::card_fetch_externa,
            commands::cards::deck_tokens_detect,
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
            commands::identify::card_link_externa,
            commands::fuentes::names_index_info,
            commands::fuentes::names_index_download,
            commands::fuentes::names_index_remove,
            commands::temas::themes_catalog,
            commands::temas::theme_fetch,
            commands::temas::theme_install,
            commands::temas::themes_installed,
            commands::temas::theme_remove,
            commands::temas::themes_open_community,
            commands::identify::identify_batch,
            commands::diseno::design_draft,
            commands::diseno::collection_list,
            commands::diseno::collection_save,
            commands::diseno::collection_delete,
            commands::diseno::fonts_list,
            commands::diseno::font_import,
            commands::diseno::font_remove,
            commands::diseno::packs_list,
            commands::diseno::pack_import,
            commands::diseno::pack_remove,
            commands::diseno::fonts_free_list,
            commands::diseno::font_free_download,
            commands::diseno::original_info,
            commands::diseno::original_download,
            commands::diseno::packs_downloadable_list,
            commands::diseno::pack_downloadable_download,
            commands::diseno::pack_extra_use,
            commands::diseno::symbols_status,
            commands::diseno::design_register,
            commands::diseno::design_overlay,
            commands::diseno::design_fit_report,
            commands::diseno::design_structure,
            commands::diseno::design_suggest_translation,
            commands::diseno::design_drafts_for_deck,
            commands::diseno::design_export_png,
            commands::diseno::design_save,
            commands::diseno::design_remove,
            commands::diseno::design_rerender,
            commands::diseno::design_renumber,
            commands::diseno::art_from_clipboard,
            commands::diseno::art_from_blob,
            commands::diseno::art_from_scryfall,
            commands::diseno::art_from_card,
        ])
        .run(tauri::generate_context!());

    if let Err(e) = resultado {
        // Sin ventana no hay dónde enseñarlo: a la salida de error y fuera.
        eprintln!("Proxy Builder no ha podido arrancar: {e}");
        std::process::exit(1);
    }
}
