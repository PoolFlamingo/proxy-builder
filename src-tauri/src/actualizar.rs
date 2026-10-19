//! **Actualizaciones dentro de la app.**
//!
//! Tres pasos, cada uno un comando (`commands/actualizar.rs`): buscar (lee el
//! `latest.json` del último release publicado de GitHub; los borradores no
//! cuentan), bajar (el paquete del instalador con el que se instaló —`.deb`,
//! AppImage, NSIS, MSI o `.app`— con su firma minisign comprobada) e instalar
//! y reiniciar. La pantalla decide cuándo: sola al arrancar o desde Ajustes.
//!
//! 🛑 La clave pública va en `tauri.conf.json` (`plugins.updater.pubkey`) y la
//! privada solo en `~/.tauri/proxy-builder.key` de Pablo y en el secreto
//! `TAURI_SIGNING_PRIVATE_KEY` de GitHub. Si se pierde, las versiones ya
//! instaladas no aceptarán ninguna actualización nueva.
//!
//! Con `requireSignedVersion` la firma lleva dentro la versión (`tauri build`
//! la pone), así nadie puede anunciar una versión nueva con un paquete viejo.

use std::sync::Mutex;

use serde::Serialize;
use tauri::utils::config::BundleType;
use tauri::AppHandle;
use tauri_plugin_updater::{Update, UpdaterExt};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};

/// Una versión nueva encontrada.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActualizacionInfo {
    pub version: String,
    /// La versión que está en marcha.
    pub actual: String,
    /// Las notas del release (Markdown, en inglés).
    pub notas: Option<String>,
    /// Cuándo se publicó (RFC 3339), si el release lo dice.
    pub fecha: Option<String>,
    /// Al instalarla el sistema pedirá la contraseña (`.deb` y `.rpm`, por
    /// `pkexec`).
    pub pide_permiso: bool,
}

/// Lo que se puede hacer en este equipo.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActualizacionEstado {
    /// `false` en desarrollo (`pnpm tauri dev`) o si el binario no viene de
    /// un instalador: no hay paquete con el que reemplazarlo.
    pub soportada: bool,
    pub actual: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ActualizacionProgressEvent {
    #[ts(type = "number")]
    pub hecho: u64,
    /// 0 si el servidor no dice cuánto ocupa.
    #[ts(type = "number")]
    pub total: u64,
}

/// La actualización encontrada y, cuando se ha bajado, su paquete.
#[derive(Default)]
pub struct Actualizador {
    pendiente: Mutex<Option<Pendiente>>,
}

struct Pendiente {
    update: Update,
    paquete: Option<Vec<u8>>,
}

impl Actualizador {
    fn guardar(&self, p: Option<Pendiente>) {
        if let Ok(mut g) = self.pendiente.lock() {
            *g = p;
        }
    }

    /// La actualización encontrada, si es esa versión (por si se buscó otra
    /// vez entre medias).
    fn update(&self, version: &str) -> AppResult<Update> {
        self.pendiente
            .lock()
            .ok()
            .and_then(|g| {
                g.as_ref()
                    .filter(|p| p.update.version == version)
                    .map(|p| p.update.clone())
            })
            .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("detalle", version))
    }
}

pub fn estado(app: &AppHandle) -> ActualizacionEstado {
    ActualizacionEstado {
        soportada: soportada(),
        actual: app.package_info().version.to_string(),
    }
}

fn soportada() -> bool {
    !cfg!(debug_assertions) && tauri::utils::platform::bundle_type().is_some()
}

/// Busca una versión más nueva. `None` si ya es la última.
pub async fn buscar(
    app: &AppHandle,
    estado: &Actualizador,
) -> AppResult<Option<ActualizacionInfo>> {
    if !soportada() {
        return Err(AppError::new(ErrorCode::Unsupported).with("detalle", "updater"));
    }
    let app2 = app.clone();
    let updater = app
        .updater_builder()
        // En Windows el instalador cierra la app sin pasar por los eventos de
        // salida: se guarda antes el tamaño y la posición de la ventana.
        .on_before_exit(move || guardar_ventana(&app2))
        .build()
        .map_err(error)?;
    let Some(update) = updater.check().await.map_err(error)? else {
        estado.guardar(None);
        return Ok(None);
    };
    tracing::info!(version = %update.version, "hay una versión nueva");
    let info = ActualizacionInfo {
        version: update.version.clone(),
        actual: update.current_version.clone(),
        notas: update.body.clone().filter(|n| !n.trim().is_empty()),
        fecha: update
            .raw_json
            .get("pub_date")
            .and_then(|f| f.as_str())
            .map(str::to_string),
        pide_permiso: matches!(
            tauri::utils::platform::bundle_type(),
            Some(BundleType::Deb | BundleType::Rpm)
        ),
    };
    estado.guardar(Some(Pendiente {
        update,
        paquete: None,
    }));
    Ok(Some(info))
}

/// Baja el paquete de la versión encontrada y comprueba su firma. Avisa del
/// avance con `al_avanzar(hecho, total)`.
pub async fn bajar(
    estado: &Actualizador,
    version: &str,
    mut al_avanzar: impl FnMut(u64, u64),
) -> AppResult<()> {
    let update = estado.update(version)?;
    let mut hecho = 0u64;
    let paquete = update
        .download(
            |trozo, total| {
                hecho += trozo as u64;
                al_avanzar(hecho, total.unwrap_or(0));
            },
            || {},
        )
        .await
        .map_err(error)?;
    // `download` ya comprueba la firma; `install` lo hará otra vez.
    estado.guardar(Some(Pendiente {
        update,
        paquete: Some(paquete),
    }));
    Ok(())
}

/// Instala la versión bajada y reinicia la app. En Windows el instalador
/// cierra la app él mismo y la abre al terminar.
pub async fn instalar(app: &AppHandle, estado: &Actualizador, version: &str) -> AppResult<()> {
    let (update, paquete) = {
        let mut g = estado
            .pendiente
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        match g.as_mut() {
            Some(p) if p.update.version == version && p.paquete.is_some() => {
                (p.update.clone(), p.paquete.take().unwrap_or_default())
            }
            _ => return Err(AppError::new(ErrorCode::NotFound).with("detalle", version)),
        }
    };
    tracing::info!(version, "instalando la versión nueva");
    // Con `.deb` o `.rpm` espera a la contraseña de `pkexec`: fuera del hilo
    // de los comandos.
    let resultado = tauri::async_runtime::spawn_blocking(move || {
        let r = update.install(&paquete);
        (update, paquete, r)
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    let (update, paquete, r) = resultado;
    if let Err(e) = r {
        // Se puede volver a intentar (p. ej. si se canceló la contraseña).
        estado.guardar(Some(Pendiente {
            update,
            paquete: Some(paquete),
        }));
        return Err(error(e));
    }
    // Pasa por los eventos de salida: el window-state guarda la ventana.
    app.request_restart();
    Ok(())
}

fn guardar_ventana(app: &AppHandle) {
    use tauri_plugin_window_state::{AppHandleExt, StateFlags};
    let _ = app.save_window_state(StateFlags::all() & !StateFlags::VISIBLE);
}

fn error(e: tauri_plugin_updater::Error) -> AppError {
    use tauri_plugin_updater::Error as E;
    tracing::warn!(error = %e, "actualización");
    let code = match &e {
        E::Reqwest(_) | E::Network(_) => ErrorCode::Network,
        // Sin `latest.json` en el último release o sin paquete para este
        // instalador.
        E::ReleaseNotFound | E::TargetNotFound(_) | E::TargetsNotFound(_) => ErrorCode::NotFound,
        E::AuthenticationFailed => ErrorCode::Cancelled,
        E::Io(_) => ErrorCode::Io,
        _ => ErrorCode::Internal,
    };
    AppError::new(code).with("detalle", e)
}

#[cfg(test)]
mod tests {
    use base64::Engine;

    /// Lo que no se puede romper sin dejar a todos sin actualizaciones.
    #[test]
    fn la_configuracion_del_actualizador_es_valida() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(conf["bundle"]["createUpdaterArtifacts"], true);
        let u = &conf["plugins"]["updater"];
        assert_eq!(u["requireSignedVersion"], true);
        let fin = u["endpoints"][0].as_str().unwrap();
        assert!(fin.starts_with("https://github.com/"));
        assert!(fin.ends_with("/releases/latest/download/latest.json"));
        let clave = base64::engine::general_purpose::STANDARD
            .decode(u["pubkey"].as_str().unwrap())
            .unwrap();
        let clave = String::from_utf8(clave).unwrap();
        assert!(clave.starts_with("untrusted comment: minisign public key: D7AD5B146D43F545\n"));
    }
}
