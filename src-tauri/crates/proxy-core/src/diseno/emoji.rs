//! **Emoji en color.** La letra de emoji de la casa (Noto Color Emoji, COLRv1)
//! trae cada emoji como capas de color. `skrifa` las recorre con un
//! `ColorPainter` y aquí se pintan con `tiny-skia`, una a una, con la misma
//! geometría que el resto del texto: colores sólidos, degradados lineales,
//! radiales y cónicos (casi todos los de Noto llevan degradado) y capas con
//! su modo de fusión.

use skrifa::color::{
    Brush, ColorPainter, ColorPalettes, ColorStop, CompositeMode, Extend, Transform as Matriz,
};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::types::BoundingBox;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use tiny_skia::{
    BlendMode, FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, PixmapMut,
    PixmapPaint, Point, RadialGradient, Rect, Shader, SpreadMode, SweepGradient, Transform,
};

use super::Color;
use crate::diseno::render::LETRA_DE_EMOJI;

/// Si la letra de emoji de la casa tiene este carácter (y no es de los que
/// cualquier letra de texto trae, por debajo de U+2190): lo que se pintaría
/// con ella. Sirve para la huella de un render (`efectivo::huella`).
pub fn es_emoji(c: char) -> bool {
    static LETRA: std::sync::OnceLock<Option<FontRef<'static>>> = std::sync::OnceLock::new();
    u32::from(c) >= 0x2190
        && LETRA
            .get_or_init(|| FontRef::new(LETRA_DE_EMOJI).ok())
            .as_ref()
            .is_some_and(|f| f.charmap().map(c).is_some())
}

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
    /// El contorno de un glifo con la transformación que había al recortar:
    /// el pincel puede traer después la suya (los degradados de Noto la
    /// llevan), que no mueve el contorno.
    Glifo(Path, Transform),
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
    /// Las capas abiertas (`push_layer`), del tamaño del lienzo, con el modo
    /// con que se funden al cerrarlas. Se pinta en la última.
    capas: Vec<(Pixmap, CompositeMode)>,
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

    /// Las paradas de un degradado con sus colores ya resueltos.
    fn paradas(&self, paradas: &[ColorStop]) -> Vec<GradientStop> {
        paradas
            .iter()
            .map(|p| {
                let [r, g, b, a] = self.color(p.palette_index, p.alpha);
                GradientStop::new(p.offset, tiny_skia::Color::from_rgba8(r, g, b, a))
            })
            .collect()
    }

    /// El color medio de unas paradas: para lo que no se puede pintar como
    /// degradado (un radio negativo, un degradado degenerado).
    fn medio(&self, paradas: &[ColorStop]) -> Option<Color> {
        if paradas.is_empty() {
            return None;
        }
        let mut suma = [0f32; 4];
        for p in paradas {
            let c = self.color(p.palette_index, p.alpha);
            for (s, v) in suma.iter_mut().zip(c) {
                *s += f32::from(v);
            }
        }
        let n = paradas.len() as f32;
        Some(suma.map(|s| (s / n).round() as u8))
    }

    /// El relleno de un pincel: un color o un degradado (en unidades del
    /// glifo; `fill_path` le aplica la misma transformación que al contorno).
    ///
    /// `ts` es la transformación del degradado respecto a la del contorno.
    fn shader(&self, brush: &Brush<'_>, ts: Transform) -> Option<Shader<'static>> {
        let solido =
            |c: Color| Shader::SolidColor(tiny_skia::Color::from_rgba8(c[0], c[1], c[2], c[3]));
        // Las sombras salen de un solo color.
        if self.forzado.is_some() {
            let (indice, alfa) = match brush {
                Brush::Solid {
                    palette_index,
                    alpha,
                } => (*palette_index, *alpha),
                Brush::LinearGradient { color_stops, .. }
                | Brush::RadialGradient { color_stops, .. }
                | Brush::SweepGradient { color_stops, .. } => {
                    let p = color_stops.first()?;
                    (p.palette_index, p.alpha)
                }
            };
            return Some(solido(self.color(indice, alfa)));
        }
        let modo = |e: &Extend| match e {
            Extend::Repeat => SpreadMode::Repeat,
            Extend::Reflect => SpreadMode::Reflect,
            _ => SpreadMode::Pad,
        };
        let degradado = match brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => return Some(solido(self.color(*palette_index, *alpha))),
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => LinearGradient::new(
                Point::from_xy(p0.x, p0.y),
                Point::from_xy(p1.x, p1.y),
                self.paradas(color_stops),
                modo(extend),
                ts,
            ),
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => RadialGradient::new(
                Point::from_xy(c0.x, c0.y),
                *r0,
                Point::from_xy(c1.x, c1.y),
                *r1,
                self.paradas(color_stops),
                modo(extend),
                ts,
            ),
            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => SweepGradient::new(
                Point::from_xy(c0.x, c0.y),
                *start_angle,
                *end_angle,
                self.paradas(color_stops),
                modo(extend),
                ts,
            ),
        };
        let paradas = match brush {
            Brush::Solid { .. } => return None,
            Brush::LinearGradient { color_stops, .. }
            | Brush::RadialGradient { color_stops, .. }
            | Brush::SweepGradient { color_stops, .. } => color_stops,
        };
        // Con una sola parada (o geometría degenerada) `tiny-skia` ya da un
        // color sólido; si no lo admite, el color medio.
        degradado.or_else(|| self.medio(paradas).map(solido))
    }
}

/// El modo de fusión de `tiny-skia` de un modo de COLRv1.
fn fusion(m: CompositeMode) -> BlendMode {
    match m {
        CompositeMode::Clear => BlendMode::Clear,
        CompositeMode::Src => BlendMode::Source,
        CompositeMode::Dest => BlendMode::Destination,
        CompositeMode::DestOver => BlendMode::DestinationOver,
        CompositeMode::SrcIn => BlendMode::SourceIn,
        CompositeMode::DestIn => BlendMode::DestinationIn,
        CompositeMode::SrcOut => BlendMode::SourceOut,
        CompositeMode::DestOut => BlendMode::DestinationOut,
        CompositeMode::SrcAtop => BlendMode::SourceAtop,
        CompositeMode::DestAtop => BlendMode::DestinationAtop,
        CompositeMode::Xor => BlendMode::Xor,
        CompositeMode::Plus => BlendMode::Plus,
        CompositeMode::Screen => BlendMode::Screen,
        CompositeMode::Overlay => BlendMode::Overlay,
        CompositeMode::Darken => BlendMode::Darken,
        CompositeMode::Lighten => BlendMode::Lighten,
        CompositeMode::ColorDodge => BlendMode::ColorDodge,
        CompositeMode::ColorBurn => BlendMode::ColorBurn,
        CompositeMode::HardLight => BlendMode::HardLight,
        CompositeMode::SoftLight => BlendMode::SoftLight,
        CompositeMode::Difference => BlendMode::Difference,
        CompositeMode::Exclusion => BlendMode::Exclusion,
        CompositeMode::Multiply => BlendMode::Multiply,
        CompositeMode::HslHue => BlendMode::Hue,
        CompositeMode::HslSaturation => BlendMode::Saturation,
        CompositeMode::HslColor => BlendMode::Color,
        CompositeMode::HslLuminosity => BlendMode::Luminosity,
        _ => BlendMode::SourceOver,
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
            Some(ruta) => {
                let t = self.actual();
                self.recortes.push(Recorte::Glifo(ruta, t));
            }
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
        let actual = self.actual();
        let recorte = self.recortes.iter().rev().find_map(|r| match r {
            Recorte::Glifo(p, t) => Some((p.clone(), *t)),
            Recorte::Caja => None,
        });
        // El contorno va con la transformación con que se recortó; el
        // degradado, con la actual, contada desde la del contorno.
        let (ruta, t, ts) = match recorte {
            Some((p, t)) => {
                let ts = t
                    .invert()
                    .map_or(Transform::identity(), |inv| actual.post_concat(inv));
                (Some(p), t, ts)
            }
            None => (None, actual, Transform::identity()),
        };
        let Some(shader) = self.shader(&brush, ts) else {
            return;
        };
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        let rellenar = |destino: &mut PixmapMut<'_>| match &ruta {
            Some(ruta) => destino.fill_path(ruta, &paint, FillRule::Winding, t, None),
            None => {
                // Sin contorno que recorte, rellena todo el cuadro del emoji.
                if let Some(r) = Rect::from_xywh(-4096.0, -4096.0, 12288.0, 12288.0) {
                    destino.fill_rect(r, &paint, t, None);
                }
            }
        };
        match self.capas.last_mut() {
            Some((capa, _)) => rellenar(&mut capa.as_mut()),
            None => rellenar(self.lienzo),
        }
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        let (ancho, alto) = (self.lienzo.width(), self.lienzo.height());
        if let Some(p) = Pixmap::new(ancho, alto) {
            self.capas.push((p, composite_mode));
        }
    }

    fn pop_layer(&mut self) {
        let Some((capa, modo)) = self.capas.pop() else {
            return;
        };
        let paint = PixmapPaint {
            blend_mode: fusion(modo),
            ..PixmapPaint::default()
        };
        let fundir = |destino: &mut PixmapMut<'_>| {
            destino.draw_pixmap(0, 0, capa.as_ref(), &paint, Transform::identity(), None);
        };
        match self.capas.last_mut() {
            Some((debajo, _)) => fundir(&mut debajo.as_mut()),
            None => fundir(self.lienzo),
        }
    }
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
        capas: Vec::new(),
    };
    let _ = color.paint(LocationRef::default(), &mut pintor);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn pintar(c: char, tam: u32, forzado: Option<Color>) -> Pixmap {
        let fuente = FontRef::new(LETRA_DE_EMOJI).unwrap();
        let glifo = fuente.charmap().map(c).unwrap();
        let mut p = Pixmap::new(tam, tam).unwrap();
        let tamf = tam as f32;
        assert!(pintar_glifo(
            &mut p.as_mut(),
            &fuente,
            glifo.to_u32(),
            0.0,
            tamf * 0.88,
            tamf,
            1.0,
            [0, 0, 0, 255],
            forzado,
        ));
        p
    }

    fn colores(p: &Pixmap) -> HashSet<[u8; 3]> {
        p.pixels()
            .iter()
            .filter(|c| c.alpha() == 255)
            .map(|c| [c.red(), c.green(), c.blue()])
            .collect()
    }

    #[test]
    fn los_degradados_de_noto_no_salen_planos() {
        let p = pintar('😀', 256, None);
        let n = colores(&p).len();
        assert!(n > 20, "solo {n} colores: el degradado sale plano");
        // La cara es amarilla y anaranjada.
        assert!(p
            .pixels()
            .iter()
            .any(|c| c.alpha() == 255 && c.red() > 220 && c.green() > 150 && c.blue() < 120));
    }

    #[test]
    fn la_sombra_sale_de_un_solo_color() {
        let p = pintar('😀', 128, Some([10, 20, 30, 255]));
        assert_eq!(colores(&p), HashSet::from([[10, 20, 30]]));
    }

    #[test]
    fn solo_es_emoji_lo_que_trae_la_letra_de_emoji() {
        assert!(es_emoji('😀'));
        assert!(es_emoji('👍'));
        assert!(!es_emoji('a'));
        assert!(!es_emoji('é'));
        assert!(!es_emoji('漢'));
        assert!(!es_emoji('—'));
    }
}
