//! **Emoji en color.** La letra de emoji de la casa (Twemoji, COLR) trae cada
//! emoji como capas de color. `skrifa` las recorre con un `ColorPainter` y
//! aquí se pintan con `tiny-skia`, una a una, con la misma geometría que el
//! resto del texto. Son colores sólidos: los degradados de COLRv1 se pintan
//! con el color de su primera parada.

use skrifa::color::{Brush, ColorPainter, ColorPalettes, CompositeMode, Transform as Matriz};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::types::BoundingBox;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, PixmapMut, Rect, Transform};

use super::Color;

/// `palette_index` de «el color del texto».
const PRIMER_PLANO: u16 = 0xFFFF;

/// Un contorno en unidades de la letra (con la y hacia arriba).
struct Contorno(PathBuilder);

impl OutlinePen for Contorno {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.line_to(x, y);
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.quad_to(cx0, cy0, x, y);
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.cubic_to(cx0, cy0, cx1, cy1, x, y);
    }
    fn close(&mut self) {
        self.0.close();
    }
}

enum Recorte {
    Glifo(Path),
    Caja,
}

struct Pintor<'a, 'b, 'f> {
    lienzo: &'a mut PixmapMut<'b>,
    fuente: &'a FontRef<'f>,
    paleta: &'a [skrifa::color::Color],
    /// Del glifo (unidades de la letra, y hacia arriba) a píxeles.
    base: Transform,
    transformaciones: Vec<Transform>,
    recortes: Vec<Recorte>,
    /// Color del texto, para `palette_index` 0xFFFF.
    texto: Color,
    /// Si se pide, todo se pinta de este color (las sombras).
    forzado: Option<Color>,
}

impl Pintor<'_, '_, '_> {
    fn actual(&self) -> Transform {
        self.transformaciones
            .iter()
            .fold(self.base, |t, c| t.pre_concat(*c))
    }

    fn color(&self, indice: u16, alfa: f32) -> Color {
        let [r, g, b, a] = if let Some(c) = self.forzado {
            c
        } else if indice == PRIMER_PLANO {
            self.texto
        } else {
            self.paleta
                .get(usize::from(indice))
                .map_or(self.texto, |c| [c.red, c.green, c.blue, c.alpha])
        };
        [r, g, b, (f32::from(a) * alfa.clamp(0.0, 1.0)).round() as u8]
    }
}

fn a_transform(m: Matriz) -> Transform {
    Transform::from_row(m.xx, m.yx, m.xy, m.yy, m.dx, m.dy)
}

impl ColorPainter for Pintor<'_, '_, '_> {
    fn push_transform(&mut self, transform: Matriz) {
        self.transformaciones.push(a_transform(transform));
    }

    fn pop_transform(&mut self) {
        self.transformaciones.pop();
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        let mut contorno = Contorno(PathBuilder::new());
        if let Some(g) = self.fuente.outline_glyphs().get(glyph_id) {
            let _ = g.draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut contorno,
            );
        }
        match contorno.0.finish() {
            Some(ruta) => self.recortes.push(Recorte::Glifo(ruta)),
            None => self.recortes.push(Recorte::Caja),
        }
    }

    fn push_clip_box(&mut self, _clip_box: BoundingBox<f32>) {
        self.recortes.push(Recorte::Caja);
    }

    fn pop_clip(&mut self) {
        self.recortes.pop();
    }

    fn fill(&mut self, brush: Brush<'_>) {
        let (indice, alfa) = match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => (palette_index, alpha),
            Brush::LinearGradient { color_stops, .. }
            | Brush::RadialGradient { color_stops, .. }
            | Brush::SweepGradient { color_stops, .. } => match color_stops.first() {
                Some(p) => (p.palette_index, p.alpha),
                None => return,
            },
        };
        let c = self.color(indice, alfa);
        let mut paint = Paint::default();
        paint.set_color_rgba8(c[0], c[1], c[2], c[3]);
        paint.anti_alias = true;
        let t = self.actual();
        match self.recortes.iter().rev().find_map(|r| match r {
            Recorte::Glifo(p) => Some(p),
            Recorte::Caja => None,
        }) {
            Some(ruta) => self
                .lienzo
                .fill_path(ruta, &paint, FillRule::Winding, t, None),
            None => {
                // Sin contorno que recorte, rellena todo el cuadro del emoji.
                if let Some(r) = Rect::from_xywh(-4096.0, -4096.0, 12288.0, 12288.0) {
                    self.lienzo.fill_rect(r, &paint, t, None);
                }
            }
        }
    }

    fn push_layer(&mut self, _composite_mode: CompositeMode) {}
}

/// Pinta un glifo en color con el origen de su línea base en `(x, y)`.
/// `tam` es el tamaño de la letra (px) y `sx` el estrechado horizontal.
/// Devuelve `false` si el glifo no es en color.
#[allow(clippy::too_many_arguments)]
pub fn pintar_glifo(
    lienzo: &mut PixmapMut,
    fuente: &FontRef<'_>,
    glifo: u32,
    x: f32,
    y: f32,
    tam: f32,
    sx: f32,
    texto: Color,
    forzado: Option<Color>,
) -> bool {
    let Some(color) = fuente.color_glyphs().get(GlyphId::new(glifo)) else {
        return false;
    };
    let upem = fuente
        .metrics(Size::unscaled(), LocationRef::default())
        .units_per_em
        .max(1);
    let escala = tam / f32::from(upem);
    let paletas = ColorPalettes::new(fuente);
    let paleta = paletas.get(0);
    let paleta = paleta.as_ref().map_or(&[][..], |p| p.colors());
    let mut pintor = Pintor {
        lienzo,
        fuente,
        paleta,
        // Unidades de la letra a píxeles, con la y vuelta hacia abajo.
        base: Transform::from_row(escala * sx, 0.0, 0.0, -escala, x, y),
        transformaciones: Vec::new(),
        recortes: Vec::new(),
        texto,
        forzado,
    };
    let _ = color.paint(LocationRef::default(), &mut pintor);
    true
}
