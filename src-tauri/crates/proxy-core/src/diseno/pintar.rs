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
