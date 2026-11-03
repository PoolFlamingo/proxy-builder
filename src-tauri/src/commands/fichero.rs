//! **Compartir lo que se exporta** (Android): «Compartir» junto a «Guardar».
//! El fichero se escribe antes en la caché (el `FileProvider` de la plantilla
//! solo ve ahí) y el plugin propio abre el selector de apps del sistema.

use tauri::AppHandle;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::fichero;

/// Un camino de la caché donde exportar algo para compartirlo (`nombre` con su
/// extensión). Se queda hasta que el barrido de la caché lo borra.
#[tauri::command]
pub fn fichero_temporal(app: AppHandle, nombre: String) -> AppResult<String> {
    Ok(fichero::temporal(&app, &nombre)?
        .conservar()
        .to_string_lossy()
        .into_owned())
}

/// Comparte un fichero (una ruta o un `content://`) con otra app del móvil.
#[tauri::command]
pub async fn compartir_fichero(app: AppHandle, ruta: String) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let (camino, tipo) = fichero::para_compartir(&app, &ruta)?;
        crate::android::compartir(&app, &camino, tipo)
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?
}
