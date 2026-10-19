//! **Pasar texto maquetado a píxeles.** Cada glifo se dibuja con su contorno
//! (`skrifa`, sin *hinting*, también en letras variables) y se rellena con
//! `tiny-skia` con antialias. Los símbolos se pintan con `resvg`.

use parley::PositionedLayoutItem;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::types::F2Dot14;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{FillRule, Paint, PathBuilder, PixmapMut, Stroke, Transform};

use super::simbolos::Simbolos;
use super::texto::{Bloque, CajaTexto, BAJA_SIMBOLO};
use super::Color;

/// Cómo se pinta un bloque.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pincel {
    /// Estrechado horizontal (1 = sin estrechar), como en los títulos largos.
    pub escala_x: f32,
    /// Color que sustituye al del texto (para sombras).
    pub color: Option<Color>,
}

impl Default for Pincel {
    fn default() -> Self {
        Pincel {
            escala_x: 1.0,
            color: None,
        }
    }
}

struct Lapiz<'a> {
    pb: &'a mut PathBuilder,
    x: f32,
    y: f32,
    sx: f32,
    /// Inclinación de la cursiva sintética (tangente).
    inclina: f32,
}

impl Lapiz<'_> {
    fn p(&self, x: f32, y: f32) -> (f32, f32) {
        (self.x + (x + y * self.inclina) * self.sx, self.y - y)
    }
}

impl OutlinePen for Lapiz<'_> {
    fn move_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        self.pb.move_to(a, b);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let (a, b) = self.p(x, y);
        self.pb.line_to(a, b);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        let (a, b) = self.p(cx0, cy0);
        let (c, d) = self.p(x, y);
        self.pb.quad_to(a, b, c, d);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let (a, b) = self.p(cx0, cy0);
        let (c, d) = self.p(cx1, cy1);
        let (e, f) = self.p(x, y);
        self.pb.cubic_to(a, b, c, d, e, f);
    }
    fn close(&mut self) {
        self.pb.close();
    }
}

fn pintura(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    p
}

/// Pinta un bloque con su esquina de arriba a la izquierda en `(x, y)`.
/// Devuelve los símbolos que faltaban (pintados en gris).
pub fn pintar_bloque(
    lienzo: &mut PixmapMut,
    bloque: &Bloque,
    x: f32,
    y: f32,
    pincel: Pincel,
    simbolos: &Simbolos,
) -> Vec<String> {
    let mut faltan = Vec::new();
    let sx = pincel.escala_x;
    for linea in bloque.layout.lines() {
        for item in linea.items() {
            match item {
                PositionedLayoutItem::GlyphRun(gr) => {
                    let run = gr.run();
                    let datos = run.font();
                    let Ok(fuente) = FontRef::from_index(datos.data.as_ref(), datos.index) else {
                        continue;
                    };
                    let contornos = fuente.outline_glyphs();
                    let coords: Vec<F2Dot14> = run
                        .normalized_coords()
                        .iter()
                        .map(|c| F2Dot14::from_bits(c.to_bits()))
                        .collect();
                    let tam = run.font_size();
                    let sintesis = run.synthesis();
                    let inclina = sintesis
                        .skew()
                        .map_or(0.0, |grados| grados.to_radians().tan());
                    let mut pb = PathBuilder::new();
                    for g in gr.positioned_glyphs() {
                        let Some(contorno) = contornos.get(GlyphId::new(g.id)) else {
                            continue;
                        };
                        let mut lapiz = Lapiz {
                            pb: &mut pb,
                            x: x + g.x * sx,
                            y: y + g.y,
                            sx,
                            inclina,
                        };
                        let ajustes =
                            DrawSettings::unhinted(Size::new(tam), LocationRef::new(&coords));
                        let _ = contorno.draw(ajustes, &mut lapiz);
                    }
                    let Some(ruta) = pb.finish() else {
                        continue;
                    };
                    let color = pincel.color.unwrap_or(gr.style().brush);
                    let paint = pintura(color);
                    lienzo.fill_path(
                        &ruta,
                        &paint,
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                    if sintesis.embolden() {
                        let trazo = Stroke {
                            width: tam * 0.035,
                            ..Stroke::default()
                        };
                        lienzo.stroke_path(&ruta, &paint, &trazo, Transform::identity(), None);
                    }
                }
                PositionedLayoutItem::InlineBox(caja) => {
                    let Some(grupo) = bloque.grupos.get(caja.id as usize) else {
                        continue;
                    };
                    let mut cx = x + caja.x * sx;
                    let cy = y + caja.y;
                    for s in &grupo.simbolos {
                        if pincel.color.is_none() {
                            if !simbolos.pintar(s, lienzo, cx, cy, grupo.lado) {
                                faltan.push(s.clone());
                            }
                        } else {
                            // En la sombra, el símbolo es un círculo del color de la sombra.
                            super::simbolos::circulo(
                                lienzo,
                                cx + grupo.lado / 2.0,
                                cy + grupo.lado / 2.0,
                                grupo.lado / 2.0,
                                pincel.color.unwrap_or([0, 0, 0, 255]),
                            );
                        }
                        cx += (grupo.lado + grupo.hueco) * sx;
                    }
                    if let Some(cola) = &grupo.cola {
                        // La base de la cola coincide con la de la línea.
                        let base_linea = cy + grupo.lado - grupo.lado / LADO_A_TAM * BAJA_SIMBOLO;
                        faltan.extend(pintar_bloque(
                            lienzo,
                            cola,
                            cx,
                            base_linea - cola.base,
                            pincel,
                            simbolos,
                        ));
                    }
                }
            }
        }
    }
    faltan
}

const LADO_A_TAM: f32 = super::texto::LADO_SIMBOLO;

/// Pinta una caja de reglas en `(x, y)`, con la raya del sabor de `ancho`.
pub fn pintar_caja(
    lienzo: &mut PixmapMut,
    caja: &CajaTexto,
    x: f32,
    y: f32,
    ancho: f32,
    color_raya: Color,
    simbolos: &Simbolos,
) -> Vec<String> {
    let mut faltan = Vec::new();
    for (b, by) in &caja.bloques {
        faltan.extend(pintar_bloque(
            lienzo,
            b,
            x,
            y + by,
            Pincel::default(),
            simbolos,
        ));
    }
    if let Some(r) = caja.raya {
        let grosor = (caja.tam * 0.06).max(1.0);
        let margen = ancho * 0.02;
        if let Some(rect) = tiny_skia::Rect::from_xywh(
            x + margen,
            y + r - grosor / 2.0,
            ancho - 2.0 * margen,
            grosor,
        ) {
            lienzo.fill_rect(rect, &pintura(color_raya), Transform::identity(), None);
        }
    }
    faltan
}

/// Ancho que ocupa un bloque pintado con estrechado `escala_x`.
pub fn ancho_pintado(bloque: &Bloque, escala_x: f32) -> f32 {
    bloque.ancho * escala_x
}

/// Lo usa el diseño para saber si una letra tiene un glifo (p. ej. «★»).
pub fn tiene_glifo(datos: &[u8], c: char) -> bool {
    FontRef::new(datos)
        .map(|f| f.charmap().map(c).is_some())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::render::Motor;
    use crate::diseno::texto::{self, Alinear, Estilo};
    use tiny_skia::Pixmap;

    fn estilo() -> Estilo {
        Estilo {
            familia: "Spectral".into(),
            peso: 400.0,
            tam: 30.0,
            interlinea: 1.06,
            color: [0, 0, 0, 255],
            alinear: Alinear::Inicio,
            versalitas: false,
            resalte: None,
            habilidad: None,
        }
    }

    fn bloque(m: &mut Motor, t: &str) -> Bloque {
        let p = texto::con_simbolos(t);
        m.m.maquetar(&mut m.letras, &p, &estilo(), None)
    }

    fn simbolos(codigos: &[&str]) -> Simbolos {
        let mut s = Simbolos::nuevos();
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="48" fill="#00ff00"/></svg>"##;
        for c in codigos {
            s.cargar(c, svg).unwrap();
        }
        s
    }

    /// El rectángulo (x0, y0, x1, y1) de lo pintado, si hay algo.
    fn pintado(p: &Pixmap) -> Option<(u32, u32, u32, u32)> {
        let mut r: Option<(u32, u32, u32, u32)> = None;
        for y in 0..p.height() {
            for x in 0..p.width() {
                if p.pixel(x, y).is_some_and(|c| c.alpha() > 0) {
                    r = Some(match r {
                        None => (x, y, x, y),
                        Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
                    });
                }
            }
        }
        r
    }

    fn pintar(b: &Bloque, x: f32, y: f32, pincel: Pincel, s: &Simbolos) -> (Pixmap, Vec<String>) {
        let mut p = Pixmap::new(600, 120).unwrap();
        let faltan = pintar_bloque(&mut p.as_mut(), b, x, y, pincel, s);
        (p, faltan)
    }

    #[test]
    fn las_palabras_marcadas_van_de_su_color_y_sin_color_igual_que_antes() {
        let mut m = Motor::de_fabrica().unwrap();
        let reglas = texto::reglas("Two-Headed Coin — When it enters, [[draw]] a card.");
        let p = &reglas[0];
        // Sin colores en el pack: lo mismo que sin marcas, letra a letra.
        let sin = m.m.maquetar(&mut m.letras, p, &estilo(), Some(2000.0));
        let llano = texto::reglas("Two-Headed Coin — When it enters, draw a card.");
        let normal =
            m.m.maquetar(&mut m.letras, &llano[0], &estilo(), Some(2000.0));
        assert_eq!(sin.ancho, normal.ancho);
        assert_eq!(sin.alto, normal.alto);
        let (a, _) = pintar(&sin, 0.0, 10.0, Pincel::default(), &simbolos(&[]));
        assert!(a.pixels().iter().all(|c| c.blue() == 0 || c.alpha() == 0));
        // Con colores: hay píxeles azules (resalte) y amarillos (habilidad).
        let con = Estilo {
            resalte: Some([0, 0, 255, 255]),
            habilidad: Some([255, 200, 0, 255]),
            ..estilo()
        };
        let b = m.m.maquetar(&mut m.letras, p, &con, Some(2000.0));
        assert_eq!(b.ancho, normal.ancho);
        let (pm, _) = pintar(&b, 0.0, 10.0, Pincel::default(), &simbolos(&[]));
        let hay = |f: &dyn Fn(&tiny_skia::PremultipliedColorU8) -> bool| pm.pixels().iter().any(f);
        assert!(hay(&|c| c.blue() > 200 && c.red() == 0));
        assert!(hay(&|c| c.red() > 200 && c.green() > 150 && c.blue() == 0));
    }

    #[test]
    fn los_caracteres_que_la_letra_no_tiene_se_cambian() {
        let mut m = Motor::de_fabrica().unwrap();
        let guion = bloque(&mut m, "Criatura - Dragón");
        m.m.sustituir = vec![("—".into(), "-".into())];
        let raya = bloque(&mut m, "Criatura — Dragón");
        assert_eq!(raya.ancho, guion.ancho);
        m.m.sustituir.clear();
        assert_ne!(bloque(&mut m, "Criatura — Dragón").ancho, guion.ancho);
    }

    #[test]
    fn un_bloque_se_pinta_donde_se_pide() {
        let mut m = Motor::de_fabrica().unwrap();
        let b = bloque(&mut m, "Ocaso");
        let (p, faltan) = pintar(&b, 50.0, 20.0, Pincel::default(), &Simbolos::nuevos());
        assert!(faltan.is_empty());
        let (x0, y0, x1, y1) = pintado(&p).unwrap();
        assert!(x0 >= 49, "empieza en x = {x0}");
        assert!(y0 >= 19, "empieza en y = {y0}");
        assert!(x1 as f32 <= 50.0 + b.ancho + 2.0);
        assert!(y1 as f32 <= 20.0 + b.alto + 2.0);
        // El color es el del estilo (negro).
        let opacos: Vec<_> = p.pixels().iter().filter(|c| c.alpha() == 255).collect();
        assert!(!opacos.is_empty());
        assert!(opacos
            .iter()
            .all(|c| c.red() == 0 && c.green() == 0 && c.blue() == 0));
    }

    #[test]
    fn el_estrechado_hace_el_texto_mas_estrecho() {
        let mut m = Motor::de_fabrica().unwrap();
        let b = bloque(&mut m, "Crepúsculo de Veldora");
        let s = Simbolos::nuevos();
        let (normal, _) = pintar(&b, 0.0, 10.0, Pincel::default(), &s);
        let (estrecho, _) = pintar(
            &b,
            0.0,
            10.0,
            Pincel {
                escala_x: 0.6,
                color: None,
            },
            &s,
        );
        let ancho = |p: &Pixmap| pintado(p).map_or(0, |(a, _, c, _)| c - a);
        let (a, e) = (ancho(&normal) as f32, ancho(&estrecho) as f32);
        assert!((e / a - 0.6).abs() < 0.05, "{e} / {a}");
        assert_eq!(ancho_pintado(&b, 0.6), b.ancho * 0.6);
        assert_eq!(ancho_pintado(&b, 1.0), b.ancho);
    }

    #[test]
    fn los_simbolos_que_faltan_se_devuelven_y_los_que_hay_no() {
        let mut m = Motor::de_fabrica().unwrap();
        let b = bloque(&mut m, "Agrega {G}{W}.");
        let (_, faltan) = pintar(&b, 0.0, 10.0, Pincel::default(), &simbolos(&["G"]));
        assert_eq!(faltan, ["W"]);
        let (p, faltan) = pintar(&b, 0.0, 10.0, Pincel::default(), &simbolos(&["G", "W"]));
        assert!(faltan.is_empty());
        // Los símbolos se ven (verde del SVG de prueba).
        let verde = p.pixels().iter().any(|c| c.green() > 200 && c.red() < 50);
        assert!(verde);
    }

    #[test]
    fn la_cola_tras_un_simbolo_tambien_se_pinta() {
        // «{T}:» lleva los dos puntos pegados al símbolo (la cola del grupo).
        let mut m = Motor::de_fabrica().unwrap();
        let b = bloque(&mut m, "{T}: roba.");
        let (p, faltan) = pintar(&b, 0.0, 10.0, Pincel::default(), &Simbolos::nuevos());
        assert_eq!(faltan, ["T"]);
        assert!(pintado(&p).is_some());
    }

    #[test]
    fn la_sombra_pinta_todo_de_su_color_y_no_echa_en_falta_simbolos() {
        let mut m = Motor::de_fabrica().unwrap();
        let b = bloque(&mut m, "Agrega {G}.");
        let sombra = Pincel {
            escala_x: 1.0,
            color: Some([255, 0, 0, 255]),
        };
        let (p, faltan) = pintar(&b, 0.0, 10.0, sombra, &Simbolos::nuevos());
        assert!(faltan.is_empty(), "en la sombra el símbolo es un círculo");
        let opacos: Vec<_> = p.pixels().iter().filter(|c| c.alpha() == 255).collect();
        assert!(!opacos.is_empty());
        assert!(opacos
            .iter()
            .all(|c| c.red() == 255 && c.green() == 0 && c.blue() == 0));
    }

    #[test]
    fn la_caja_pinta_sus_bloques_y_la_raya_del_sabor() {
        let mut m = Motor::de_fabrica().unwrap();
        let caja = texto::caja_de_reglas(
            &mut m.m,
            &mut m.letras,
            &texto::reglas("Vuela."),
            &texto::sabor("«Hola.»"),
            &estilo(),
            500.0,
            texto::Separaciones::default(),
        );
        let r = caja.raya.unwrap();
        let mut p = Pixmap::new(600, 160).unwrap();
        let faltan = pintar_caja(
            &mut p.as_mut(),
            &caja,
            20.0,
            10.0,
            500.0,
            [0, 0, 255, 255],
            &Simbolos::nuevos(),
        );
        assert!(faltan.is_empty());
        // La raya azul, centrada a lo ancho y con su margen.
        let fila = (10.0 + r).round() as u32;
        let azul = |x: u32| {
            p.pixel(x, fila)
                .is_some_and(|c| c.blue() > 200 && c.red() < 50)
        };
        assert!(azul(270));
        assert!(!azul(22), "margen a la izquierda");
        assert!(!azul(518), "margen a la derecha");
    }

    #[test]
    fn una_caja_sin_sabor_no_lleva_raya() {
        let mut m = Motor::de_fabrica().unwrap();
        let caja = texto::caja_de_reglas(
            &mut m.m,
            &mut m.letras,
            &texto::reglas("Vuela."),
            &[],
            &estilo(),
            500.0,
            texto::Separaciones::default(),
        );
        assert!(caja.raya.is_none());
        let mut p = Pixmap::new(600, 160).unwrap();
        pintar_caja(
            &mut p.as_mut(),
            &caja,
            0.0,
            0.0,
            500.0,
            [0, 0, 255, 255],
            &Simbolos::nuevos(),
        );
        assert!(!p.pixels().iter().any(|c| c.blue() > 200 && c.red() < 50));
    }

    #[test]
    fn saber_si_una_letra_tiene_un_glifo() {
        let spectral = include_bytes!("../../assets/fuentes/Spectral-Regular.ttf");
        assert!(tiene_glifo(spectral, 'A'));
        assert!(tiene_glifo(spectral, 'ñ'));
        // Un carácter de uso privado no está.
        assert!(!tiene_glifo(spectral, '\u{E000}'));
        // Unos bytes que no son una letra no revientan.
        assert!(!tiene_glifo(b"no soy una letra", 'A'));
    }
}
