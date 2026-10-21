//! Codificar a JPEG o PNG. El JPEG va sin alfa (las esquinas ya están
//! rellenas) y con submuestreo de color 4:2:0, que es lo de `image`.

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageEncoder, RgbaImage};

use super::ImagingError;

pub fn jpeg(img: &RgbaImage, calidad: u8) -> Result<Vec<u8>, ImagingError> {
    let rgb = DynamicImage::ImageRgba8(img.clone()).into_rgb8();
    let mut out = Vec::with_capacity(rgb.as_raw().len() / 6);
    JpegEncoder::new_with_quality(&mut out, calidad.clamp(1, 100))
        .encode_image(&rgb)
        .map_err(|e| ImagingError::Encode(e.to_string()))?;
    Ok(out)
}

pub fn png(img: &RgbaImage) -> Result<Vec<u8>, ImagingError> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| ImagingError::Encode(e.to_string()))?;
    Ok(out)
}

/// Los formatos en que se puede guardar una carta suelta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatoImagen {
    Png,
    Jpeg,
    Webp,
    Tiff,
    Bmp,
}

impl FormatoImagen {
    /// El formato de una extensión (`png`, `jpg`, `jpeg`, `webp`, `tif`…).
    pub fn de_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            "tif" | "tiff" => Some(Self::Tiff),
            "bmp" => Some(Self::Bmp),
            _ => None,
        }
    }

    /// ¿Guarda transparencia? Los que no, reciben la carta sobre blanco.
    pub fn con_alfa(self) -> bool {
        matches!(self, Self::Png | Self::Webp | Self::Tiff)
    }
}

/// Codifica una imagen en cualquiera de esos formatos. WebP va sin pérdida;
/// `calidad` solo cuenta para JPEG.
pub fn en_formato(
    img: &RgbaImage,
    formato: FormatoImagen,
    calidad: u8,
) -> Result<Vec<u8>, ImagingError> {
    let fallo = |e: image::ImageError| ImagingError::Encode(e.to_string());
    let mut sobre_blanco;
    let img = if formato.con_alfa() {
        img
    } else {
        sobre_blanco = img.clone();
        super::corners::sobre_blanco(&mut sobre_blanco);
        &sobre_blanco
    };
    let destino = match formato {
        FormatoImagen::Jpeg => return jpeg(img, calidad),
        FormatoImagen::Png => return png(img),
        FormatoImagen::Webp => image::ImageFormat::WebP,
        FormatoImagen::Tiff => image::ImageFormat::Tiff,
        FormatoImagen::Bmp => image::ImageFormat::Bmp,
    };
    let mut out = std::io::Cursor::new(Vec::new());
    let dinamica = if formato.con_alfa() {
        DynamicImage::ImageRgba8(img.clone())
    } else {
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(img.clone()).into_rgb8())
    };
    dinamica.write_to(&mut out, destino).map_err(fallo)?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::decode::decode;
    use image::Rgba;

    /// Una imagen con degradado y un hueco transparente, siempre igual.
    fn muestra() -> RgbaImage {
        RgbaImage::from_fn(31, 17, |x, y| {
            let a = if x < 4 && y < 4 { 0 } else { 255 };
            Rgba([(x * 8) as u8, (y * 15) as u8, 120, a])
        })
    }

    #[test]
    fn el_png_es_determinista_y_sin_perdida() {
        let img = muestra();
        let a = png(&img).unwrap();
        let b = png(&img).unwrap();
        // La misma imagen da los mismos bytes (las cachés y los PDF dependen de ello).
        assert_eq!(a, b);
        assert_eq!(&a[..8], b"\x89PNG\r\n\x1a\n");
        // Vuelve igual, alfa incluido.
        assert_eq!(decode(&a).unwrap(), img);
    }

    #[test]
    fn el_jpeg_es_determinista_y_pierde_el_alfa() {
        let img = muestra();
        let a = jpeg(&img, 90).unwrap();
        assert_eq!(a, jpeg(&img, 90).unwrap());
        assert_eq!(&a[..2], &[0xFF, 0xD8]);
        let d = decode(&a).unwrap();
        assert_eq!(d.dimensions(), (31, 17));
        // El JPEG no lleva alfa: lo transparente sale opaco.
        assert!(d.pixels().all(|p| p.0[3] == 255));
    }

    #[test]
    fn la_calidad_del_jpeg_se_recorta_al_rango() {
        let img = muestra();
        // 0 no es una calidad válida para el codificador: se usa 1.
        assert_eq!(jpeg(&img, 0).unwrap(), jpeg(&img, 1).unwrap());
        // Más calidad, más bytes.
        assert!(jpeg(&img, 100).unwrap().len() > jpeg(&img, 1).unwrap().len());
        assert_eq!(jpeg(&img, 255).unwrap(), jpeg(&img, 100).unwrap());
    }

    #[test]
    fn una_imagen_de_un_pixel_se_codifica() {
        let img = RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 4]));
        assert_eq!(decode(&png(&img).unwrap()).unwrap(), img);
        assert_eq!(
            decode(&jpeg(&img, 80).unwrap()).unwrap().dimensions(),
            (1, 1)
        );
    }

    #[test]
    fn todos_los_formatos_se_leen_de_vuelta() {
        let img = muestra();
        for (f, ext) in [
            (FormatoImagen::Png, "png"),
            (FormatoImagen::Jpeg, "jpg"),
            (FormatoImagen::Webp, "webp"),
            (FormatoImagen::Tiff, "tiff"),
            (FormatoImagen::Bmp, "bmp"),
        ] {
            assert_eq!(FormatoImagen::de_extension(ext), Some(f));
            let bytes = en_formato(&img, f, 90).unwrap();
            let leida = image::load_from_memory(&bytes).unwrap();
            assert_eq!((leida.width(), leida.height()), img.dimensions(), "{ext}");
        }
        assert_eq!(FormatoImagen::de_extension("gif"), None);
    }
}
