//! **Oscurecer bordes**: igualar a negro los bordes grisáceos de escaneos
//! viejos, sin tocar cartas de borde blanco, sin borde o de arte completo.

use image::RgbaImage;

use crate::model::{EdgeDarkening, EdgeMode};

fn luma(p: [u8; 4]) -> f32 {
    (0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])) / 255.0
}

fn saturacion(p: [u8; 4]) -> f32 {
    let max = p[0].max(p[1]).max(p[2]);
    let min = p[0].min(p[1]).min(p[2]);
    f32::from(max - min) / 255.0
}

/// Un píxel «de borde»: oscuro y casi sin color.
fn es_de_borde(p: [u8; 4]) -> bool {
    luma(p) < 0.40 && saturacion(p) < 0.25
}

/// Detección automática: ¿esta carta tiene un borde oscuro uniforme?
/// Mira el anillo exterior del 1,5 %: borde claro (luma media > 0,55) o muy
/// variado (desviación > 0,12, arte a sangre) → no se toca.
pub fn tiene_borde_oscuro(img: &RgbaImage) -> bool {
    let (w, h) = img.dimensions();
    let a = ((w as f32) * 0.015).round().max(1.0) as u32;
    let mut v = Vec::new();
    let paso = (w.min(h) / 150).max(1);
    for d in 0..a {
        let mut x = 0;
        while x < w {
            v.push(luma(img.get_pixel(x, d).0));
            v.push(luma(img.get_pixel(x, h - 1 - d).0));
            x += paso;
        }
        let mut y = 0;
        while y < h {
            v.push(luma(img.get_pixel(d, y).0));
            v.push(luma(img.get_pixel(w - 1 - d, y).0));
            y += paso;
        }
    }
    if v.is_empty() {
        return false;
    }
    let n = v.len() as f32;
    let media = v.iter().sum::<f32>() / n;
    let desv = (v.iter().map(|x| (x - media).powi(2)).sum::<f32>() / n).sqrt();
    media <= 0.55 && desv <= 0.12
}

/// Aplica el oscurecido. Devuelve `false` si la detección automática lo descartó.
pub fn apply(img: &mut RgbaImage, e: &EdgeDarkening) -> bool {
    if e.auto_detect && !tiene_borde_oscuro(img) {
        return false;
    }
    let (w, h) = img.dimensions();
    let banda = ((w as f64) * e.width_pct / 100.0).round().max(1.0) as u32;
    let cantidad = (e.amount / 100.0).clamp(0.0, 1.0) as f32;

    // Máscara de la zona de borde: desde cada canto hacia dentro, mientras el
    // píxel sea «de borde» y sin pasar de la banda.
    let mut zona = vec![false; (w * h) as usize];
    let marcar = |x: u32, y: u32, zona: &mut Vec<bool>| zona[(y * w + x) as usize] = true;
    for y in 0..h {
        for x in 0..banda.min(w) {
            if !es_de_borde(img.get_pixel(x, y).0) {
                break;
            }
            marcar(x, y, &mut zona);
        }
        for x in (w.saturating_sub(banda)..w).rev() {
            if !es_de_borde(img.get_pixel(x, y).0) {
                break;
            }
            marcar(x, y, &mut zona);
        }
    }
    for x in 0..w {
        for y in 0..banda.min(h) {
            if !es_de_borde(img.get_pixel(x, y).0) {
                break;
            }
            marcar(x, y, &mut zona);
        }
        for y in (h.saturating_sub(banda)..h).rev() {
            if !es_de_borde(img.get_pixel(x, y).0) {
                break;
            }
            marcar(x, y, &mut zona);
        }
    }

    for y in 0..h {
        for x in 0..w {
            let en_banda = x < banda || y < banda || x >= w - banda || y >= h - banda;
            if !en_banda {
                continue;
            }
            let p = img.get_pixel_mut(x, y);
            let en_zona = zona[(y * w + x) as usize];
            match e.mode {
                EdgeMode::SolidBlack if en_zona => p.0[..3].fill(0),
                EdgeMode::ContrastEdges if en_zona => {
                    for c in &mut p.0[..3] {
                        *c = (f32::from(*c) * (1.0 - cantidad)).round() as u8;
                    }
                }
                EdgeMode::Threshold if luma(p.0) < 0.5 * cantidad => p.0[..3].fill(0),
                _ => {}
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::corners::tests::carta;

    fn ajustes(mode: EdgeMode) -> EdgeDarkening {
        EdgeDarkening {
            enabled: true,
            mode,
            ..Default::default()
        }
    }

    #[test]
    fn negro_solido_pone_el_borde_a_cero_y_no_toca_el_interior() {
        let mut img = carta(400, 560, [51, 51, 51, 255], [51, 51, 51, 255]);
        assert!(apply(&mut img, &ajustes(EdgeMode::SolidBlack)));
        assert_eq!(img.get_pixel(5, 200).0[..3], [0, 0, 0]);
        assert_eq!(img.get_pixel(200, 280).0[..3], [180, 170, 160]);
    }

    #[test]
    fn contraste_oscurece_segun_la_cantidad() {
        let mut img = carta(400, 560, [50, 50, 50, 255], [50, 50, 50, 255]);
        apply(&mut img, &ajustes(EdgeMode::ContrastEdges));
        assert_eq!(img.get_pixel(5, 200).0[0], 10);
    }

    #[test]
    fn una_carta_de_borde_blanco_no_se_toca() {
        let mut img = carta(400, 560, [245, 245, 245, 255], [245, 245, 245, 255]);
        let antes = img.clone();
        assert!(!apply(&mut img, &ajustes(EdgeMode::SolidBlack)));
        assert_eq!(img, antes);
    }

    #[test]
    fn un_arte_a_sangre_no_se_toca() {
        let mut img = RgbaImage::from_fn(400, 560, |x, y| {
            image::Rgba([
                ((x * 37 + y * 11) % 256) as u8,
                ((x * 7) % 256) as u8,
                ((y * 13) % 256) as u8,
                255,
            ])
        });
        let antes = img.clone();
        assert!(!apply(&mut img, &ajustes(EdgeMode::SolidBlack)));
        assert_eq!(img, antes);
    }
}
