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
