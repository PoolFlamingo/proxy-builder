//! **Hacer ligera una imagen para compartirla**: un PNG sin transparencia pasa
//! a JPEG con las mismas medidas. Lo usa el `.pbdeck` «optimizado»; lo que hay
//! en el almacén de blobs de quien comparte no cambia.
//!
//! Un JPEG nunca se vuelve a comprimir (perdería calidad dos veces), y un PNG
//! con transparencia se queda como está: las esquinas de las cartas salen del
//! color del papel. Tampoco se toca lo que apenas ganaría: un icono pequeño o
//! de colores planos se ve peor y no ahorra nada.

use super::{decode, encode};

const FIRMA_PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// Por debajo de esto no compensa perder nada.
const PNG_MINIMO: usize = 64 * 1024;

/// El JPEG de este PNG, si no tiene transparencia y pesa al menos un 10 %
/// menos. `None`: la imagen se queda como está (también si no se puede leer:
/// el paquete la lleva tal cual y quien la reciba decide).
pub fn png_a_jpeg(bytes: &[u8], calidad: u8) -> Option<Vec<u8>> {
    if bytes.len() < PNG_MINIMO || !bytes.starts_with(FIRMA_PNG) {
        return None;
    }
    let img = decode::decode(bytes).ok()?;
    // Con una orientación EXIF, el deck apunta otras medidas: mejor no tocarlo.
    let medidas = imagesize::blob_size(bytes).ok()?;
    if (medidas.width as u32, medidas.height as u32) != img.dimensions() {
        return None;
    }
    if img.pixels().any(|p| p.0[3] < u8::MAX) {
        return None;
    }
    let jpeg = encode::jpeg(&img, calidad).ok()?;
    (jpeg.len() <= bytes.len() / 10 * 9).then_some(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    /// Una imagen con detalle: como PNG pesa mucho más que como JPEG.
    fn foto(alfa: u8) -> RgbaImage {
        let mut semilla = 7u32;
        RgbaImage::from_fn(400, 300, |x, y| {
            semilla = semilla.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let ruido = (semilla >> 28) as u8;
            Rgba([
                (x / 2) as u8 + ruido,
                (y / 2) as u8 + ruido,
                ((x + y) / 4) as u8 + ruido,
                alfa,
            ])
        })
    }

    #[test]
    fn un_png_opaco_pasa_a_jpeg_con_las_mismas_medidas() {
        let png = encode::png(&foto(u8::MAX)).unwrap();
        let jpeg = png_a_jpeg(&png, 92).unwrap();
        assert!(jpeg.starts_with(&[0xFF, 0xD8]));
        assert!(jpeg.len() < png.len());
        assert_eq!(decode::decode(&jpeg).unwrap().dimensions(), (400, 300));
        // El mismo PNG da siempre el mismo JPEG (y la misma huella).
        assert_eq!(png_a_jpeg(&png, 92).unwrap(), jpeg);
    }

    #[test]
    fn con_transparencia_se_queda_como_esta() {
        let mut img = foto(u8::MAX);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
        let png = encode::png(&img).unwrap();
        assert!(png.len() >= PNG_MINIMO);
        assert_eq!(png_a_jpeg(&png, 92), None);
    }

    #[test]
    fn un_jpeg_no_se_vuelve_a_comprimir() {
        let jpeg = encode::jpeg(&foto(u8::MAX), 100).unwrap();
        assert!(jpeg.len() >= PNG_MINIMO);
        assert_eq!(png_a_jpeg(&jpeg, 50), None);
    }

    #[test]
    fn lo_pequeno_o_lo_que_apenas_gana_se_queda_como_esta() {
        let pequeno = encode::png(&RgbaImage::from_pixel(8, 8, Rgba([9, 9, 9, 255]))).unwrap();
        assert_eq!(png_a_jpeg(&pequeno, 92), None);
        assert_eq!(png_a_jpeg(b"no soy una imagen", 92), None);
        // Basura con cabecera de PNG y tamaño de sobra: no revienta.
        let mut roto = FIRMA_PNG.to_vec();
        roto.resize(PNG_MINIMO, 0);
        assert_eq!(png_a_jpeg(&roto, 92), None);
    }
}
