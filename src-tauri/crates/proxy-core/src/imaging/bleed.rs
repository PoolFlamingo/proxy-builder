//! **Sangrado**: ampliar la imagen `b` píxeles por cada lado, repitiendo el
//! píxel del canto (o en espejo), para que un corte imperfecto no deje filos.
//!
//! El render lleva siempre el sangrado completo en los cuatro lados; el
//! recorte entre cartas vecinas lo hace el PDF (y la pantalla) con la caja de
//! cada casilla.

use image::RgbaImage;
use rayon::prelude::*;

use crate::model::BleedMode;

fn indice(i: i64, n: i64, modo: BleedMode) -> u32 {
    let j = match modo {
        BleedMode::Clamp => i.clamp(0, n - 1),
        BleedMode::Mirror => {
            let p = 2 * n;
            let m = i.rem_euclid(p);
            if m < n {
                m
            } else {
                p - 1 - m
            }
        }
    };
    j as u32
}

pub fn extend(img: &RgbaImage, b: u32, modo: BleedMode) -> RgbaImage {
    if b == 0 {
        return img.clone();
    }
    let (w, h) = img.dimensions();
    let (nw, nh) = (w + 2 * b, h + 2 * b);
    let mut out = RgbaImage::new(nw, nh);
    let src = img.as_raw();
    out.as_mut()
        .par_chunks_mut(nw as usize * 4)
        .enumerate()
        .for_each(|(y, fila)| {
            let sy = indice(y as i64 - i64::from(b), i64::from(h), modo);
            for x in 0..nw {
                let sx = indice(i64::from(x) - i64::from(b), i64::from(w), modo);
                let s = ((sy * w + sx) * 4) as usize;
                let d = (x * 4) as usize;
                fila[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn a_600_ppp_un_milimetro_son_24_px_por_lado() {
        let img = RgbaImage::from_fn(1488, 2079, |x, _| Rgba([(x % 256) as u8, 0, 0, 255]));
        let r = extend(&img, 24, BleedMode::Clamp);
        assert_eq!(r.dimensions(), (1536, 2127));
        // La fila 0 del sangrado repite la fila 0 de la imagen.
        assert_eq!(r.get_pixel(24 + 7, 0), img.get_pixel(7, 0));
        assert_eq!(r.get_pixel(0, 500), img.get_pixel(0, 476));
        assert_eq!(r.get_pixel(24 + 100, 24 + 100), img.get_pixel(100, 100));
    }

    #[test]
    fn en_espejo_se_refleja() {
        let img = RgbaImage::from_fn(10, 10, |x, _| Rgba([x as u8, 0, 0, 255]));
        let r = extend(&img, 3, BleedMode::Mirror);
        assert_eq!(r.get_pixel(0, 5).0[0], 2);
        assert_eq!(r.get_pixel(2, 5).0[0], 0);
    }

    #[test]
    fn sin_sangrado_es_la_misma_imagen() {
        let img = RgbaImage::from_pixel(5, 5, Rgba([1, 2, 3, 255]));
        assert_eq!(extend(&img, 0, BleedMode::Clamp), img);
    }
}
