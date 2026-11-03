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
//!
//! **Android** (`release` solamente): el plugin del actualizador busca y baja
//! (con la firma comprobada) si se le da el objetivo `android-<arquitectura>`
//! (`android-aarch64` en el `latest.json`; lo añade `scripts/release.mjs`), pero
//! no instala. El APK se deja en `app_cache_dir/actualizacion/`, se vuelve a
//! comprobar su firma y se entrega al instalador de Android
//! (`crate::android::instalar_apk`): no hay reinicio, el sistema pide confirmar
//! y relanza la app él. Android ya exige la misma clave de firma y un
//! `versionCode` mayor: la firma de minisign es una segunda barrera.
//!
//! Para probarlo en un emulador (que es `x86_64`): la feature de Cargo
//! `actualizar-local` lee la URL de un `latest.json` local de `PB_ACTUALIZAR_URL`
//! (en tiempo de ejecución, o la que había al compilar). 🛑 El CI no la activa
//! nunca (un test lo vigila).

use std::path::{Path, PathBuf};
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
    /// Android: la instala el instalador del sistema, que pide confirmar y
    /// relanza la app; no hay «reiniciar» (la pantalla cambia sus textos).
    pub instala_el_sistema: bool,
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
    paquete: Option<Paquete>,
}

/// El paquete bajado y con la firma comprobada.
enum Paquete {
    /// Escritorio: en memoria hasta instalarlo.
    Memoria(Vec<u8>),
    /// Android: el APK ya escrito en la caché (pesa decenas de MB y el
    /// instalador del sistema lo lee de un fichero).
    Apk(PathBuf),
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

/// Dónde están los instaladores de todas las versiones.
pub const PAGINA_DE_RELEASES: &str =
    "https://github.com/PoolFlamingo/proxy-builder/releases/latest";

pub fn estado(app: &AppHandle) -> ActualizacionEstado {
    ActualizacionEstado {
        soportada: soportada(),
        actual: app.package_info().version.to_string(),
        instala_el_sistema: es_android(),
    }
}

fn soportada() -> bool {
    // Android no tiene `bundle_type`: el APK es su único paquete.
    !cfg!(debug_assertions) && (tauri::utils::platform::bundle_type().is_some() || es_android())
}

fn es_android() -> bool {
    cfg!(target_os = "android")
}

/// La clave de este equipo en las `platforms` del `latest.json` de Android.
fn objetivo_android(arquitectura: &str) -> String {
    format!("android-{arquitectura}")
}

/// Dónde se deja el APK bajado (dentro de la caché, que cubre el
/// `FileProvider` de la plantilla de Android).
fn carpeta_apk(cache: &Path) -> PathBuf {
    cache.join("actualizacion")
}

fn ruta_apk(cache: &Path, version: &str) -> PathBuf {
    carpeta_apk(cache).join(format!("Proxy-Builder_{version}.apk"))
}

/// Con la feature `actualizar-local` (solo pruebas), el `latest.json` sale de
/// `PB_ACTUALIZAR_URL` en vez de GitHub.
#[cfg(feature = "actualizar-local")]
fn con_endpoint_local(
    builder: tauri_plugin_updater::UpdaterBuilder,
) -> AppResult<tauri_plugin_updater::UpdaterBuilder> {
    let Some(url) = std::env::var("PB_ACTUALIZAR_URL")
        .ok()
        .or_else(|| option_env!("PB_ACTUALIZAR_URL").map(str::to_string))
    else {
        return Ok(builder);
    };
    let url: tauri::Url = url
        .parse()
        .map_err(|e| AppError::new(ErrorCode::InvalidInput).with("detalle", e))?;
    tracing::warn!(%url, "actualizaciones desde una URL local (PB_ACTUALIZAR_URL)");
    builder.endpoints(vec![url]).map_err(error)
}

#[cfg(not(feature = "actualizar-local"))]
fn con_endpoint_local(
    builder: tauri_plugin_updater::UpdaterBuilder,
) -> AppResult<tauri_plugin_updater::UpdaterBuilder> {
    Ok(builder)
}

/// Busca una versión más nueva. `None` si ya es la última.
pub async fn buscar(
    app: &AppHandle,
    estado: &Actualizador,
    cache: &Path,
) -> AppResult<Option<ActualizacionInfo>> {
    if !soportada() {
        return Err(AppError::new(ErrorCode::Unsupported).with("detalle", "updater"));
    }
    let app2 = app.clone();
    let mut builder = app
        .updater_builder()
        .configure_client(crate::tls::reqwest)
        // En Windows el instalador cierra la app sin pasar por los eventos de
        // salida: se guarda antes el tamaño y la posición de la ventana.
        .on_before_exit(move || guardar_ventana(&app2));
    if es_android() {
        builder = builder.target(objetivo_android(std::env::consts::ARCH));
    }
    let updater = con_endpoint_local(builder)?.build().map_err(error)?;
    let Some(update) = updater.check().await.map_err(error)? else {
        estado.guardar(None);
        // Al día: si había un APK bajado de antes (ya instalado), sobra.
        if es_android() {
            let _ = std::fs::remove_dir_all(carpeta_apk(cache));
        }
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
    cache: &Path,
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
    let paquete = if es_android() {
        let ruta = ruta_apk(cache, version);
        let destino = ruta.clone();
        tauri::async_runtime::spawn_blocking(move || -> std::io::Result<()> {
            if let Some(carpeta) = destino.parent() {
                // Solo cabe un APK: los de antes sobran.
                let _ = std::fs::remove_dir_all(carpeta);
                std::fs::create_dir_all(carpeta)?;
            }
            std::fs::write(&destino, paquete)
        })
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))??;
        Paquete::Apk(ruta)
    } else {
        Paquete::Memoria(paquete)
    };
    estado.guardar(Some(Pendiente {
        update,
        paquete: Some(paquete),
    }));
    Ok(())
}

/// Instala la versión bajada y reinicia la app. En Windows el instalador
/// cierra la app él mismo y la abre al terminar.
pub async fn instalar(app: &AppHandle, estado: &Actualizador, version: &str) -> AppResult<()> {
    if es_android() {
        return instalar_android(app, estado, version).await;
    }
    let (update, paquete) = {
        let mut g = estado
            .pendiente
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        match g.as_mut() {
            Some(p)
                if p.update.version == version
                    && matches!(p.paquete, Some(Paquete::Memoria(_))) =>
            {
                let Some(Paquete::Memoria(bytes)) = p.paquete.take() else {
                    return Err(AppError::new(ErrorCode::Internal));
                };
                (p.update.clone(), bytes)
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
            paquete: Some(Paquete::Memoria(paquete)),
        }));
        return Err(error(e));
    }
    // Pasa por los eventos de salida: el window-state guarda la ventana.
    app.request_restart();
    Ok(())
}

/// Android: vuelve a comprobar la firma del APK ya bajado y se lo da al
/// instalador del sistema. Sin reiniciar: el sistema pide confirmar, sustituye
/// la app y la relanza. El paquete se queda por si se cancela y se reintenta; si
/// Android no deja instalar desde la app, `instalar_apk` abre su ajuste de
/// «instalar apps desconocidas» y devuelve `Unsupported` con
/// `motivo: instalar_permiso`.
async fn instalar_android(app: &AppHandle, estado: &Actualizador, version: &str) -> AppResult<()> {
    let (update, ruta) = {
        let g = estado
            .pendiente
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        match g.as_ref() {
            Some(Pendiente {
                update,
                paquete: Some(Paquete::Apk(ruta)),
            }) if update.version == version => (update.clone(), ruta.clone()),
            _ => return Err(AppError::new(ErrorCode::NotFound).with("detalle", version)),
        }
    };
    tracing::info!(version, "abriendo el instalador de Android");
    let ruta2 = ruta.clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        let bytes = std::fs::read(&ruta2)?;
        update.verify(&bytes).map_err(error)
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))??;
    crate::android::instalar_apk(app, &ruta)
}

#[cfg(desktop)]
fn guardar_ventana(app: &AppHandle) {
    use tauri_plugin_window_state::{AppHandleExt, StateFlags};
    let _ = app.save_window_state(StateFlags::all() & !StateFlags::VISIBLE);
}

/// En móvil no hay estado de ventana que guardar.
#[cfg(mobile)]
fn guardar_ventana(_app: &AppHandle) {}

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
    use std::path::Path;

    use base64::Engine;

    use super::{carpeta_apk, objetivo_android, ruta_apk};

    /// Los objetivos del `latest.json` de Android que escribe `scripts/release.mjs`.
    #[test]
    fn el_objetivo_de_android_lleva_la_arquitectura() {
        assert_eq!(objetivo_android("aarch64"), "android-aarch64");
        assert_eq!(objetivo_android("x86_64"), "android-x86_64");
    }

    #[test]
    fn el_apk_bajado_va_a_una_carpeta_de_la_cache() {
        let cache = Path::new("/cache");
        assert_eq!(carpeta_apk(cache), Path::new("/cache/actualizacion"));
        assert_eq!(
            ruta_apk(cache, "3.2.4"),
            Path::new("/cache/actualizacion/Proxy-Builder_3.2.4.apk")
        );
    }

    /// 🛑 `actualizar-local` (la URL local de pruebas) no puede estar en las
    /// features por defecto ni en el CI ni en el workflow de la release.
    #[test]
    fn la_feature_de_pruebas_no_se_activa_nunca_sola() {
        let cargo = include_str!("../Cargo.toml");
        assert!(
            !cargo.contains("default = ["),
            "ninguna feature por defecto: `actualizar-local` no puede entrar"
        );
        for fichero in [
            include_str!("../../.github/workflows/ci.yml"),
            include_str!("../../.github/workflows/release.yml"),
            include_str!("../../package.json"),
        ] {
            assert!(!fichero.contains("actualizar-local"));
        }
        let conf = include_str!("../tauri.conf.json");
        assert!(!conf.contains("actualizar-local"));
    }

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
