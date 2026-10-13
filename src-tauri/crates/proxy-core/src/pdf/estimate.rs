//! Cuánto pesará el PDF, aproximadamente (para avisar antes de exportar).

use crate::imaging::frame::{trim_px, Target};
use crate::model::{CardSize, ImageFormat};

/// Bytes estimados de una imagen de carta con su sangrado.
pub fn bytes_por_imagen(
    card: CardSize,
    dpi: f64,
    bleed_mm: f64,
    formato: ImageFormat,
    calidad: u8,
) -> u64 {
    let (w, h) = trim_px(card, Target::Dpi(dpi));
    let b = (bleed_mm / 25.4 * dpi).round() as u64 * 2;
    let px = (u64::from(w) + b) * (u64::from(h) + b);
    let por_px = match formato {
        ImageFormat::Jpeg => 0.15 + 0.30 * (f64::from(calidad.clamp(1, 100)) / 100.0).powi(3),
        ImageFormat::Png => 2.2,
    };
    (px as f64 * por_px) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_600_ppp_un_jpeg_95_ronda_el_megabyte() {
        let b = bytes_por_imagen(CardSize::MTG, 600.0, 1.0, ImageFormat::Jpeg, 95);
        assert!((800_000..1_600_000).contains(&b), "{b}");
        assert!(bytes_por_imagen(CardSize::MTG, 600.0, 1.0, ImageFormat::Png, 95) > b * 4);
    }
}
