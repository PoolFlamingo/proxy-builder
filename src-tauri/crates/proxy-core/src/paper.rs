//! Formatos de papel, en milímetros y en vertical (ancho < alto).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Los formatos que ofrece fabricard: DIN/ISO, EE. UU. y foto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum PaperFormat {
    A3,
    A4,
    A5,
    Letter,
    Legal,
    Tabloid,
    Photo4x6,
    Photo5x7,
    Photo8x10,
    Photo8x12,
}

impl PaperFormat {
    /// Todos, en el orden en que se ofrecen en pantalla.
    pub const TODOS: [PaperFormat; 10] = [
        PaperFormat::A4,
        PaperFormat::A3,
        PaperFormat::A5,
        PaperFormat::Letter,
        PaperFormat::Legal,
        PaperFormat::Tabloid,
        PaperFormat::Photo4x6,
        PaperFormat::Photo5x7,
        PaperFormat::Photo8x10,
        PaperFormat::Photo8x12,
    ];

    /// (ancho, alto) en milímetros, en vertical.
    pub fn size_mm(self) -> (f64, f64) {
        match self {
            PaperFormat::A3 => (297.0, 420.0),
            PaperFormat::A4 => (210.0, 297.0),
            PaperFormat::A5 => (148.0, 210.0),
            PaperFormat::Letter => (215.9, 279.4),
            PaperFormat::Legal => (215.9, 355.6),
            PaperFormat::Tabloid => (279.4, 431.8),
            PaperFormat::Photo4x6 => (101.6, 152.4),
            PaperFormat::Photo5x7 => (127.0, 177.8),
            PaperFormat::Photo8x10 => (203.2, 254.0),
            PaperFormat::Photo8x12 => (203.2, 304.8),
        }
    }

    /// (ancho, alto) según la orientación.
    pub fn oriented_mm(self, landscape: bool) -> (f64, f64) {
        let (w, h) = self.size_mm();
        if landscape {
            (h, w)
        } else {
            (w, h)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todos_los_formatos_van_en_vertical() {
        for f in PaperFormat::TODOS {
            let (w, h) = f.size_mm();
            assert!(w < h, "{f:?}");
        }
    }

    #[test]
    fn apaisado_intercambia_lados() {
        assert_eq!(PaperFormat::A4.oriented_mm(true), (297.0, 210.0));
        assert_eq!(PaperFormat::Letter.oriented_mm(false), (215.9, 279.4));
    }
}
