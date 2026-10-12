//! Decodificar a RGBA8, con la orientación EXIF aplicada.

use std::io::Cursor;

use image::{DynamicImage, ImageDecoder, ImageReader, Limits, RgbaImage};

use super::{ImagingError, LADO_MAXIMO};

pub fn decode(bytes: &[u8]) -> Result<RgbaImage, ImagingError> {
    let mut lector = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| ImagingError::Corrupt(e.to_string()))?;
    if lector.format().is_none() {
        return Err(ImagingError::Unsupported);
    }
    let mut limites = Limits::default();
    limites.max_image_width = Some(LADO_MAXIMO);
    limites.max_image_height = Some(LADO_MAXIMO);
    lector.limits(limites);
    let mut decoder = lector.into_decoder().map_err(error)?;
    let orientacion = decoder.orientation().ok();
    let mut img = DynamicImage::from_decoder(decoder).map_err(error)?;
    if let Some(o) = orientacion {
        img.apply_orientation(o);
    }
    Ok(img.into_rgba8())
}

fn error(e: image::ImageError) -> ImagingError {
    match e {
        image::ImageError::Limits(_) => ImagingError::TooLarge,
        image::ImageError::Unsupported(_) => ImagingError::Unsupported,
        otro => ImagingError::Corrupt(otro.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::encode;

    #[test]
    fn png_y_jpeg_salen_en_rgba() {
        let mut img = RgbaImage::new(8, 6);
        for p in img.pixels_mut() {
            *p = image::Rgba([200, 10, 10, 255]);
        }
        let png = encode::png(&img).unwrap();
        let d = decode(&png).unwrap();
        assert_eq!(d.dimensions(), (8, 6));
        assert_eq!(d.get_pixel(3, 3).0, [200, 10, 10, 255]);
        let jpg = encode::jpeg(&img, 95).unwrap();
        let d = decode(&jpg).unwrap();
        assert_eq!(d.dimensions(), (8, 6));
        assert!(d.get_pixel(3, 3).0[0] > 180);
    }

    #[test]
    fn la_basura_no_entra_ni_revienta() {
        assert!(decode(b"no soy una imagen").is_err());
        assert!(decode(&[0x89, b'P', b'N', b'G', 0, 0, 0]).is_err());
    }

    #[test]
    fn una_imagen_gigante_se_rechaza() {
        // Cabecera PNG de 13.000 × 10 (no hace falta el resto: el límite salta antes).
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        v.extend_from_slice(&13u32.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&13_000u32.to_be_bytes());
        v.extend_from_slice(&10u32.to_be_bytes());
        v.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
        assert!(matches!(
            decode(&v),
            Err(ImagingError::TooLarge) | Err(ImagingError::Corrupt(_))
        ));
    }
}
