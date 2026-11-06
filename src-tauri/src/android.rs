//! **Lo que solo hace Android**, por el plugin propio (`plugins/android-extra`,
//! en Kotlin). Se llama desde Rust: la pantalla sigue hablando solo por
//! `ipc.ts`. En escritorio ninguna hace nada (devuelven `Unsupported` las que
//! no tienen sentido sin ella, y `Ok` las que solo retocan la ventana).

use std::path::Path;

use tauri::AppHandle;

use crate::error::{AppError, AppResult, ErrorCode};

#[cfg(not(target_os = "android"))]
fn sin_android() -> AppError {
    AppError::new(ErrorCode::Unsupported).with("detalle", "android")
}

/// Un fallo del Kotlin, como error de la app (el texto va al registro).
#[cfg(target_os = "android")]
fn de_plugin(e: tauri::plugin::mobile::PluginInvokeError) -> AppError {
    tracing::warn!("plugin de Android: {e}");
    AppError::new(ErrorCode::Internal).with("detalle", format!("android: {e}"))
}

#[cfg(target_os = "android")]
fn plugin(
    app: &AppHandle,
) -> tauri::State<'_, tauri_plugin_android_extra::AndroidExtra<tauri::Wry>> {
    use tauri::Manager;
    app.state()
}

/// Comparte un fichero (ya escrito en la caché) con otra app: `ACTION_SEND`.
#[cfg(target_os = "android")]
pub fn compartir(app: &AppHandle, ruta: &Path, tipo: &str) -> AppResult<()> {
    plugin(app).compartir(ruta, tipo).map_err(de_plugin)
}

#[cfg(not(target_os = "android"))]
pub fn compartir(_app: &AppHandle, _ruta: &Path, _tipo: &str) -> AppResult<()> {
    Err(sin_android())
}

/// Quién pide que la pantalla no se apague: una mesa abierta y un intercambio
/// de decks pueden coincidir, y acabar uno no debe apagársela al otro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsoPantalla {
    Mesa,
    Pasar,
}

static USOS_PANTALLA: std::sync::Mutex<std::collections::BTreeSet<UsoPantalla>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Que la pantalla no se apague mientras alguien lo pida.
pub fn pantalla_encendida(app: &AppHandle, uso: UsoPantalla, si: bool) -> AppResult<()> {
    let mut usos = match USOS_PANTALLA.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let previos = usos.clone();
    match contar_usos(&mut usos, uso, si) {
        Some(encender) => pantalla_sistema(app, encender).inspect_err(|_| *usos = previos),
        None => Ok(()),
    }
}

#[cfg(target_os = "android")]
fn pantalla_sistema(app: &AppHandle, si: bool) -> AppResult<()> {
    plugin(app).pantalla_encendida(si).map_err(de_plugin)
}

#[cfg(not(target_os = "android"))]
fn pantalla_sistema(_app: &AppHandle, _si: bool) -> AppResult<()> {
    Ok(())
}

/// Comparte un texto con otra app (el selector del sistema): `ACTION_SEND`.
#[cfg(target_os = "android")]
pub fn compartir_texto(app: &AppHandle, texto: &str) -> AppResult<()> {
    plugin(app).compartir_texto(texto).map_err(de_plugin)
}

#[cfg(not(target_os = "android"))]
pub fn compartir_texto(_app: &AppHandle, _texto: &str) -> AppResult<()> {
    Err(sin_android())
}

/// Esconde o enseña las barras del sistema (pantalla completa de la mesa).
#[cfg(target_os = "android")]
pub fn pantalla_completa(app: &AppHandle, si: bool) -> AppResult<()> {
    plugin(app).pantalla_completa(si).map_err(de_plugin)
}

#[cfg(not(target_os = "android"))]
pub fn pantalla_completa(_app: &AppHandle, _si: bool) -> AppResult<()> {
    Err(sin_android())
}

/// Quién pide el `MulticastLock`: la búsqueda de equipos cerca y «Recibir»
/// lo usan a la vez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UsoMulticast {
    Buscar,
    Recibir,
}

static USOS_MULTICAST: std::sync::Mutex<std::collections::BTreeSet<UsoMulticast>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Apunta que `uso` pide (o suelta) algo del sistema y dice qué hacer con ello:
/// `Some(true)` cogerlo (el primero), `Some(false)` soltarlo (el último), `None`
/// nada. Pedirlo o soltarlo dos veces el mismo uso no cuenta doble.
fn contar_usos<U: Ord>(usos: &mut std::collections::BTreeSet<U>, uso: U, si: bool) -> Option<bool> {
    let antes = !usos.is_empty();
    if si {
        usos.insert(uso);
    } else {
        usos.remove(&uso);
    }
    let ahora = !usos.is_empty();
    (antes != ahora).then_some(ahora)
}

/// Un `MulticastLock` de la Wi-Fi: sin él Android tira el mDNS. El Kotlin
/// tiene uno solo, sin contador: aquí se cuenta quién lo usa, y se suelta
/// cuando ya no lo usa nadie (cerrar «Recibir» no deja sin él a la búsqueda).
pub fn multicast(app: &AppHandle, uso: UsoMulticast, si: bool) -> AppResult<()> {
    let mut usos = match USOS_MULTICAST.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let previos = usos.clone();
    match contar_usos(&mut usos, uso, si) {
        Some(coger) => multicast_sistema(app, coger).inspect_err(|_| *usos = previos),
        None => Ok(()),
    }
}

#[cfg(target_os = "android")]
fn multicast_sistema(app: &AppHandle, si: bool) -> AppResult<()> {
    plugin(app).multicast(si).map_err(de_plugin)
}

#[cfg(not(target_os = "android"))]
fn multicast_sistema(_app: &AppHandle, _si: bool) -> AppResult<()> {
    Ok(())
}

/// Abre el instalador de Android con este APK (ya bajado a la caché). Si falta
/// el permiso de «instalar apps desconocidas», abre su ajuste y devuelve
/// `Unsupported` con `motivo: instalar_permiso` (la persona vuelve y repite).
#[cfg(target_os = "android")]
pub fn instalar_apk(app: &AppHandle, ruta: &Path) -> AppResult<()> {
    match plugin(app).instalar_apk(ruta).map_err(de_plugin)? {
        true => Ok(()),
        false => Err(AppError::new(ErrorCode::Unsupported).with("motivo", "instalar_permiso")),
    }
}

#[cfg(not(target_os = "android"))]
pub fn instalar_apk(_app: &AppHandle, _ruta: &Path) -> AppResult<()> {
    Err(sin_android())
}

/// El nombre que Android da a un `content://` (`DISPLAY_NAME`): tiene la
/// extensión, que la URI no siempre lleva.
#[cfg(target_os = "android")]
pub fn nombre_de_uri(app: &AppHandle, uri: &str) -> Option<String> {
    plugin(app).nombre_de_uri(uri).ok().flatten()
}

#[cfg(not(target_os = "android"))]
pub fn nombre_de_uri(_app: &AppHandle, _uri: &str) -> Option<String> {
    None
}

/// El nombre de este equipo según el sistema: el que le puso la persona en
/// Android y, si no hay, su marca y modelo; en escritorio, el del ordenador.
#[cfg(target_os = "android")]
pub fn nombre_equipo(app: &AppHandle) -> Option<String> {
    plugin(app).nombre_equipo().ok().flatten()
}

#[cfg(not(target_os = "android"))]
pub fn nombre_equipo(_app: &AppHandle) -> Option<String> {
    sin_dominio(&gethostname::gethostname().to_string_lossy())
}

/// `pablo-pc.local` → `pablo-pc`; `localhost` no le dice nada a nadie.
#[cfg(not(target_os = "android"))]
fn sin_dominio(nombre: &str) -> Option<String> {
    let corto = nombre.split('.').next().unwrap_or_default().trim();
    (!corto.is_empty() && !corto.eq_ignore_ascii_case("localhost")).then(|| corto.to_string())
}

/// El Android Keystore, para que la clave de la identidad no se pueda usar fuera de
/// este móvil.
#[cfg(target_os = "android")]
struct CofreAndroid(AppHandle);

#[cfg(target_os = "android")]
impl crate::red::identidad::Cofre for CofreAndroid {
    fn sellar(&self, claro: &[u8]) -> AppResult<Vec<u8>> {
        use base64::Engine;
        let b = base64::engine::general_purpose::STANDARD;
        let s = plugin(&self.0)
            .cofre_sellar(&b.encode(claro))
            .map_err(de_plugin)?;
        b.decode(s)
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
    }

    fn abrir(&self, sellado: &[u8]) -> AppResult<Vec<u8>> {
        use base64::Engine;
        let b = base64::engine::general_purpose::STANDARD;
        let s = plugin(&self.0)
            .cofre_abrir(&b.encode(sellado))
            .map_err(de_plugin)?;
        b.decode(s)
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
    }
}

/// Dónde guardar secretos en este sistema: en el escritorio, en ningún cofre (el
/// fichero con permisos del usuario).
#[cfg(target_os = "android")]
pub fn cofre(app: &AppHandle) -> Option<Box<dyn crate::red::identidad::Cofre>> {
    Some(Box::new(CofreAndroid(app.clone())))
}

#[cfg(not(target_os = "android"))]
pub fn cofre(_app: &AppHandle) -> Option<Box<dyn crate::red::identidad::Cofre>> {
    None
}

/// Pide que cada fichero que otra app abra con esta (`ACTION_VIEW`, incluido
/// el que arrancó la app) llegue a `f` como `content://`.
#[cfg(target_os = "android")]
pub fn al_abrir(app: &AppHandle, f: impl Fn(String) + Send + Sync + 'static) -> AppResult<()> {
    plugin(app).al_abrir(f).map_err(de_plugin)
}

#[cfg(not(target_os = "android"))]
pub fn al_abrir(_app: &AppHandle, _f: impl Fn(String) + Send + Sync + 'static) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "android"))]
    #[test]
    fn el_nombre_del_ordenador_va_sin_dominio() {
        assert_eq!(sin_dominio("pablo-pc.local").as_deref(), Some("pablo-pc"));
        assert_eq!(sin_dominio("Zeus").as_deref(), Some("Zeus"));
        assert_eq!(sin_dominio("localhost"), None);
        assert_eq!(sin_dominio(""), None);
    }

    #[test]
    fn acabar_un_intercambio_no_apaga_la_pantalla_de_la_mesa() {
        let mut u = std::collections::BTreeSet::new();
        assert_eq!(contar_usos(&mut u, UsoPantalla::Mesa, true), Some(true));
        assert_eq!(contar_usos(&mut u, UsoPantalla::Pasar, true), None);
        assert_eq!(contar_usos(&mut u, UsoPantalla::Pasar, false), None);
        assert_eq!(contar_usos(&mut u, UsoPantalla::Mesa, false), Some(false));
    }

    #[test]
    fn el_multicast_se_suelta_cuando_no_lo_usa_nadie() {
        let mut u = std::collections::BTreeSet::new();
        assert_eq!(contar_usos(&mut u, UsoMulticast::Buscar, true), Some(true));
        assert_eq!(contar_usos(&mut u, UsoMulticast::Recibir, true), None);
        // Cerrar «Recibir» no se lo quita a la búsqueda.
        assert_eq!(contar_usos(&mut u, UsoMulticast::Recibir, false), None);
        // Soltarlo otra vez no cuenta doble.
        assert_eq!(contar_usos(&mut u, UsoMulticast::Recibir, false), None);
        assert_eq!(contar_usos(&mut u, UsoMulticast::Buscar, true), None);
        assert_eq!(
            contar_usos(&mut u, UsoMulticast::Buscar, false),
            Some(false)
        );
        assert_eq!(contar_usos(&mut u, UsoMulticast::Buscar, false), None);
    }
}
