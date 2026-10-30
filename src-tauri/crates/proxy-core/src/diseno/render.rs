//! **El render de una carta diseñada.** De un `DisenoEfectivo` a una imagen
//! RGBA, siempre igual para la misma entrada.
//!
//! Orden: fondo negro → ilustración (con su encuadre) → capas del pack que
//! tocan → textos (título, coste, cinta, tipo, reglas, fuerza/resistencia) →
//! emblema → pie. Para imprimir, la carta sale opaca y con esquinas
//! cuadradas (el corte las redondea y el sangrado repite el borde); en
//! pantalla, con las esquinas del pack.

use std::collections::BTreeMap;
use std::sync::Arc;

use fast_image_resize::images::{Image, ImageRef};
use fast_image_resize::{FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::RgbaImage;
use resvg::usvg;
use tiny_skia::{FillRule, FilterQuality, Mask, PathBuilder, Pixmap, PixmapPaint, Rect, Transform};

use super::borrador::{nombre_efectivo, nombre_original, reglas_efectivas};
use super::cuerpos::{self, Contenido, Hecho};
use super::efectivo::DisenoEfectivo;
use super::estructura;
use super::letras::Letras;
use super::maquetacion::Cuerpo;
use super::modelo::{
    Aviso, CardData, DesignLang, DesignStyle, FrameColor, OnOffAuto, OriginalName, Palette,
    RulesBox,
};
use super::pack::{
    rellenar, rgba, Alineacion, CajaPorDefecto, Capa, EstiloDef, Hechos, Hueco, LetraDef, Mezcla,
    Pack, PaletaDef,
};
use super::pintar::{pintar_bloque, Pincel};
use super::simbolos::{secuencia, Simbolos};
use super::texto::{
    self, linea_ajustada, linea_ajustada_con_base, Alinear, Estilo, Maquetador, Parrafo,
};
use super::{Color, DisenoError, MM_POR_PT};

pub use super::componer::{render, render_capa, CapaVista};

/// Las imágenes del almacén (arte, emblemas), ya decodificadas. El núcleo no
/// lee disco: se las da quien llama.
pub trait Imagenes {
    fn imagen(&self, blob: &str) -> Option<Arc<RgbaImage>>;
}

/// Sin imágenes (tests, vista sin arte).
pub struct SinImagenes;

impl Imagenes for SinImagenes {
    fn imagen(&self, _: &str) -> Option<Arc<RgbaImage>> {
        None
    }
}

/// Lo compartido entre renders.
pub struct Recursos<'a> {
    pub pack: &'a Pack,
    pub simbolos: &'a Simbolos,
    pub imagenes: &'a dyn Imagenes,
}

/// Lo que cada hilo necesita para maquetar (las letras y su caché).
pub struct Motor {
    pub letras: Letras,
    pub(super) m: Maquetador,
}

impl Motor {
    /// Con las letras de la casa (Spectral y Montserrat) y la de emoji.
    pub fn de_fabrica() -> Result<Motor, DisenoError> {
        let mut letras = Letras::nuevas();
        for f in LETRAS_DE_FABRICA {
            letras.cargar(f.to_vec())?;
        }
        letras.cargar_emoji(LETRA_DE_EMOJI.to_vec())?;
        // Las de la casa tienen el latín de todos los idiomas de la app y el
        // cirílico ruso y ucraniano.
        letras.respaldos(&["Spectral", "Montserrat"]);
        Ok(Motor {
            letras,
            m: Maquetador::nuevo(),
        })
    }
}

const LETRAS_DE_FABRICA: &[&[u8]] = &[
    include_bytes!("../../assets/fuentes/Spectral-Regular.ttf"),
    include_bytes!("../../assets/fuentes/Spectral-Italic.ttf"),
    include_bytes!("../../assets/fuentes/Spectral-SemiBold.ttf"),
    include_bytes!("../../assets/fuentes/Spectral-Bold.ttf"),
    include_bytes!("../../assets/fuentes/Montserrat-Variable.ttf"),
];

/// Noto Color Emoji (COLRv1), para los emoji de cualquier texto: el fichero
/// `2D/fonts/Noto-COLRv1.ttf` de `googlefonts/noto-emoji`, commit
/// `e20cbc2bbec1926686be9f9bee7d1d2cfa1fea0e`, 5.038.236 bytes, SHA-256
/// `b8e25ea68db82f9e4d0aee921f4420be2be39887bd5c893a2ad98710531f9d0c`.
/// Licencia OFL 1.1; ver `Noto-COLRv1-LICENSE.txt`.
pub(crate) const LETRA_DE_EMOJI: &[u8] = include_bytes!("../../assets/fuentes/Noto-COLRv1.ttf");

/// Para qué es el render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Salida {
    /// Vista previa: esquinas redondeadas transparentes.
    Pantalla,
    /// PDF y PNG: opaca y con esquinas cuadradas.
    Impresion,
}

pub struct Render {
    pub imagen: RgbaImage,
    pub avisos: Vec<Aviso>,
}

// ── Colores ────────────────────────────────────────────────────────────────

pub(super) fn hex(c: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn mezclar(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2]), 255]
}

pub(super) fn luminancia(c: [u8; 4]) -> f32 {
    (0.2126 * c[0] as f32 + 0.7152 * c[1] as f32 + 0.0722 * c[2] as f32) / 255.0
}

/// Una paleta resuelta a colores.
#[derive(Debug, Clone, Copy)]
struct Tonos {
    claro: [u8; 4],
    medio: [u8; 4],
    oscuro: [u8; 4],
    tinte: [u8; 4],
    caja: [u8; 4],
    tinta: [u8; 4],
}

const NEGRO: [u8; 4] = [0, 0, 0, 255];
const BLANCO: [u8; 4] = [255, 255, 255, 255];

impl Tonos {
    fn de(p: &PaletaDef) -> Tonos {
        let c = |s: &str| rgba(s).unwrap_or(NEGRO);
        Tonos {
            claro: c(&p.claro),
            medio: c(&p.medio),
            oscuro: c(&p.oscuro),
            tinte: c(&p.tinte),
            caja: c(&p.caja),
            tinta: c(&p.tinta),
        }
    }
    /// Una paleta a partir de un solo color.
    fn de_color(m: [u8; 4]) -> Tonos {
        Tonos {
            claro: mezclar(m, BLANCO, 0.45),
            medio: m,
            oscuro: mezclar(m, NEGRO, 0.5),
            tinte: mezclar(m, NEGRO, 0.82),
            caja: mezclar(m, BLANCO, 0.86),
            tinta: [21, 21, 21, 255],
        }
    }
    fn mezcla(a: &Tonos, b: &Tonos) -> Tonos {
        Tonos {
            claro: mezclar(a.claro, b.claro, 0.5),
            medio: mezclar(a.medio, b.medio, 0.5),
            oscuro: mezclar(a.oscuro, b.oscuro, 0.5),
            tinte: mezclar(a.tinte, b.tinte, 0.5),
            caja: mezclar(a.caja, b.caja, 0.5),
            tinta: mezclar(a.tinta, b.tinta, 0.5),
        }
    }
}

fn clave_paleta(p: Palette) -> &'static str {
    match p {
        Palette::W => "W",
        Palette::U => "U",
        Palette::B => "B",
        Palette::R => "R",
        Palette::G => "G",
        Palette::Gold => "gold",
        Palette::Artifact => "artifact",
        Palette::Land => "land",
        Palette::Colorless => "colorless",
    }
}

fn paleta(pack: &Pack, p: Palette) -> Tonos {
    pack.def
        .paletas
        .get(clave_paleta(p))
        .map(Tonos::de)
        .unwrap_or_else(|| Tonos::de_color([128, 128, 128, 255]))
}

fn de_letra(c: &str) -> Option<Palette> {
    Some(match c {
        "W" => Palette::W,
        "U" => Palette::U,
        "B" => Palette::B,
        "R" => Palette::R,
        "G" => Palette::G,
        _ => return None,
    })
}

/// El color del marco que toca, antes de resolverlo a colores.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Eleccion {
    Una(Palette),
    Dos(Palette, Palette),
    Propio([u8; 4]),
}

/// El color del marco: con `Auto`, un color el suyo, dos un degradado y tres
/// o más oro; sin color, artefacto, tierra o incoloro (§3.7 del plan).
fn colores_marco(e: &DisenoEfectivo, cara: usize, pack: &Pack) -> (Tonos, Option<Tonos>) {
    match eleccion_marco(e, cara) {
        Eleccion::Una(p) => (paleta(pack, p), None),
        Eleccion::Dos(a, b) => (paleta(pack, a), Some(paleta(pack, b))),
        Eleccion::Propio(c) => (Tonos::de_color(c), None),
    }
}

/// La paleta de los marcos de imagen (`{color}` de las capas PNG). Las
/// cartas de dos colores van en oro, como las multicolor de verdad.
pub(super) fn clave_marco(e: &DisenoEfectivo, cara: usize) -> &'static str {
    if let FrameColor::Gradient { a, .. } = &e.frame.color {
        return clave_paleta(*a);
    }
    match eleccion_marco(e, cara) {
        Eleccion::Una(p) => clave_paleta(p),
        Eleccion::Dos(..) => "gold",
        // Se tiñe el marco neutro (`marco_png`).
        Eleccion::Propio(_) => "colorless",
    }
}

/// Lo que piden a los marcos de imagen un degradado o un color propio
/// elegidos a mano: la segunda paleta (se funde de izquierda a derecha) o el
/// color con que se tiñe el marco neutro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ExtraMarco {
    Nada,
    Degradado(&'static str),
    Tinte([u8; 4]),
}

pub(super) fn extra_marco(e: &DisenoEfectivo) -> ExtraMarco {
    match &e.frame.color {
        FrameColor::Gradient { b, .. } => ExtraMarco::Degradado(clave_paleta(*b)),
        FrameColor::Custom { hex } => ExtraMarco::Tinte(rgba(hex).unwrap_or([128, 128, 128, 255])),
        _ => ExtraMarco::Nada,
    }
}

fn eleccion_marco(e: &DisenoEfectivo, cara: usize) -> Eleccion {
    match &e.frame.color {
        FrameColor::Palette { palette } => Eleccion::Una(*palette),
        FrameColor::Gradient { a, b } => Eleccion::Dos(*a, *b),
        FrameColor::Custom { hex } => Eleccion::Propio(rgba(hex).unwrap_or([128, 128, 128, 255])),
        FrameColor::Auto => {
            let card = &e.design.card;
            let f = card.faces.get(cara);
            let tipo = f.map(|f| f.type_line.en.as_str()).unwrap_or("");
            let mut colores: Vec<Palette> = f
                .map(|f| f.colors.as_slice())
                .filter(|c| !c.is_empty())
                .unwrap_or(&card.colors)
                .iter()
                .filter_map(|c| de_letra(c))
                .collect();
            if colores.is_empty() && tipo.contains("Land") {
                colores = card
                    .color_identity
                    .iter()
                    .filter_map(|c| de_letra(c))
                    .collect();
                if colores.is_empty() {
                    return Eleccion::Una(Palette::Land);
                }
            }
            match colores.as_slice() {
                [] if tipo.contains("Artifact") => Eleccion::Una(Palette::Artifact),
                [] => Eleccion::Una(Palette::Colorless),
                [c] => Eleccion::Una(*c),
                [a, b] => Eleccion::Dos(*a, *b),
                _ => Eleccion::Una(Palette::Gold),
            }
        }
    }
}

/// Los degradados que usan las capas (`{{defs}}`).
fn defs(a: &Tonos, b: Option<&Tonos>, mezcla: &Tonos) -> String {
    let marco = match b {
        None => format!(
            r##"<linearGradient id="marco" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{}"/><stop offset="0.45" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient>
<linearGradient id="brillo" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.10"/><stop offset="0.5" stop-color="#ffffff" stop-opacity="0"/><stop offset="1" stop-color="#000000" stop-opacity="0.10"/></linearGradient>"##,
            hex(a.claro),
            hex(a.medio),
            hex(a.oscuro)
        ),
        Some(b) => format!(
            r##"<linearGradient id="marco" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{}"/><stop offset="0.35" stop-color="{}"/><stop offset="0.65" stop-color="{}"/><stop offset="1" stop-color="{}"/></linearGradient>
<linearGradient id="brillo" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.45"/><stop offset="0.45" stop-color="#ffffff" stop-opacity="0"/><stop offset="1" stop-color="#000000" stop-opacity="0.40"/></linearGradient>"##,
            hex(a.medio),
            hex(a.medio),
            hex(b.medio),
            hex(b.medio)
        ),
    };
    let m = mezcla;
    format!(
        r##"{marco}
<linearGradient id="panel" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{oscuro}"/><stop offset="1" stop-color="{tinte}"/></linearGradient>
<linearGradient id="panelfr" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{claro}"/><stop offset="1" stop-color="{medio}"/></linearGradient>
<linearGradient id="crema" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{caja}"/><stop offset="1" stop-color="{caja2}"/></linearGradient>
<linearGradient id="lateral" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#ffffff" stop-opacity="0.30"/><stop offset="0.3" stop-color="#ffffff" stop-opacity="0"/><stop offset="0.55" stop-color="#ffffff" stop-opacity="0.22"/><stop offset="1" stop-color="#000000" stop-opacity="0.28"/></linearGradient>
<linearGradient id="vetas" x1="0" y1="0" x2="1" y2="0.6"><stop offset="0" stop-color="#ffffff" stop-opacity="0"/><stop offset="0.12" stop-color="#ffffff" stop-opacity="0.16"/><stop offset="0.2" stop-color="#ffffff" stop-opacity="0"/><stop offset="0.42" stop-color="#000000" stop-opacity="0.12"/><stop offset="0.5" stop-color="#ffffff" stop-opacity="0"/><stop offset="0.7" stop-color="#ffffff" stop-opacity="0.14"/><stop offset="0.8" stop-color="#ffffff" stop-opacity="0"/><stop offset="0.93" stop-color="#000000" stop-opacity="0.12"/></linearGradient>
<linearGradient id="bisel" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="0.5" stop-color="#bdbdbd"/><stop offset="1" stop-color="#f0f0f0"/></linearGradient>
<linearGradient id="barra" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#5a5a5a"/><stop offset="0.5" stop-color="#2a2a2a"/><stop offset="1" stop-color="#3c3c3c"/></linearGradient>"##,
        oscuro = hex(m.oscuro),
        tinte = hex(m.tinte),
        claro = hex(m.claro),
        medio = hex(m.medio),
        caja = hex(m.caja),
        caja2 = hex(mezclar(m.caja, m.oscuro, 0.12)),
    )
}

/// Las variables de ejemplo para comprobar que las capas se entienden.
pub fn vars_de_ejemplo() -> BTreeMap<String, String> {
    let t = Tonos::de_color([200, 150, 60, 255]);
    let mut v = BTreeMap::new();
    v.insert("defs".into(), defs(&t, Some(&t), &t));
    for (k, val) in [
        ("claro", "#ffffff"),
        ("medio", "#888888"),
        ("oscuro", "#333333"),
        ("tinte", "#222222"),
        ("fondoCaja", "#222222"),
        ("opacidadCaja", "0.8"),
        ("tintaCaja", "#ffffff"),
        ("yReglas", "60"),
        ("subeCaja", "0"),
        ("extensionArte", "0"),
        ("altoReglas", "23"),
        ("yTipo", "54"),
        ("yTipo04", "54.4"),
    ] {
        v.insert(k.into(), val.into());
    }
    v
}

// ── Lienzo ─────────────────────────────────────────────────────────────────

/// Un lienzo en píxeles con sus medidas en mm.
pub(super) struct Lienzo {
    pub(super) px: Pixmap,
    pub(super) kx: f32,
    pub(super) ky: f32,
    pub(super) w_mm: f32,
    pub(super) h_mm: f32,
}

impl Lienzo {
    pub(super) fn nuevo(w_mm: f32, h_mm: f32, ancho: u32, alto: u32) -> Option<Lienzo> {
        Some(Lienzo {
            px: Pixmap::new(ancho.max(1), alto.max(1))?,
            kx: ancho.max(1) as f32 / w_mm,
            ky: alto.max(1) as f32 / h_mm,
            w_mm,
            h_mm,
        })
    }

    pub(super) fn x(&self, mm: f32) -> f32 {
        mm * self.kx
    }
    pub(super) fn y(&self, mm: f32) -> f32 {
        mm * self.ky
    }

    /// Pinta un SVG que cubre el lienzo entero (su `viewBox` son los mm).
    pub(super) fn svg(&mut self, svg: &str) -> Result<(), String> {
        let arbol =
            usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
        let t = arbol.size();
        resvg::render(
            &arbol,
            Transform::from_scale(
                self.kx * self.w_mm / t.width(),
                self.ky * self.h_mm / t.height(),
            ),
            &mut self.px.as_mut(),
        );
        Ok(())
    }
}

fn premultiplicar(rgba: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(rgba.len());
    for p in rgba.as_chunks::<4>().0 {
        let a = p[3] as u16;
        v.extend_from_slice(&[
            ((p[0] as u16 * a + 127) / 255) as u8,
            ((p[1] as u16 * a + 127) / 255) as u8,
            ((p[2] as u16 * a + 127) / 255) as u8,
            p[3],
        ]);
    }
    v
}

pub(super) fn a_pixmap(img: &RgbaImage) -> Option<Pixmap> {
    Pixmap::from_vec(
        premultiplicar(img.as_raw()),
        tiny_skia::IntSize::from_wh(img.width(), img.height())?,
    )
}

/// Reescala pixel art: al ampliar, vecino más cercano (cada píxel se
/// repite); al reducir, la media del área (filtro caja).
pub(super) fn reescalar_pixel(img: &RgbaImage, w: u32, h: u32) -> Option<RgbaImage> {
    let src = ImageRef::new(img.width(), img.height(), img.as_raw(), PixelType::U8x4).ok()?;
    let mut dst = Image::new(w.max(1), h.max(1), PixelType::U8x4);
    let ampliar = w >= img.width() && h >= img.height();
    let alg = if ampliar {
        ResizeAlg::Nearest
    } else {
        ResizeAlg::Convolution(FilterType::Box)
    };
    let op = ResizeOptions::new().resize_alg(alg).use_alpha(true);
    Resizer::new().resize(&src, &mut dst, &op).ok()?;
    RgbaImage::from_raw(w.max(1), h.max(1), dst.into_vec())
}

/// La pintura de una capa según su mezcla y su opacidad.
pub(super) fn pintura_con(mezcla: Mezcla, opacidad: f32) -> PixmapPaint {
    PixmapPaint {
        opacity: opacidad.clamp(0.0, 1.0),
        blend_mode: match mezcla {
            Mezcla::Normal => tiny_skia::BlendMode::SourceOver,
            Mezcla::Oscurecer => tiny_skia::BlendMode::Darken,
            Mezcla::Multiplicar => tiny_skia::BlendMode::Multiply,
        },
        ..PixmapPaint::default()
    }
}

/// Reescala con Lanczos3 (la calidad de `imaging::frame`).
fn reescalar(
    img: &RgbaImage,
    w: u32,
    h: u32,
    recorte: Option<(f64, f64, f64, f64)>,
) -> Option<RgbaImage> {
    let src = ImageRef::new(img.width(), img.height(), img.as_raw(), PixelType::U8x4).ok()?;
    let mut dst = Image::new(w.max(1), h.max(1), PixelType::U8x4);
    let mut op = ResizeOptions::new()
        .resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3))
        .use_alpha(true);
    if let Some((l, t, cw, ch)) = recorte {
        op = op.crop(l, t, cw, ch);
    }
    Resizer::new().resize(&src, &mut dst, &op).ok()?;
    RgbaImage::from_raw(w.max(1), h.max(1), dst.into_vec())
}

/// La imagen con su lado más largo en `lado` px como mucho (sin deformar);
/// si ya cabe, tal cual. Es la ilustración que mueve la vista previa.
pub fn reducida(img: &RgbaImage, lado: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let k = lado as f64 / w.max(h).max(1) as f64;
    if k >= 1.0 {
        return img.clone();
    }
    let (nw, nh) = (
        ((w as f64 * k).round() as u32).max(1),
        ((h as f64 * k).round() as u32).max(1),
    );
    reescalar(img, nw, nh, None).unwrap_or_else(|| img.clone())
}

/// La ilustración cubriendo su hueco, con zoom, desplazamiento y giro.
///
/// `extra` (mm): cuánto sigue por debajo del hueco (la extensión del
/// difuminado), con el mismo encuadre: lo que haya de la imagen debajo de lo
/// que se ve y, si se acaba, su última fila estirada (ahí ya está casi negro).
pub(super) fn pintar_arte(
    l: &mut Lienzo,
    img: &RgbaImage,
    hueco: (f32, f32, f32, f32),
    f: &super::modelo::ArtFraming,
    extra: f32,
) {
    let (x, y, w, h) = (l.x(hueco.0), l.y(hueco.1), l.x(hueco.2), l.y(hueco.3));
    let (tw, th) = (w.round().max(1.0) as u32, h.round().max(1.0) as u32);
    let te = l.y(extra.max(0.0)).round() as u32;
    let zoom = f.zoom.max(ZOOM_MIN);
    let girada = f.rotation.rem_euclid(360.0).abs() > 0.01;
    let pieza = if !girada && zoom >= 1.0 {
        // Sin giro y cubriendo: recorte exacto + Lanczos en un paso.
        recorte_exacto(img, tw, th, te, zoom, f.offset_x, f.offset_y)
    } else {
        arte_libre(img, tw, th, te, zoom, f)
    };
    if let Some(p) = pieza.as_ref().and_then(a_pixmap) {
        l.px.draw_pixmap(
            x.round() as i32,
            y.round() as i32,
            p.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

/// El zoom más pequeño de la ilustración: por debajo de 1 cabe entera en
/// su hueco (lo que sobra, la misma ilustración desenfocada).
pub const ZOOM_MIN: f64 = 0.1;

/// La ilustración cubriendo el hueco (`tw`×`th` px, y `te` px más por debajo
/// con la extensión del difuminado), sin giro y con zoom de 1 o más.
fn recorte_exacto(
    img: &RgbaImage,
    tw: u32,
    th: u32,
    te: u32,
    zoom: f64,
    offset_x: f64,
    offset_y: f64,
) -> Option<RgbaImage> {
    let (iw, ih) = (img.width() as f64, img.height() as f64);
    let cubre = (tw as f64 / iw).max(th as f64 / ih) * zoom;
    let (cw, ch) = (tw as f64 / cubre, th as f64 / cubre);
    let (hx, hy) = ((iw - cw) / 2.0, (ih - ch) / 2.0);
    let l0 = hx + offset_x.clamp(-1.0, 1.0) * hx;
    let t0 = hy + offset_y.clamp(-1.0, 1.0) * hy;
    // Lo de debajo, en el mismo paso (sin costura): lo que quede.
    let debajo = (te as f64 / cubre).min((ih - t0 - ch).max(0.0));
    let th_debajo = (debajo * cubre).round() as u32;
    let debajo = if th_debajo == 0 { 0.0 } else { debajo };
    reescalar(img, tw, th + th_debajo, Some((l0, t0, cw, ch + debajo)))
        .map(|p| estirar_abajo(p, th + te))
}

/// La ilustración girada o más pequeña que su hueco: escalada entera y
/// colocada con su centro. El desplazamiento recorre lo que sobra (si la
/// imagen es más grande) o lo que falta (si es más pequeña), igual que la
/// cuenta de la pantalla (`encuadre.ts`). Si no cubre el hueco, lo que falta
/// sale de sus bordes alargados y desenfocados (con fondo blanco, blanco).
fn arte_libre(
    img: &RgbaImage,
    tw: u32,
    th: u32,
    te: u32,
    zoom: f64,
    f: &super::modelo::ArtFraming,
) -> Option<RgbaImage> {
    let (iw, ih) = (img.width() as f64, img.height() as f64);
    let girada = f.rotation.rem_euclid(360.0).abs() > 0.01;
    let a = f.rotation.to_radians();
    let (c, s) = (a.cos().abs(), a.sin().abs());
    let (wf, hf) = (tw as f64, th as f64);
    let cubre = ((wf * c + hf * s) / iw).max((wf * s + hf * c) / ih) * zoom;
    let (rw, rh) = (
        (iw * cubre).round().max(1.0) as u32,
        (ih * cubre).round().max(1.0) as u32,
    );
    let escalada = reescalar(img, rw, rh, None)?;
    let pm = a_pixmap(&escalada)?;
    let mut out = Pixmap::new(tw, th + te)?;
    let (bw, bh) = (rw as f64 * c + rh as f64 * s, rw as f64 * s + rh as f64 * c);
    let dx = f.offset_x.clamp(-1.0, 1.0) * (bw - wf) / 2.0;
    let dy = f.offset_y.clamp(-1.0, 1.0) * (bh - hf) / 2.0;
    if zoom < 1.0 {
        let esquina = (
            (wf / 2.0 - dx - rw as f64 / 2.0).round() as i64,
            (hf / 2.0 - dy - rh as f64 / 2.0).round() as i64,
        );
        let fondo = desenfocar(&bordes_alargados(&escalada, esquina, tw, th + te));
        if let Some(fondo) = fondo.as_ref().and_then(a_pixmap) {
            out.draw_pixmap(
                0,
                0,
                fondo.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }
    let t = if girada {
        Transform::from_translate((wf / 2.0 - dx) as f32, (hf / 2.0 - dy) as f32)
            .pre_rotate(f.rotation as f32)
            .pre_translate(-(rw as f32) / 2.0, -(rh as f32) / 2.0)
    } else {
        // Sin giro, a píxel entero: nítida.
        Transform::from_translate(
            (wf / 2.0 - dx - rw as f64 / 2.0).round() as f32,
            (hf / 2.0 - dy - rh as f64 / 2.0).round() as f32,
        )
    };
    out.draw_pixmap(
        0,
        0,
        pm.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..PixmapPaint::default()
        },
        t,
        None,
    );
    Some(de_pixmap(&out))
}

/// Un lienzo de `w`×`h` con `img` en `esquina` y, fuera de ella, el píxel de
/// su borde más cercano (los bordes alargados).
fn bordes_alargados(img: &RgbaImage, esquina: (i64, i64), w: u32, h: u32) -> RgbaImage {
    let (iw, ih) = (i64::from(img.width()), i64::from(img.height()));
    RgbaImage::from_fn(w, h, |x, y| {
        let sx = (i64::from(x) - esquina.0).clamp(0, iw - 1);
        let sy = (i64::from(y) - esquina.1).clamp(0, ih - 1);
        *img.get_pixel(sx as u32, sy as u32)
    })
}

/// El color medio del borde de una imagen (un anillo del 3 % de su lado): el
/// fondo de la ilustración en la vista en directo cuando no cubre su hueco.
pub fn color_de_borde(img: &RgbaImage) -> [u8; 3] {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return [0, 0, 0];
    }
    let anillo = (w.min(h) / 33).max(1);
    let (mut suma, mut n) = ([0u64; 3], 0u64);
    for (x, y, p) in img.enumerate_pixels() {
        if x < anillo || y < anillo || x >= w - anillo || y >= h - anillo {
            for (s, v) in suma.iter_mut().zip(p.0) {
                *s += u64::from(v);
            }
            n += 1;
        }
    }
    suma.map(|s| (s / n.max(1)) as u8)
}

/// Muy desenfocada (reducida mucho y vuelta a su tamaño): el fondo de una
/// ilustración que no cubre su hueco.
fn desenfocar(img: &RgbaImage) -> Option<RgbaImage> {
    let (w, h) = img.dimensions();
    let k = 24;
    let pequena = reescalar(img, (w / k).max(1), (h / k).max(1), None)?;
    let pequena = image::imageops::blur(&pequena, 1.5);
    reescalar(&pequena, w, h, None)
}

/// La imagen alargada hasta `alto` px repitiendo su última fila, suavizada
/// en horizontal (repetida tal cual, salen rayas).
fn estirar_abajo(img: RgbaImage, alto: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if h >= alto || h == 0 || w == 0 {
        return img;
    }
    let radio = (w / 40).max(1) as i64;
    let fila: Vec<image::Rgba<u8>> = (0..w as i64)
        .map(|x| {
            let (a, b) = ((x - radio).max(0), (x + radio).min(w as i64 - 1));
            let mut suma = [0u32; 4];
            for xx in a..=b {
                let p = img.get_pixel(xx as u32, h - 1);
                for (s, v) in suma.iter_mut().zip(p.0) {
                    *s += u32::from(v);
                }
            }
            let n = (b - a + 1) as u32;
            image::Rgba(suma.map(|s| (s / n) as u8))
        })
        .collect();
    let mut out = RgbaImage::new(w, alto);
    image::imageops::replace(&mut out, &img, 0, 0);
    for y in h..alto {
        for (x, p) in fila.iter().enumerate() {
            out.put_pixel(x as u32, y, *p);
        }
    }
    out
}

/// Lo oscuro del difuminado en cada punto (de 0, arriba, a 1): muy suave al
/// empezar y al acabar (smootherstep), que no se note dónde empieza ni dónde
/// la imagen se pierde.
fn curva_difuminado(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Cuánto antes del final del hueco de la ilustración llega el difuminado a
/// negro del todo (mm).
const DIFUMINADO_COLCHON_MM: f32 = 1.5;

/// Funde con negro los `mm` de abajo del hueco de la ilustración. Es un
/// degradado encima (no transparencia), así sale igual en la capa de la
/// vista en directo, donde la ilustración va por debajo.
fn difuminar_abajo(l: &mut Lienzo, hueco: (f32, f32, f32, f32), mm: f32) {
    let alto = mm.min(hueco.3);
    if alto <= 0.0 {
        return;
    }
    // Negro del todo un poco antes del final del hueco: el marco suele tapar
    // el filo de abajo del arte, y la imagen tiene que haberse perdido antes
    // de llegar ahí (si no, se corta en seco contra él).
    let colchon = DIFUMINADO_COLCHON_MM.min(alto * 0.25);
    let fondo = hueco.1 + hueco.3;
    let (x, w) = (l.x(hueco.0), l.x(hueco.2));
    let (y0, y_negro, y1) = (l.y(fondo - alto), l.y(fondo - colchon), l.y(fondo));
    let negro =
        |a: f32| tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, a).unwrap_or(tiny_skia::Color::BLACK);
    let paradas = (0..=12)
        .map(|i| {
            let t = i as f32 / 12.0;
            let a = curva_difuminado(t);
            tiny_skia::GradientStop::new(t, negro(a))
        })
        .collect();
    let (Some(sombra), Some(rect)) = (
        tiny_skia::LinearGradient::new(
            tiny_skia::Point::from_xy(0.0, y0),
            tiny_skia::Point::from_xy(0.0, y_negro),
            paradas,
            tiny_skia::SpreadMode::Pad,
            Transform::identity(),
        ),
        tiny_skia::Rect::from_xywh(x, y0, w, y1 - y0),
    ) else {
        return;
    };
    let pintura = tiny_skia::Paint {
        shader: sombra,
        anti_alias: false,
        ..Default::default()
    };
    l.px.fill_rect(rect, &pintura, Transform::identity(), None);
}

/// La ilustración (o lo que haya) y lo de dentro de la ventana, otra vez,
/// en las zonas `repintar` de la ventana: dentro de la ventana con el radio
/// pedido, la ilustración; fuera, negro. En la capa de la vista en directo
/// (`capa`), lo de dentro se deja transparente (la ilustración va debajo).
type ArtePintado = (
    Option<(Arc<RgbaImage>, super::modelo::ArtFraming)>,
    (f32, f32, f32, f32),
);

fn repintar_ventana(
    l: &mut Lienzo,
    v: &super::pack::VentanaArte,
    radio: f32,
    arte: Option<&ArtePintado>,
    capa: bool,
    desde_y: f32,
) {
    let (w, h) = (l.px.width(), l.px.height());
    let r = radio.clamp(0.0, v.w.min(v.h) / 2.0);
    let (Some(mut zonas), Some(mut dentro)) = (Mask::new(w, h), Mask::new(w, h)) else {
        return;
    };
    for z in &v.repintar {
        // Sin pasar de la barra del tipo, que en la caja que crece baja con
        // ella: repintar sus extremos se los comería.
        let y = z[1].max(desde_y);
        let alto = z[1] + z[3] - y;
        if alto <= 0.0 {
            continue;
        }
        if let Some(rc) = Rect::from_xywh(l.x(z[0]), l.y(y), l.x(z[2]), l.y(alto)) {
            zonas.fill_path(
                &PathBuilder::from_rect(rc),
                FillRule::Winding,
                false,
                Transform::identity(),
            );
        }
    }
    let (x0, y0, x1, y1) = (l.x(v.x), l.y(v.y), l.x(v.x + v.w), l.y(v.y + v.h));
    let Some(ventana) = ruta_redondeada(x0, y0, x1, y1, l.x(r), v.solo_abajo) else {
        return;
    };
    dentro.fill_path(&ventana, FillRule::Winding, true, Transform::identity());
    // Dentro y fuera de la ventana, solo en las zonas.
    let mut fuera = zonas.clone();
    for ((d, f), z) in dentro
        .data_mut()
        .iter_mut()
        .zip(fuera.data_mut().iter_mut())
        .zip(zonas.data())
    {
        let (vd, vz) = (u16::from(*d), u16::from(*z));
        *f = (vz * (255 - vd) / 255) as u8;
        *d = (vz * vd / 255) as u8;
    }
    let Some(todo) = Rect::from_xywh(0.0, 0.0, w as f32, h as f32) else {
        return;
    };
    let mut negro = tiny_skia::Paint::default();
    negro.set_color(tiny_skia::Color::BLACK);
    l.px.fill_rect(todo, &negro, Transform::identity(), Some(&fuera));
    if capa {
        let borrar = tiny_skia::Paint {
            blend_mode: tiny_skia::BlendMode::Clear,
            ..Default::default()
        };
        l.px.fill_rect(todo, &borrar, Transform::identity(), Some(&dentro));
        return;
    }
    let Some((img, hueco)) = arte else {
        return;
    };
    let Some(mut otro) = Lienzo::nuevo(l.w_mm, l.h_mm, w, h) else {
        return;
    };
    match img {
        Some((i, f)) => pintar_arte(&mut otro, i, *hueco, f, 0.0),
        None => sin_arte(&mut otro, *hueco),
    }
    l.px.draw_pixmap(
        0,
        0,
        otro.px.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        Some(&dentro),
    );
}

/// Un rectángulo (px) con las esquinas redondeadas (o solo las de abajo).
fn ruta_redondeada(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    r: f32,
    solo_abajo: bool,
) -> Option<tiny_skia::Path> {
    let k = 0.552_284_8 * r;
    let mut pb = PathBuilder::new();
    let ra = if solo_abajo { 0.0 } else { r };
    let ka = if solo_abajo { 0.0 } else { k };
    pb.move_to(x0 + ra, y0);
    pb.line_to(x1 - ra, y0);
    pb.cubic_to(x1 - ra + ka, y0, x1, y0 + ra - ka, x1, y0 + ra);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r);
    pb.line_to(x0, y0 + ra);
    pb.cubic_to(x0, y0 + ra - ka, x0 + ra - ka, y0, x0 + ra, y0);
    pb.close();
    pb.finish()
}

/// Negro fuera de la ventana de la ilustración, con sus esquinas redondeadas.
pub(super) fn ventana_svg(
    w_mm: f32,
    h_mm: f32,
    v: &super::pack::VentanaArte,
    radio: f32,
) -> String {
    let r = radio.clamp(0.0, v.w.min(v.h) / 2.0);
    let (x0, y0, x1, y1) = (v.x, v.y, v.x + v.w, v.y + v.h);
    let arco = |x: f32, y: f32| {
        if r > 0.0 {
            format!("A{r},{r} 0 0 1 {x},{y}")
        } else {
            format!("L{x},{y}")
        }
    };
    let ventana = if v.solo_abajo {
        format!(
            "M{x0},{y0} H{x1} V{} {} H{} {} Z",
            y1 - r,
            arco(x1 - r, y1),
            x0 + r,
            arco(x0, y1 - r)
        )
    } else {
        format!(
            "M{} {y0} H{} {} V{} {} H{} {} V{} {} Z",
            x0 + r,
            x1 - r,
            arco(x1, y0 + r),
            y1 - r,
            arco(x1 - r, y1),
            x0 + r,
            arco(x0, y1 - r),
            y0 + r,
            arco(x0 + r, y0)
        )
    };
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w_mm} {h_mm}" width="{w_mm}" height="{h_mm}"><path fill-rule="evenodd" fill="#000" d="M-1,-1 H{} V{} H-1 Z {ventana}"/></svg>"##,
        w_mm + 1.0,
        h_mm + 1.0
    )
}

/// Sin ilustración: un degradado oscuro neutro (borradores).
pub(super) fn sin_arte(l: &mut Lienzo, hueco: (f32, f32, f32, f32)) {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} {}" width="{}" height="{}"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#3a3f47"/><stop offset="1" stop-color="#15181c"/></linearGradient></defs><rect x="{}" y="{}" width="{}" height="{}" fill="url(#g)"/></svg>"##,
        l.w_mm, l.h_mm, l.w_mm, l.h_mm, hueco.0, hueco.1, hueco.2, hueco.3
    );
    let _ = l.svg(&svg);
}

/// Una imagen encajada sin deformar en el hueco, centrada.
fn pintar_imagen(l: &mut Lienzo, img: &RgbaImage, hueco: (f32, f32, f32, f32)) {
    let (w, h) = (l.x(hueco.2), l.y(hueco.3));
    let esc = (w / img.width() as f32).min(h / img.height() as f32);
    let (rw, rh) = (
        (img.width() as f32 * esc).round().max(1.0) as u32,
        (img.height() as f32 * esc).round().max(1.0) as u32,
    );
    if let Some(p) = reescalar(img, rw, rh, None).as_ref().and_then(a_pixmap) {
        l.px.draw_pixmap(
            (l.x(hueco.0) + (w - rw as f32) / 2.0).round() as i32,
            (l.y(hueco.1) + (h - rh as f32) / 2.0).round() as i32,
            p.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

/// Los PNG de los packs, decodificados y reescalados al pedirlos y guardados
/// un rato: al arrastrar el arte se repinta la carta muchas veces seguidas
/// con el mismo marco. Se guardan por la huella del pack, así que un pack
/// cambiado no reutiliza lo viejo.
pub(super) mod imagenes_png {
    use std::sync::{Arc, LazyLock, Mutex};

    use image::RgbaImage;

    use super::{reescalar, reescalar_pixel};
    use crate::diseno::pack::{Escalado, Pack};

    /// Lo que se guarda como mucho (bytes RGBA) de cada tipo.
    const MAX_ORIGINALES: usize = 160 << 20;
    const MAX_A_MEDIDA: usize = 160 << 20;

    type Lista = Vec<(String, Arc<RgbaImage>)>;
    static ORIGINALES: LazyLock<Mutex<Lista>> = LazyLock::new(|| Mutex::new(Vec::new()));
    static A_MEDIDA: LazyLock<Mutex<Lista>> = LazyLock::new(|| Mutex::new(Vec::new()));

    fn tamano(i: &RgbaImage) -> usize {
        i.as_raw().len()
    }

    /// Busca y sube al principio (lo más usado se queda).
    fn buscar(lista: &Mutex<Lista>, clave: &str) -> Option<Arc<RgbaImage>> {
        let mut l = lista.lock().ok()?;
        let i = l.iter().position(|(k, _)| k == clave)?;
        let e = l.remove(i);
        let img = e.1.clone();
        l.insert(0, e);
        Some(img)
    }

    fn guardar(lista: &Mutex<Lista>, clave: String, img: Arc<RgbaImage>, max: usize) {
        let Ok(mut l) = lista.lock() else {
            return;
        };
        l.insert(0, (clave, img));
        let mut total = 0;
        let mut corte = l.len();
        for (i, (_, im)) in l.iter().enumerate() {
            total += tamano(im);
            // Siempre se queda al menos la recién guardada.
            if total > max && i > 0 {
                corte = i;
                break;
            }
        }
        l.truncate(corte);
    }

    /// El PNG `ruta` del pack, decodificado.
    pub fn original(pack: &Pack, ruta: &str) -> Option<Arc<RgbaImage>> {
        let clave = format!("{}/{ruta}", pack.huella);
        if let Some(i) = buscar(&ORIGINALES, &clave) {
            return Some(i);
        }
        let bytes = pack.imagenes.get(ruta)?;
        let img = Arc::new(image::load_from_memory(bytes).ok()?.to_rgba8());
        guardar(&ORIGINALES, clave, img.clone(), MAX_ORIGINALES);
        Some(img)
    }

    /// El PNG a `w`×`h` píxeles.
    pub fn a_medida(
        pack: &Pack,
        ruta: &str,
        w: u32,
        h: u32,
        escalado: Escalado,
    ) -> Option<Arc<RgbaImage>> {
        let clave = format!("{}/{ruta}@{w}x{h}/{escalado:?}", pack.huella);
        if let Some(i) = buscar(&A_MEDIDA, &clave) {
            return Some(i);
        }
        let o = original(pack, ruta)?;
        let img = if o.width() == w && o.height() == h {
            o
        } else if escalado == Escalado::Pixel {
            Arc::new(reescalar_pixel(&o, w, h)?)
        } else {
            Arc::new(reescalar(&o, w, h, None)?)
        };
        guardar(&A_MEDIDA, clave, img.clone(), MAX_A_MEDIDA);
        Some(img)
    }

    /// Como `a_medida`, con solo un trozo del PNG (`recorte`: `[x, y, ancho,
    /// alto]` en fracciones de la imagen) y, con `mascara`, recortado por la
    /// transparencia de ese otro PNG (el mismo trozo, a la misma medida).
    pub fn trozo(
        pack: &Pack,
        ruta: &str,
        recorte: Option<[f32; 4]>,
        mascara: Option<&str>,
        w: u32,
        h: u32,
        escalado: Escalado,
    ) -> Option<Arc<RgbaImage>> {
        if recorte.is_none() && mascara.is_none() {
            return a_medida(pack, ruta, w, h, escalado);
        }
        let clave = format!(
            "{}/{ruta}[{recorte:?}|{mascara:?}]@{w}x{h}/{escalado:?}",
            pack.huella
        );
        if let Some(i) = buscar(&A_MEDIDA, &clave) {
            return Some(i);
        }
        let a_su_medida = |r: &str| -> Option<RgbaImage> {
            let o = original(pack, r)?;
            let (ow, oh) = (o.width() as f64, o.height() as f64);
            let [x, y, rw, rh] = recorte
                .unwrap_or([0.0, 0.0, 1.0, 1.0])
                .map(|v| f64::from(v.clamp(0.0, 1.0)));
            let (l, t) = (x * ow, y * oh);
            let (cw, ch) = ((rw * ow).min(ow - l), (rh * oh).min(oh - t));
            if cw <= 0.0 || ch <= 0.0 {
                return None;
            }
            if escalado == Escalado::Pixel {
                let (x0, y0) = (l.floor() as u32, t.floor() as u32);
                let trozo = image::imageops::crop_imm(
                    o.as_ref(),
                    x0,
                    y0,
                    ((l + cw).ceil() as u32).saturating_sub(x0).max(1),
                    ((t + ch).ceil() as u32).saturating_sub(y0).max(1),
                )
                .to_image();
                reescalar_pixel(&trozo, w, h)
            } else {
                reescalar(&o, w, h, Some((l, t, cw, ch)))
            }
        };
        let mut img = a_su_medida(ruta)?;
        if let Some(m) = mascara {
            let m = a_su_medida(m)?;
            for (p, q) in img.pixels_mut().zip(m.pixels()) {
                p.0[3] = ((u16::from(p.0[3]) * u16::from(q.0[3]) + 127) / 255) as u8;
            }
        }
        let img = Arc::new(img);
        guardar(&A_MEDIDA, clave, img.clone(), MAX_A_MEDIDA);
        Some(img)
    }
}

/// Qué parte de un PNG de capa se pinta: un trozo (`[x, y, ancho, alto]` en
/// fracciones de la imagen; sin él, entera) y la máscara que la recorta.
#[derive(Debug, Clone, Copy, Default)]
struct Trozo<'a> {
    recorte: Option<[f32; 4]>,
    mascara: Option<&'a str>,
}

/// La segunda imagen de un marco en degradado: transparente a la izquierda y
/// entera a la derecha, con el paso entre el 35 % y el 65 % del ancho de la
/// carta (como el degradado de los marcos SVG). `x0`: dónde empieza la imagen
/// en el lienzo (px); `k`: px por mm.
fn fundir_a_la_derecha(img: &mut RgbaImage, x0: f32, k: f32) {
    let (a, b) = (63.0 * 0.35 * k, 63.0 * 0.65 * k);
    for (x, _, p) in img.enumerate_pixels_mut() {
        let t = ((x0 + x as f32 - a) / (b - a)).clamp(0.0, 1.0);
        p.0[3] = (f32::from(p.0[3]) * t).round() as u8;
    }
}

/// Tiñe una imagen de un color conservando sus luces y sombras (fusión
/// «superponer» de la luminosidad con el color): la textura del marco neutro
/// queda del color pedido.
fn tenir(img: &mut RgbaImage, color: [u8; 4]) {
    let c = color.map(|v| f32::from(v) / 255.0);
    for p in img.pixels_mut() {
        let [r, g, b, _] = p.0.map(|v| f32::from(v) / 255.0);
        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        for (canal, ci) in p.0.iter_mut().zip(c).take(3) {
            let v = if l < 0.5 {
                2.0 * l * ci
            } else {
                1.0 - 2.0 * (1.0 - l) * (1.0 - ci)
            };
            *canal = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
}

/// Hasta dónde baja la letra de las reglas antes de que la caja que crece pase
/// de su altura cómoda (`altoComodo`).
const LETRA_COMODA: f32 = 0.75;

/// Un pixmap (premultiplicado) a imagen RGBA.
pub(super) fn de_pixmap(p: &Pixmap) -> RgbaImage {
    let mut img = RgbaImage::new(p.width(), p.height());
    for (d, s) in img.pixels_mut().zip(p.pixels()) {
        let c = s.demultiply();
        *d = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    img
}

// ── Contexto de una carta ──────────────────────────────────────────────────

pub(super) struct Carta<'a> {
    pub(super) l: Lienzo,
    pub(super) motor: &'a mut Motor,
    pub(super) rec: &'a Recursos<'a>,
    pub(super) estilo: EstiloDef,
    pub(super) vars: BTreeMap<String, String>,
    pub(super) nums: BTreeMap<&'static str, f32>,
    pub(super) avisos: Vec<Aviso>,
    pub(super) faltan: Vec<String>,
    /// La paleta de las capas PNG (`W`, `gold`…).
    pub(super) clave_color: &'static str,
    /// Degradado o color propio de los marcos de imagen.
    pub(super) extra_marco: ExtraMarco,
    /// El tinte de la caja de reglas que pide el diseño (color y opacidad),
    /// para las capas `cajaReglas`; `None` con la caja automática.
    pub(super) caja_pedida: Option<([u8; 4], f32)>,
    /// Los colores de la carta en orden WUBRG (`{color1}`/`{color2}`).
    pub(super) colores: Vec<String>,
    /// Lo que se sabe de la carta para las condiciones de las capas.
    pub(super) hechos: Hechos,
    /// Estrechado mínimo del título y el tipo (1 = ninguno).
    pub(super) estrechado_base: f32,
    /// Grosores pedidos por la colección.
    pub(super) pesos: super::efectivo::Pesos,
    /// Ajustes finos de posición (los huecos de texto se mueven con ellos).
    pub(super) retoques: super::modelo::FineTuning,
    letra_titulo: Option<String>,
    letra_reglas: Option<String>,
}

/// Un hueco ya evaluado (mm).
#[derive(Debug, Clone, Copy)]
pub(super) struct Caja {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
}

impl Carta<'_> {
    pub(super) fn hueco(&self, nombre: &str) -> Option<(Hueco, Caja)> {
        let h = self.estilo.huecos.get(nombre)?.clone();
        let ev = |v: &super::pack::Valor| v.evaluar(&self.nums).unwrap_or(0.0);
        let r = &self.retoques;
        // Los ajustes finos bajan (o suben) cada texto; las reglas se mueven
        // al pintarlas, no aquí (su hueco también mide la caja).
        let dy = match nombre {
            "titulo" => r.title,
            "coste" => r.cost,
            "tipo" => r.type_line,
            "emblema" => r.emblem,
            "fr" => r.pt,
            "pie" => r.footer,
            _ => 0.0,
        } as f32;
        let c = Caja {
            x: ev(&h.x),
            y: ev(&h.y) + dy,
            w: ev(&h.w),
            h: ev(&h.h),
        };
        (c.w > 0.0 && c.h > 0.0).then_some((h, c))
    }

    pub(super) fn color(&self, h: &Hueco) -> Color {
        let c = h.color.as_deref().unwrap_or("#000000");
        let c = rellenar(c, &self.vars);
        rgba(&c).unwrap_or(NEGRO)
    }

    /// La letra de un papel: la del pack, o la que pida la colección si está.
    fn letra(&mut self, rol: &str) -> LetraDef {
        let base = self
            .rec
            .pack
            .def
            .letras
            .get(rol)
            .cloned()
            .unwrap_or(LetraDef {
                familia: "Spectral".into(),
                peso: 400.0,
                mayuscula: 0.66,
            });
        let pedida = match rol {
            "titulo" | "tipo" | "fr" => self.letra_titulo.clone(),
            "reglas" | "cinta" => self.letra_reglas.clone(),
            _ => None,
        };
        // El grosor pedido por la colección pisa al del pack.
        let peso = match rol {
            "titulo" => self.pesos.titulo,
            "tipo" => self.pesos.tipo,
            "reglas" => self.pesos.reglas,
            _ => None,
        };
        let mut l = self.letra_sin_peso(base, pedida);
        if let Some(p) = peso {
            l.peso = p as f32;
        }
        l
    }

    fn letra_sin_peso(&mut self, base: LetraDef, pedida: Option<String>) -> LetraDef {
        match pedida {
            Some(f) if self.motor.letras.tiene(&f) => LetraDef {
                familia: f,
                // La proporción de la importada no se conoce: la de la casa.
                ..base
            },
            Some(f) => {
                let aviso = Aviso::LetraNoEncontrada { familia: f };
                if !self.avisos.contains(&aviso) {
                    self.avisos.push(aviso);
                }
                base
            }
            None => base,
        }
    }

    /// Lo que se agranda o achica el texto de un papel (ajustes finos).
    pub(super) fn escala_texto(&self, rol: &str) -> f32 {
        let r = &self.retoques;
        let v = match rol {
            "titulo" | "cinta" => r.title_size,
            "coste" => r.cost_size,
            "tipo" => r.type_size,
            "reglas" => r.rules_size,
            "fr" => r.pt_size,
            "pie" => r.footer_size,
            _ => None,
        };
        v.unwrap_or(1.0) as f32
    }

    fn estilo_texto(&mut self, h: &Hueco) -> (Estilo, f32) {
        let rol = h.rol.as_deref().unwrap_or("reglas");
        let letra = self.letra(rol);
        let tam = self.l.x(h.mayuscula.unwrap_or(1.78)) / letra.mayuscula.max(0.3)
            * self.escala_texto(rol);
        (
            Estilo {
                familia: letra.familia,
                peso: letra.peso,
                tam,
                interlinea: 1.06
                    * if rol == "reglas" {
                        self.retoques.rules_line_spacing.unwrap_or(1.0) as f32
                    } else {
                        1.0
                    },
                color: self.color(h),
                alinear: match h.alinear {
                    Alineacion::Inicio => Alinear::Inicio,
                    Alineacion::Centro => Alinear::Centro,
                    Alineacion::Fin => Alinear::Fin,
                },
                versalitas: h.versalitas,
                // Los colores de las reglas, si el pack los trae.
                resalte: self
                    .rec
                    .pack
                    .def
                    .colores_texto
                    .as_ref()
                    .filter(|_| rol == "reglas")
                    .and_then(|c| c.resalte.as_deref())
                    .and_then(rgba),
                habilidad: self
                    .rec
                    .pack
                    .def
                    .colores_texto
                    .as_ref()
                    .filter(|_| rol == "reglas")
                    .and_then(|c| c.habilidad.as_deref())
                    .and_then(rgba),
            },
            letra.mayuscula,
        )
    }

    /// Un trozo de SVG en mm del lienzo, con los degradados de la carta.
    pub(super) fn forma(&mut self, cuerpo: &str) {
        let defs = self.vars.get("defs").cloned().unwrap_or_default();
        let (w, h) = (self.l.w_mm, self.l.h_mm);
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}"><defs>{defs}</defs>{cuerpo}</svg>"#
        );
        let _ = self.l.svg(&svg);
    }

    /// Un texto corto centrado en `(cx, cy)` mm, estrechado si no cabe.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn texto_centrado(
        &mut self,
        texto: &str,
        rol: &str,
        cap_mm: f32,
        color: Color,
        cx: f32,
        cy: f32,
        ancho_max: f32,
    ) {
        let letra = self.letra(rol);
        let e = Estilo {
            familia: letra.familia,
            peso: letra.peso,
            tam: self.l.x(cap_mm) / letra.mayuscula.max(0.3) * self.escala_texto(rol),
            interlinea: 1.06,
            color,
            alinear: Alinear::Inicio,
            versalitas: false,
            resalte: None,
            habilidad: None,
        };
        let p = texto::con_simbolos(texto);
        let (b, f, _) = linea_ajustada(
            &mut self.motor.m,
            &mut self.motor.letras,
            &p,
            &e,
            self.l.x(ancho_max),
            0.6,
        );
        let w = b.ancho * f;
        let x0 = self.l.x(cx) - w / 2.0;
        let y0 = self.l.y(cy) - b.base + e.tam * letra.mayuscula / 2.0;
        self.faltan.extend(pintar_bloque(
            &mut self.l.px.as_mut(),
            &b,
            x0,
            y0,
            Pincel {
                escala_x: f,
                color: None,
            },
            self.rec.simbolos,
        ));
    }

    /// Símbolos de un coste en fila, centrados en `(cx, cy)` mm.
    pub(super) fn simbolos_en_fila(
        &mut self,
        coste: &str,
        cx: f32,
        cy: f32,
        lado: f32,
        ancho_max: f32,
    ) {
        let s = secuencia(coste);
        if s.is_empty() {
            return;
        }
        let n = s.len() as f32;
        let lado = lado.min(ancho_max / (n * 1.05));
        let total = n * lado + (n - 1.0) * lado * 0.05;
        let mut x = cx - total / 2.0;
        for sim in s {
            let (sx, sy, sl) = (self.l.x(x), self.l.y(cy - lado / 2.0), self.l.x(lado));
            if !self
                .rec
                .simbolos
                .pintar(&sim, &mut self.l.px.as_mut(), sx, sy, sl)
            {
                self.faltan.push(sim);
            }
            x += lado * 1.05;
        }
    }

    /// Una línea centrada en vertical en su hueco, estrechada si no cabe.
    pub(super) fn linea(
        &mut self,
        nombre: &str,
        p: &Parrafo,
        ancho_max: Option<f32>,
        squeeze: f32,
    ) {
        let Some((h, c)) = self.hueco(nombre) else {
            return;
        };
        let mayus;
        let p = if h.mayusculas {
            mayus = texto::en_mayusculas(p);
            &mayus
        } else {
            p
        };
        let (e, cap) = self.estilo_texto(&h);
        let ancho = ancho_max.unwrap_or(c.w).min(c.w);
        // El estrechado mínimo de la colección, solo en el título y el tipo.
        let base = if matches!(nombre, "titulo" | "tipo") {
            self.estrechado_base
        } else {
            1.0
        };
        let (b, f, reducido) = linea_ajustada_con_base(
            &mut self.motor.m,
            &mut self.motor.letras,
            p,
            &e,
            self.l.x(ancho),
            squeeze,
            base,
        );
        if reducido {
            self.avisos.push(Aviso::TituloReducido {
                hueco: nombre.into(),
            });
        }
        let w = b.ancho * f;
        let x0 = match h.alinear {
            Alineacion::Inicio => self.l.x(c.x),
            Alineacion::Centro => self.l.x(c.x) + (self.l.x(c.w) - w) / 2.0,
            Alineacion::Fin => self.l.x(c.x + c.w) - w,
        };
        let cy = self.l.y(c.y + c.h / 2.0);
        let y0 = cy - b.base + e.tam * cap / 2.0;
        if h.sombra {
            let d = (e.tam * 0.05).max(1.0);
            self.faltan.extend(pintar_bloque(
                &mut self.l.px.as_mut(),
                &b,
                x0 + d,
                y0 + d,
                Pincel {
                    escala_x: f,
                    color: Some([0, 0, 0, 170]),
                },
                self.rec.simbolos,
            ));
        }
        self.faltan.extend(pintar_bloque(
            &mut self.l.px.as_mut(),
            &b,
            x0,
            y0,
            Pincel {
                escala_x: f,
                color: None,
            },
            self.rec.simbolos,
        ));
    }

    /// El coste, alineado a la derecha del hueco `coste`. Devuelve dónde
    /// empieza (mm), para que el título no lo pise.
    fn coste(&mut self, coste: &str) -> Option<f32> {
        let (h, c) = self.hueco("coste")?;
        let simbolos = secuencia(coste);
        if simbolos.is_empty() {
            return None;
        }
        // Con símbolos propios del pack (las fichas de Balatro): sin la sombra
        // de debajo y con la separación del pack.
        let propios = self.rec.pack.simbolos.is_some();
        // Los símbolos más pequeños (ajustes finos) siguen pegados a la derecha
        // y centrados en la altura del hueco.
        let lado = c.h * self.escala_texto("coste");
        let hueco = lado * self.rec.pack.def.separacion_coste.unwrap_or(0.05);
        let n = simbolos.len() as f32;
        let total = n * lado + (n - 1.0) * hueco;
        // A la derecha del hueco, o centrado si lo pide.
        let mut x = if h.alinear == Alineacion::Centro {
            c.x + (c.w - total) / 2.0
        } else {
            c.x + c.w - total
        };
        let x0 = x;
        let cy = c.y + c.h / 2.0;
        for s in simbolos {
            let (px, py, r) = (
                self.l.x(x + lado / 2.0 - lado * 0.05),
                self.l.y(cy + lado * 0.08),
                self.l.x(lado / 2.0),
            );
            if !propios {
                super::simbolos::circulo(&mut self.l.px.as_mut(), px, py, r, [0, 0, 0, 200]);
            }
            let (sx, sy, sl) = (self.l.x(x), self.l.y(cy - lado / 2.0), self.l.x(lado));
            if !self
                .rec
                .simbolos
                .pintar(&s, &mut self.l.px.as_mut(), sx, sy, sl)
            {
                self.faltan.push(s);
            }
            x += lado + hueco;
        }
        Some(x0)
    }

    fn imagen(&mut self, nombre: &str, blob: &str) {
        let Some((_, c)) = self.hueco(nombre) else {
            return;
        };
        if let Some(img) = self.rec.imagenes.imagen(blob) {
            pintar_imagen(&mut self.l, &img, (c.x, c.y, c.w, c.h));
        }
    }

    pub(super) fn simbolo(&mut self, nombre: &str, codigo: &str) {
        let Some((_, c)) = self.hueco(nombre) else {
            return;
        };
        let lado = c.w.min(c.h);
        let (x, y, l) = (
            self.l.x(c.x + (c.w - lado) / 2.0),
            self.l.y(c.y + (c.h - lado) / 2.0),
            self.l.x(lado),
        );
        if !self
            .rec
            .simbolos
            .pintar(codigo, &mut self.l.px.as_mut(), x, y, l)
        {
            self.faltan.push(codigo.into());
        }
    }

    fn capas(&mut self, capas: &[Capa], hechos: &Hechos) {
        for capa in capas {
            if !capa.si.cumple(hechos) {
                continue;
            }
            if capa.caja_reglas {
                self.tinte_caja(capa);
                continue;
            }
            if capa.png.is_some() {
                self.capa_png(capa);
                continue;
            }
            let Some(svg) =
                self.rec.pack.ficheros.get(&capa.svg).or_else(|| {
                    Pack::de_fabrica_compartido().and_then(|p| p.ficheros.get(&capa.svg))
                })
            else {
                continue;
            };
            let s = rellenar(svg, &self.vars);
            let _ = self.l.svg(&s);
        }
    }

    /// Una capa PNG: la imagen del color del marco, estirada a su zona (o a
    /// la carta entera).
    pub(super) fn capa_png(&mut self, capa: &Capa) {
        let pack = self.rec.pack;
        let Some(ruta) = capa.png_para(&pack.def, self.clave_color, &self.colores) else {
            return;
        };
        // Una variable que esta carta no tiene (caja fija): no va.
        let desde = match capa.desde.as_ref().map(|d| d.evaluar(&self.nums)) {
            Some(Ok(d)) => Some(d),
            Some(Err(_)) => return,
            None => None,
        };
        let z = match (&capa.en, desde) {
            (Some(z), _) => match z.evaluar(&self.nums) {
                Ok(z) => z,
                Err(_) => return,
            },
            // El trozo, en su mismo sitio de la carta.
            (None, Some(d)) => d,
            (None, None) => super::pack::Zona {
                x: 0.0,
                y: 0.0,
                w: self.l.w_mm,
                h: self.l.h_mm,
            },
        };
        let trozo = Trozo {
            recorte: desde.map(|d| {
                let (w, h) = (self.l.w_mm, self.l.h_mm);
                [d.x / w, d.y / h, d.w / w, d.h / h]
            }),
            mascara: capa.mascara.as_deref(),
        };
        let escalado = capa.escalado.unwrap_or(pack.def.escalado);
        if let Some([arriba, abajo]) = capa.tramos {
            self.png_en_tramos(
                &ruta,
                z,
                arriba,
                abajo,
                escalado,
                capa.mezcla.unwrap_or_default(),
                capa.opacidad.unwrap_or(1.0),
            );
            return;
        }
        let mezcla = capa.mezcla.unwrap_or_default();
        let opacidad = capa.opacidad.unwrap_or(1.0);
        // Solo las capas que cambian con el color del marco.
        let de_color = capa.png.as_deref().is_some_and(|p| p.contains("{color}"));
        match self.extra_marco {
            ExtraMarco::Degradado(b) if de_color => {
                self.png_trozo(&ruta, trozo, z, escalado, mezcla, opacidad);
                if let Some(ruta_b) = capa.png_para(&pack.def, b, &self.colores) {
                    self.png_con(
                        &ruta_b,
                        trozo,
                        z,
                        escalado,
                        mezcla,
                        opacidad,
                        fundir_a_la_derecha,
                    );
                }
            }
            ExtraMarco::Tinte(color) if de_color => {
                self.png_con(&ruta, trozo, z, escalado, mezcla, opacidad, |img, _, _| {
                    tenir(img, color)
                });
            }
            _ => self.png_trozo(&ruta, trozo, z, escalado, mezcla, opacidad),
        }
    }

    /// El tinte de la caja de reglas en la zona de la capa, si se pide.
    fn tinte_caja(&mut self, capa: &Capa) {
        let (Some((color, opacidad)), Some(z)) = (self.caja_pedida, &capa.en) else {
            return;
        };
        let Ok(z) = z.evaluar(&self.nums) else {
            return;
        };
        self.forma(&format!(
            r##"<rect x="{}" y="{}" width="{}" height="{}" fill="{}" fill-opacity="{opacidad}"/>"##,
            z.x,
            z.y,
            z.w,
            z.h,
            hex(color)
        ));
    }

    /// Como `png_trozo`, pasando antes la imagen (ya a su medida) por `f`
    /// con dónde empieza (px del lienzo) y los px por mm.
    #[allow(clippy::too_many_arguments)]
    fn png_con(
        &mut self,
        ruta: &str,
        trozo: Trozo<'_>,
        z: super::pack::Zona,
        escalado: super::pack::Escalado,
        mezcla: Mezcla,
        opacidad: f32,
        f: impl Fn(&mut RgbaImage, f32, f32),
    ) {
        let (x, y) = (self.l.x(z.x).round(), self.l.y(z.y).round());
        let tw = (self.l.x(z.x + z.w).round() - x).max(1.0) as u32;
        let th = (self.l.y(z.y + z.h).round() - y).max(1.0) as u32;
        let Some(img) = imagenes_png::trozo(
            self.rec.pack,
            ruta,
            trozo.recorte,
            trozo.mascara,
            tw,
            th,
            escalado,
        ) else {
            return;
        };
        let mut img = (*img).clone();
        f(&mut img, x, self.l.kx);
        if let Some(p) = a_pixmap(&img) {
            self.l.px.draw_pixmap(
                x as i32,
                y as i32,
                p.as_ref(),
                &pintura_con(mezcla, opacidad),
                Transform::identity(),
                None,
            );
        }
    }

    /// Un PNG del pack en la zona `z` estirado solo por el medio: `arriba` y
    /// `abajo` mm (a la escala de su ancho) se pintan tal cual.
    #[allow(clippy::too_many_arguments)]
    fn png_en_tramos(
        &mut self,
        ruta: &str,
        z: super::pack::Zona,
        arriba: f32,
        abajo: f32,
        escalado: super::pack::Escalado,
        mezcla: Mezcla,
        opacidad: f32,
    ) {
        let Some(o) = imagenes_png::original(self.rec.pack, ruta) else {
            return;
        };
        if z.w <= 0.0 || o.width() == 0 {
            return;
        }
        // Píxeles del PNG por mm (los de la imagen son cuadrados).
        let k = o.width() as f32 / z.w;
        let a_src = ((arriba * k).round() as u32).min(o.height());
        let b_src = ((abajo * k).round() as u32).min(o.height() - a_src);
        let m_src = o.height() - a_src - b_src;
        let x = self.l.x(z.x).round();
        let tw = (self.l.x(z.x + z.w).round() - x).max(1.0) as u32;
        let y0 = self.l.y(z.y).round();
        let y3 = self.l.y(z.y + z.h).round();
        let y1 = self.l.y(z.y + arriba).round().min(y3);
        let y2 = self.l.y(z.y + z.h - abajo).round().max(y1);
        for (sy, sh, dy, dh) in [
            (0, a_src, y0, y1 - y0),
            (a_src, m_src, y1, y2 - y1),
            (a_src + m_src, b_src, y2, y3 - y2),
        ] {
            if sh == 0 || dh < 1.0 {
                continue;
            }
            let trozo = image::imageops::crop_imm(o.as_ref(), 0, sy, o.width(), sh).to_image();
            let img = if escalado == super::pack::Escalado::Pixel {
                reescalar_pixel(&trozo, tw, dh as u32)
            } else {
                reescalar(&trozo, tw, dh as u32, None)
            };
            let Some(p) = img.as_ref().and_then(a_pixmap) else {
                continue;
            };
            self.l.px.draw_pixmap(
                x as i32,
                dy as i32,
                p.as_ref(),
                &pintura_con(mezcla, opacidad),
                Transform::identity(),
                None,
            );
        }
    }

    /// Un PNG del pack estirado a la zona `z` (mm).
    pub(super) fn png_del_pack(
        &mut self,
        ruta: &str,
        z: super::pack::Zona,
        escalado: super::pack::Escalado,
        mezcla: Mezcla,
        opacidad: f32,
    ) {
        self.png_trozo(ruta, Trozo::default(), z, escalado, mezcla, opacidad);
    }

    /// Como `png_del_pack`, con solo un trozo del PNG o recortado por una
    /// máscara (`Trozo`).
    fn png_trozo(
        &mut self,
        ruta: &str,
        trozo: Trozo<'_>,
        z: super::pack::Zona,
        escalado: super::pack::Escalado,
        mezcla: Mezcla,
        opacidad: f32,
    ) {
        let pack = self.rec.pack;
        let (x, y) = (self.l.x(z.x).round(), self.l.y(z.y).round());
        let tw = (self.l.x(z.x + z.w).round() - x).max(1.0) as u32;
        let th = (self.l.y(z.y + z.h).round() - y).max(1.0) as u32;
        let Some(img) =
            imagenes_png::trozo(pack, ruta, trozo.recorte, trozo.mascara, tw, th, escalado)
        else {
            return;
        };
        if let Some(p) = a_pixmap(&img) {
            self.l.px.draw_pixmap(
                x as i32,
                y as i32,
                p.as_ref(),
                &pintura_con(mezcla, opacidad),
                Transform::identity(),
                None,
            );
        }
    }

    /// La caja de la etiqueta de la línea de tipo, si esta carta la lleva.
    pub(super) fn inicio_etiqueta(&self, card: &CardData, cara: usize) -> Option<Caja> {
        let (h, caja) = self.hueco("etiqueta")?;
        let valor = valor_de_etiqueta(card, cara, h.contenido.as_deref())?;
        self.rec
            .pack
            .def
            .etiquetas
            .contains_key(&valor)
            .then_some(caja)
    }

    /// La etiqueta de la línea de tipo (la rareza, «Legendaria» o el tipo
    /// de tierra básica): su fondo (un PNG del pack o un color) y su texto en
    /// el idioma del diseño.
    pub(super) fn etiqueta(&mut self, card: &CardData, cara: usize, lang: DesignLang) {
        let Some((h, caja)) = self.hueco("etiqueta") else {
            return;
        };
        let Some((valor, def)) = valor_de_etiqueta(card, cara, h.contenido.as_deref())
            .and_then(|v| Some((v.clone(), self.rec.pack.def.etiquetas.get(&v)?.clone())))
        else {
            return;
        };
        let z = super::pack::Zona {
            x: caja.x,
            y: caja.y,
            w: caja.w,
            h: caja.h,
        };
        if def.fondo.ends_with(".png") {
            let escalado = self.rec.pack.def.escalado;
            self.png_del_pack(&def.fondo, z, escalado, Mezcla::Normal, 1.0);
        } else if let Some(c) = rgba(&def.fondo) {
            let r = caja.h * 0.3;
            self.forma(&format!(
                r##"<rect x="{}" y="{}" width="{}" height="{}" rx="{r}" fill="rgb({},{},{})" fill-opacity="{}"/>"##,
                caja.x,
                caja.y,
                caja.w,
                caja.h,
                c[0],
                c[1],
                c[2],
                c[3] as f32 / 255.0
            ));
        }
        let codigo = lang.codigo();
        let texto = def
            .texto
            .get(codigo)
            .or_else(|| def.texto.get("en"))
            .cloned()
            .unwrap_or(valor);
        let color = def
            .color
            .as_deref()
            .and_then(rgba)
            .unwrap_or_else(|| self.color(&h));
        let rol = h.rol.clone().unwrap_or_else(|| "tipo".into());
        let cap = h.mayuscula.unwrap_or(caja.h * 0.42);
        let (cx, cy) = (caja.x + caja.w / 2.0, caja.y + caja.h / 2.0);
        // La sombra: el mismo texto, oscuro y un poco abajo a la izquierda.
        if h.sombra {
            let d = cap * 0.14;
            self.texto_centrado(
                &texto,
                &rol,
                cap,
                [0, 0, 0, 120],
                cx - d,
                cy + d,
                caja.w * 0.9,
            );
        }
        self.texto_centrado(&texto, &rol, cap, color, cx, cy, caja.w * 0.9);
    }

    /// El pie: líneas separadas por `\n`, columnas por `\t` (tabuladores del
    /// hueco) y lo que va tras `{>}`, pegado a la derecha. `★` y `✎` van como
    /// iconos.
    fn pie(&mut self, texto: &str) {
        let Some((h, c)) = self.hueco("pie") else {
            return;
        };
        let (e, cap) = self.estilo_texto(&h);
        let paso =
            h.interlinea.unwrap_or(1.6) * self.retoques.footer_line_spacing.unwrap_or(1.0) as f32;
        // Sin tabuladores, las columnas van seguidas con este hueco entre ellas.
        const HUECO: f32 = 1.2;
        let maquetar = |motor: &mut Motor, col: &str| {
            let p = texto::con_simbolos(
                &col.trim()
                    .replace('★', "{ESTRELLA}")
                    .replace('✎', "{PINCEL}"),
            );
            motor.m.maquetar(&mut motor.letras, &p, &e, None)
        };
        for (i, linea) in texto.split('\n').enumerate() {
            let cy = c.y + (i as f32 + 0.5) * paso;
            let y0 = |b: &texto::Bloque, l: &Lienzo| l.y(cy) - b.base + e.tam * cap / 2.0;
            let (linea, derecha) = linea.split_once("{>}").unwrap_or((linea, ""));
            // Lo de la derecha, al final del hueco (si no cabe en la mitad, se
            // estrecha); lo demás llega hasta donde empieza.
            let mut ancho = c.w;
            if !derecha.trim().is_empty() {
                let b = maquetar(self.motor, derecha);
                let cabe = self.l.x(c.w * 0.5);
                let escala = if b.ancho > cabe {
                    (cabe / b.ancho).max(0.5)
                } else {
                    1.0
                };
                let x0 = self.l.x(c.x + c.w) - b.ancho * escala;
                ancho = (x0 / self.l.kx - c.x - HUECO).max(0.0);
                let y = y0(&b, &self.l);
                self.faltan.extend(pintar_bloque(
                    &mut self.l.px.as_mut(),
                    &b,
                    x0,
                    y,
                    Pincel {
                        escala_x: escala,
                        color: None,
                    },
                    self.rec.simbolos,
                ));
            }
            let columnas: Vec<(usize, texto::Bloque)> = linea
                .split('\t')
                .enumerate()
                .filter(|(_, col)| !col.trim().is_empty())
                .map(|(j, col)| (j, maquetar(self.motor, col)))
                .collect();
            // Seguidas: si no caben, se estrecha la línea entera (hasta la mitad).
            let seguidas = h.tabs.is_empty();
            let total: f32 = columnas.iter().map(|(_, b)| b.ancho).sum::<f32>()
                + self.l.x(HUECO) * columnas.len().saturating_sub(1) as f32;
            let escala_linea = if seguidas && total > self.l.x(ancho) {
                (self.l.x(ancho) / total).max(0.5)
            } else {
                1.0
            };
            let mut x_seguida = self.l.x(c.x);
            let indices: Vec<usize> = columnas.iter().map(|(j, _)| *j).collect();
            let limites = limites_de_columnas(&h.tabs, c.x, ancho, &indices);
            for ((_, b), (x, fin)) in columnas.iter().zip(limites) {
                let (x0, escala) = if seguidas {
                    (x_seguida, escala_linea)
                } else {
                    let cabe = self.l.x(fin - x);
                    let escala = if b.ancho > cabe && cabe > 0.0 {
                        (cabe / b.ancho).max(0.5)
                    } else {
                        1.0
                    };
                    (self.l.x(x), escala)
                };
                let y = y0(b, &self.l);
                self.faltan.extend(pintar_bloque(
                    &mut self.l.px.as_mut(),
                    b,
                    x0,
                    y,
                    Pincel {
                        escala_x: escala,
                        color: None,
                    },
                    self.rec.simbolos,
                ));
                x_seguida += (b.ancho + self.l.x(HUECO)) * escala;
            }
        }
    }

    /// El indicador de cara a la izquierda del título (sol/luna de las
    /// transformables, triángulos de las MDFC). Devuelve cuánto ocupa (mm).
    fn indicador(&mut self, ind: Indicador) -> f32 {
        let Some((_, t)) = self.hueco("titulo") else {
            return 0.0;
        };
        let lado = (t.h * 0.72).min(3.6);
        let (cx, cy) = (t.x + lado / 2.0 + 0.1, t.y + t.h / 2.0);
        let r = lado / 2.0;
        let cuerpo = match ind {
            Indicador::Sol => format!(
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" fill="#f2f2f2" stroke="#000" stroke-opacity="0.6" stroke-width="0.12"/><circle cx="{cx}" cy="{cy}" r="{ri}" fill="#1a1a1a"/>"##,
                ri = r * 0.45
            ),
            Indicador::Luna => format!(
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" fill="#1a1a1a" stroke="#f2f2f2" stroke-width="0.16"/><path d="M{a},{b} A{rl},{rl} 0 1 0 {c},{d} A{rm},{rm} 0 1 1 {a},{b} Z" fill="#f2f2f2"/>"##,
                a = cx + r * 0.1,
                b = cy - r * 0.62,
                c = cx + r * 0.1,
                d = cy + r * 0.62,
                rl = r * 0.62,
                rm = r * 0.5
            ),
            Indicador::Delante => format!(
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" fill="#1a1a1a" stroke="#f2f2f2" stroke-width="0.16"/><path d="M{x0},{y1} L{cx},{y0} L{x1},{y1} Z" fill="#f2f2f2"/>"##,
                x0 = cx - r * 0.5,
                x1 = cx + r * 0.5,
                y0 = cy - r * 0.45,
                y1 = cy + r * 0.38
            ),
            Indicador::Detras => format!(
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" fill="#1a1a1a" stroke="#f2f2f2" stroke-width="0.16"/><path d="M{x0},{y0} L{x1},{y0} L{cx},{y1} Z" fill="#f2f2f2"/>"##,
                x0 = cx - r * 0.5,
                x1 = cx + r * 0.5,
                y0 = cy - r * 0.38,
                y1 = cy + r * 0.45
            ),
        };
        self.forma(&cuerpo);
        lado + 0.7
    }
}

fn nombre_estilo(s: DesignStyle) -> &'static str {
    match s {
        DesignStyle::Borderless => "borderless",
        DesignStyle::Extended => "extended",
        DesignStyle::Classic => "classic",
        DesignStyle::FullArt => "fullArt",
    }
}

/// El estilo que toca: el del diseño con los cambios del layout que pide la
/// carta (`ficha`) o un layout completo del pack (`saga`, `batalla`…). Si el
/// pack no lo trae, el estilo normal con el aviso.
pub(super) fn estilo_para(
    rec: &Recursos<'_>,
    style: DesignStyle,
    propio: Option<&str>,
    layout: Option<&str>,
    hechos: Option<&Hechos>,
    avisos: &mut Vec<Aviso>,
) -> Result<EstiloDef, DisenoError> {
    // Un estilo propio del pack, si lo tiene; si no, el de serie con aviso.
    let del_pack = propio.and_then(|id| match rec.pack.estilo(id) {
        Ok(e) => Some(e),
        Err(_) => {
            avisos.push(Aviso::EstiloNoEnElPack { estilo: id.into() });
            None
        }
    });
    // Un estilo de serie que el pack no trae: uno propio del pack que lo
    // sustituya (`Pack::propio_para`); si no tiene, el del pack de la casa
    // (sus capas se buscan allí); si tampoco, el despejado del pack.
    let fabrica = Pack::de_fabrica_compartido();
    let base = match del_pack {
        Some(e) => e,
        None => rec
            .pack
            .estilo(nombre_estilo(style))
            .or_else(|e| {
                // Un pack con estilos propios (Balatro) pinta con uno suyo.
                rec.pack
                    .propio_para(nombre_estilo(style))
                    .and_then(|id| rec.pack.estilo(id).ok())
                    .ok_or(e)
            })
            .or_else(|e| {
                fabrica
                    .and_then(|f| f.estilo(nombre_estilo(style)).ok())
                    .ok_or(e)
            })
            .or_else(|_| rec.pack.estilo("borderless"))
            .map_err(|_| DisenoError::Lienzo)?,
    };
    // Sin un layout pedido por la carta (ficha, saga…): el de la variante
    // elegida si el estilo tiene uno con su id; si no, el primero de
    // `layoutSi` que cumpla la carta (tierra no básica…).
    let por_carta: Option<String> = match (layout, hechos) {
        (None, Some(h)) => h
            .variante
            .clone()
            .filter(|v| base.layouts.contains_key(v))
            .or_else(|| {
                base.layout_si
                    .iter()
                    .find(|r| r.si.cumple(h))
                    .map(|r| r.layout.clone())
            }),
        _ => None,
    };
    let layout = layout.or(por_carta.as_deref());
    // Las fichas de un pack sin marco de ficha salen con el estilo del propio
    // pack, tal cual, para que se vean como el resto de cartas del tema (nunca
    // con el marco de la casa), y sin aviso.
    // La clase sin layout propio en el pack: la caja alta, como las demás
    // cartas con secciones.
    let layout = match layout {
        Some("clase")
            if !base.layouts.contains_key("clase") && rec.pack.layout("clase").is_none() =>
        {
            Some("alto")
        }
        l => l,
    };
    let layout = match layout {
        Some("ficha")
            if !base.layouts.contains_key("ficha") && rec.pack.layout("ficha").is_none() =>
        {
            None
        }
        l => l,
    };
    let mut e = base.clone();
    if let Some(l) = layout {
        if let Some(def) = base.layouts.get(l) {
            if let Some(a) = &def.arte {
                e.arte = a.clone();
            }
            if let Some(c) = &def.caja {
                e.caja = Some(c.clone());
            }
            if let Some(c) = &def.capas {
                e.capas = c.clone();
            }
            if let Some(f) = &def.fondo {
                e.fondo = f.clone();
            }
            for (k, v) in &def.huecos {
                e.huecos.insert(k.clone(), v.clone());
            }
        } else if let Some(completo) = rec.pack.layout(l) {
            e = completo.clone();
        } else if let Some(completo) = Pack::de_fabrica_compartido().and_then(|p| p.layout(l)) {
            // Un pack de antes sin este layout: el de la casa (sus capas se
            // buscan también en el pack de la casa).
            e = completo.clone();
        } else {
            // La variante alta es opcional: sin ella, la caja normal.
            if l != "alto" {
                avisos.push(Aviso::PackSinLayout { layout: l.into() });
            }
        }
    }
    Ok(e)
}

/// Qué etiqueta lleva una cara: su rareza (`rare`…), `legendary` si es
/// legendaria o su tipo de tierra básica (`mountain`…), según `contenido`.
pub(super) fn valor_de_etiqueta(
    card: &CardData,
    cara: usize,
    contenido: Option<&str>,
) -> Option<String> {
    let tipo = card
        .faces
        .get(cara)
        .or_else(|| card.faces.first())
        .map(|f| f.type_line.en.to_lowercase())
        .unwrap_or_default();
    match contenido.unwrap_or("rareza") {
        "rareza" => (!card.rarity.is_empty()).then(|| card.rarity.to_lowercase()),
        "supertipo" => {
            (tipo.contains("legendary") || card.es_legendaria()).then(|| "legendary".into())
        }
        "subtipoTierra" => ["plains", "island", "swamp", "mountain", "forest", "wastes"]
            .into_iter()
            .find(|t| tipo.split(|c: char| !c.is_alphanumeric()).any(|p| p == *t))
            .map(str::to_string),
        _ => None,
    }
}

/// Los colores de una cara (o de la carta) en orden WUBRG. Una carta sin
/// color (una tierra), los de su identidad: los que produce.
pub(super) fn colores_wubrg(e: &DisenoEfectivo, cara: usize) -> Vec<String> {
    let card = &e.design.card;
    let propios = card
        .faces
        .get(cara)
        .map(|f| f.colors.as_slice())
        .filter(|c| !c.is_empty())
        .or_else(|| Some(card.colors.as_slice()).filter(|c| !c.is_empty()))
        .unwrap_or(&card.color_identity);
    ["W", "U", "B", "R", "G"]
        .iter()
        .filter(|w| propios.iter().any(|c| c == *w))
        .map(|w| w.to_string())
        .collect()
}

/// Lo que se sabe de una cara para las condiciones del pack (sin lo que
/// depende del marco elegido: corona, cinta y emblema, que pone `pintar_cara`).
pub(super) fn hechos_de(e: &DisenoEfectivo, cara: usize) -> Hechos {
    let card = &e.design.card;
    let f = card.faces.get(cara).or_else(|| card.faces.first());
    let tipo = f.map(|f| f.type_line.en.to_lowercase()).unwrap_or_default();
    Hechos {
        fr: f.is_some_and(|f| f.power.is_some() && f.toughness.is_some()),
        tipos: tipo
            .split(|c: char| !c.is_alphanumeric())
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect(),
        basica: tipo.contains("basic"),
        legendaria: tipo.contains("legendary") || card.es_legendaria(),
        colores: colores_wubrg(e, cara).len() as u8,
        simbolos: f.map_or(0, |f| secuencia(&f.mana_cost).len() as u8),
        variante: e.design.variante_pack.clone(),
        caja: match e.frame.rules_box {
            RulesBox::Auto => "auto",
            RulesBox::Dark => "dark",
            RulesBox::Light => "light",
            RulesBox::Tint { .. } => "tint",
        }
        .into(),
        ..Hechos::default()
    }
}

/// Rellena las fichas del pie.
pub(super) fn pie_texto(e: &DisenoEfectivo, lang: DesignLang, original: &str) -> String {
    pie_con_plantilla(e, lang, original, e.pie.template.get(lang))
}

/// Dónde empieza y hasta dónde puede llegar (mm) cada columna con texto de
/// una línea del pie (`indices`: su número de columna en la plantilla): en su
/// tabulador, hasta donde empieza la siguiente que lleve texto; la última,
/// hasta el final del hueco.
fn limites_de_columnas(tabs: &[f32], x: f32, w: f32, indices: &[usize]) -> Vec<(f32, f32)> {
    let tab = |j: usize| {
        x + if j == 0 {
            0.0
        } else {
            tabs.get(j - 1).copied().unwrap_or(0.0)
        }
    };
    indices
        .iter()
        .enumerate()
        .map(|(i, j)| {
            let fin = indices.get(i + 1).map_or(x + w, |k| tab(*k) - 0.4);
            (tab(*j), fin)
        })
        .collect()
}

/// El pie con la plantilla del pack (`plantillaPie`) si la colección no ha
/// puesto una propia.
pub(super) fn pie_del_pack(
    e: &DisenoEfectivo,
    lang: DesignLang,
    original: &str,
    pack: &super::pack::PackDef,
) -> String {
    let propia = e.pie.template.get(lang);
    let del_pack = pack
        .plantilla_pie
        .get(lang.codigo())
        .or_else(|| pack.plantilla_pie.get("en"));
    match del_pack {
        Some(t) if propia == super::efectivo::PIE_DEFECTO => {
            pie_con_plantilla(e, lang, original, t)
        }
        _ => pie_texto(e, lang, original),
    }
}

fn pie_con_plantilla(
    e: &DisenoEfectivo,
    lang: DesignLang,
    original: &str,
    plantilla: &str,
) -> String {
    let p = &e.pie;
    let idioma = super::textos_motor::codigo_pie(lang);
    let mut t = plantilla.to_string();
    // Universo, año y créditos: si la plantilla no los pone, van a la derecha
    // de la última línea (como el «™ & ©» de las cartas impresas).
    if !["{credit}", "{universe}", "{year}"]
        .iter()
        .any(|k| plantilla.contains(k))
    {
        let derechos = format!("{} {}", p.year.trim(), p.credit.trim());
        let derechos = match derechos.trim() {
            "" => String::new(),
            d => format!("© {d}"),
        };
        let extra = [p.universe.trim(), derechos.as_str()]
            .into_iter()
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if !extra.is_empty() {
            t = format!("{t}{{>}}{extra}");
        }
    }
    for (k, v) in [
        ("{num}", p.number.as_str()),
        ("{rarity}", p.rarity.as_str()),
        ("{original}", original),
        ("{set}", p.set_code.as_str()),
        ("{lang}", idioma),
        ("{artist}", p.artist.as_str()),
        ("{credit}", p.credit.as_str()),
        ("{universe}", p.universe.as_str()),
        ("{year}", p.year.as_str()),
        ("{brush}", "{PINCEL}"),
        ("{star}", "{ESTRELLA}"),
    ] {
        t = t.replace(k, v);
    }
    t
}

/// La fuerza/resistencia (o lealtad, o defensa) de una cara.
fn fr_texto(f: &super::modelo::FaceData) -> Option<String> {
    match (&f.power, &f.toughness) {
        (Some(p), Some(t)) => Some(format!("{p}/{t}")),
        _ => f.loyalty.clone().or_else(|| f.defense.clone()),
    }
}

/// El indicador de cara de las cartas de doble cara.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Indicador {
    Sol,
    Luna,
    Delante,
    Detras,
}

/// Cómo se pinta una cara (o una pieza de una carta compuesta).
pub(super) struct Opciones {
    pub(super) cara: usize,
    pub(super) cuerpo: Cuerpo,
    /// Lienzo transparente (piezas que se pegan sobre otra cosa).
    pub(super) transparente: bool,
    pub(super) ancho_px: u32,
    pub(super) alto_px: u32,
    pub(super) indicador: Option<Indicador>,
    /// MDFC: la franja con el tipo y el coste de la otra cara.
    pub(super) otra_cara: Option<(String, String)>,
    pub(super) sin_pie: bool,
    pub(super) sin_arte: bool,
    /// Aventura: la cara del libro (va dentro de la caja de reglas).
    pub(super) aventura: Option<usize>,
    /// El libro a la derecha (Preparar) en vez de a la izquierda.
    pub(super) libro_derecha: bool,
}

impl Opciones {
    pub(super) fn de(cara: usize, cuerpo: Cuerpo, ancho_px: u32, alto_px: u32) -> Opciones {
        Opciones {
            cara,
            cuerpo,
            transparente: false,
            ancho_px,
            alto_px,
            indicador: None,
            otra_cara: None,
            sin_pie: false,
            sin_arte: false,
            aventura: None,
            libro_derecha: false,
        }
    }
}

/// Una cara pintada.
pub(super) struct Pieza {
    pub(super) px: Pixmap,
    pub(super) avisos: Vec<Aviso>,
    pub(super) faltan: Vec<String>,
    /// El hueco de la ilustración en fracciones del lienzo (x, y, ancho,
    /// alto), si lo tiene.
    pub(super) arte: Option<[f32; 4]>,
    /// Cuánto sigue la ilustración por debajo de su hueco (extensión del
    /// difuminado), en fracciones del alto del lienzo.
    pub(super) arte_extension: f32,
    /// Cómo se ve girada la ilustración en el hueco (grados, positivo a la
    /// derecha): la mitad de abajo de una Secuela, 90; una partida, -90.
    pub(super) arte_giro: f32,
}

/// Las líneas de la caja de una mazmorra: salas en filas.
fn contenido_de_la_cara(
    d: &super::modelo::CardDesign,
    o: &Opciones,
    lang: DesignLang,
    sabor: Vec<Parrafo>,
) -> Contenido {
    let reglas_txt = reglas_efectivas(d, o.cara, lang);
    if let Some(av) = o.aventura {
        let f = d.card.faces.get(av).cloned().unwrap_or_default();
        return cuerpos::aventura(
            o.libro_derecha,
            &nombre_efectivo(d, av, lang),
            &f.mana_cost,
            f.type_line.get(lang),
            &reglas_efectivas(d, av, lang),
            &reglas_txt,
            sabor,
        );
    }
    let reglas_txt = if d.card.layout == "planar" {
        con_caos(&reglas_txt)
    } else {
        reglas_txt
    };
    cuerpos::contenido(o.cuerpo, &reglas_txt, sabor, lang)
}

/// Los planos llevan el símbolo del caos delante de su habilidad de caos.
fn con_caos(reglas: &str) -> String {
    reglas
        .lines()
        .map(|l| {
            let caos = [
                "Whenever chaos ensues",
                "Siempre que lances caos",
                "Siempre que se desate el caos",
            ]
            .iter()
            .any(|p| l.starts_with(p));
            if caos && !l.starts_with('{') {
                format!("{{CHAOS}} {l}")
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Pinta una cara de una carta diseñada sobre el estilo dado (que ya trae
/// sus huecos, capas y medida).
pub(super) fn pintar_cara(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    estilo: EstiloDef,
    mut avisos: Vec<Aviso>,
    o: &Opciones,
) -> Result<Pieza, DisenoError> {
    let d = &e.design;
    let cara = o.cara;
    let [w_mm, h_mm] = estilo.tamano.unwrap_or([63.0, 88.0]);
    let mut l = Lienzo::nuevo(w_mm, h_mm, o.ancho_px, o.alto_px).ok_or(DisenoError::Lienzo)?;
    if !o.transparente {
        l.px.fill(tiny_skia::Color::BLACK);
    }

    let datos = d.card.faces.get(cara).cloned().unwrap_or_default();
    let (a, b) = colores_marco(e, cara, rec.pack);
    let mezcla = b.as_ref().map_or(a, |b| Tonos::mezcla(&a, b));

    // La caja de reglas: la del estilo, o la que pida el diseño.
    let modo_caja = match &e.frame.rules_box {
        RulesBox::Auto => match estilo.caja_por_defecto {
            CajaPorDefecto::Tinte => (mezcla.tinte, 0.80),
            CajaPorDefecto::Clara => (mezcla.caja, 1.0),
        },
        RulesBox::Dark => ([20, 20, 20, 255], 0.85),
        RulesBox::Light => ([242, 236, 220, 255], 0.92),
        // Con opacidad propia (`#rrggbbaa`) manda ella; sin ella, el 80 % de
        // siempre.
        RulesBox::Tint { hex } => match rgba(hex) {
            Some(c) if hex.len() == 9 => ([c[0], c[1], c[2], 255], f32::from(c[3]) / 255.0),
            Some(c) => (c, 0.80),
            None => (mezcla.tinte, 0.80),
        },
    };
    // Más opaca si se pide en los ajustes finos.
    let refuerzo = e.retoques.rules_box_opacity as f32;
    let modo_caja = (modo_caja.0, modo_caja.1 + (1.0 - modo_caja.1) * refuerzo);
    let tinta_caja = if luminancia(modo_caja.0) * modo_caja.1 + (1.0 - modo_caja.1) * 0.2 > 0.5 {
        mezcla.tinta
    } else {
        BLANCO
    };

    let mut vars = BTreeMap::new();
    vars.insert("defs".to_string(), defs(&a, b.as_ref(), &mezcla));
    for (k, c) in [
        ("claro", mezcla.claro),
        ("medio", mezcla.medio),
        ("oscuro", mezcla.oscuro),
        ("tinte", mezcla.tinte),
        ("caja", mezcla.caja),
        ("fondoCaja", modo_caja.0),
        ("tintaCaja", tinta_caja),
        ("tintaCrema", mezcla.tinta),
        (
            "tintaFr",
            if luminancia(mezcla.medio) > 0.45 {
                mezcla.tinta
            } else {
                BLANCO
            },
        ),
    ] {
        vars.insert(k.to_string(), hex(c));
    }
    vars.insert("opacidadCaja".into(), format!("{}", modo_caja.1));
    // Para los marcos con la caja en la imagen (Original): cuánto oscurecerla.
    vars.insert("refuerzoCaja".into(), format!("{refuerzo}"));
    // La caja de reglas a medida (ajustes finos): cuánto sube su borde de
    // arriba. Los marcos fijos lo usan en sus huecos y en sus SVG.
    let sube_caja = e.retoques.rules_box_height.unwrap_or(0.0) as f32;
    vars.insert("subeCaja".into(), format!("{sube_caja}"));
    // Cuánto sigue la ilustración por debajo de su ventana (el negro del
    // marco empieza ese tanto más abajo). Con un difuminado propio, la
    // extensión también es la propia; sin él, las dos del marco.
    let extension_arte = match e.retoques.art_fade {
        Some(_) => e.retoques.art_fade_extension.unwrap_or(0.0) as f32,
        None => e
            .retoques
            .art_fade_extension
            .map_or(estilo.extension_difuminado, |v| v as f32),
    };
    vars.insert("extensionArte".into(), format!("{extension_arte}"));

    let mut c = Carta {
        l,
        motor,
        rec,
        estilo,
        vars,
        nums: BTreeMap::from([("subeCaja", sube_caja), ("extensionArte", extension_arte)]),
        avisos: Vec::new(),
        faltan: Vec::new(),
        clave_color: clave_marco(e, cara),
        extra_marco: extra_marco(e),
        caja_pedida: (!matches!(e.frame.rules_box, RulesBox::Auto)).then_some(modo_caja),
        colores: colores_wubrg(e, cara),
        hechos: hechos_de(e, cara),
        estrechado_base: e.squeeze_base as f32,
        pesos: e.pesos,
        retoques: e.retoques,
        letra_titulo: e.font_title.clone(),
        letra_reglas: e.font_rules.clone(),
    };
    c.avisos.append(&mut avisos);
    // Con un tinte pedido sobre la caja pintada en la imagen, el texto de las
    // reglas toma el color que se lee sobre él.
    if c.caja_pedida.is_some() && c.estilo.capas.iter().any(|k| k.caja_reglas) {
        if let Some(h) = c.estilo.huecos.get_mut("reglas") {
            h.color = Some(hex(tinta_caja));
        }
    }

    // ── Reglas: maquetar primero, porque la caja que crece mueve el resto.
    if d.faces
        .get(cara)
        .is_some_and(|f| f.textos(lang).rules_override.is_some())
    {
        c.avisos.push(Aviso::ReglasEditadas {
            cara: cara as u32,
            lang,
        });
    }
    let sabor_txt = d
        .faces
        .get(cara)
        .map(|f| f.textos(lang).flavor.clone())
        .filter(|s| d.options.show_flavor && !s.trim().is_empty())
        .unwrap_or_default();
    let sabor = texto::sabor(&sabor_txt);
    let salas = if o.cuerpo == Cuerpo::Mazmorra {
        estructura::mazmorra(&reglas_efectivas(d, cara, lang))
    } else {
        Vec::new()
    };
    let mut contenido = if salas.is_empty() {
        contenido_de_la_cara(d, o, lang, sabor)
    } else {
        Contenido::Texto {
            reglas: Vec::new(),
            sabor: Vec::new(),
        }
    };
    let mut fr = fr_texto(&datos);
    // Leveler: la F/R de base va en la franja de «Subir de nivel», como las
    // de cada rango, y la carta se queda sin la caja de abajo (como impresa).
    if o.cuerpo == Cuerpo::Leveler {
        if let Contenido::Secciones { secciones, .. } = &mut contenido {
            if let Some(s) = secciones.first_mut().filter(|s| s.fr.is_none()) {
                s.fr = fr.take();
            }
        }
    }
    // MDFC: la franja de la otra cara se lleva un poco de la caja.
    let franja = if o.otra_cara.is_some() { 2.6 } else { 0.0 };

    let hueco_reglas = c.estilo.huecos.get("reglas").cloned();
    let caja_din = c.estilo.caja.clone();
    let mut caja_txt: Option<(Hecho, Caja)> = None;
    // La caja que crece, en su alto máximo (altura dinámica apagada): el texto
    // se centra en ella como en las cajas fijas.
    let mut caja_creciente_fija = false;
    let mut caja_reglas_mm: Option<Caja> = None;
    if let Some(h) = hueco_reglas {
        let (est, _) = c.estilo_texto(&h);
        // La letra mínima legible: 6 pt de cuerpo.
        let tam_min = c.l.x(6.0 * MM_POR_PT);
        let paso = c.l.x(0.1 * MM_POR_PT);
        let k = c.l.kx;
        if let Some(din) = &caja_din {
            let ancho = match &h.w {
                super::pack::Valor::Num(n) => *n,
                super::pack::Valor::Expr(_) => 50.0,
            };
            let x_txt = match &h.x {
                super::pack::Valor::Num(n) => *n,
                super::pack::Valor::Expr(_) => 5.95,
            };
            // Encaja el texto en una caja de hasta `max` mm: primero en la
            // altura cómoda (`comodo`) con la letra hasta el 75 %; si no
            // cabe, en la máxima con la letra entera (y bajando lo que haga
            // falta). Con un tamaño elegido a mano en los ajustes finos, ese
            // manda.
            let mut encajar = |max: f32, comodo: Option<f32>| {
                let mut pad_abajo = din.pad_abajo + franja;
                let comoda = comodo
                    .filter(|a| *a < max && c.retoques.rules_size.is_none())
                    .map(|a| {
                        cuerpos::ajustar(
                            &contenido,
                            &mut c.motor.m,
                            &mut c.motor.letras,
                            &est,
                            c.l.x(ancho),
                            c.l.y(a - din.pad_arriba - pad_abajo),
                            (est.tam * LETRA_COMODA).max(tam_min).min(est.tam),
                            paso,
                            k,
                        )
                    })
                    .filter(|aj| aj.1 <= 0.0);
                let mut aj = match comoda {
                    Some(aj) => aj,
                    None => cuerpos::ajustar(
                        &contenido,
                        &mut c.motor.m,
                        &mut c.motor.letras,
                        &est,
                        c.l.x(ancho),
                        c.l.y(max - din.pad_arriba - pad_abajo),
                        tam_min.min(est.tam),
                        paso,
                        k,
                    ),
                };
                let mut alto_txt = aj.0.alto() / c.l.ky;
                // Si el último renglón llega a la caja de fuerza/resistencia,
                // se deja ese hueco libre por abajo.
                if fr.is_some() {
                    if let Some(frh) = c.estilo.huecos.get("fr") {
                        let fr_x = frh.x.evaluar(&BTreeMap::new()).unwrap_or(48.0);
                        let fr_y = frh.y.evaluar(&BTreeMap::new()).unwrap_or(80.0);
                        let libre = c.l.x(fr_x - 0.6 - x_txt);
                        let invade = din.fondo - fr_y + 0.4;
                        if aj.0.fin_ultima_linea() > libre && pad_abajo < invade {
                            pad_abajo = invade;
                            aj = cuerpos::ajustar(
                                &contenido,
                                &mut c.motor.m,
                                &mut c.motor.letras,
                                &est,
                                c.l.x(ancho),
                                c.l.y(max - din.pad_arriba - pad_abajo),
                                tam_min.min(est.tam),
                                paso,
                                k,
                            );
                            alto_txt = aj.0.alto() / c.l.ky;
                        }
                    }
                }
                (aj, alto_txt, pad_abajo)
            };
            // La caja a medida (ajustes finos): más alta, crece por arriba y
            // el texto tiene más sitio; más baja, el texto se vuelve a encajar
            // en ella (con la letra más pequeña si hace falta).
            let sube = c.retoques.rules_box_height.unwrap_or(0.0) as f32;
            let mas = sube.max(0.0);
            // Sin altura dinámica (la pide el usuario en los ajustes finos o
            // la trae el marco de serie), la caja es siempre del alto máximo
            // del marco y la letra solo baja si ni así cabe.
            let fija = c
                .retoques
                .rules_box_dynamic
                .map_or(din.fija_de_serie, |dinamica| !dinamica);
            caja_creciente_fija = fija;
            let comodo = if fija {
                None
            } else {
                din.alto_comodo.map(|a| a + mas)
            };
            let (mut aj, alto_txt, mut pad_abajo) = encajar(din.alto_max + mas, comodo);
            let natural = alto_txt + din.pad_arriba + pad_abajo;
            let base = if fija {
                din.alto_max
            } else {
                natural.clamp(din.alto_min, din.alto_max)
            };
            let pedida = base + sube;
            let alto_caja = if sube < 0.0 {
                let alto = pedida.max(din.pad_arriba + pad_abajo + 3.0);
                (aj, _, pad_abajo) = encajar(alto, None);
                alto
            } else {
                pedida.max(natural).min(din.alto_max + mas)
            };
            if aj.1 > 0.0 {
                c.avisos.push(Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(aj.1 / c.l.ky),
                });
            }
            let y_reglas = din.fondo - alto_caja;
            let y_tipo = y_reglas - din.barra_tipo;
            c.nums.insert("yReglas", y_reglas + din.pad_arriba);
            c.nums
                .insert("altoReglas", alto_caja - din.pad_arriba - pad_abajo);
            c.nums.insert("yTipo", y_tipo);
            c.vars.insert("yReglas".into(), format!("{y_reglas}"));
            c.vars.insert("altoReglas".into(), format!("{alto_caja}"));
            c.vars.insert("yTipo".into(), format!("{y_tipo}"));
            c.vars.insert("yTipo04".into(), format!("{}", y_tipo + 0.4));
            caja_reglas_mm = Some(Caja {
                x: x_txt,
                y: y_reglas,
                w: ancho,
                h: alto_caja,
            });
            caja_txt = Some((
                aj.0,
                Caja {
                    x: x_txt,
                    y: y_reglas + din.pad_arriba,
                    w: ancho,
                    h: alto_caja - din.pad_arriba - pad_abajo,
                },
            ));
        } else if let Some((h, caja)) = c.hueco("reglas") {
            let (est, _) = c.estilo_texto(&h);
            let mut caja = Caja {
                h: caja.h - franja,
                ..caja
            };
            let mut aj = cuerpos::ajustar(
                &contenido,
                &mut c.motor.m,
                &mut c.motor.letras,
                &est,
                c.l.x(caja.w),
                c.l.y(caja.h),
                tam_min.min(est.tam),
                paso,
                k,
            );
            // Altura dinámica pedida en un marco de caja fija (la de serie
            // es la fija): si el texto cabe, la caja se acorta lo que sobra
            // (hasta lo que dejan los ajustes finos: 10 mm menos). El marco
            // la sigue por `subeCaja`; si no lo usa, no pasa nada.
            if c.retoques.rules_box_dynamic == Some(true) && aj.1 <= 0.0 {
                let sube = c.nums.get("subeCaja").copied().unwrap_or(0.0);
                let necesario = aj.0.alto() / c.l.ky + 1.0;
                let recorte = (caja.h - necesario)
                    .min(sube + super::modelo::ALTO_CAJA_MAX_MM as f32)
                    .max(0.0);
                if recorte > 0.05 {
                    c.nums.insert("subeCaja", sube - recorte);
                    match c.hueco("reglas") {
                        Some((_, otra)) if otra.h - franja < caja.h - 0.01 => {
                            c.vars
                                .insert("subeCaja".into(), format!("{}", sube - recorte));
                            caja = Caja {
                                h: otra.h - franja,
                                ..otra
                            };
                            aj = cuerpos::ajustar(
                                &contenido,
                                &mut c.motor.m,
                                &mut c.motor.letras,
                                &est,
                                c.l.x(caja.w),
                                c.l.y(caja.h),
                                tam_min.min(est.tam),
                                paso,
                                k,
                            );
                        }
                        _ => {
                            c.nums.insert("subeCaja", sube);
                        }
                    }
                }
            }
            if aj.1 > 0.0 {
                c.avisos.push(Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(aj.1 / c.l.ky),
                });
            }
            caja_reglas_mm = Some(Caja {
                h: caja.h + franja,
                ..caja
            });
            caja_txt = Some((aj.0, caja));
        }
    }

    // ── El fondo de la ventana del arte (debajo de la ilustración). En la
    // carta sin ilustración (el arrastre en vivo) no: taparía el arte que la
    // pantalla pone debajo.
    if !o.sin_arte && !c.estilo.fondo.is_empty() {
        let fondo = c.estilo.fondo.clone();
        let hechos = c.hechos.clone();
        c.capas(&fondo, &hechos);
    }

    // ── Ilustración.
    let mut hueco_arte = None;
    let mut extension_hueco = 0.0;
    let mut arte_pintado = None;
    {
        let h = c.estilo.arte.clone();
        let ev = |v: &super::pack::Valor| v.evaluar(&c.nums).unwrap_or(0.0);
        let ventana = Caja {
            x: ev(&h.x),
            y: ev(&h.y),
            w: ev(&h.w),
            h: ev(&h.h),
        };
        // Con la extensión del difuminado, la ilustración sigue por debajo
        // de su ventana (hasta el final de la carta como mucho).
        let arte = Caja {
            h: (ventana.h + extension_arte).min(h_mm - ventana.y),
            ..ventana
        };
        let extendido = arte.h - ventana.h;
        if arte.w > 0.0 && arte.h > 0.0 {
            hueco_arte = Some([
                ventana.x / w_mm,
                ventana.y / h_mm,
                ventana.w / w_mm,
                ventana.h / h_mm,
            ]);
            extension_hueco = extendido / h_mm;
        }
        if !o.sin_arte && arte.w > 0.0 && arte.h > 0.0 {
            let r = (ventana.x, ventana.y, ventana.w, ventana.h);
            let img = d
                .faces
                .get(cara)
                .and_then(|f| f.art.as_ref())
                .and_then(|a| rec.imagenes.imagen(&a.image.blob).map(|i| (i, a.framing)));
            match &img {
                Some((i, f)) => pintar_arte(&mut c.l, i, r, f, extendido),
                // Sin ilustración, el fondo del estilo si trae; si no, el gris.
                None if c.estilo.fondo.is_empty() => {
                    sin_arte(&mut c.l, (arte.x, arte.y, arte.w, arte.h))
                }
                None => {}
            }
            arte_pintado = Some((img, r));
        }
        let difuminado = e
            .retoques
            .art_fade
            .map_or(c.estilo.difuminado_arte, |v| v as f32);
        // El difuminado empieza donde siempre y acaba al final de la
        // extensión.
        if arte.w > 0.0 && arte.h > 0.0 {
            difuminar_abajo(
                &mut c.l,
                (arte.x, arte.y, arte.w, arte.h),
                difuminado + extendido,
            );
        }
    }
    if let Some(v) = c.estilo.ventana_arte.clone() {
        let radio = e.retoques.art_radius.map_or(v.radio, |r| r as f32);
        let _ = c.l.svg(&ventana_svg(c.l.w_mm, c.l.h_mm, &v, radio));
    }

    // ── Capas.
    let corona = match e.frame.crown {
        OnOffAuto::On => true,
        OnOffAuto::Off => false,
        OnOffAuto::Auto => {
            d.card.es_legendaria()
                && c.estilo
                    .corona_legendaria
                    .unwrap_or(d.style == DesignStyle::Borderless)
        }
    };
    let original = nombre_original(&d.card);
    let en_cinta = e.original_name == OriginalName::Ribbon && c.estilo.huecos.contains_key("cinta");
    if e.original_name == OriginalName::Ribbon && !en_cinta {
        c.avisos.push(Aviso::SinCinta);
    }
    let cond = Hechos {
        corona,
        fr: fr.is_some(),
        cinta: en_cinta,
        emblema: e.emblem.is_some(),
        ..c.hechos.clone()
    };
    let capas = c.estilo.capas.clone();
    // La curva de la ventana dibujada en una capa de imagen: con un radio
    // propio, se rehace encima de esa capa (y debajo de las siguientes).
    let repinta = match (c.estilo.ventana_arte.clone(), e.retoques.art_radius) {
        (Some(v), Some(radio)) if !v.repintar.is_empty() => Some((v, radio as f32)),
        _ => None,
    };
    let corte = repinta
        .as_ref()
        .and_then(|(v, _)| v.repintar_tras)
        .unwrap_or(capas.len())
        .min(capas.len());
    c.capas(&capas[..corte], &cond);
    if let Some((v, radio)) = &repinta {
        // La caja que crece deja dicho dónde acaba la barra del tipo.
        let bajo_tipo = c
            .vars
            .get("yReglas")
            .and_then(|y| y.parse::<f32>().ok())
            .unwrap_or(0.0);
        repintar_ventana(
            &mut c.l,
            v,
            *radio,
            arte_pintado.as_ref(),
            o.sin_arte,
            bajo_tipo,
        );
    }
    c.capas(&capas[corte..], &cond);

    // ── Textos.
    let squeeze = e.squeeze_min as f32;
    let fin_coste = if d.card.es_ficha() {
        None
    } else {
        c.coste(&datos.mana_cost)
    };
    let nombre = nombre_efectivo(d, cara, lang);
    if d.faces
        .get(cara)
        .is_some_and(|f| f.textos(lang).name.trim().is_empty())
    {
        c.avisos.push(Aviso::FaltaNombre {
            cara: cara as u32,
            lang,
        });
    }
    // El indicador de cara empuja el título a la derecha.
    if let Some(ind) = o.indicador {
        let corre = c.indicador(ind);
        if let Some(t) = c.estilo.huecos.get_mut("titulo") {
            if let (super::pack::Valor::Num(x), super::pack::Valor::Num(w)) = (&mut t.x, &mut t.w) {
                *x += corre;
                *w -= corre;
            }
        }
    }
    // El coste solo le quita sitio al título si van a la misma altura (en
    // algunos marcos va debajo, centrado).
    let coste_al_lado = match (c.hueco("titulo"), c.hueco("coste")) {
        (Some((_, t)), Some((_, k))) => k.y < t.y + t.h && t.y < k.y + k.h,
        _ => false,
    };
    let ancho_titulo = c.hueco("titulo").map(|(_, t)| {
        fin_coste
            .filter(|_| coste_al_lado)
            .map_or(t.w, |x0| x0 - 0.6 - t.x)
    });
    c.linea("titulo", &texto::llano(&nombre), ancho_titulo, squeeze);
    if en_cinta {
        let p = vec![texto::Trozo::Texto {
            texto: original.clone(),
            cursiva: true,
        }];
        c.linea("cinta", &p, None, squeeze);
    }
    let tipo = datos.type_line.get(lang).clone();
    // Lo mismo con la etiqueta (la píldora de la rareza) en la línea de tipo.
    let ancho_tipo = match (c.hueco("tipo"), c.inicio_etiqueta(&d.card, cara)) {
        (Some((_, t)), Some(e)) if e.y < t.y + t.h && t.y < e.y + e.h && e.x > t.x => {
            Some(t.w.min(e.x - 0.6 - t.x))
        }
        _ => None,
    };
    c.linea("tipo", &texto::con_simbolos(&tipo), ancho_tipo, squeeze);
    c.etiqueta(&d.card, cara, lang);
    if let Some(em) = &e.emblem {
        c.imagen("emblema", &em.blob);
    }
    if d.style == DesignStyle::FullArt && c.estilo.huecos.contains_key("simbolo") {
        let s = d
            .card
            .color_identity
            .first()
            .cloned()
            .unwrap_or_else(|| "C".into());
        c.simbolo("simbolo", &s);
    }
    let tinta = c
        .vars
        .get("tintaCaja")
        .and_then(|t| rgba(t))
        .unwrap_or(BLANCO);
    if let Some((hecho, sitio)) = &caja_txt {
        // En las cajas fijas (también la que crece con la altura dinámica
        // apagada), el texto normal va centrado en vertical; las secciones
        // que reparten llenan la caja.
        let reparte = matches!(contenido, Contenido::Secciones { repartir: true, .. });
        let vertical = c
            .estilo
            .huecos
            .get("reglas")
            .and_then(|h| h.alinear_v)
            .unwrap_or(Alineacion::Centro);
        let libre = if (caja_din.is_some() && !caja_creciente_fija) || reparte {
            0.0
        } else {
            let sobra = (sitio.h - hecho.alto() / c.l.ky).max(0.0);
            match vertical {
                Alineacion::Inicio => 0.0,
                Alineacion::Centro => sobra / 2.0,
                Alineacion::Fin => sobra,
            }
        };
        c.pintar_contenido(
            &contenido,
            hecho,
            sitio.x,
            sitio.y + libre + e.retoques.rules as f32,
            sitio.w,
            sitio.h - libre,
            tinta,
        );
    }
    if !salas.is_empty() {
        if let Some((h, mut caja)) = c.hueco("reglas") {
            caja.y += e.retoques.rules as f32;
            let (est, _) = c.estilo_texto(&h);
            let nota = estructura::nota_de_mazmorra(&reglas_efectivas(d, cara, lang));
            c.pintar_mazmorra(&nota, &salas, caja, &est, tinta);
        }
    }
    // Vanguard: los modificadores de mano y vida.
    for (hueco, valor, es, en) in [
        ("mano", &d.card.hand_modifier, "Mano", "Hand"),
        ("vida", &d.card.life_modifier, "Vida", "Life"),
    ] {
        if let Some(v) = valor {
            let palabra = if lang == DesignLang::Es { es } else { en };
            c.linea(
                hueco,
                &texto::llano(&format!("{palabra} {v}")),
                None,
                squeeze,
            );
        }
    }
    if let (Some((tipo_otra, coste_otra)), Some(caja)) = (&o.otra_cara, caja_reglas_mm) {
        c.franja_otra_cara(tipo_otra, coste_otra, caja, tinta);
    }
    if let Some(fr) = &fr {
        c.linea("fr", &texto::llano(fr), None, squeeze);
    }
    if !o.sin_pie {
        let original_pie = if en_cinta || e.original_name == OriginalName::Hidden {
            String::new()
        } else {
            original
        };
        let pie = pie_del_pack(e, lang, &original_pie, &c.rec.pack.def);
        c.pie(&pie);
    }

    // ── Avisos.
    let falta = Aviso::FaltanLetras { lang };
    if !c.motor.letras.tiene_escritura(lang.escritura()) && !c.avisos.contains(&falta) {
        c.avisos.push(falta);
    }
    if d.card.sin_texto_en(lang) && !c.avisos.contains(&Aviso::SinTextoOficial) {
        c.avisos.push(Aviso::SinTextoOficial);
    }
    Ok(Pieza {
        px: c.l.px,
        avisos: c.avisos,
        faltan: c.faltan,
        arte: hueco_arte,
        arte_extension: extension_hueco,
        arte_giro: 0.0,
    })
}

impl Carta<'_> {
    /// MDFC: la pestaña de abajo a la izquierda con el tipo y el coste de la
    /// otra cara.
    fn franja_otra_cara(&mut self, tipo: &str, coste: &str, caja: Caja, tinta: Color) {
        let (h, y) = (2.2, caja.y + caja.h - 2.45);
        let w = (caja.w * 0.62).min(34.0);
        let x = caja.x - 0.3;
        self.forma(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="0.5" fill="#101010" fill-opacity="0.85" stroke="url(#marco)" stroke-width="0.2"/><path d="M{a},{b} L{c},{d} L{a},{e} Z" fill="#f2f2f2"/>"##,
            a = x + 0.7,
            b = y + 0.55,
            c = x + 1.7,
            d = y + h / 2.0,
            e = y + h - 0.55
        ));
        let _ = tinta;
        let texto = if coste.is_empty() {
            tipo.to_string()
        } else {
            format!("{tipo}  {coste}")
        };
        let ancho = w - 2.6;
        self.texto_izquierda(
            &texto,
            "reglas",
            0.95,
            [242, 242, 242, 255],
            x + 2.2,
            y + h / 2.0,
            ancho,
        );
    }

    /// Un texto corto alineado a la izquierda en `x`, centrado en `cy` (mm).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn texto_izquierda(
        &mut self,
        texto: &str,
        rol: &str,
        cap_mm: f32,
        color: Color,
        x: f32,
        cy: f32,
        ancho_max: f32,
    ) {
        let letra = self.letra(rol);
        let e = Estilo {
            familia: letra.familia,
            peso: letra.peso,
            tam: self.l.x(cap_mm) / letra.mayuscula.max(0.3) * self.escala_texto(rol),
            interlinea: 1.06,
            color,
            alinear: Alinear::Inicio,
            versalitas: false,
            resalte: None,
            habilidad: None,
        };
        let p = texto::con_simbolos(texto);
        let (b, f, _) = linea_ajustada(
            &mut self.motor.m,
            &mut self.motor.letras,
            &p,
            &e,
            self.l.x(ancho_max),
            0.6,
        );
        let y0 = self.l.y(cy) - b.base + e.tam * letra.mayuscula / 2.0;
        let x0 = self.l.x(x);
        self.faltan.extend(pintar_bloque(
            &mut self.l.px.as_mut(),
            &b,
            x0,
            y0,
            Pincel {
                escala_x: f,
                color: None,
            },
            self.rec.simbolos,
        ));
    }
}

pub(super) fn mascara_redondeada(w: u32, h: u32, r: f32) -> Option<Mask> {
    let mut m = Mask::new(w, h)?;
    let rect = Rect::from_xywh(0.0, 0.0, w as f32, h as f32)?;
    let mut pb = PathBuilder::new();
    let (x0, y0, x1, y1) = (rect.left(), rect.top(), rect.right(), rect.bottom());
    let k = 0.552_284_8 * r;
    pb.move_to(x0 + r, y0);
    pb.line_to(x1 - r, y0);
    pb.cubic_to(x1 - r + k, y0, x1, y0 + r - k, x1, y0 + r);
    pb.line_to(x1, y1 - r);
    pb.cubic_to(x1, y1 - r + k, x1 - r + k, y1, x1 - r, y1);
    pb.line_to(x0 + r, y1);
    pb.cubic_to(x0 + r - k, y1, x0, y1 - r + k, x0, y1 - r);
    pb.line_to(x0, y0 + r);
    pb.cubic_to(x0, y0 + r - k, x0 + r - k, y0, x0 + r, y0);
    pb.close();
    let ruta = pb.finish()?;
    m.fill_path(&ruta, FillRule::Winding, true, Transform::identity());
    Some(m)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::efectivo::efectivo;
    use crate::diseno::modelo::tests::diseno_de_prueba;
    use crate::diseno::modelo::{FaceData, FrameColor};

    fn casa() -> Pack {
        Pack::de_fabrica().unwrap()
    }

    #[test]
    fn pixel_art_sin_suavizar() {
        // Un damero de 2×2 ampliado ×3: bloques de 3×3 exactos.
        let img = RgbaImage::from_fn(2, 2, |x, y| {
            if (x + y) % 2 == 0 {
                image::Rgba([0, 0, 0, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        });
        let g = reescalar_pixel(&img, 6, 6).unwrap();
        for (x, y, p) in g.enumerate_pixels() {
            assert_eq!(p, img.get_pixel(x / 3, y / 3), "({x},{y})");
        }
        // Y al reducir bloques de 2×2 a 1 píxel, el color del bloque, sin halos.
        let reducida = reescalar_pixel(&reescalar_pixel(&img, 4, 4).unwrap(), 2, 2).unwrap();
        assert_eq!(reducida, img);
    }

    #[test]
    fn oscurecer_se_queda_con_el_mas_oscuro() {
        let pintar = |mezcla: Mezcla| {
            let mut fondo = Pixmap::new(1, 1).unwrap();
            fondo.fill(tiny_skia::Color::from_rgba8(100, 100, 100, 255));
            let mut capa = Pixmap::new(1, 1).unwrap();
            capa.fill(tiny_skia::Color::from_rgba8(160, 160, 160, 255));
            fondo.draw_pixmap(
                0,
                0,
                capa.as_ref(),
                &pintura_con(mezcla, 1.0),
                Transform::identity(),
                None,
            );
            fondo.pixel(0, 0).unwrap().red()
        };
        assert_eq!(pintar(Mezcla::Normal), 160);
        assert_eq!(pintar(Mezcla::Oscurecer), 100);
        assert!(pintar(Mezcla::Multiplicar) < 100);
    }

    /// El color medio de una paleta del pack.
    fn medio(pack: &Pack, clave: &str) -> [u8; 4] {
        rgba(&pack.def.paletas[clave].medio).unwrap()
    }

    /// Una carta normal de una cara con esos colores y ese tipo.
    fn carta(colores: &[&str], tipo: &str) -> DisenoEfectivo {
        let mut d = diseno_de_prueba();
        d.card.layout = "normal".into();
        d.card.faces.truncate(1);
        d.faces.truncate(1);
        d.card.colors.clear();
        d.card.faces[0].colors = colores.iter().map(|c| c.to_string()).collect();
        d.card.faces[0].type_line.en = tipo.into();
        efectivo(&d, None)
    }

    fn marco(e: &DisenoEfectivo, pack: &Pack) -> ([u8; 4], Option<[u8; 4]>) {
        let (a, b) = colores_marco(e, 0, pack);
        (a.medio, b.map(|b| b.medio))
    }

    // ── Colores del marco ──

    #[test]
    fn un_color_da_su_paleta_sin_degradado() {
        let p = casa();
        let e = carta(&["U"], "Creature — Merfolk");
        assert_eq!(marco(&e, &p), (medio(&p, "U"), None));
    }

    #[test]
    fn dos_colores_dan_un_degradado_en_su_orden() {
        let p = casa();
        let e = carta(&["W", "U"], "Instant");
        assert_eq!(marco(&e, &p), (medio(&p, "W"), Some(medio(&p, "U"))));
        let e = carta(&["U", "W"], "Instant");
        assert_eq!(marco(&e, &p), (medio(&p, "U"), Some(medio(&p, "W"))));
    }

    #[test]
    fn tres_o_mas_colores_son_oro() {
        let p = casa();
        let e = carta(&["W", "U", "B"], "Sorcery");
        assert_eq!(marco(&e, &p), (medio(&p, "gold"), None));
        let e = carta(&["W", "U", "B", "R", "G"], "Sorcery");
        assert_eq!(marco(&e, &p), (medio(&p, "gold"), None));
    }

    #[test]
    fn una_tierra_sin_color_usa_su_identidad_o_la_paleta_de_tierra() {
        let p = casa();
        // Sin identidad: el marco de tierra.
        let e = carta(&[], "Land");
        assert_eq!(marco(&e, &p), (medio(&p, "land"), None));
        // Con un color de identidad: ese color.
        let mut e = carta(&[], "Land — Forest");
        e.design.card.color_identity = vec!["G".into()];
        assert_eq!(marco(&e, &p), (medio(&p, "G"), None));
        // Con dos: degradado.
        e.design.card.color_identity = vec!["G".into(), "W".into()];
        assert_eq!(marco(&e, &p), (medio(&p, "G"), Some(medio(&p, "W"))));
        // Una tierra artefacto sin identidad es tierra, no artefacto.
        let e = carta(&[], "Artifact Land");
        assert_eq!(marco(&e, &p), (medio(&p, "land"), None));
    }

    #[test]
    fn sin_color_es_artefacto_o_incoloro() {
        let p = casa();
        let e = carta(&[], "Artifact — Equipment");
        assert_eq!(marco(&e, &p), (medio(&p, "artifact"), None));
        let e = carta(&[], "Creature — Eldrazi");
        assert_eq!(marco(&e, &p), (medio(&p, "colorless"), None));
        // Letras que no son colores se ignoran.
        let e = carta(&["C", "X"], "Creature — Eldrazi");
        assert_eq!(marco(&e, &p), (medio(&p, "colorless"), None));
    }

    #[test]
    fn sin_colores_en_la_cara_se_usan_los_de_la_carta() {
        let p = casa();
        let mut e = carta(&[], "Instant");
        e.design.card.colors = vec!["R".into()];
        assert_eq!(marco(&e, &p), (medio(&p, "R"), None));
        // Una cara que no existe también cae a los de la carta.
        let (a, b) = colores_marco(&e, 5, &p);
        assert_eq!((a.medio, b.is_none()), (medio(&p, "R"), true));
    }

    #[test]
    fn el_color_pedido_pisa_al_automatico() {
        let p = casa();
        let mut e = carta(&["U"], "Instant");
        e.frame.color = FrameColor::Palette {
            palette: Palette::Gold,
        };
        assert_eq!(marco(&e, &p), (medio(&p, "gold"), None));
        e.frame.color = FrameColor::Gradient {
            a: Palette::R,
            b: Palette::G,
        };
        assert_eq!(marco(&e, &p), (medio(&p, "R"), Some(medio(&p, "G"))));
        e.frame.color = FrameColor::Custom {
            hex: "#102030".into(),
        };
        assert_eq!(marco(&e, &p), ([16, 32, 48, 255], None));
        // Un color propio mal escrito sale gris, sin romper.
        e.frame.color = FrameColor::Custom { hex: "azul".into() };
        assert_eq!(marco(&e, &p), ([128, 128, 128, 255], None));
    }

    #[test]
    fn una_paleta_que_falta_en_el_pack_sale_gris() {
        let mut p = casa();
        p.def.paletas.remove("gold");
        let t = paleta(&p, Palette::Gold);
        assert_eq!(t.medio, [128, 128, 128, 255]);
    }

    #[test]
    fn la_paleta_de_un_color_aclara_y_oscurece_el_suyo() {
        let m = [200, 40, 40, 255];
        let t = Tonos::de_color(m);
        assert_eq!(t.medio, m);
        assert!(luminancia(t.claro) > luminancia(m));
        assert!(luminancia(t.oscuro) < luminancia(m));
        assert!(luminancia(t.tinte) < luminancia(t.oscuro));
        assert!(luminancia(t.caja) > luminancia(t.claro));
        // Mezclar en los extremos da cada color.
        assert_eq!(mezclar(m, BLANCO, 0.0), m);
        assert_eq!(mezclar(m, BLANCO, 1.0), BLANCO);
        assert!((luminancia(BLANCO) - 1.0).abs() < 1e-4);
        assert_eq!(luminancia(NEGRO), 0.0);
        assert_eq!(hex([255, 16, 0, 9]), "#ff1000");
    }

    #[test]
    fn el_degradado_del_marco_es_vertical_con_un_color_y_horizontal_con_dos() {
        let a = Tonos::de_color([200, 40, 40, 255]);
        let b = Tonos::de_color([40, 40, 200, 255]);
        let uno = defs(&a, None, &a);
        assert!(uno.contains(r#"id="marco" x1="0" y1="0" x2="0" y2="1""#));
        let dos = defs(&a, Some(&b), &Tonos::mezcla(&a, &b));
        assert!(dos.contains(r#"id="marco" x1="0" y1="0" x2="1" y2="0""#));
        assert!(dos.contains(&hex(b.medio)));
        // Las variables de ejemplo traen todo lo que usan las capas.
        let v = vars_de_ejemplo();
        for k in ["defs", "tintaCaja", "yReglas", "altoReglas", "yTipo04"] {
            assert!(v.contains_key(k), "falta {k}");
        }
    }

    // ── Estilo según el layout ──

    fn recursos<'a>(pack: &'a Pack, s: &'a Simbolos) -> Recursos<'a> {
        Recursos {
            pack,
            simbolos: s,
            imagenes: &SinImagenes,
        }
    }

    #[test]
    fn un_estilo_propio_del_pack_y_si_no_esta_su_base() {
        let mut p = casa();
        // «prueba»: el clásico con otro radio, heredando del clásico.
        let mut propio = p.estilo("classic").unwrap().clone();
        propio.radio = 9.0;
        propio.base = Some("classic".into());
        p.def.estilos.insert("prueba".into(), propio.clone());
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Classic,
            Some("prueba"),
            None,
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(e, propio);
        assert!(avisos.is_empty());
        // Otro pack sin ese estilo: el clásico, con aviso.
        let q = casa();
        let e = estilo_para(
            &recursos(&q, &s),
            DesignStyle::Classic,
            Some("prueba"),
            None,
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, q.estilo("classic").unwrap());
        assert_eq!(
            avisos,
            vec![Aviso::EstiloNoEnElPack {
                estilo: "prueba".into()
            }]
        );
    }

    #[test]
    fn el_layout_por_la_carta_y_por_la_variante() {
        let mut p = casa();
        let mut clasico = p.estilo("classic").unwrap().clone();
        // «noBasica»: otra ventana de arte; «sinCaja»: la variante.
        let mut no_basica = super::super::pack::LayoutDef::default();
        let mut arte = clasico.arte.clone();
        arte.y = super::super::pack::Valor::Num(1.0);
        no_basica.arte = Some(arte.clone());
        clasico.layouts.insert("noBasica".into(), no_basica.clone());
        let mut variante = no_basica.clone();
        arte.y = super::super::pack::Valor::Num(2.0);
        variante.arte = Some(arte);
        clasico.layouts.insert("sinCaja".into(), variante.clone());
        clasico.layout_si = vec![super::super::pack::ReglaLayout {
            si: serde_json::from_str(r#"{"tipo": ["land"], "basica": false}"#).unwrap(),
            layout: "noBasica".into(),
        }];
        p.def.estilos.insert("classic".into(), clasico);
        let s = Simbolos::nuevos();
        let r = recursos(&p, &s);
        let con = |h: Hechos| {
            estilo_para(
                &r,
                DesignStyle::Classic,
                None,
                None,
                Some(&h),
                &mut Vec::new(),
            )
            .unwrap()
        };
        let tierra = Hechos {
            tipos: vec!["land".into()],
            ..Hechos::default()
        };
        assert_eq!(Some(&con(tierra.clone()).arte), no_basica.arte.as_ref());
        // La variante elegida manda.
        let elegida = con(Hechos {
            variante: Some("sinCaja".into()),
            ..tierra.clone()
        });
        assert_eq!(Some(&elegida.arte), variante.arte.as_ref());
        // Una básica o una criatura: el normal.
        let normal = p.estilo("classic").unwrap().arte.clone();
        assert_eq!(
            con(Hechos {
                basica: true,
                ..tierra
            })
            .arte,
            normal
        );
        assert_eq!(con(Hechos::default()).arte, normal);
    }

    #[test]
    fn la_etiqueta_de_cada_carta() {
        let carta = |rareza: &str, tipo: &str| CardData {
            rarity: rareza.into(),
            faces: vec![FaceData {
                type_line: super::super::modelo::ByLang {
                    es: String::new(),
                    en: tipo.into(),
                    otros: Default::default(),
                },
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        let rara = carta("rare", "Legendary Creature — Human");
        assert_eq!(valor_de_etiqueta(&rara, 0, None).as_deref(), Some("rare"));
        assert_eq!(
            valor_de_etiqueta(&rara, 0, Some("supertipo")).as_deref(),
            Some("legendary")
        );
        assert_eq!(valor_de_etiqueta(&rara, 0, Some("subtipoTierra")), None);
        let montana = carta("common", "Basic Land — Mountain");
        assert_eq!(
            valor_de_etiqueta(&montana, 0, Some("subtipoTierra")).as_deref(),
            Some("mountain")
        );
        assert_eq!(valor_de_etiqueta(&montana, 0, Some("supertipo")), None);
        assert_eq!(valor_de_etiqueta(&carta("", "Instant"), 0, None), None);
    }

    #[test]
    fn mayusculas_sin_tocar_los_simbolos() {
        let p = texto::con_simbolos("Shivan Reef {T}");
        let m = texto::en_mayusculas(&p);
        assert_eq!(
            m[0],
            texto::Trozo::Texto {
                texto: "SHIVAN REEF ".into(),
                cursiva: false
            }
        );
        assert_eq!(m[1], texto::Trozo::Simbolo("T".into()));
    }

    #[test]
    fn sin_estilo_propio_la_huella_no_cambia() {
        // Los diseños de antes no llevan el campo: su JSON (y su huella) es igual.
        let d = diseno_de_prueba();
        assert!(d.estilo_pack.is_none());
        let json = serde_json::to_string(&d).unwrap();
        assert!(!json.contains("estiloPack"));
        let mut con = d.clone();
        con.estilo_pack = Some("tarot".into());
        assert!(serde_json::to_string(&con)
            .unwrap()
            .contains("\"estiloPack\":\"tarot\""));
    }

    #[test]
    fn sin_layout_es_el_estilo_tal_cual() {
        let p = casa();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Classic,
            None,
            None,
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, p.estilo("classic").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn el_layout_del_estilo_cambia_solo_lo_que_trae() {
        let p = casa();
        let s = Simbolos::nuevos();
        let base = p.estilo("borderless").unwrap();
        let def = &base.layouts["ficha"];
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Borderless,
            None,
            Some("ficha"),
            None,
            &mut avisos,
        )
        .unwrap();
        assert!(avisos.is_empty());
        for (k, v) in &base.huecos {
            let esperado = def.huecos.get(k).unwrap_or(v);
            assert_eq!(&e.huecos[k], esperado, "hueco {k}");
        }
        assert_eq!(&e.arte, def.arte.as_ref().unwrap_or(&base.arte));
        assert_eq!(&e.capas, def.capas.as_ref().unwrap_or(&base.capas));
    }

    #[test]
    fn un_layout_completo_sale_del_pack() {
        let p = casa();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Extended,
            None,
            Some("saga"),
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, p.layout("saga").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn un_pack_de_antes_sin_el_layout_usa_el_de_la_casa() {
        let mut p = casa();
        p.def.layouts.clear();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Borderless,
            None,
            Some("batalla"),
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, casa().layout("batalla").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn un_layout_que_no_hay_en_ningun_sitio_avisa_y_usa_el_estilo() {
        let p = casa();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Borderless,
            None,
            Some("inventado"),
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, p.estilo("borderless").unwrap());
        assert_eq!(
            avisos,
            vec![Aviso::PackSinLayout {
                layout: "inventado".into()
            }]
        );
        // El arte completo no trae «ficha»: su propio estilo, sin aviso.
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::FullArt,
            None,
            Some("ficha"),
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, p.estilo("fullArt").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn la_variante_alta_que_falta_no_avisa() {
        let p = casa();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::FullArt,
            None,
            Some("alto"),
            None,
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, p.estilo("fullArt").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn un_estilo_que_falta_sale_del_pack_de_la_casa() {
        let mut p = casa();
        let clasico = p.estilo("classic").unwrap().clone();
        p.def.estilos.remove("classic");
        p.def.estilos.get_mut("borderless").unwrap().radio = 9.0;
        let s = Simbolos::nuevos();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Classic,
            None,
            None,
            None,
            &mut Vec::new(),
        );
        assert_eq!(e.unwrap(), clasico);
    }

    #[test]
    fn un_pack_con_estilos_propios_pinta_lo_que_le_falta_con_uno_suyo() {
        let mut p = casa();
        let mut propio = p.estilo("borderless").unwrap().clone();
        propio.radio = 7.0;
        propio.base = Some("classic".into());
        p.def.estilos.insert("joker".into(), propio.clone());
        let mut otro = propio.clone();
        otro.radio = 8.0;
        p.def.estilos.insert("espectral".into(), otro.clone());
        p.def.estilos.remove("classic");
        p.def.estilos.remove("extended");
        let s = Simbolos::nuevos();
        let pide = |p: &Pack, style| {
            estilo_para(&recursos(p, &s), style, None, None, None, &mut Vec::new()).unwrap()
        };
        // Sin decir cuál, el primero por id de los basados en el clásico.
        assert_eq!(pide(&p, DesignStyle::Classic), otro);
        p.def.estilo_por_defecto = Some("joker".into());
        assert_eq!(pide(&p, DesignStyle::Classic), propio);
        // Ninguno se basa en el extendido: el de por defecto.
        assert_eq!(pide(&p, DesignStyle::Extended), propio);
        // El que el pack sí trae, el suyo.
        assert_eq!(
            &pide(&p, DesignStyle::Borderless),
            p.estilo("borderless").unwrap()
        );
    }

    #[test]
    fn las_fichas_de_un_pack_sin_marco_de_ficha_salen_con_su_propio_estilo_sin_aviso() {
        let mut p = casa();
        p.def
            .estilos
            .get_mut("borderless")
            .unwrap()
            .layouts
            .remove("ficha");
        // Un estilo del pack distinto del de la casa, para notar el cambio.
        p.def.estilos.get_mut("borderless").unwrap().radio = 9.0;
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Borderless,
            None,
            Some("ficha"),
            None,
            &mut avisos,
        )
        .unwrap();
        // Nada del marco de la casa: el estilo del pack, tal cual.
        assert_eq!(&e, p.estilo("borderless").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn el_grosor_de_la_coleccion_pisa_al_del_pack() {
        let p = casa();
        let s = Simbolos::nuevos();
        let rec = recursos(&p, &s);
        let mut motor = Motor::de_fabrica().unwrap();
        let pinta = |motor: &mut Motor, e: &DisenoEfectivo| {
            render(
                motor,
                e,
                0,
                DesignLang::Es,
                &rec,
                crate::model::CardSize::MTG,
                300,
                Salida::Impresion,
            )
            .unwrap()
            .imagen
        };
        let mut e = carta(&["U"], "Instant");
        let negrita = pinta(&mut motor, &e);
        // Pedir 700 es lo mismo que el pack (que ya pone negrita).
        e.pesos.titulo = Some(700.0);
        assert_eq!(pinta(&mut motor, &e), negrita);
        // Sin negrita, el título cambia.
        e.pesos.titulo = Some(400.0);
        assert_ne!(pinta(&mut motor, &e), negrita);
    }

    // ── Marcos de imagen ──

    #[test]
    fn la_paleta_de_los_marcos_de_imagen() {
        assert_eq!(clave_marco(&carta(&["R"], "Instant"), 0), "R");
        assert_eq!(clave_marco(&carta(&["R", "G"], "Instant"), 0), "gold");
        assert_eq!(clave_marco(&carta(&["W", "U", "B"], "Instant"), 0), "gold");
        assert_eq!(clave_marco(&carta(&[], "Artifact"), 0), "artifact");
        // Color propio: el marco neutro, teñido.
        let mut e = carta(&["R"], "Instant");
        e.frame.color = FrameColor::Custom {
            hex: "#123456".into(),
        };
        assert_eq!(clave_marco(&e, 0), "colorless");
        assert_eq!(extra_marco(&e), ExtraMarco::Tinte([0x12, 0x34, 0x56, 255]));
        // Degradado a mano: la primera paleta y, fundida, la segunda.
        e.frame.color = FrameColor::Gradient {
            a: Palette::R,
            b: Palette::U,
        };
        assert_eq!(clave_marco(&e, 0), "R");
        assert_eq!(extra_marco(&e), ExtraMarco::Degradado("U"));
    }

    #[test]
    fn el_tinte_conserva_luces_y_sombras() {
        let mut img = RgbaImage::from_pixel(3, 1, image::Rgba([0, 0, 0, 200]));
        img.put_pixel(1, 0, image::Rgba([128, 128, 128, 255]));
        img.put_pixel(2, 0, image::Rgba([255, 255, 255, 255]));
        tenir(&mut img, [0, 200, 100, 255]);
        // El negro sigue negro y el blanco, blanco; el gris medio toma el
        // color. La transparencia no cambia.
        assert_eq!(img.get_pixel(0, 0).0, [0, 0, 0, 200]);
        assert_eq!(img.get_pixel(2, 0).0, [255, 255, 255, 255]);
        let medio = img.get_pixel(1, 0).0;
        assert!(medio[1] > medio[0] && medio[1] > medio[2], "{medio:?}");
    }

    #[test]
    fn el_tamano_y_el_interlineado_cambian_el_texto() {
        use crate::diseno::modelo::FineTuning;
        let p = casa();
        let s = Simbolos::nuevos();
        let rec = recursos(&p, &s);
        let mut motor = Motor::de_fabrica().unwrap();
        let mut pinta = |e: &DisenoEfectivo| {
            render(
                &mut motor,
                e,
                0,
                DesignLang::Es,
                &rec,
                crate::model::CardSize::MTG,
                315,
                Salida::Impresion,
            )
            .unwrap()
            .imagen
        };
        let mut e = carta(&["U"], "Instant");
        let base = pinta(&e);
        for f in [
            FineTuning {
                rules_size: Some(0.7),
                ..FineTuning::default()
            },
            FineTuning {
                rules_line_spacing: Some(1.4),
                ..FineTuning::default()
            },
            FineTuning {
                title_size: Some(0.7),
                ..FineTuning::default()
            },
            FineTuning {
                cost_size: Some(0.6),
                ..FineTuning::default()
            },
            FineTuning {
                type_size: Some(1.3),
                ..FineTuning::default()
            },
            FineTuning {
                footer_size: Some(1.3),
                ..FineTuning::default()
            },
            FineTuning {
                footer_line_spacing: Some(1.4),
                ..FineTuning::default()
            },
        ] {
            e.retoques = f.limitado();
            assert_ne!(pinta(&e), base, "{f:?}");
        }
        // Al 100 %, como sin tocar.
        e.retoques = FineTuning {
            title_size: Some(1.0),
            footer_line_spacing: Some(1.0),
            ..FineTuning::default()
        };
        assert_eq!(pinta(&e), base);
    }

    #[test]
    fn el_degradado_funde_de_izquierda_a_derecha() {
        // 63 px de ancho = 1 px por mm: transparente hasta el 35 %, entero
        // desde el 65 %.
        let mut img = RgbaImage::from_pixel(63, 1, image::Rgba([255, 0, 0, 255]));
        fundir_a_la_derecha(&mut img, 0.0, 1.0);
        assert_eq!(img.get_pixel(10, 0).0[3], 0);
        assert_eq!(img.get_pixel(50, 0).0[3], 255);
        let medio = img.get_pixel(31, 0).0[3];
        assert!((100..160).contains(&medio), "{medio}");
    }

    #[test]
    fn una_capa_png_se_pinta_en_su_zona_y_del_color_de_la_carta() {
        let (f, im) = crate::diseno::pack::tests::con_png();
        let p = Pack::cargar_con_imagenes(f, im).unwrap();
        let s = Simbolos::nuevos();
        let rec = recursos(&p, &s);
        let mut motor = Motor::de_fabrica().unwrap();
        let mut e = carta(&["G"], "Creature — Elf");
        e.design.style = DesignStyle::Borderless;
        e.design.card.faces[0].power = Some("2".into());
        e.design.card.faces[0].toughness = Some("2".into());
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            crate::model::CardSize::MTG,
            630,
            Salida::Impresion,
        )
        .unwrap();
        // 10 px por mm. El marco cubre la carta (no se ve el arte)…
        assert_eq!(r.imagen.get_pixel(15, 400).0, [10, 20, 30, 255]);
        // …y la caja de F/R va en su zona (40, 70, 10×5 mm).
        assert_eq!(r.imagen.get_pixel(402, 702).0, [200, 0, 0, 255]);
        assert_eq!(r.imagen.get_pixel(395, 702).0, [10, 20, 30, 255]);
        // Sin fuerza/resistencia no hay caja.
        e.design.card.faces[0].power = None;
        e.design.card.faces[0].toughness = None;
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            crate::model::CardSize::MTG,
            630,
            Salida::Impresion,
        )
        .unwrap();
        assert_eq!(r.imagen.get_pixel(402, 702).0, [10, 20, 30, 255]);
    }

    #[test]
    fn una_capa_png_pinta_solo_su_trozo_y_lo_que_deja_su_mascara() {
        // Un PNG mitad rojo y mitad verde; una máscara opaca a la izquierda.
        let mitades = |a: [u8; 4], b: [u8; 4]| {
            let img = RgbaImage::from_fn(2, 1, |x, _| image::Rgba(if x == 0 { a } else { b }));
            Arc::new(crate::imaging::encode::png(&img).unwrap())
        };
        let (mut f, mut im) = crate::diseno::pack::tests::con_png();
        im.insert(
            "mitades.png".into(),
            mitades([255, 0, 0, 255], [0, 255, 0, 255]),
        );
        im.insert("mascara.png".into(), mitades([0, 0, 0, 255], [0, 0, 0, 0]));
        crate::diseno::pack::tests::con_json(&mut f, |v| {
            v["estilos"]["borderless"]["capas"] = serde_json::json!([
                { "png": "marco{color}.png" },
                // La mitad derecha (verde) en un cuadro de arriba a la izquierda.
                { "png": "mitades.png", "desde": { "x": 31.5, "y": 0, "w": 31.5, "h": 88 },
                  "en": { "x": 0, "y": 0, "w": 10, "h": 10 } },
                // Sin `en`: en su sitio (la mitad derecha de la carta, abajo).
                { "png": "mitades.png", "desde": { "x": 31.5, "y": 60, "w": 31.5, "h": 28 } },
                // Entera, pero solo lo que deja la máscara (la mitad izquierda).
                { "png": "mitades.png", "mascara": "mascara.png",
                  "en": { "x": 0, "y": 30, "w": 63, "h": 20 } }
            ]);
        });
        let p = Pack::cargar_con_imagenes(f, im).unwrap();
        let s = Simbolos::nuevos();
        let rec = recursos(&p, &s);
        let mut motor = Motor::de_fabrica().unwrap();
        let mut e = carta(&["G"], "Instant");
        e.design.style = DesignStyle::Borderless;
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            crate::model::CardSize::MTG,
            630,
            Salida::Impresion,
        )
        .unwrap();
        // 10 px por mm.
        let px = |x, y| r.imagen.get_pixel(x, y).0;
        assert_eq!(px(50, 50), [0, 255, 0, 255]);
        assert_eq!(px(150, 50), [10, 20, 30, 255]);
        assert_eq!(px(500, 700), [0, 255, 0, 255]);
        assert_eq!(px(200, 700), [10, 20, 30, 255]);
        assert_eq!(px(50, 400), [255, 0, 0, 255]);
        assert_eq!(px(580, 400), [10, 20, 30, 255]);
    }

    #[test]
    fn con_zoom_bajo_1_la_imagen_cabe_entera_y_el_resto_es_su_borde() {
        // Una imagen de 40×20 blanca con el centro rojo, en un hueco de 40×20 mm.
        let img = RgbaImage::from_fn(40, 20, |x, y| {
            if (10..30).contains(&x) && (5..15).contains(&y) {
                image::Rgba([220, 20, 20, 255])
            } else {
                image::Rgba([255, 255, 255, 255])
            }
        });
        let mut l = Lienzo::nuevo(40.0, 20.0, 400, 200).unwrap();
        let f = super::super::modelo::ArtFraming {
            zoom: 0.5,
            ..Default::default()
        };
        pintar_arte(&mut l, &img, (0.0, 0.0, 40.0, 20.0), &f, 0.0);
        let px = |x: u32, y: u32| {
            let c = l.px.pixel(x, y).unwrap();
            [c.red(), c.green(), c.blue()]
        };
        // Centrada a la mitad: el rojo en el centro y blanco alrededor (los
        // bordes alargados), también en las esquinas del hueco.
        assert!(px(200, 100)[0] > 200 && px(200, 100)[1] < 60);
        for (x, y) in [(5, 5), (395, 5), (5, 195), (395, 195), (120, 100)] {
            assert!(px(x, y).iter().all(|v| *v > 240), "{x},{y}: {:?}", px(x, y));
        }
        assert_eq!(color_de_borde(&img), [255, 255, 255]);
    }

    // ── Textos ──

    #[test]
    fn el_pie_rellena_todas_las_fichas() {
        let mut e = carta(&["W"], "Instant");
        e.pie.number = "007".into();
        e.pie.rarity = "M".into();
        e.pie.set_code = "RTS".into();
        e.pie.artist = "S.J.".into();
        e.pie.credit = "Pablo".into();
        e.pie.universe = "Tensura".into();
        e.pie.year = "2026".into();
        let todas = "{num}|{rarity}|{original}|{set}|{lang}|{artist}|{credit}|{universe}|{year}|{brush}|{star}";
        e.pie.template.es = todas.into();
        e.pie.template.en = format!("{todas}|{{num}}|{{otra}}");
        assert_eq!(
            pie_texto(&e, DesignLang::Es, "Dusk"),
            "007|M|Dusk|RTS|ES|S.J.|Pablo|Tensura|2026|{PINCEL}|{ESTRELLA}"
        );
        // Cada idioma con su plantilla; una ficha repetida se rellena las dos
        // veces y una que no existe se queda tal cual.
        assert_eq!(
            pie_texto(&e, DesignLang::En, ""),
            "007|M||RTS|EN|S.J.|Pablo|Tensura|2026|{PINCEL}|{ESTRELLA}|007|{otra}"
        );
    }

    #[test]
    fn el_pie_de_fabrica_va_en_dos_lineas() {
        let e = carta(&["W"], "Instant");
        let t = pie_texto(&e, DesignLang::Es, "Dusk");
        assert_eq!(t, "0002\tR\tDusk\nAKH • ES\t{PINCEL} ");
    }

    #[test]
    fn universo_ano_y_creditos_van_a_la_derecha_si_la_plantilla_no_los_pone() {
        let mut e = carta(&["W"], "Instant");
        e.pie.universe = "Monster Hunter".into();
        e.pie.year = "2026".into();
        e.pie.credit = "Capcom".into();
        let t = pie_texto(&e, DesignLang::Es, "");
        assert!(
            t.ends_with("{PINCEL} {>}Monster Hunter · © 2026 Capcom"),
            "{t}"
        );
        // Solo el año.
        e.pie.universe.clear();
        e.pie.credit.clear();
        assert!(pie_texto(&e, DesignLang::Es, "").ends_with("{>}© 2026"));
        // Si la plantilla pone alguno, manda ella.
        e.pie.template.es = "{num}\t{year}".into();
        assert_eq!(pie_texto(&e, DesignLang::Es, ""), "0002\t2026");
    }

    #[test]
    fn la_ultima_columna_del_pie_llega_al_final() {
        // El de fábrica: tabuladores en 8 y 10,6 mm. En la segunda línea
        // («MH2 • ES» y el artista) el artista llega al final del hueco, no
        // al tabulador que viene detrás (que esa línea no usa).
        let casi = |l: Vec<(f32, f32)>, e: &[(f32, f32)]| {
            assert_eq!(l.len(), e.len(), "{l:?}");
            for ((a, b), (c, d)) in l.iter().zip(e) {
                assert!(
                    (a - c).abs() < 1e-4 && (b - d).abs() < 1e-4,
                    "{l:?} != {e:?}"
                );
            }
        };
        casi(
            limites_de_columnas(&[8.0, 10.6], 4.0, 50.0, &[0, 1]),
            &[(4.0, 11.6), (12.0, 54.0)],
        );
        // En la primera, cada una hasta la siguiente.
        casi(
            limites_de_columnas(&[8.0, 10.6], 4.0, 50.0, &[0, 1, 2]),
            &[(4.0, 11.6), (12.0, 14.2), (14.6, 54.0)],
        );
        // Una columna vacía en medio no cuenta.
        casi(
            limites_de_columnas(&[8.0, 10.6], 4.0, 50.0, &[0, 2]),
            &[(4.0, 14.2), (14.6, 54.0)],
        );
    }

    #[test]
    fn el_pie_del_pack_solo_si_la_coleccion_no_pone_uno() {
        let mut e = carta(&["W"], "Instant");
        let mut pack = Pack::de_fabrica().unwrap().def;
        pack.plantilla_pie
            .insert("en".into(), "{set} • {lang} {num}\t{brush} {artist}".into());
        // Sin el del idioma, el inglés.
        assert_eq!(
            pie_del_pack(&e, DesignLang::Es, "Dusk", &pack),
            "AKH • ES 0002\t{PINCEL} "
        );
        // La de la colección manda.
        e.pie.template.es = "{num}".into();
        assert_eq!(pie_del_pack(&e, DesignLang::Es, "Dusk", &pack), "0002");
    }

    #[test]
    fn el_pie_del_pack_en_su_idioma_o_en_ingles() {
        let e = carta(&["W"], "Instant");
        let mut pack = Pack::de_fabrica().unwrap().def;
        pack.plantilla_pie
            .insert("en".into(), "EN {lang} {num}".into());
        pack.plantilla_pie
            .insert("es".into(), "ES {lang} {num}".into());
        pack.plantilla_pie
            .insert("fr".into(), "FR {lang} {num}".into());
        assert_eq!(
            pie_del_pack(&e, DesignLang::Fr, "Dusk", &pack),
            "FR FR 0002"
        );
        assert_eq!(
            pie_del_pack(&e, DesignLang::De, "Dusk", &pack),
            "EN DE 0002"
        );
        assert_eq!(
            pie_del_pack(&e, DesignLang::Ko, "Dusk", &pack),
            "EN KR 0002"
        );
        assert_eq!(
            pie_del_pack(&e, DesignLang::Es, "Dusk", &pack),
            "ES ES 0002"
        );
    }

    fn cara(p: Option<&str>, t: Option<&str>, l: Option<&str>, d: Option<&str>) -> FaceData {
        FaceData {
            power: p.map(Into::into),
            toughness: t.map(Into::into),
            loyalty: l.map(Into::into),
            defense: d.map(Into::into),
            ..FaceData::default()
        }
    }

    #[test]
    fn fuerza_resistencia_lealtad_o_defensa() {
        assert_eq!(
            fr_texto(&cara(Some("2"), Some("3"), None, None)).as_deref(),
            Some("2/3")
        );
        assert_eq!(
            fr_texto(&cara(Some("*"), Some("1+*"), None, None)).as_deref(),
            Some("*/1+*")
        );
        assert_eq!(
            fr_texto(&cara(None, None, Some("4"), None)).as_deref(),
            Some("4")
        );
        assert_eq!(
            fr_texto(&cara(None, None, None, Some("5"))).as_deref(),
            Some("5")
        );
        // La F/R manda sobre la lealtad.
        assert_eq!(
            fr_texto(&cara(Some("1"), Some("1"), Some("3"), None)).as_deref(),
            Some("1/1")
        );
        // Solo la fuerza no vale: no se pinta media caja.
        assert_eq!(fr_texto(&cara(Some("2"), None, None, None)), None);
        assert_eq!(fr_texto(&FaceData::default()), None);
    }

    #[test]
    fn el_caos_va_delante_de_su_habilidad_y_nada_mas() {
        let r = "Cuando caminas a este plano, roba.\nSiempre que se desate el caos, gana 2 vidas.";
        assert_eq!(
            con_caos(r),
            "Cuando caminas a este plano, roba.\n{CHAOS} Siempre que se desate el caos, gana 2 vidas."
        );
        assert_eq!(
            con_caos("Whenever chaos ensues, draw."),
            "{CHAOS} Whenever chaos ensues, draw."
        );
        assert_eq!(
            con_caos("Siempre que lances caos, roba."),
            "{CHAOS} Siempre que lances caos, roba."
        );
        // Ya con su símbolo, o con la frase en medio, no se toca.
        assert_eq!(
            con_caos("{CHAOS} Whenever chaos ensues, draw."),
            "{CHAOS} Whenever chaos ensues, draw."
        );
        assert_eq!(
            con_caos("Draw. Whenever chaos ensues, draw."),
            "Draw. Whenever chaos ensues, draw."
        );
        assert_eq!(con_caos(""), "");
    }

    // ── Píxeles ──

    #[test]
    fn la_mascara_redondeada_deja_las_esquinas_fuera() {
        let m = mascara_redondeada(40, 60, 8.0).unwrap();
        let a = |x: u32, y: u32| m.data()[(y * 40 + x) as usize];
        for (x, y) in [(0, 0), (39, 0), (0, 59), (39, 59)] {
            assert_eq!(a(x, y), 0, "esquina ({x}, {y})");
        }
        assert_eq!(a(20, 30), 255);
        assert_eq!(a(0, 30), 255);
        assert_eq!(a(20, 0), 255);
        assert!(mascara_redondeada(0, 10, 2.0).is_none());
    }

    #[test]
    fn de_imagen_a_pixmap_y_vuelta() {
        let img = RgbaImage::from_fn(5, 4, |x, y| {
            let a = if x == 0 { 0 } else { 255 };
            image::Rgba([(x * 40) as u8, (y * 50) as u8, 7, a])
        });
        let vuelta = de_pixmap(&a_pixmap(&img).unwrap());
        for (p, q) in img.pixels().zip(vuelta.pixels()) {
            if p.0[3] == 255 {
                assert_eq!(p, q);
            } else {
                assert_eq!(q.0[3], 0);
            }
        }
    }

    #[test]
    fn un_lienzo_de_cero_pixeles_tiene_al_menos_uno() {
        let l = Lienzo::nuevo(63.0, 88.0, 0, 0).unwrap();
        assert_eq!((l.px.width(), l.px.height()), (1, 1));
        let l = Lienzo::nuevo(63.0, 88.0, 630, 880).unwrap();
        assert_eq!((l.x(1.0), l.y(1.0)), (10.0, 10.0));
    }
}
