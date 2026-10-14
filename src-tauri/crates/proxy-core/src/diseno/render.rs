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
    Aviso, DesignLang, DesignStyle, FrameColor, OnOffAuto, OriginalName, Palette, RulesBox,
};
use super::pack::{
    rellenar, rgba, Alineacion, CajaPorDefecto, Capa, Condicion, EstiloDef, Hueco, LetraDef, Pack,
    PaletaDef,
};
use super::pintar::{pintar_bloque, Pincel};
use super::simbolos::{secuencia, Simbolos};
use super::texto::{self, linea_ajustada, Alinear, Estilo, Maquetador, Parrafo};
use super::{Color, DisenoError, MM_POR_PT};

pub use super::componer::render;

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
    /// Con las letras de la casa (Spectral y Montserrat).
    pub fn de_fabrica() -> Result<Motor, DisenoError> {
        let mut letras = Letras::nuevas();
        for f in LETRAS_DE_FABRICA {
            letras.cargar(f.to_vec())?;
        }
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

fn luminancia(c: [u8; 4]) -> f32 {
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

/// El color del marco: con `Auto`, un color el suyo, dos un degradado y tres
/// o más oro; sin color, artefacto, tierra o incoloro (§3.7 del plan).
fn colores_marco(e: &DisenoEfectivo, cara: usize, pack: &Pack) -> (Tonos, Option<Tonos>) {
    match &e.frame.color {
        FrameColor::Palette { palette } => (paleta(pack, *palette), None),
        FrameColor::Gradient { a, b } => (paleta(pack, *a), Some(paleta(pack, *b))),
        FrameColor::Custom { hex } => (
            Tonos::de_color(rgba(hex).unwrap_or([128, 128, 128, 255])),
            None,
        ),
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
                    return (paleta(pack, Palette::Land), None);
                }
            }
            match colores.as_slice() {
                [] if tipo.contains("Artifact") => (paleta(pack, Palette::Artifact), None),
                [] => (paleta(pack, Palette::Colorless), None),
                [c] => (paleta(pack, *c), None),
                [a, b] => (paleta(pack, *a), Some(paleta(pack, *b))),
                _ => (paleta(pack, Palette::Gold), None),
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

/// La ilustración cubriendo su hueco, con zoom, desplazamiento y giro.
pub(super) fn pintar_arte(
    l: &mut Lienzo,
    img: &RgbaImage,
    hueco: (f32, f32, f32, f32),
    f: &super::modelo::ArtFraming,
) {
    let (x, y, w, h) = (l.x(hueco.0), l.y(hueco.1), l.x(hueco.2), l.y(hueco.3));
    let (tw, th) = (w.round().max(1.0) as u32, h.round().max(1.0) as u32);
    let (iw, ih) = (img.width() as f64, img.height() as f64);
    let zoom = f.zoom.max(1.0);
    let girada = f.rotation.rem_euclid(360.0).abs() > 0.01;
    let pieza = if !girada {
        // Sin giro: recorte exacto + Lanczos en un paso.
        let cubre = (tw as f64 / iw).max(th as f64 / ih) * zoom;
        let (cw, ch) = (tw as f64 / cubre, th as f64 / cubre);
        let (hx, hy) = ((iw - cw) / 2.0, (ih - ch) / 2.0);
        let l0 = hx + f.offset_x.clamp(-1.0, 1.0) * hx;
        let t0 = hy + f.offset_y.clamp(-1.0, 1.0) * hy;
        reescalar(img, tw, th, Some((l0, t0, cw, ch)))
    } else {
        let a = f.rotation.to_radians();
        let (c, s) = (a.cos().abs(), a.sin().abs());
        let (wf, hf) = (tw as f64, th as f64);
        let cubre = ((wf * c + hf * s) / iw).max((wf * s + hf * c) / ih) * zoom;
        let (rw, rh) = ((iw * cubre).round() as u32, (ih * cubre).round() as u32);
        let grande = reescalar(img, rw, rh, None);
        grande.and_then(|g| {
            let pm = a_pixmap(&g)?;
            let mut out = Pixmap::new(tw, th)?;
            let (bw, bh) = (rw as f64 * c + rh as f64 * s, rw as f64 * s + rh as f64 * c);
            let dx = f.offset_x.clamp(-1.0, 1.0) * ((bw - wf) / 2.0).max(0.0);
            let dy = f.offset_y.clamp(-1.0, 1.0) * ((bh - hf) / 2.0).max(0.0);
            let t = Transform::from_translate((wf / 2.0 - dx) as f32, (hf / 2.0 - dy) as f32)
                .pre_rotate(f.rotation as f32)
                .pre_translate(-(rw as f32) / 2.0, -(rh as f32) / 2.0);
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
        })
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
        let c = Caja {
            x: ev(&h.x),
            y: ev(&h.y),
            w: ev(&h.w),
            h: ev(&h.h),
        };
        (c.w > 0.0 && c.h > 0.0).then_some((h, c))
    }

    fn color(&self, h: &Hueco) -> Color {
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

    fn estilo_texto(&mut self, h: &Hueco) -> (Estilo, f32) {
        let rol = h.rol.as_deref().unwrap_or("reglas");
        let letra = self.letra(rol);
        let tam = self.l.x(h.mayuscula.unwrap_or(1.78)) / letra.mayuscula.max(0.3);
        (
            Estilo {
                familia: letra.familia,
                peso: letra.peso,
                tam,
                interlinea: 1.06,
                color: self.color(h),
                alinear: match h.alinear {
                    Alineacion::Inicio => Alinear::Inicio,
                    Alineacion::Centro => Alinear::Centro,
                    Alineacion::Fin => Alinear::Fin,
                },
                versalitas: h.versalitas,
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
            tam: self.l.x(cap_mm) / letra.mayuscula.max(0.3),
            interlinea: 1.06,
            color,
            alinear: Alinear::Inicio,
            versalitas: false,
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
        let (e, cap) = self.estilo_texto(&h);
        let ancho = ancho_max.unwrap_or(c.w).min(c.w);
        let (b, f, reducido) = linea_ajustada(
            &mut self.motor.m,
            &mut self.motor.letras,
            p,
            &e,
            self.l.x(ancho),
            squeeze,
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
        let (_, c) = self.hueco("coste")?;
        let simbolos = secuencia(coste);
        if simbolos.is_empty() {
            return None;
        }
        let lado = c.h;
        let hueco = lado * 0.05;
        let n = simbolos.len() as f32;
        let total = n * lado + (n - 1.0) * hueco;
        let mut x = c.x + c.w - total;
        let x0 = x;
        let cy = c.y + c.h / 2.0;
        for s in simbolos {
            let (px, py, r) = (
                self.l.x(x + lado / 2.0 - lado * 0.05),
                self.l.y(cy + lado * 0.08),
                self.l.x(lado / 2.0),
            );
            super::simbolos::circulo(&mut self.l.px.as_mut(), px, py, r, [0, 0, 0, 200]);
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

    fn capas(&mut self, capas: &[Capa], cond: &Condicion) {
        for capa in capas {
            let si = &capa.si;
            let pasa = [
                (si.corona, cond.corona),
                (si.fr, cond.fr),
                (si.cinta, cond.cinta),
                (si.emblema, cond.emblema),
            ]
            .iter()
            .all(|(pide, hay)| pide.is_none_or(|p| Some(p) == *hay));
            if !pasa {
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

    /// El pie: líneas separadas por `\n`, columnas por `\t` (tabuladores del
    /// hueco). `★` y `✎` van como iconos.
    fn pie(&mut self, texto: &str) {
        let Some((h, c)) = self.hueco("pie") else {
            return;
        };
        let (e, cap) = self.estilo_texto(&h);
        let paso = h.interlinea.unwrap_or(1.6);
        for (i, linea) in texto.split('\n').enumerate() {
            let cy = c.y + (i as f32 + 0.5) * paso;
            for (j, col) in linea.split('\t').enumerate() {
                let col = col.trim();
                if col.is_empty() {
                    continue;
                }
                let x = c.x
                    + if j == 0 {
                        0.0
                    } else {
                        h.tabs.get(j - 1).copied().unwrap_or(0.0)
                    };
                let p =
                    texto::con_simbolos(&col.replace('★', "{ESTRELLA}").replace('✎', "{PINCEL}"));
                let b = self.motor.m.maquetar(&mut self.motor.letras, &p, &e, None);
                let y0 = self.l.y(cy) - b.base + e.tam * cap / 2.0;
                let x0 = self.l.x(x);
                self.faltan.extend(pintar_bloque(
                    &mut self.l.px.as_mut(),
                    &b,
                    x0,
                    y0,
                    Pincel::default(),
                    self.rec.simbolos,
                ));
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
    layout: Option<&str>,
    avisos: &mut Vec<Aviso>,
) -> Result<EstiloDef, DisenoError> {
    let base = rec
        .pack
        .estilo(nombre_estilo(style))
        .or_else(|_| rec.pack.estilo("borderless"))
        .map_err(|_| DisenoError::Lienzo)?;
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

/// Rellena las fichas del pie.
pub(super) fn pie_texto(e: &DisenoEfectivo, lang: DesignLang, original: &str) -> String {
    let p = &e.pie;
    let idioma = match lang {
        DesignLang::Es => "ES",
        DesignLang::En => "EN",
    };
    let mut t = p.template.get(lang).clone();
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
        }
    }
}

/// Una cara pintada.
pub(super) struct Pieza {
    pub(super) px: Pixmap,
    pub(super) avisos: Vec<Aviso>,
    pub(super) faltan: Vec<String>,
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
        RulesBox::Tint { hex } => (rgba(hex).unwrap_or(mezcla.tinte), 0.80),
    };
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

    let mut c = Carta {
        l,
        motor,
        rec,
        estilo,
        vars,
        nums: BTreeMap::new(),
        avisos: Vec::new(),
        faltan: Vec::new(),
        letra_titulo: e.font_title.clone(),
        letra_reglas: e.font_rules.clone(),
    };
    c.avisos.append(&mut avisos);

    // ── Reglas: maquetar primero, porque la caja que crece mueve el resto.
    if d.faces
        .get(cara)
        .is_some_and(|f| f.texts.get(lang).rules_override.is_some())
    {
        c.avisos.push(Aviso::ReglasEditadas {
            cara: cara as u32,
            lang,
        });
    }
    let sabor_txt = d
        .faces
        .get(cara)
        .map(|f| f.texts.get(lang).flavor.clone())
        .filter(|s| d.options.show_flavor && !s.trim().is_empty())
        .unwrap_or_default();
    let sabor = texto::sabor(&sabor_txt);
    let salas = if o.cuerpo == Cuerpo::Mazmorra {
        estructura::mazmorra(&reglas_efectivas(d, cara, lang))
    } else {
        Vec::new()
    };
    let contenido = if salas.is_empty() {
        contenido_de_la_cara(d, o, lang, sabor)
    } else {
        Contenido::Texto {
            reglas: Vec::new(),
            sabor: Vec::new(),
        }
    };
    let fr = if o.cuerpo == Cuerpo::Leveler || o.cuerpo == Cuerpo::Estacion {
        // La F/R de cada tramo va en su franja; la de abajo, la de base.
        fr_texto(&datos)
    } else {
        fr_texto(&datos)
    };
    // MDFC: la franja de la otra cara se lleva un poco de la caja.
    let franja = if o.otra_cara.is_some() { 2.6 } else { 0.0 };

    let hueco_reglas = c.estilo.huecos.get("reglas").cloned();
    let caja_din = c.estilo.caja.clone();
    let mut caja_txt: Option<(Hecho, Caja)> = None;
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
            let mut pad_abajo = din.pad_abajo + franja;
            let mut aj = cuerpos::ajustar(
                &contenido,
                &mut c.motor.m,
                &mut c.motor.letras,
                &est,
                c.l.x(ancho),
                c.l.y(din.alto_max - din.pad_arriba - pad_abajo),
                tam_min.min(est.tam),
                paso,
                k,
            );
            let mut alto_txt = aj.0.alto() / c.l.ky;
            // Si el último renglón llega a la caja de fuerza/resistencia, se
            // deja ese hueco libre por abajo.
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
                            c.l.y(din.alto_max - din.pad_arriba - pad_abajo),
                            tam_min.min(est.tam),
                            paso,
                            k,
                        );
                        alto_txt = aj.0.alto() / c.l.ky;
                    }
                }
            }
            if aj.1 > 0.0 {
                c.avisos.push(Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(aj.1 / c.l.ky),
                });
            }
            let alto_caja =
                (alto_txt + din.pad_arriba + pad_abajo).clamp(din.alto_min, din.alto_max);
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
            let caja = Caja {
                h: caja.h - franja,
                ..caja
            };
            let aj = cuerpos::ajustar(
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

    // ── Ilustración.
    if !o.sin_arte {
        let h = c.estilo.arte.clone();
        let ev = |v: &super::pack::Valor| v.evaluar(&c.nums).unwrap_or(0.0);
        let arte = Caja {
            x: ev(&h.x),
            y: ev(&h.y),
            w: ev(&h.w),
            h: ev(&h.h),
        };
        if arte.w > 0.0 && arte.h > 0.0 {
            let r = (arte.x, arte.y, arte.w, arte.h);
            let img = d
                .faces
                .get(cara)
                .and_then(|f| f.art.as_ref())
                .and_then(|a| rec.imagenes.imagen(&a.image.blob).map(|i| (i, a.framing)));
            match img {
                Some((i, f)) => pintar_arte(&mut c.l, &i, r, &f),
                None => sin_arte(&mut c.l, r),
            }
        }
    }

    // ── Capas.
    let corona = match e.frame.crown {
        OnOffAuto::On => true,
        OnOffAuto::Off => false,
        OnOffAuto::Auto => d.style == DesignStyle::Borderless && d.card.es_legendaria(),
    };
    let original = nombre_original(&d.card);
    let en_cinta = e.original_name == OriginalName::Ribbon && c.estilo.huecos.contains_key("cinta");
    let cond = Condicion {
        corona: Some(corona),
        fr: Some(fr.is_some()),
        cinta: Some(en_cinta),
        emblema: Some(e.emblem.is_some()),
    };
    let capas = c.estilo.capas.clone();
    c.capas(&capas, &cond);

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
        .is_some_and(|f| f.texts.get(lang).name.trim().is_empty())
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
    let ancho_titulo = c
        .hueco("titulo")
        .map(|(_, t)| fin_coste.map_or(t.w, |x0| x0 - 0.6 - t.x));
    c.linea("titulo", &texto::llano(&nombre), ancho_titulo, squeeze);
    if en_cinta {
        let p = vec![texto::Trozo::Texto {
            texto: original.clone(),
            cursiva: true,
        }];
        c.linea("cinta", &p, None, squeeze);
    }
    let tipo = datos.type_line.get(lang).clone();
    c.linea("tipo", &texto::con_simbolos(&tipo), None, squeeze);
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
        // En las cajas fijas, el texto normal va centrado en vertical; las
        // secciones que reparten llenan la caja.
        let reparte = matches!(contenido, Contenido::Secciones { repartir: true, .. });
        let libre = if caja_din.is_some() || reparte {
            0.0
        } else {
            (sitio.h - hecho.alto() / c.l.ky).max(0.0) / 2.0
        };
        c.pintar_contenido(
            &contenido,
            hecho,
            sitio.x,
            sitio.y + libre,
            sitio.w,
            sitio.h - libre,
            tinta,
        );
    }
    if !salas.is_empty() {
        if let Some((h, caja)) = c.hueco("reglas") {
            let (est, _) = c.estilo_texto(&h);
            c.pintar_mazmorra(&salas, caja, &est, tinta);
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
        let pie = pie_texto(e, lang, &original_pie);
        c.pie(&pie);
    }

    // ── Avisos.
    if lang == DesignLang::Es && d.card.sin_texto_es && !c.avisos.contains(&Aviso::SinTextoOficial)
    {
        c.avisos.push(Aviso::SinTextoOficial);
    }
    Ok(Pieza {
        px: c.l.px,
        avisos: c.avisos,
        faltan: c.faltan,
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
            tam: self.l.x(cap_mm) / letra.mayuscula.max(0.3),
            interlinea: 1.06,
            color,
            alinear: Alinear::Inicio,
            versalitas: false,
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
    fn sin_layout_es_el_estilo_tal_cual() {
        let p = casa();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(&recursos(&p, &s), DesignStyle::Classic, None, &mut avisos).unwrap();
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
            Some("ficha"),
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
            Some("saga"),
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
            Some("batalla"),
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
            Some("inventado"),
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
        // El arte completo no trae «ficha»: aviso.
        let mut avisos = Vec::new();
        estilo_para(
            &recursos(&p, &s),
            DesignStyle::FullArt,
            Some("ficha"),
            &mut avisos,
        )
        .unwrap();
        assert_eq!(
            avisos,
            vec![Aviso::PackSinLayout {
                layout: "ficha".into()
            }]
        );
    }

    #[test]
    fn la_variante_alta_que_falta_no_avisa() {
        let p = casa();
        let s = Simbolos::nuevos();
        let mut avisos = Vec::new();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::FullArt,
            Some("alto"),
            &mut avisos,
        )
        .unwrap();
        assert_eq!(&e, p.estilo("fullArt").unwrap());
        assert!(avisos.is_empty());
    }

    #[test]
    fn un_estilo_que_falta_cae_al_despejado_y_sin_ninguno_falla() {
        let mut p = casa();
        p.def.estilos.remove("classic");
        let s = Simbolos::nuevos();
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Classic,
            None,
            &mut Vec::new(),
        );
        assert_eq!(&e.unwrap(), p.estilo("borderless").unwrap());
        p.def.estilos.remove("borderless");
        let e = estilo_para(
            &recursos(&p, &s),
            DesignStyle::Classic,
            None,
            &mut Vec::new(),
        );
        assert_eq!(e.unwrap_err(), DisenoError::Lienzo);
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
