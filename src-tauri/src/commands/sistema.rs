//! Comandos de sistema: comprobar que el puente con la pantalla funciona.

use crate::error::{AppError, AppResult, ErrorCode};
use crate::red::servidor::mensajes::Plataforma;

/// Contesta `"pong"`. Sirve para comprobar el puente IPC de punta a punta.
#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}

/// Devuelve siempre un error `InvalidInput`: prueba de que los errores llegan
/// a la pantalla con su código y se traducen.
#[tauri::command]
pub fn ping_error() -> AppResult<()> {
    Err(AppError::new(ErrorCode::InvalidInput).with("detalle", "prueba"))
}

/// En qué sistema corre la app (la pantalla decide con ello si se toca con el
/// dedo y qué enseñar: sin «Mostrar en la carpeta» en móvil, por ejemplo).
#[tauri::command]
pub fn app_plataforma() -> Plataforma {
    Plataforma::actual()
}

/// Sale de la app (el botón «atrás» de Android en la biblioteca: no hay «cerrar»).
/// La pantalla guarda antes lo suyo.
#[tauri::command]
pub fn app_salir(app: tauri::AppHandle) {
    app.exit(0);
}

/// Esconde o enseña las barras del sistema de Android (la pantalla completa de
/// la mesa). En escritorio no hace nada: la pantalla usa la ventana.
#[tauri::command]
pub fn app_pantalla_completa(app: tauri::AppHandle, si: bool) -> AppResult<()> {
    sin_barras_es_ok(crate::android::pantalla_completa(&app, si))
}

/// Sin Android no hay barras del sistema que esconder: `Unsupported` es un
/// «nada que hacer», no un fallo. Cualquier otro error sí se devuelve.
fn sin_barras_es_ok(r: AppResult<()>) -> AppResult<()> {
    match r {
        Err(e) if e.code == ErrorCode::Unsupported => Ok(()),
        otro => otro,
    }
}

/// Comparte un texto (el código o el enlace de invitación de una partida) con
/// otra app del móvil. El plugin de Android solo comparte ficheros, así que el
/// texto va en uno temporal de la caché (`invitacion.txt`, borrado por el
/// barrido). Fuera de Android da `Unsupported`: la pantalla solo lo ofrece allí.
#[tauri::command]
pub async fn app_compartir_texto(app: tauri::AppHandle, texto: String) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let copia = crate::fichero::temporal(&app, "invitacion.txt")?;
        std::fs::write(copia.ruta(), texto.as_bytes())?;
        let ruta = copia.conservar();
        crate::android::compartir(&app, &ruta, "text/plain")
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_pantalla_completa_sin_android_no_es_un_fallo() {
        assert!(sin_barras_es_ok(Err(AppError::new(ErrorCode::Unsupported))).is_ok());
        assert!(sin_barras_es_ok(Ok(())).is_ok());
        let fallo = sin_barras_es_ok(Err(AppError::new(ErrorCode::Internal)));
        assert_eq!(fallo.map_err(|e| e.code), Err(ErrorCode::Internal));
    }
}
