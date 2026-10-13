//! **Encuadre**: llevar cualquier imagen al tamaño exacto de la carta.
//! Giro (sin pérdidas) → caja de recorte (zoom y desplazamiento) →
//! remuestreo Lanczos3 al hueco exacto.
//!
//! 🛑 **La carta se estira para llenar el hueco** (Pablo, 13/10/2026): una
//! imagen de proporción algo distinta a 63×88 se deforma un poco en vez de
//! perder la línea del nombre o el pie por un recorte. Solo si la proporción
//! se aleja más de [`MAX_ESTIRADO`] (una foto apaisada sin girar, por
//! ejemplo) se recorta para cubrir, porque estirarla la destrozaría.

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{imageops, RgbaImage};

use super::ImagingError;
use crate::model::{CardSize, Crop, Rotation};
use crate::units::mm_to_px;

/// Para qué se renderiza: el PDF pide una resolución; la pantalla, un ancho.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Dpi(f64),
    /// Ancho del trim en píxeles (la miniatura).
    Width(u32),
}

/// Medidas del trim en píxeles. 🛑 La única función que las calcula.
pub fn trim_px(card: CardSize, target: Target) -> (u32, u32) {
    match target {
        Target::Dpi(d) => (mm_to_px(card.width_mm, d), mm_to_px(card.height_mm, d)),
        Target::Width(w) => {
            let h = (f64::from(w) * card.height_mm / card.width_mm).round() as u32;
            (w.max(1), h.max(1))
        }
    }
}

/// Píxeles por milímetro con ese trim (para los radios de los retoques y el sangrado).
pub fn px_por_mm(card: CardSize, trim_w: u32) -> f64 {
    f64::from(trim_w) / card.width_mm
}

pub fn rotar(img: RgbaImage, r: Rotation) -> RgbaImage {
    match r {
        Rotation::R0 => img,
        Rotation::R90 => imageops::rotate90(&img),
        Rotation::R180 => imageops::rotate180(&img),
        Rotation::R270 => imageops::rotate270(&img),
    }
}

/// Diferencia de proporción (relativa) hasta la que se estira en vez de recortar.
pub const MAX_ESTIRADO: f64 = 0.15;

/// ¿Se estira esta imagen al hueco (en vez de recortarla para cubrirlo)?
pub fn se_estira(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> bool {
    let a = f64::from(src_w) / f64::from(src_h.max(1));
    let b = f64::from(dst_w) / f64::from(dst_h.max(1));
    (a / b - 1.0).abs() <= MAX_ESTIRADO
}

/// La caja de recorte (left, top, w, h) en píxeles de origen. Sin zoom es la
/// imagen entera (que luego se estira al hueco); si la proporción está muy
/// lejos, la mayor caja con la proporción del hueco.
pub fn caja(src_w: u32, src_h: u32, dst_w: u32, dst_h: u32, crop: Crop) -> (f64, f64, f64, f64) {
    let (sw, sh) = (f64::from(src_w), f64::from(src_h));
    let aspecto = f64::from(dst_w) / f64::from(dst_h);
    let (mut cw, mut ch) = if se_estira(src_w, src_h, dst_w, dst_h) {
        (sw, sh)
    } else if sw / sh > aspecto {
        (sh * aspecto, sh)
    } else {
        (sw, sw / aspecto)
    };
    let z = crop.zoom.max(1.0);
    cw /= z;
    ch /= z;
    let mx = (sw - cw) / 2.0;
    let my = (sh - ch) / 2.0;
    let left = mx + crop.offset_x.clamp(-1.0, 1.0) * mx;
    let top = my + crop.offset_y.clamp(-1.0, 1.0) * my;
    (left, top, cw, ch)
}

pub fn fit(
    img: RgbaImage,
    rotation: Rotation,
    crop: Crop,
    dst_w: u32,
    dst_h: u32,
) -> Result<RgbaImage, ImagingError> {
    let img = rotar(img, rotation);
    let (sw, sh) = img.dimensions();
    let (l, t, cw, ch) = caja(sw, sh, dst_w, dst_h, crop);
    let src = ImageRef::new(sw, sh, img.as_raw(), PixelType::U8x4)
        .map_err(|e| ImagingError::Encode(e.to_string()))?;
    let mut dst = Image::new(dst_w, dst_h, PixelType::U8x4);
    let opciones = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3))
        .crop(l, t, cw, ch)
        .use_alpha(false);
    Resizer::new()
        .resize(&src, &mut dst, &opciones)
        .map_err(|e| ImagingError::Encode(e.to_string()))?;
    RgbaImage::from_raw(dst_w, dst_h, dst.into_vec())
        .ok_or_else(|| ImagingError::Encode("búfer de tamaño inesperado".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    #[test]
    fn medidas_del_trim() {
        assert_eq!(trim_px(CardSize::MTG, Target::Dpi(600.0)), (1488, 2079));
        assert_eq!(trim_px(CardSize::MTG, Target::Width(745)), (745, 1041));
        assert_eq!(trim_px(CardSize::MTG, Target::Width(488)), (488, 682));
    }

    #[test]
    fn el_centro_cae_en_el_centro() {
        let mut img = RgbaImage::from_pixel(745, 1040, Rgba([0, 0, 0, 255]));
        for y in 510..530 {
            for x in 362..382 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let r = fit(img, Rotation::R0, Crop::default(), 1488, 2079).unwrap();
        assert_eq!(r.dimensions(), (1488, 2079));
        assert!(r.get_pixel(744, 1039).0[0] > 200);
        assert!(r.get_pixel(100, 100).0[0] < 20);
    }

    #[test]
    fn girar_una_imagen_apaisada_la_pone_vertical() {
        let img = RgbaImage::from_pixel(1040, 745, Rgba([9, 9, 9, 255]));
        let r = rotar(img, Rotation::R90);
        assert_eq!(r.dimensions(), (745, 1040));
    }

    #[test]
    fn el_desplazamiento_no_deja_franjas_vacias() {
        for (ox, oy) in [(-1.0, -1.0), (1.0, 1.0), (1.0, -1.0)] {
            let (l, t, w, h) = caja(
                1000,
                1000,
                630,
                880,
                Crop {
                    zoom: 1.5,
                    offset_x: ox,
                    offset_y: oy,
                },
            );
            assert!(l >= -1e-9 && t >= -1e-9);
            assert!(l + w <= 1000.0 + 1e-9 && t + h <= 1000.0 + 1e-9);
        }
    }

    #[test]
    fn una_proporcion_parecida_se_estira_entera() {
        // 700×1040 es más estrecha que 63×88: no se corta nada, se estira.
        let (l, t, w, h) = caja(700, 1040, 630, 880, Crop::default());
        assert_eq!((l, t, w, h), (0.0, 0.0, 700.0, 1040.0));
        // Con zoom, la caja conserva la proporción de la imagen.
        let (_, _, w, h) = caja(
            700,
            1040,
            630,
            880,
            Crop {
                zoom: 2.0,
                ..Crop::default()
            },
        );
        assert!((w - 350.0).abs() < 1e-9 && (h - 520.0).abs() < 1e-9);
    }

    #[test]
    fn la_linea_del_nombre_no_se_pierde() {
        // Una imagen más alta que la carta: arriba una franja roja (el nombre).
        let mut img = RgbaImage::from_pixel(700, 1100, Rgba([0, 0, 0, 255]));
        for y in 0..20 {
            for x in 0..700 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let r = fit(img, Rotation::R0, Crop::default(), 630, 880).unwrap();
        assert!(r.get_pixel(315, 3).0[0] > 200, "la franja de arriba sigue");
    }

    #[test]
    fn una_imagen_ancha_se_recorta_por_los_lados() {
        let (l, t, w, h) = caja(2000, 1000, 630, 880, Crop::default());
        assert!((h - 1000.0).abs() < 1e-9 && t.abs() < 1e-9);
        assert!((w - 1000.0 * 630.0 / 880.0).abs() < 1e-6);
        assert!((l - (2000.0 - w) / 2.0).abs() < 1e-6);
    }
}
