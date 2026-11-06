//! La parte de Rust del plugin de Android: cada función llama a su
//! `@Command` de `AndroidExtraPlugin.kt`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::ipc::{Channel, InvokeResponseBody};
use tauri::plugin::PluginHandle;
use tauri::Runtime;

/// El paquete Kotlin del plugin.
pub const IDENTIFICADOR: &str = "com.estigio.proxybuilder.extra";

pub type Resultado<T> = Result<T, tauri::plugin::mobile::PluginInvokeError>;

pub struct AndroidExtra<R: Runtime>(PluginHandle<R>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Compartir<'a> {
    ruta: &'a str,
    tipo: &'a str,
}

#[derive(Serialize)]
struct Si {
    si: bool,
}

#[derive(Serialize)]
struct Texto<'a> {
    texto: &'a str,
}

#[derive(Serialize)]
struct Ruta<'a> {
    ruta: &'a str,
}

#[derive(Serialize)]
struct Uri<'a> {
    uri: &'a str,
}

#[derive(Serialize)]
struct Escucha {
    handler: Channel,
}

#[derive(Serialize, Deserialize)]
struct Datos {
    datos: String,
}

#[derive(Deserialize)]
struct Nombre {
    nombre: Option<String>,
}

#[derive(Deserialize)]
struct Instalar {
    /// `false`: Android no deja instalar desde esta app (todavía); se ha
    /// abierto su ajuste.
    lanzado: bool,
}

#[derive(Deserialize)]
struct Abierto {
    uri: Option<String>,
}

impl<R: Runtime> AndroidExtra<R> {
    pub(crate) fn nuevo(h: PluginHandle<R>) -> Self {
        Self(h)
    }

    /// Comparte un fichero (en la caché de la app) con otra app.
    pub fn compartir(&self, ruta: &Path, tipo: &str) -> Resultado<()> {
        let ruta = ruta.to_string_lossy();
        self.0
            .run_mobile_plugin("compartir", Compartir { ruta: &ruta, tipo })
    }

    /// Comparte un texto con otra app.
    pub fn compartir_texto(&self, texto: &str) -> Resultado<()> {
        self.0.run_mobile_plugin("compartirTexto", Texto { texto })
    }

    /// Sella estos datos (en base64) con la clave del Android Keystore.
    pub fn cofre_sellar(&self, datos: &str) -> Resultado<String> {
        let r: Datos = self.0.run_mobile_plugin(
            "cofreSellar",
            Datos {
                datos: datos.to_string(),
            },
        )?;
        Ok(r.datos)
    }

    /// Abre lo que selló `cofre_sellar`.
    pub fn cofre_abrir(&self, datos: &str) -> Resultado<String> {
        let r: Datos = self.0.run_mobile_plugin(
            "cofreAbrir",
            Datos {
                datos: datos.to_string(),
            },
        )?;
        Ok(r.datos)
    }

    pub fn pantalla_encendida(&self, si: bool) -> Resultado<()> {
        self.0.run_mobile_plugin("pantallaEncendida", Si { si })
    }

    pub fn pantalla_completa(&self, si: bool) -> Resultado<()> {
        self.0.run_mobile_plugin("pantallaCompleta", Si { si })
    }

    pub fn multicast(&self, si: bool) -> Resultado<()> {
        self.0.run_mobile_plugin("multicast", Si { si })
    }

    /// Abre el instalador con este APK. `Ok(false)`: falta el permiso de
    /// «instalar apps desconocidas» y se ha abierto el ajuste.
    pub fn instalar_apk(&self, ruta: &Path) -> Resultado<bool> {
        let ruta = ruta.to_string_lossy();
        let r: Instalar = self
            .0
            .run_mobile_plugin("instalarApk", Ruta { ruta: &ruta })?;
        Ok(r.lanzado)
    }

    /// El nombre que da Android a un `content://` (`DISPLAY_NAME`).
    pub fn nombre_de_uri(&self, uri: &str) -> Resultado<Option<String>> {
        let r: Nombre = self.0.run_mobile_plugin("nombreDeUri", Uri { uri })?;
        Ok(r.nombre.filter(|n| !n.is_empty()))
    }

    /// El nombre del dispositivo: el que le puso la persona o su marca y modelo.
    pub fn nombre_equipo(&self) -> Resultado<Option<String>> {
        let r: Nombre = self.0.run_mobile_plugin("nombreEquipo", ())?;
        Ok(r.nombre.filter(|n| !n.is_empty()))
    }

    /// Avisa de cada fichero que otra app abre con esta (`ACTION_VIEW`),
    /// también del que arrancó la app.
    pub fn al_abrir(&self, f: impl Fn(String) + Send + Sync + 'static) -> Resultado<()> {
        let handler = Channel::new(move |cuerpo| {
            if let InvokeResponseBody::Json(json) = cuerpo {
                if let Ok(Abierto { uri: Some(u) }) = serde_json::from_str(&json) {
                    f(u);
                }
            }
            Ok(())
        });
        self.0
            .run_mobile_plugin("escucharAbiertos", Escucha { handler })
    }
}
