//! **Símbolos de maná** y demás (`{T}`, `{2/U}`, `{W/P}`…): los SVG de
//! Scryfall (`/symbology`), que la app descarga y guarda. Aquí solo se
//! interpretan y se pintan.

use std::collections::HashMap;

use resvg::tiny_skia::{self, Paint, PathBuilder, PixmapMut, Transform};
use resvg::usvg;

use super::DisenoError;

/// Los símbolos cargados, por su código sin llaves («T», «2/U», «W/P»).
#[derive(Default)]
pub struct Simbolos {
    arboles: HashMap<String, usvg::Tree>,
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
                s.arboles.insert(c.into(), a);
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
        self.arboles.insert(codigo(simbolo), arbol);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.arboles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.arboles.is_empty()
    }

    pub fn tiene(&self, simbolo: &str) -> bool {
        self.arboles.contains_key(&codigo(simbolo))
    }

    /// Pinta el símbolo dentro del cuadrado `(x, y, lado)`. Si no está
    /// cargado, pinta un círculo gris y devuelve `false` (la app avisa).
    pub fn pintar(&self, simbolo: &str, lienzo: &mut PixmapMut, x: f32, y: f32, lado: f32) -> bool {
        let Some(arbol) = self.arboles.get(&codigo(simbolo)) else {
            circulo(
                lienzo,
                x + lado / 2.0,
                y + lado / 2.0,
                lado / 2.0,
                [150, 150, 150, 255],
            );
            return false;
        };
        let t = arbol.size();
        let escala = lado / t.width().max(t.height());
        let dx = (lado - t.width() * escala) / 2.0;
        let dy = (lado - t.height() * escala) / 2.0;
        resvg::render(
            arbol,
            Transform::from_row(escala, 0.0, 0.0, escala, x + dx, y + dy),
            lienzo,
        );
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
