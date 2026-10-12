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
}
