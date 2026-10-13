//! **Rellenar esquinas.** Los PNG de Scryfall traen las esquinas redondeadas
//! transparentes, y muchos escaneos las traen blancas: al recortar o sangrar
//! saldrían como triángulos claros. Se pintan del color del borde.

use image::{Rgba, RgbaImage};

/// Distancia (0..1) entre dos colores RGB.
fn distancia(a: [u8; 4], b: [u8; 4]) -> f32 {
    let d: f32 = (0..3)
        .map(|i| {
            let x = (f32::from(a[i]) - f32::from(b[i])) / 255.0;
            x * x
        })
        .sum();
    (d / 3.0).sqrt()
}

/// Color del borde: la mediana (por canal) de los píxeles opacos de un anillo
/// entre el 1 % y el 2,5 % del ancho desde el canto.
pub fn color_de_borde(img: &RgbaImage) -> [u8; 4] {
    let (w, h) = img.dimensions();
    let a = ((w as f32) * 0.01).round().max(1.0) as u32;
    let b = ((w as f32) * 0.025).round().max(2.0) as u32;
    let mut canales: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut meter = |x: u32, y: u32| {
        let p = img.get_pixel(x, y).0;
        if p[3] > 200 {
            for (i, c) in canales.iter_mut().enumerate() {
                c.push(p[i]);
            }
        }
    };
    let paso = (w.min(h) / 200).max(1);
    for d in a..b.min(w / 2).min(h / 2) {
        let mut x = 0;
        while x < w {
            meter(x, d);
            meter(x, h - 1 - d);
            x += paso;
        }
        let mut y = 0;
        while y < h {
            meter(d, y);
            meter(w - 1 - d, y);
            y += paso;
        }
    }
    let mediana = |v: &mut Vec<u8>| {
        if v.is_empty() {
            0
        } else {
            v.sort_unstable();
            v[v.len() / 2]
        }
    };
    let [r, g, bb] = &mut canales;
    [mediana(r), mediana(g), mediana(bb), 255]
}

/// Radio de la esquina redondeada, medido en la diagonal de la esquina
/// superior izquierda: el primer píxel opaco está a `r·(1 − 1/√2)` del vértice.
fn radio_por_alfa(img: &RgbaImage) -> Option<u32> {
    let (w, h) = img.dimensions();
    if img.get_pixel(0, 0).0[3] == 255 {
        return None;
    }
    let tope = (w.min(h) / 8).max(1);
    let d = (0..tope).find(|&d| img.get_pixel(d, d).0[3] == 255)?;
    let r = (d as f32 / (1.0 - std::f32::consts::FRAC_1_SQRT_2)).round() as u32;
    Some(r.clamp(1, tope))
}

/// Rellena esquinas transparentes y, en imágenes opacas, las esquinas claras
/// que no casan con el borde. Devuelve el color de borde usado.
pub fn fill(img: &mut RgbaImage) -> [u8; 4] {
    let borde = color_de_borde(img);
    let (w, h) = img.dimensions();

    // (0) Esquinas redondeadas con alfa (los PNG de Scryfall): todo lo que
    // queda fuera del círculo de la esquina —y un píxel y medio hacia dentro,
    // donde está el borde suavizado, que trae RGB claro y dejaba un arco gris
    // al imprimir— se pinta del color del borde.
    if let Some(r) = radio_por_alfa(img) {
        let r_f = r as f32;
        let limite = (r_f - 1.5).max(0.0);
        for (cx, cy, x0, y0) in [
            (r_f, r_f, 0u32, 0u32),
            (w as f32 - 1.0 - r_f, r_f, w - r, 0),
            (r_f, h as f32 - 1.0 - r_f, 0, h - r),
            (w as f32 - 1.0 - r_f, h as f32 - 1.0 - r_f, w - r, h - r),
        ] {
            for y in y0..(y0 + r).min(h) {
                for x in x0..(x0 + r).min(w) {
                    let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                    if d > limite {
                        img.put_pixel(x, y, Rgba(borde));
                    }
                }
            }
        }
    }

    // (1) Lo transparente que quede se compone sobre el color del borde.
    let mut hubo_alfa = false;
    for p in img.pixels_mut() {
        let a = u32::from(p.0[3]);
        if a < 255 {
            hubo_alfa = true;
            for (c, &b) in p.0[..3].iter_mut().zip(&borde[..3]) {
                *c = ((u32::from(*c) * a + u32::from(b) * (255 - a)) / 255) as u8;
            }
            p.0[3] = 255;
        }
    }
    if hubo_alfa {
        return borde;
    }

    // (2) Sin alfa: inundar desde cada vértice, sin salir de una caja del 4 %.
    let caja = ((w as f32) * 0.04).round().max(2.0) as u32;
    for (vx, vy) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
        let semilla = img.get_pixel(vx, vy).0;
        if distancia(semilla, borde) <= 0.25 {
            continue;
        }
        let x0 = if vx == 0 { 0 } else { w - caja };
        let y0 = if vy == 0 { 0 } else { h - caja };
        let mut pila = vec![(vx, vy)];
        let mut visto = vec![false; (caja * caja) as usize];
        while let Some((x, y)) = pila.pop() {
            let i = ((y - y0) * caja + (x - x0)) as usize;
            if visto[i] {
                continue;
            }
            visto[i] = true;
            if distancia(img.get_pixel(x, y).0, semilla) > 0.12 {
                continue;
            }
            img.put_pixel(x, y, Rgba(borde));
            if x > x0 {
                pila.push((x - 1, y));
            }
            if x + 1 < x0 + caja {
                pila.push((x + 1, y));
            }
            if y > y0 {
                pila.push((x, y - 1));
            }
            if y + 1 < y0 + caja {
                pila.push((x, y + 1));
            }
        }
    }
    borde
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Una carta de mentira: borde de 35 px del color dado, interior gris
    /// claro, y esquinas redondeadas de radio 30 con el color `esquina`.
    pub fn carta(w: u32, h: u32, borde: [u8; 4], esquina: [u8; 4]) -> RgbaImage {
        let mut img = RgbaImage::from_pixel(w, h, Rgba([180, 170, 160, 255]));
        let g = 35;
        for y in 0..h {
            for x in 0..w {
                if x < g || y < g || x >= w - g || y >= h - g {
                    img.put_pixel(x, y, Rgba(borde));
                }
            }
        }
        let r = 30i64;
        for (cx, cy) in [
            (r, r),
            (w as i64 - 1 - r, r),
            (r, h as i64 - 1 - r),
            (w as i64 - 1 - r, h as i64 - 1 - r),
        ] {
            for y in 0..h as i64 {
                for x in 0..w as i64 {
                    let fuera_x = (x < r && cx == r) || (x > w as i64 - 1 - r && cx != r);
                    let fuera_y = (y < r && cy == r) || (y > h as i64 - 1 - r && cy != r);
                    if fuera_x && fuera_y && (x - cx).pow(2) + (y - cy).pow(2) > r * r {
                        img.put_pixel(x as u32, y as u32, Rgba(esquina));
                    }
                }
            }
        }
        img
    }

    #[test]
    fn las_esquinas_transparentes_se_pintan_del_borde() {
        let mut img = carta(745, 1040, [10, 10, 10, 255], [0, 0, 0, 0]);
        let b = fill(&mut img);
        assert!(b[0] < 20);
        assert!(img.pixels().all(|p| p.0[3] == 255));
        assert!(img.get_pixel(0, 0).0[0] < 20);
        assert!(img.get_pixel(744, 1039).0[0] < 20);
    }

    #[test]
    fn el_borde_suavizado_de_la_esquina_no_deja_un_arco_claro() {
        // Esquina transparente con un anillo semitransparente BLANCO, como el
        // suavizado de los PNG de Scryfall.
        let mut img = carta(745, 1040, [10, 10, 10, 255], [0, 0, 0, 0]);
        let r = 30.0f32;
        for y in 0..31u32 {
            for x in 0..31u32 {
                let d = ((x as f32 - r).powi(2) + (y as f32 - r).powi(2)).sqrt();
                if (r - 0.8..=r + 0.8).contains(&d) {
                    img.put_pixel(x, y, Rgba([255, 255, 255, 120]));
                }
            }
        }
        fill(&mut img);
        for y in 0..31u32 {
            for x in 0..31u32 {
                assert!(
                    img.get_pixel(x, y).0[0] < 40,
                    "({x},{y}) = {:?}",
                    img.get_pixel(x, y)
                );
            }
        }
    }

    #[test]
    fn las_esquinas_blancas_de_un_escaneo_se_pintan_del_borde() {
        let mut img = carta(745, 1040, [12, 12, 12, 255], [255, 255, 255, 255]);
        fill(&mut img);
        assert!(img.get_pixel(0, 0).0[0] < 30, "{:?}", img.get_pixel(0, 0));
        assert!(img.get_pixel(744, 0).0[0] < 30);
        // El interior no se toca.
        assert_eq!(img.get_pixel(372, 520).0, [180, 170, 160, 255]);
    }

    #[test]
    fn una_carta_de_borde_blanco_no_cambia() {
        let mut img = carta(745, 1040, [250, 250, 250, 255], [255, 255, 255, 255]);
        let antes = img.clone();
        fill(&mut img);
        assert_eq!(img, antes);
    }
}
