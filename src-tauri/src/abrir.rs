//! **Abrir un `.pbdeck` desde otra app** (Android, `ACTION_VIEW`): el plugin
//! propio (`plugins/android-extra`) recoge la `content://` del intent y llega
//! aquí. Rust la guarda y avisa a la pantalla con `pbdeck://abierto` (la carga
//! es la URI). Si la pantalla aún no escucha (la app arrancó con ese fichero),
//! la pide con `pbdeck_pendiente`, que la devuelve **una sola vez**.
//!
//! Nada se importa aquí: la pantalla lo ofrece (con las opciones del
//! importador) y la persona confirma. Rust solo comprueba, al leerlo, que sea
//! un paquete de verdad (`package::import`).

use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager};

/// Una URI más larga que esto no es de un proveedor de ficheros.
const MAX_URI: usize = 2048;

/// El último fichero abierto que nadie ha recogido.
static PENDIENTE: Mutex<Option<String>> = Mutex::new(None);

fn pendiente() -> std::sync::MutexGuard<'static, Option<String>> {
    match PENDIENTE.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// ¿Vale esta URI como fichero abierto desde fuera?
fn valida(uri: &str) -> bool {
    crate::fichero::es_uri(uri) && uri.len() <= MAX_URI
}

/// Llega un fichero abierto desde otra app: se guarda, se avisa y la ventana
/// pasa al frente.
pub fn recibir(app: &AppHandle, uri: &str) {
    if !valida(uri) {
        return;
    }
    *pendiente() = Some(uri.to_string());
    let _ = app.emit("pbdeck://abierto", uri);
    if let Some(v) = app.get_webview_window("main") {
        let _ = v.show();
        let _ = v.set_focus();
    }
}

/// Empieza a escuchar a la otra app (en Android; en escritorio no hay nada).
pub fn escuchar(app: &AppHandle) {
    let h = app.clone();
    if let Err(e) = crate::android::al_abrir(app, move |uri| recibir(&h, &uri)) {
        tracing::warn!(codigo = ?e.code, "no se pudo escuchar los ficheros abiertos desde fuera");
    }
}

/// El fichero que abrió la app (o el último), una sola vez: la pantalla lo
/// pide al arrancar.
#[tauri::command]
pub fn pbdeck_pendiente() -> Option<String> {
    pendiente().take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_valen_las_content_de_tamano_razonable() {
        assert!(valida("content://com.x/document/1"));
        assert!(!valida("file:///sdcard/x.pbdeck"));
        assert!(!valida("proxybuilder://unirse/ABCDE12345"));
        assert!(!valida(&format!("content://{}", "a".repeat(MAX_URI))));
    }

    #[test]
    fn lo_pendiente_se_entrega_una_sola_vez() {
        *pendiente() = Some("content://x/1".into());
        assert_eq!(pbdeck_pendiente().as_deref(), Some("content://x/1"));
        assert_eq!(pbdeck_pendiente(), None);
    }
}
