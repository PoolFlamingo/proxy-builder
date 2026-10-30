//! **El error que ve la pantalla.**
//!
//! Todos los comandos devuelven `Result<T, AppError>`. El error viaja como
//! JSON `{ code, params }` y React lo traduce con `t()` a partir de `code`
//! (`src/lib/errores.ts`): Rust nunca escribe prosa para el usuario, así la
//! app puede contar el error en español o en inglés.
//!
//! A diferencia de judlex —que mete el código dentro de un `String` porque
//! tenía 183 comandos que no podía cambiar de firma—, aquí no hay pasado que
//! preservar y el error es un tipo de verdad.

use std::collections::BTreeMap;

use serde::Serialize;
use ts_rs::TS;

/// Qué ha ido mal, en una palabra. La pantalla tiene un texto por cada uno.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
pub enum ErrorCode {
    /// Sin conexión o el servidor no responde.
    Network,
    /// Scryfall (u otro servicio) pide que bajemos el ritmo.
    RateLimited,
    /// Lo pedido no existe (una carta, un deck, una imagen).
    NotFound,
    /// Fallo leyendo o escribiendo en disco.
    Io,
    /// Fallo de la base de datos local.
    Db,
    /// Lo que llega de la pantalla no tiene sentido (p. ej. una carta que no
    /// cabe en el papel elegido).
    InvalidInput,
    /// El usuario canceló el trabajo.
    Cancelled,
    /// No se pudo generar el PDF.
    Pdf,
    /// No se pudo leer o procesar una imagen.
    Image,
    /// Formato de fichero no admitido.
    Unsupported,
    /// Hay un trabajo en marcha que impide hacer esto ahora.
    Busy,
    /// Fallo interno inesperado (un error de programación).
    Internal,
    /// La mesa de juego no acepta esa acción (`params.motivo`: el
    /// `mesa::Rechazo`, «SinPermiso», «ZonaVacia»…).
    Rejected,
    /// El servidor de salas ha dicho que no (`params.code`; Fase F).
    Server,
}

/// El error de un comando: un código y, si hace falta, datos para el texto
/// (`{ "linea": "12" }` → «La línea {linea} no se entiende»).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, thiserror::Error)]
#[ts(export)]
#[error("{code:?} {params:?}")]
pub struct AppError {
    pub code: ErrorCode,
    pub params: BTreeMap<String, String>,
}

impl AppError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            params: BTreeMap::new(),
        }
    }

    /// Añade un dato para el texto del error.
    pub fn with(mut self, clave: &str, valor: impl ToString) -> Self {
        self.params.insert(clave.to_string(), valor.to_string());
        self
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::new(ErrorCode::Io).with("detalle", e)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::new(ErrorCode::Db).with("detalle", e)
    }
}

impl From<proxy_core::layout::LayoutError> for AppError {
    fn from(e: proxy_core::layout::LayoutError) -> Self {
        use proxy_core::layout::LayoutError as L;
        let motivo = match e {
            L::CardDoesNotFit => "card_does_not_fit",
            L::InvalidSettings => "invalid_settings",
        };
        AppError::new(ErrorCode::InvalidInput).with("motivo", motivo)
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Un fallo de `reqwest`, con el porqué en `motivo` para que la pantalla no
/// lo cuente todo como «sin conexión»: `conexion` (no se llegó: DNS,
/// cortafuegos, antivirus, proxy), `tiempo` (se pasó el plazo) o `respuesta`
/// (llegó algo que no se entiende). `detalle` lleva la cadena entera de
/// causas, que `reqwest` esconde en su `Display`, y queda en el registro.
pub fn red(e: reqwest::Error) -> AppError {
    let motivo = if e.is_timeout() {
        "tiempo"
    } else if e.is_connect() {
        "conexion"
    } else if e.is_decode() || e.is_body() {
        "respuesta"
    } else {
        "otro"
    };
    let mut detalle = e.to_string();
    let mut causa = std::error::Error::source(&e);
    while let Some(c) = causa {
        detalle.push_str(": ");
        detalle.push_str(&c.to_string());
        causa = c.source();
    }
    let host = e.url().and_then(|u| u.host_str()).unwrap_or("").to_string();
    tracing::warn!(%host, motivo, %detalle, "red: fallo");
    AppError::new(ErrorCode::Network)
        .with("motivo", motivo)
        .with("detalle", detalle)
}

/// El servidor contestó con un estado de error que no se reintenta.
/// `motivo`: `rechazo` si es 401/403 (lo típico de Cloudflare con una IP que
/// no le gusta, una VPN o un proxy: el navegador pasa el reto y la app no) o
/// `estado` para los demás.
pub fn estado_http(estado: reqwest::StatusCode) -> AppError {
    let motivo = if matches!(estado.as_u16(), 401 | 403) {
        "rechazo"
    } else {
        "estado"
    };
    tracing::warn!(estado = estado.as_u16(), motivo, "red: estado de error");
    AppError::new(ErrorCode::Network)
        .with("motivo", motivo)
        .with("estado", estado.as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_error_viaja_como_codigo_y_datos() {
        let e = AppError::new(ErrorCode::InvalidInput).with("linea", 12);
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["code"], "InvalidInput");
        assert_eq!(json["params"]["linea"], "12");
    }

    #[test]
    fn un_403_es_un_rechazo_y_no_falta_de_conexion() {
        let e = estado_http(reqwest::StatusCode::FORBIDDEN);
        assert_eq!(e.code, ErrorCode::Network);
        assert_eq!(e.params["motivo"], "rechazo");
        assert_eq!(e.params["estado"], "403");
        assert_eq!(
            estado_http(reqwest::StatusCode::BAD_REQUEST).params["motivo"],
            "estado"
        );
    }

    #[tokio::test]
    async fn sin_servidor_es_un_fallo_de_conexion() {
        // Nadie escucha en el puerto 9 de la máquina: la conexión se rechaza.
        let e = reqwest::Client::new()
            .get("http://127.0.0.1:9/")
            .send()
            .await
            .unwrap_err();
        let e = red(e);
        assert_eq!(e.params["motivo"], "conexion");
        assert!(e.params["detalle"].len() > 10);
    }
}
