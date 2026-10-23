//! **Las dos ilustraciones de una carta de dos mitades.** Scryfall da un solo
//! `art_crop` para las partidas, las de Fusionar, las Habitaciones y las
//! Secuelas, con las dos ilustraciones una al lado de la otra (la de la
//! primera mitad a la izquierda) y siempre de pie, aunque en la carta vayan
//! giradas. Aquí se corta por la costura para que cada mitad lleve la suya.

use image::RgbaImage;

/// Dónde se juntan las dos en las partidas, Fusionar y Habitaciones
/// (fracción del ancho): a la mitad.
pub const COSTURA_PARTIDA: f32 = 0.5;
/// En las Secuelas: la de arriba es más ancha (Dusk // Dawn, Cut // Ribbons,
/// Never // Return y Commit // Memory la tienen al 61,2 %).
pub const COSTURA_SECUELA: f32 = 0.612;
/// Cuánto se busca a cada lado de lo esperado (fracción del ancho).
const VENTANA: f32 = 0.03;

/// La columna donde empieza la segunda ilustración, cerca de `esperada`
/// (fracción del ancho): la del mayor salto de color con la de su izquierda,
/// tomando la mediana de las filas. Entre dos cuadros distintos salta en casi
/// todas; un borde fuerte dentro de un cuadro, solo en unas pocas (con la
/// suma, Commit // Memory se iba 12 px).
pub fn costura(img: &RgbaImage, esperada: f32) -> u32 {
    let (w, h) = img.dimensions();
    let centro = (w as f32 * esperada).round() as i64;
    if w < 4 || h == 0 {
        return centro.clamp(0, i64::from(w)) as u32;
    }
    let radio = (w as f32 * VENTANA).ceil() as i64;
    let desde = (centro - radio).max(1);
    let hasta = (centro + radio).min(i64::from(w) - 1);
    let salto = |x: u32| -> u16 {
        let mut filas: Vec<u16> = (0..h)
            .map(|y| {
                let (a, b) = (img.get_pixel(x - 1, y), img.get_pixel(x, y));
                (0..3).map(|c| u16::from(a[c].abs_diff(b[c]))).sum::<u16>()
            })
            .collect();
        let medio = filas.len() / 2;
        *filas.select_nth_unstable(medio).1
    };
    (desde..=hasta)
        .map(|x| x as u32)
        .max_by_key(|&x| (salto(x), std::cmp::Reverse((i64::from(x) - centro).abs())))
        .unwrap_or(centro as u32)
}

/// La ilustración de la mitad `cual` (0, la de la izquierda; 1, la de la
/// derecha), sin unos píxeles a cada lado de la costura (el JPEG la emborrona).
pub fn mitad(img: &RgbaImage, esperada: f32, cual: usize) -> RgbaImage {
    let (w, h) = img.dimensions();
    let x = costura(img, esperada);
    let margen = (w / 200).max(2);
    let (x0, x1) = if cual == 0 {
        (0, x.saturating_sub(margen))
    } else {
        ((x + margen).min(w), w)
    };
    if x1 <= x0 {
        return img.clone();
    }
    image::imageops::crop_imm(img, x0, 0, x1 - x0, h).to_image()
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    /// Dos «cuadros» con textura (rayas finas) que cambian en `corte`.
    fn doble(w: u32, h: u32, corte: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            let raya = if (x / 3 + y / 5) % 2 == 0 { 20 } else { 0 };
            if x < corte {
                Rgba([180 + raya, 60, 40, 255])
            } else {
                Rgba([40, 90, 170 + raya, 255])
            }
        })
    }

    #[test]
    fn la_costura_se_encuentra_aunque_no_este_justo_donde_se_espera() {
        let img = doble(808, 280, 413);
        assert_eq!(costura(&img, COSTURA_PARTIDA), 413);
        let img = doble(1024, 231, 627);
        assert_eq!(costura(&img, COSTURA_SECUELA), 627);
    }

    #[test]
    fn cada_mitad_lleva_solo_su_cuadro() {
        let img = doble(808, 280, 404);
        let a = mitad(&img, COSTURA_PARTIDA, 0);
        let b = mitad(&img, COSTURA_PARTIDA, 1);
        assert_eq!(a.height(), 280);
        assert!(a.width() < 404 && a.width() > 390, "{}", a.width());
        assert!(b.width() < 404 && b.width() > 390, "{}", b.width());
        assert!(a.pixels().all(|p| p[0] > p[2]));
        assert!(b.pixels().all(|p| p[2] > p[0]));
    }

    #[test]
    fn una_imagen_minima_no_rompe() {
        // Sin sitio para cortar, la imagen entera.
        let img = RgbaImage::new(2, 2);
        assert_eq!(mitad(&img, COSTURA_PARTIDA, 1).dimensions(), (2, 2));
        let vacia = RgbaImage::new(0, 0);
        assert_eq!(mitad(&vacia, COSTURA_PARTIDA, 0).dimensions(), (0, 0));
    }
}
