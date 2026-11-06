//! **El plugin propio de Android** (primer Kotlin del proyecto): lo que Tauri
//! y sus plugins oficiales no hacen. 🛑 Lo llama solo Rust (la app lo envuelve
//! en `src/android.rs`): la pantalla sigue hablando únicamente por `ipc.ts`,
//! así que no declara comandos ni permisos de vista web.
//!
//! - `compartir`: `ACTION_SEND` con el `FileProvider` de la plantilla.
//! - `pantalla_encendida`, `pantalla_completa`, `multicast`.
//! - `instalar_apk`: el instalador de Android, con el permiso de «instalar
//!   apps desconocidas».
//! - `nombre_de_uri`: el nombre que Android da a un `content://`.
//! - `nombre_equipo`: el nombre del dispositivo, para presentarse a otros equipos.
//! - `compartir_texto`: `ACTION_SEND` de un texto.
//! - `cofre_sellar`, `cofre_abrir`: AES-GCM con una clave del Android Keystore.
//! - Abrir un fichero desde otra app (`ACTION_VIEW`): la URI llega aquí.
//!
//! En escritorio y en iOS el plugin se registra sin hacer nada.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::Runtime;

#[cfg(target_os = "android")]
mod movil;
#[cfg(target_os = "android")]
pub use movil::AndroidExtra;

/// Registra el plugin. En Android carga el Kotlin y deja `AndroidExtra` en el
/// estado de la app.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    #[allow(unused_mut)]
    let mut constructor = Builder::<R>::new("android-extra");
    #[cfg(target_os = "android")]
    {
        constructor = constructor.setup(|app, api| {
            use tauri::Manager;
            let handle = api.register_android_plugin(movil::IDENTIFICADOR, "AndroidExtraPlugin")?;
            app.manage(AndroidExtra::nuevo(handle));
            Ok(())
        });
    }
    constructor.build()
}
