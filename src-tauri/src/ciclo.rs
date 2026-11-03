//! **El ciclo de vida en Android.** Ir a segundo plano no avisa antes de que el
//! sistema congele o mate el proceso (en Android 14 o más nuevo, unos 10 s
//! después), así que al irse se guarda todo y al volver se avisa a la red.
//!
//! - Al irse (`WindowEvent::Suspended`): se guarda la partida abierta y se
//!   avisa a la pantalla (`ciclo://pausa`), que guarda el deck y la carta que
//!   se diseña (`src/lib/ciclo-de-vida.ts`).
//! - Al volver (`WindowEvent::Resumed`): el nodo de iroh mira si la red ha
//!   cambiado (`network_change`; nunca `install_android_jni_context`, que tao
//!   ya hace y dos veces entra en pánico) y se avisa a la pantalla
//!   (`ciclo://vuelta`). La mesa en red reconecta sola, como tras un corte.
//!
//! En escritorio no se llama a nada de esto.

#[cfg(mobile)]
use tauri::{AppHandle, Emitter, Manager};
use tauri::{Window, WindowEvent};

#[cfg(mobile)]
use crate::state::AppState;

/// El `on_window_event` de la app.
pub fn evento(window: &Window, event: &WindowEvent) {
    match event {
        #[cfg(mobile)]
        WindowEvent::Suspended => al_irse(window.app_handle()),
        #[cfg(mobile)]
        WindowEvent::Resumed => al_volver(window.app_handle()),
        _ => {
            let _ = window;
        }
    }
}

#[cfg(mobile)]
fn al_irse(app: &AppHandle) {
    if let Some(estado) = app.try_state::<AppState>() {
        crate::commands::mesa::guardar_si_toca(&estado, true);
    }
    let _ = app.emit("ciclo://pausa", ());
}

#[cfg(mobile)]
fn al_volver(app: &AppHandle) {
    let _ = app.emit("ciclo://vuelta", ());
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Some(estado) = app.try_state::<AppState>() {
            estado.red.cambio_de_red().await;
        }
    });
}
