//! **Imagen**: todo el procesado de las cartas, en Rust y en un orden fijo.
//!
//! decode → esquinas → encuadre (giro, zoom, desplazamiento, remuestreo) →
//! retoques → oscurecer bordes → sangrado → codificar.
//!
//! La misma tubería sirve las miniaturas de la pantalla y las imágenes del
//! PDF: lo que se ve es lo que se imprime.

pub mod adjust;
pub mod bleed;
pub mod blur;
pub mod corners;
pub mod decode;
pub mod edges;
pub mod encode;
pub mod frame;
pub mod mitades;
pub mod pipeline;
pub mod probe;

/// Lo que puede fallar al procesar una imagen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImagingError {
    #[error("formato no admitido")]
    Unsupported,
    #[error("imagen dañada: {0}")]
    Corrupt(String),
    #[error("imagen demasiado grande")]
    TooLarge,
    #[error("no se pudo codificar: {0}")]
    Encode(String),
}

/// Lado máximo que se acepta al decodificar.
pub const LADO_MAXIMO: u32 = 12_000;
