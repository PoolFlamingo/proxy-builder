//! **La rejilla de una página**: cuántas cartas caben y dónde, en milímetros.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::LayoutError;

/// Un rectángulo en milímetros, con origen arriba a la izquierda.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RectMm {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl RectMm {
    pub fn right(&self) -> f64 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }
}

/// Las casillas de una página: `cols × rows` cartas centradas en el papel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Grid {
    pub cols: u32,
    pub rows: u32,
    /// Esquina superior izquierda de la primera carta.
    pub origin_x: f64,
    pub origin_y: f64,
    /// Margen efectivo usado para calcular cuántas caben.
    pub margin_mm: f64,
}

impl Grid {
    pub fn capacity(&self) -> u32 {
        self.cols * self.rows
    }

    /// El trim (rectángulo de corte) de la casilla `(row, col)`, sin
    /// desplazamientos.
    pub fn cell(&self, row: u32, col: u32, card_w: f64, card_h: f64, gap: f64) -> RectMm {
        RectMm {
            x: self.origin_x + f64::from(col) * (card_w + gap),
            y: self.origin_y + f64::from(row) * (card_h + gap),
            w: card_w,
            h: card_h,
        }
    }
}

/// Tolerancia para no perder una columna por un error de redondeo
/// (215,9 − 2·5 + 2 dividido entre 65 no puede dar 2,99999).
const EPS: f64 = 1e-9;

/// Calcula la rejilla. El margen efectivo es `max(min_margin, bleed)`: el
/// sangrado exterior tiene que caber en el papel. La rejilla se centra.
pub fn grid(
    paper_w: f64,
    paper_h: f64,
    card_w: f64,
    card_h: f64,
    gap: f64,
    min_margin: f64,
    bleed: f64,
) -> Result<Grid, LayoutError> {
    if !(card_w > 0.0 && card_h > 0.0 && gap >= 0.0 && min_margin >= 0.0 && bleed >= 0.0) {
        return Err(LayoutError::InvalidSettings);
    }
    let m = min_margin.max(bleed);
    let cuantas = |papel: f64, carta: f64| -> u32 {
        let util = papel - 2.0 * m + gap;
        if util <= 0.0 {
            0
        } else {
            ((util / (carta + gap)) + EPS).floor() as u32
        }
    };
    let cols = cuantas(paper_w, card_w);
    let rows = cuantas(paper_h, card_h);
    if cols == 0 || rows == 0 {
        return Err(LayoutError::CardDoesNotFit);
    }
    let ancho = f64::from(cols) * card_w + f64::from(cols - 1) * gap;
    let alto = f64::from(rows) * card_h + f64::from(rows - 1) * gap;
    Ok(Grid {
        cols,
        rows,
        origin_x: (paper_w - ancho) / 2.0,
        origin_y: (paper_h - alto) / 2.0,
        margin_mm: m,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paper::PaperFormat;

    fn de(p: PaperFormat, landscape: bool) -> Grid {
        let (w, h) = p.oriented_mm(landscape);
        grid(w, h, 63.0, 88.0, 2.0, 5.0, 1.0).unwrap()
    }

    fn cerca(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn a4_vertical_son_3x3_con_el_origen_en_8_5_y_14_5() {
        let g = de(PaperFormat::A4, false);
        assert_eq!((g.cols, g.rows), (3, 3));
        assert!(cerca(g.origin_x, 8.5) && cerca(g.origin_y, 14.5), "{g:?}");
    }

    #[test]
    fn a4_vertical_a_la_medida_de_la_funda_siguen_siendo_3x3() {
        let (w, h) = PaperFormat::A4.oriented_mm(false);
        let g = grid(w, h, 65.0, 90.0, 2.0, 5.0, 1.0).unwrap();
        assert_eq!((g.cols, g.rows), (3, 3));
    }

    #[test]
    fn a4_apaisado_son_4x2() {
        let g = de(PaperFormat::A4, true);
        assert_eq!((g.cols, g.rows), (4, 2));
    }

    #[test]
    fn letter_vertical_son_3x3_y_el_origen_x_es_11_45() {
        let g = de(PaperFormat::Letter, false);
        assert_eq!((g.cols, g.rows), (3, 3));
        assert!(cerca(g.origin_x, 11.45), "{g:?}");
    }

    #[test]
    fn a5_son_2x2_y_foto_4x6_es_una() {
        let a5 = de(PaperFormat::A5, false);
        assert_eq!((a5.cols, a5.rows), (2, 2));
        let foto = de(PaperFormat::Photo4x6, false);
        assert_eq!((foto.cols, foto.rows), (1, 1));
    }

    #[test]
    fn la_segunda_casilla_va_un_ancho_mas_un_hueco_despues() {
        let g = de(PaperFormat::A4, false);
        let c = g.cell(1, 2, 63.0, 88.0, 2.0);
        assert!(cerca(c.x, 8.5 + 2.0 * 65.0) && cerca(c.y, 14.5 + 90.0));
    }

    #[test]
    fn una_carta_que_no_cabe_es_un_error() {
        assert_eq!(
            grid(50.0, 50.0, 63.0, 88.0, 2.0, 5.0, 1.0),
            Err(LayoutError::CardDoesNotFit)
        );
        assert_eq!(
            grid(210.0, 297.0, 0.0, 88.0, 2.0, 5.0, 1.0),
            Err(LayoutError::InvalidSettings)
        );
    }

    #[test]
    fn todos_los_formatos_caben_al_menos_una_carta() {
        for p in PaperFormat::TODOS {
            for l in [false, true] {
                let (w, h) = p.oriented_mm(l);
                let g = grid(w, h, 63.0, 88.0, 2.0, 5.0, 1.0).unwrap();
                assert!(g.capacity() >= 1, "{p:?} {l}");
                assert!(g.origin_x >= g.margin_mm - 1e-9 && g.origin_y >= g.margin_mm - 1e-9);
            }
        }
    }
}
