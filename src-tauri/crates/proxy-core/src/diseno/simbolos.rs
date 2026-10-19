//! **Símbolos de maná** y demás (`{T}`, `{2/U}`, `{W/P}`…): los SVG de
//! Scryfall (`/symbology`), que la app descarga y guarda. Aquí solo se
//! interpretan y se pintan.

use std::collections::HashMap;
use std::sync::Arc;

use image::RgbaImage;
use resvg::tiny_skia::{self, Paint, PathBuilder, PixmapMut, PixmapPaint, Transform};
use resvg::usvg;

use super::DisenoError;

/// Los símbolos cargados, por su código sin llaves («T», «2/U», «W/P»). Los
/// de un pack de marcos (`propios`) tapan a los de Scryfall.
#[derive(Default, Clone)]
pub struct Simbolos {
    arboles: Arc<HashMap<String, usvg::Tree>>,
    propios: Option<Arc<SimbolosPropios>>,
}

/// Los símbolos propios de un pack de marcos (las fichas de póker del pack
/// Balatro): SVG o PNG por código. Los que no trae, los de Scryfall.
#[derive(Default)]
pub struct SimbolosPropios {
    arboles: HashMap<String, usvg::Tree>,
    imagenes: HashMap<String, RgbaImage>,
    /// Pixel art: al reescalar, sin suavizar.
    pixel: bool,
}

impl std::fmt::Debug for SimbolosPropios {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut c: Vec<&String> = self.arboles.keys().chain(self.imagenes.keys()).collect();
        c.sort();
        f.debug_struct("SimbolosPropios")
            .field("codigos", &c)
            .finish()
    }
}

impl SimbolosPropios {
    pub fn nuevos(pixel: bool) -> SimbolosPropios {
        SimbolosPropios {
            pixel,
            ..SimbolosPropios::default()
        }
    }

    pub fn cargar_svg(&mut self, simbolo: &str, svg: &[u8]) -> Result<(), DisenoError> {
        let arbol = usvg::Tree::from_data(svg, &usvg::Options::default())
            .map_err(|e| DisenoError::SimboloNoValido(format!("{simbolo}: {e}")))?;
        self.arboles.insert(codigo(simbolo), arbol);
        Ok(())
    }

    pub fn cargar_png(&mut self, simbolo: &str, png: &[u8]) -> Result<(), DisenoError> {
        let img = image::load_from_memory(png)
            .map_err(|e| DisenoError::SimboloNoValido(format!("{simbolo}: {e}")))?
            .to_rgba8();
        self.imagenes.insert(codigo(simbolo), img);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.arboles.len() + self.imagenes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn tiene(&self, c: &str) -> bool {
        self.arboles.contains_key(c) || self.imagenes.contains_key(c)
    }

    fn pintar(&self, c: &str, lienzo: &mut PixmapMut, x: f32, y: f32, lado: f32) -> bool {
        if let Some(arbol) = self.arboles.get(c) {
            pintar_arbol(arbol, lienzo, x, y, lado);
            return true;
        }
        let Some(img) = self.imagenes.get(c) else {
            return false;
        };
        // Sin deformar, centrado en el cuadrado.
        let escala = lado / img.width().max(img.height()).max(1) as f32;
        let (w, h) = (
            (img.width() as f32 * escala).round().max(1.0) as u32,
            (img.height() as f32 * escala).round().max(1.0) as u32,
        );
        let filtro = if !self.pixel {
            image::imageops::FilterType::Lanczos3
        } else if w >= img.width() {
            image::imageops::FilterType::Nearest
        } else {
            image::imageops::FilterType::Triangle
        };
        let a_medida = image::imageops::resize(img, w, h, filtro);
        if let Some(p) = super::render::a_pixmap(&a_medida) {
            let dx = (lado - w as f32) / 2.0;
            let dy = (lado - h as f32) / 2.0;
            lienzo.draw_pixmap(
                (x + dx).round() as i32,
                (y + dy).round() as i32,
                p.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
        true
    }
}

fn pintar_arbol(arbol: &usvg::Tree, lienzo: &mut PixmapMut, x: f32, y: f32, lado: f32) {
    let t = arbol.size();
    let escala = lado / t.width().max(t.height());
    let dx = (lado - t.width() * escala) / 2.0;
    let dy = (lado - t.height() * escala) / 2.0;
    resvg::render(
        arbol,
        Transform::from_row(escala, 0.0, 0.0, escala, x + dx, y + dy),
        lienzo,
    );
}

/// El código de un símbolo tal como se guarda: sin llaves y en mayúsculas.
pub fn codigo(texto: &str) -> String {
    texto.trim_matches(|c| c == '{' || c == '}').to_uppercase()
}

impl Simbolos {
    /// Sin símbolos de Scryfall, solo los iconos propios del pie (`{PINCEL}`
    /// y `{ESTRELLA}`), que van dentro de la app.
    pub fn nuevos() -> Simbolos {
        let mut s = Simbolos::default();
        for (c, svg) in [
            (
                "PINCEL",
                &include_bytes!("../../assets/iconos/pincel.svg")[..],
            ),
            (
                "ESTRELLA",
                &include_bytes!("../../assets/iconos/estrella.svg")[..],
            ),
        ] {
            if let Ok(a) = usvg::Tree::from_data(svg, &usvg::Options::default()) {
                Arc::make_mut(&mut s.arboles).insert(c.into(), a);
            }
        }
        s
    }

    /// Los símbolos de Scryfall cargados (sin contar los iconos propios).
    pub fn de_scryfall(&self) -> usize {
        self.arboles.len().saturating_sub(2)
    }

    /// Carga un símbolo. `simbolo` puede venir con llaves («{T}») o sin ellas.
    pub fn cargar(&mut self, simbolo: &str, svg: &[u8]) -> Result<(), DisenoError> {
        let arbol = usvg::Tree::from_data(svg, &usvg::Options::default())
            .map_err(|e| DisenoError::SimboloNoValido(format!("{simbolo}: {e}")))?;
        Arc::make_mut(&mut self.arboles).insert(codigo(simbolo), arbol);
        Ok(())
    }

    /// Los mismos con los propios de un pack por encima (sin copiar nada:
    /// todo va compartido).
    pub fn con_propios(&self, propios: Arc<SimbolosPropios>) -> Simbolos {
        Simbolos {
            arboles: self.arboles.clone(),
            propios: Some(propios),
        }
    }

    pub fn len(&self) -> usize {
        self.arboles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arboles.is_empty()
    }

    pub fn tiene(&self, simbolo: &str) -> bool {
        let c = codigo(simbolo);
        self.propios.as_ref().is_some_and(|p| p.tiene(&c)) || self.arboles.contains_key(&c)
    }

    /// Pinta el símbolo dentro del cuadrado `(x, y, lado)`. Si no está
    /// cargado, pinta un círculo gris y devuelve `false` (la app avisa).
    pub fn pintar(&self, simbolo: &str, lienzo: &mut PixmapMut, x: f32, y: f32, lado: f32) -> bool {
        let c = codigo(simbolo);
        if let Some(p) = &self.propios {
            if p.pintar(&c, lienzo, x, y, lado) {
                return true;
            }
        }
        let Some(arbol) = self.arboles.get(&c) else {
            circulo(
                lienzo,
                x + lado / 2.0,
                y + lado / 2.0,
                lado / 2.0,
                [150, 150, 150, 255],
            );
            return false;
        };
        pintar_arbol(arbol, lienzo, x, y, lado);
        true
    }
}

/// Los símbolos de un coste, en orden: `{2}{W}{U/B}` → `["2", "W", "U/B"]`.
pub fn secuencia(coste: &str) -> Vec<String> {
    let mut v = Vec::new();
    let mut resto = coste;
    while let Some(i) = resto.find('{') {
        let Some(f) = resto[i..].find('}') else {
            break;
        };
        v.push(codigo(&resto[i..=i + f]));
        resto = &resto[i + f + 1..];
    }
    v
}

/// Un círculo relleno (también lo usa el pintado de cajas).
pub fn circulo(lienzo: &mut PixmapMut, cx: f32, cy: f32, r: f32, color: super::Color) {
    let Some(ruta) = PathBuilder::from_circle(cx, cy, r) else {
        return;
    };
    let mut paint = Paint::default();
    paint.set_color_rgba8(color[0], color[1], color[2], color[3]);
    paint.anti_alias = true;
    lienzo.fill_path(
        &ruta,
        &paint,
        tiny_skia::FillRule::Winding,
        Transform::identity(),
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="50" fill="#ff0000"/></svg>"##;

    #[test]
    fn los_del_pack_tapan_a_los_de_scryfall() {
        let mut base = Simbolos::nuevos();
        base.cargar("{W}", SVG).unwrap();
        base.cargar("{U}", SVG).unwrap();
        // Una ficha verde de 2×2 para la W, en pixel art.
        let verde = image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 200, 0, 255]));
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(verde)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let mut propios = SimbolosPropios::nuevos(true);
        propios.cargar_png("W", &png).unwrap();
        let s = base.con_propios(Arc::new(propios));
        let color = |codigo: &str| {
            let mut px = tiny_skia::Pixmap::new(20, 20).unwrap();
            assert!(s.pintar(codigo, &mut px.as_mut(), 0.0, 0.0, 20.0));
            let c = px.pixel(10, 10).unwrap();
            (c.red(), c.green())
        };
        assert_eq!(color("W"), (0, 200));
        // La U no la trae el pack: la de Scryfall (roja).
        assert_eq!(color("U"), (255, 0));
        assert!(s.tiene("W") && s.tiene("U"));
        // La base no cambia.
        assert!(base.propios.is_none());
    }

    #[test]
    fn carga_y_pinta() {
        let mut s = Simbolos::nuevos();
        assert!(s.tiene("PINCEL") && s.tiene("ESTRELLA"));
        assert_eq!(s.de_scryfall(), 0);
        s.cargar("{T}", SVG).unwrap();
        assert!(s.tiene("t"));
        assert!(s.tiene("{T}"));
        let mut p = tiny_skia::Pixmap::new(20, 20).unwrap();
        assert!(s.pintar("T", &mut p.as_mut(), 0.0, 0.0, 20.0));
        assert_eq!(p.pixel(10, 10).unwrap().red(), 255);
        // El que falta sale gris y lo dice.
        assert!(!s.pintar("Q", &mut p.as_mut(), 0.0, 0.0, 20.0));
        assert!(s.cargar("X", b"<nada").is_err());
        assert_eq!(secuencia("{2}{W}{u/b}{G/P}"), ["2", "W", "U/B", "G/P"]);
        assert!(secuencia("").is_empty());
    }
}
