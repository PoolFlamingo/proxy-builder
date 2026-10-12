//! **Los once retoques**, en un orden fijo y con radios en milímetros (así la
//! miniatura de 488 px y el PDF a 600 PPP se ven igual).
//!
//! Orden: reducción de ruido → exposición → brillo → contraste → calidez →
//! saturación → intensidad → claridad → nitidez → desvanecido → viñeta.

use image::RgbaImage;
use rayon::prelude::*;

use super::blur::gauss;
use crate::model::Adjustments;

const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Aplica los retoques. `px_mm` = píxeles por milímetro de esta imagen.
pub fn apply(img: &mut RgbaImage, a: &Adjustments, px_mm: f64) {
    if a.is_identity() {
        return;
    }
    let (w, h) = (img.width() as usize, img.height() as usize);
    let px_mm = px_mm as f32;
    let mut planos: [Vec<f32>; 3] = std::array::from_fn(|_| vec![0.0; w * h]);
    for (i, p) in img.pixels().enumerate() {
        for (plano, &v) in planos.iter_mut().zip(&p.0[..3]) {
            plano[i] = f32::from(v) / 255.0;
        }
    }

    // 1. Reducción de ruido: desenfoque suave (σ = 0,04 mm a tope).
    if a.noise_reduction > 0.0 {
        let s = 0.04 * px_mm * (a.noise_reduction as f32 / 100.0);
        for p in &mut planos {
            *p = gauss(p, w, h, s);
        }
    }

    // 2–7. Operaciones punto a punto.
    let exp = 2f32.powf(a.exposure as f32 / 50.0);
    let bri = a.brightness as f32 / 200.0;
    let con = 1.0 + a.contrast as f32 / 100.0;
    let cal = a.warmth as f32 / 400.0;
    let sat = 1.0 + a.saturation as f32 / 100.0;
    let vib = a.vibrance as f32 / 100.0;
    let [r, g, b] = &mut planos;
    r.par_iter_mut()
        .zip(g.par_iter_mut())
        .zip(b.par_iter_mut())
        .for_each(|((r, g), b)| {
            let mut px = [*r, *g, *b];
            for v in &mut px {
                *v = ((*v * exp + bri) - 0.5) * con + 0.5;
            }
            px[0] += cal;
            px[2] -= cal;
            let l = px[0] * LUMA[0] + px[1] * LUMA[1] + px[2] * LUMA[2];
            for v in &mut px {
                *v = l + (*v - l) * sat;
            }
            if vib != 0.0 {
                let max = px[0].max(px[1]).max(px[2]);
                let min = px[0].min(px[1]).min(px[2]);
                let s_actual = (max - min).clamp(0.0, 1.0);
                let f = 1.0 + vib * (1.0 - s_actual);
                let l = px[0] * LUMA[0] + px[1] * LUMA[1] + px[2] * LUMA[2];
                for v in &mut px {
                    *v = l + (*v - l) * f;
                }
            }
            (*r, *g, *b) = (px[0], px[1], px[2]);
        });

    // 8. Claridad: máscara de enfoque grande ponderada a tonos medios.
    if a.clarity > 0.0 {
        let s = 1.5 * px_mm;
        let k = 0.6 * a.clarity as f32 / 100.0;
        for p in &mut planos {
            let borroso = gauss(p, w, h, s);
            p.par_iter_mut().zip(borroso.par_iter()).for_each(|(v, b)| {
                let l = v.clamp(0.0, 1.0);
                *v += k * 4.0 * l * (1.0 - l) * (*v - b);
            });
        }
    }

    // 9. Nitidez: máscara de enfoque fina.
    if a.sharpness > 0.0 {
        let s = (0.08 * px_mm).max(0.5);
        let k = 1.5 * a.sharpness as f32 / 100.0;
        for p in &mut planos {
            let borroso = gauss(p, w, h, s);
            p.par_iter_mut()
                .zip(borroso.par_iter())
                .for_each(|(v, b)| *v += k * (*v - b));
        }
    }

    // 10 y 11. Desvanecido y viñeta.
    let fade = a.fade as f32 / 100.0;
    let vig = a.vignette as f32 / 100.0;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let dmax = (cx * cx + cy * cy).sqrt();
    let rgba = img.as_mut();
    rgba.par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, fila)| {
            for x in 0..w {
                let i = y * w + x;
                let factor = if vig > 0.0 {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() / dmax;
                    1.0 - vig * smoothstep(0.5, 1.0, d)
                } else {
                    1.0
                };
                for c in 0..3 {
                    let mut v = planos[c][i];
                    if fade > 0.0 {
                        v = v * (1.0 - fade / 2.0) + fade / 4.0;
                    }
                    v *= factor;
                    fila[x * 4 + c] = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn gris(v: u8) -> RgbaImage {
        RgbaImage::from_pixel(20, 20, Rgba([v, v, v, 255]))
    }

    #[test]
    fn sin_retoques_no_cambia_nada() {
        let mut img = RgbaImage::from_fn(30, 20, |x, y| {
            Rgba([(x * 8) as u8, (y * 12) as u8, 77, 255])
        });
        let antes = img.clone();
        apply(&mut img, &Adjustments::default(), 10.0);
        assert_eq!(img, antes);
    }

    #[test]
    fn brillo_a_tope_sobre_gris_medio_es_blanco() {
        let mut img = gris(128);
        apply(
            &mut img,
            &Adjustments {
                brightness: 100.0,
                ..Default::default()
            },
            10.0,
        );
        assert_eq!(img.get_pixel(5, 5).0[0], 255);
    }

    #[test]
    fn contraste_minimo_lo_deja_todo_en_gris_medio() {
        let mut img = RgbaImage::from_fn(10, 10, |x, _| Rgba([(x * 25) as u8, 0, 255, 255]));
        apply(
            &mut img,
            &Adjustments {
                contrast: -100.0,
                ..Default::default()
            },
            10.0,
        );
        assert!(img
            .pixels()
            .all(|p| p.0[..3].iter().all(|&c| (127..=128).contains(&c))));
    }

    #[test]
    fn saturacion_minima_es_escala_de_grises() {
        let mut img = RgbaImage::from_pixel(10, 10, Rgba([200, 40, 90, 255]));
        apply(
            &mut img,
            &Adjustments {
                saturation: -100.0,
                ..Default::default()
            },
            10.0,
        );
        let p = img.get_pixel(3, 3).0;
        assert!(p[0] == p[1] && p[1] == p[2], "{p:?}");
    }

    #[test]
    fn la_vineta_oscurece_las_esquinas_y_no_el_centro() {
        let mut img = RgbaImage::from_pixel(101, 101, Rgba([200, 200, 200, 255]));
        apply(
            &mut img,
            &Adjustments {
                vignette: 100.0,
                ..Default::default()
            },
            10.0,
        );
        assert_eq!(img.get_pixel(50, 50).0[0], 200);
        assert!(img.get_pixel(0, 0).0[0] < 20);
    }

    #[test]
    fn la_nitidez_no_cambia_una_zona_lisa() {
        let mut img = gris(90);
        apply(
            &mut img,
            &Adjustments {
                sharpness: 100.0,
                clarity: 100.0,
                noise_reduction: 100.0,
                ..Default::default()
            },
            23.6,
        );
        assert!(img.pixels().all(|p| (89..=91).contains(&p.0[0])));
    }

    /// `cargo test -p proxy-core --release -- --ignored retoques_rapidos --nocapture`
    #[test]
    #[ignore]
    fn retoques_rapidos() {
        let mut img = RgbaImage::from_fn(1488, 2079, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, 100, 255])
        });
        let todo = Adjustments {
            brightness: 10.0,
            exposure: 10.0,
            contrast: 10.0,
            saturation: 10.0,
            warmth: 10.0,
            vibrance: 10.0,
            sharpness: 50.0,
            clarity: 50.0,
            noise_reduction: 50.0,
            vignette: 30.0,
            fade: 20.0,
        };
        let t = std::time::Instant::now();
        apply(&mut img, &todo, 23.62);
        println!("once retoques a 600 PPP: {:?}", t.elapsed());
    }
}
