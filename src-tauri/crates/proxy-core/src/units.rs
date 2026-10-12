//! Unidades. Todo el núcleo trabaja en **milímetros** (`f64`); los puntos de
//! PDF y los píxeles salen de aquí y solo de aquí.

/// Milímetros por pulgada.
pub const MM_POR_PULGADA: f64 = 25.4;

/// Milímetros a puntos PDF (1 pt = 1/72 de pulgada).
pub fn mm_to_pt(mm: f64) -> f64 {
    mm * 72.0 / MM_POR_PULGADA
}

/// Milímetros a píxeles a una resolución dada, redondeando al entero más
/// cercano (una imagen no tiene medio píxel).
pub fn mm_to_px(mm: f64, dpi: f64) -> u32 {
    let px = (mm / MM_POR_PULGADA * dpi).round();
    if px <= 0.0 {
        0
    } else {
        px as u32
    }
}

/// Píxeles CSS (1/96 de pulgada) a puntos PDF: el grosor de las guías de
/// corte se elige en «px» como en fabricard.
pub fn css_px_to_pt(px: f64) -> f64 {
    px * 0.75
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_pulgada_son_72_puntos() {
        assert!((mm_to_pt(25.4) - 72.0).abs() < 1e-9);
    }

    #[test]
    fn una_carta_de_63_mm_a_600_ppp_son_1488_px() {
        assert_eq!(mm_to_px(63.0, 600.0), 1488);
        assert_eq!(mm_to_px(88.0, 600.0), 2079);
        assert_eq!(mm_to_px(1.0, 600.0), 24);
    }

    #[test]
    fn un_px_css_son_tres_cuartos_de_punto() {
        assert!((css_px_to_pt(0.5) - 0.375).abs() < 1e-12);
    }
}
