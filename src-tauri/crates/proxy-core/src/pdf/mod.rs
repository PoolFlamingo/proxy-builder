//! **El PDF**, con krilla (la misma librería que judlex).
//!
//! Cómo dibuja krilla (comprobado en su código, 0.8.2): el origen de la
//! `Surface` está ARRIBA a la izquierda, en puntos; `draw_image(img, size)`
//! pinta la imagen con su esquina superior izquierda en el origen actual; el
//! giro de 180° alrededor de un punto es `Transform::from_rotate_at(180, cx, cy)`
//! (grados). Un JPEG se incrusta tal cual con `DCTDecode` y las imágenes con
//! el mismo contenido se escriben una sola vez.
//!
//! 🛑 Este módulo no calcula posiciones: dibuja el `PagePlan`.

pub mod calibration;
pub mod compose;
pub mod estimate;

pub use compose::{compose, PdfMeta};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PdfError {
    #[error("medidas imposibles en el PDF")]
    Geometry,
    #[error("imagen no válida para el PDF: {0}")]
    Image(String),
    #[error("no se pudo escribir el PDF: {0}")]
    Write(String),
    #[error("cancelado")]
    Cancelled,
}

/// La letra de las etiquetas (Caladea, licencia en `assets/fuentes`).
pub(crate) const LETRA: &[u8] = include_bytes!("../../assets/fuentes/Caladea-Regular.ttf");

/// `#rrggbb` → (r, g, b). Lo que no se entienda, negro.
pub(crate) fn color_hex(s: &str) -> (u8, u8, u8) {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return (0, 0, 0);
    }
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    (c(0), c(2), c(4))
}

#[cfg(test)]
mod tests {
    #[test]
    fn colores_hex() {
        assert_eq!(super::color_hex("#00c853"), (0, 200, 83));
        assert_eq!(super::color_hex("rojo"), (0, 0, 0));
    }
}
