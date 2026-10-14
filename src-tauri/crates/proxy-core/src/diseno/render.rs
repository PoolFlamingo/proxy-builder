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
use super::efectivo::DisenoEfectivo;
use super::letras::Letras;
use super::modelo::{
    Aviso, DesignLang, DesignStyle, FrameColor, OnOffAuto, OriginalName, Palette, RulesBox,
};
use super::pack::{
    rellenar, rgba, Alineacion, CajaPorDefecto, Capa, Condicion, EstiloDef, Hueco, LetraDef, Pack,
    PaletaDef,
};
use super::pintar::{pintar_bloque, pintar_caja, Pincel};
use super::simbolos::{secuencia, Simbolos};
use super::texto::{
    self, ajustar_reglas, linea_ajustada, Alinear, CajaTexto, Estilo, Maquetador, Parrafo,
};
use super::{Color, DisenoError, MM_POR_PT};
use crate::model::CardSize;

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
    m: Maquetador,
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

fn hex(c: [u8; 4]) -> String {
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

struct Lienzo {
    px: Pixmap,
    kx: f32,
    ky: f32,
}

impl Lienzo {
    fn x(&self, mm: f32) -> f32 {
        mm * self.kx
    }
    fn y(&self, mm: f32) -> f32 {
        mm * self.ky
    }

    fn svg(&mut self, svg: &str) -> Result<(), String> {
        let arbol =
            usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;
        let t = arbol.size();
        resvg::render(
            &arbol,
            Transform::from_scale(self.kx * 63.0 / t.width(), self.ky * 88.0 / t.height()),
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

fn a_pixmap(img: &RgbaImage) -> Option<Pixmap> {
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
fn pintar_arte(
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
            let datos = out.take();
            // `out` está premultiplicada; para pegarla da igual: se pega tal cual.
            Pixmap::from_vec(datos, tiny_skia::IntSize::from_wh(tw, th)?).map(|p| {
                let mut img = RgbaImage::new(tw, th);
                for (d, s) in img.pixels_mut().zip(p.pixels()) {
                    let c = s.demultiply();
                    *d = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
                }
                img
            })
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
fn sin_arte(l: &mut Lienzo, hueco: (f32, f32, f32, f32)) {
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 63 88" width="63" height="88"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#3a3f47"/><stop offset="1" stop-color="#15181c"/></linearGradient></defs><rect x="{}" y="{}" width="{}" height="{}" fill="url(#g)"/></svg>"##,
        hueco.0, hueco.1, hueco.2, hueco.3
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

// ── Contexto de una carta ──────────────────────────────────────────────────

struct Carta<'a> {
    l: Lienzo,
    motor: &'a mut Motor,
    rec: &'a Recursos<'a>,
    estilo: EstiloDef,
    vars: BTreeMap<String, String>,
    nums: BTreeMap<&'static str, f32>,
    avisos: Vec<Aviso>,
    faltan: Vec<String>,
    letra_titulo: Option<String>,
    letra_reglas: Option<String>,
}

/// Un hueco ya evaluado (mm).
#[derive(Debug, Clone, Copy)]
struct Caja {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Carta<'_> {
    fn hueco(&self, nombre: &str) -> Option<(Hueco, Caja)> {
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

    /// Una línea centrada en vertical en su hueco, estrechada si no cabe.
    fn linea(&mut self, nombre: &str, p: &Parrafo, ancho_max: Option<f32>, squeeze: f32) {
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

    fn simbolo(&mut self, nombre: &str, codigo: &str) {
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
            let Some(svg) = self.rec.pack.ficheros.get(&capa.svg) else {
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
}

fn nombre_estilo(s: DesignStyle) -> &'static str {
    match s {
        DesignStyle::Borderless => "borderless",
        DesignStyle::Extended => "extended",
        DesignStyle::Classic => "classic",
        DesignStyle::FullArt => "fullArt",
    }
}

/// El estilo con los cambios del layout (ficha…) aplicados.
fn estilo_con_layout(base: &EstiloDef, layout: Option<&str>, avisos: &mut Vec<Aviso>) -> EstiloDef {
    let mut e = base.clone();
    if let Some(l) = layout {
        match base.layouts.get(l) {
            Some(def) => {
                if let Some(c) = &def.capas {
                    e.capas = c.clone();
                }
                for (k, v) in &def.huecos {
                    e.huecos.insert(k.clone(), v.clone());
                }
            }
            None => avisos.push(Aviso::PackSinLayout { layout: l.into() }),
        }
    }
    e
}

/// Rellena las fichas del pie.
fn pie_texto(e: &DisenoEfectivo, lang: DesignLang, original: &str) -> String {
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

/// ¿El último renglón de la caja choca con la caja de fuerza/resistencia?
fn choca(caja: &CajaTexto, ancho_libre: f32) -> bool {
    caja.bloques
        .last()
        .is_some_and(|(b, _)| b.ultima_linea_ancho() > ancho_libre)
}

/// Pinta una cara de una carta diseñada.
#[allow(clippy::too_many_arguments)]
pub fn render(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    cara: usize,
    lang: DesignLang,
    rec: &Recursos<'_>,
    card: CardSize,
    ancho_px: u32,
    salida: Salida,
) -> Result<Render, DisenoError> {
    let d = &e.design;
    let mut avisos = Vec::new();
    let ancho = ancho_px.max(16);
    let alto = (ancho as f64 * card.height_mm / card.width_mm).round() as u32;
    let mut px = Pixmap::new(ancho, alto).ok_or(DisenoError::Lienzo)?;
    px.fill(tiny_skia::Color::BLACK);
    let l = Lienzo {
        px,
        kx: ancho as f32 / 63.0,
        ky: alto as f32 / 88.0,
    };
    let base = rec
        .pack
        .estilo(nombre_estilo(d.style))
        .or_else(|_| rec.pack.estilo("borderless"))
        .map_err(|_| DisenoError::Lienzo)?;
    let layout = d.card.es_ficha().then_some("ficha");
    let estilo = estilo_con_layout(base, layout, &mut avisos);

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
        avisos,
        faltan: Vec::new(),
        letra_titulo: e.font_title.clone(),
        letra_reglas: e.font_rules.clone(),
    };

    // ── Reglas: maquetar primero, porque la caja que crece mueve el resto.
    let reglas_txt = reglas_efectivas(d, cara, lang);
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
    let reglas = texto::reglas(&reglas_txt);
    let sabor = texto::sabor(&sabor_txt);
    let fr = fr_texto(&datos);

    let hueco_reglas = c.estilo.huecos.get("reglas").cloned();
    let caja_din = c.estilo.caja.clone();
    let mut caja_txt: Option<(CajaTexto, Caja)> = None;
    if let Some(h) = hueco_reglas {
        let (est, _) = c.estilo_texto(&h);
        // La letra mínima legible: 6 pt de cuerpo.
        let tam_min = c.l.x(6.0 * MM_POR_PT);
        let paso = c.l.x(0.1 * MM_POR_PT);
        if let Some(din) = &caja_din {
            let alto_max_txt = din.alto_max - din.pad_arriba - din.pad_abajo;
            let ancho = match &h.w {
                super::pack::Valor::Num(n) => *n,
                super::pack::Valor::Expr(_) => 50.0,
            };
            let x_txt = match &h.x {
                super::pack::Valor::Num(n) => *n,
                super::pack::Valor::Expr(_) => 5.95,
            };
            let mut aj = ajustar_reglas(
                &mut c.motor.m,
                &mut c.motor.letras,
                &reglas,
                &sabor,
                &est,
                c.l.x(ancho),
                c.l.y(alto_max_txt),
                tam_min.min(est.tam),
                paso,
            );
            let mut alto_txt = aj.caja.alto / c.l.ky;
            let mut pad_abajo = din.pad_abajo;
            // Si el último renglón llega a la caja de fuerza/resistencia, se
            // deja ese hueco libre por abajo.
            if fr.is_some() {
                if let Some(frh) = c.estilo.huecos.get("fr") {
                    let fr_x = frh.x.evaluar(&BTreeMap::new()).unwrap_or(48.0);
                    let fr_y = frh.y.evaluar(&BTreeMap::new()).unwrap_or(80.0);
                    let libre = c.l.x(fr_x - 0.6 - x_txt);
                    let invade = din.fondo - fr_y + 0.4;
                    if choca(&aj.caja, libre) && pad_abajo < invade {
                        pad_abajo = invade;
                        aj = ajustar_reglas(
                            &mut c.motor.m,
                            &mut c.motor.letras,
                            &reglas,
                            &sabor,
                            &est,
                            c.l.x(ancho),
                            c.l.y(din.alto_max - din.pad_arriba - pad_abajo),
                            tam_min.min(est.tam),
                            paso,
                        );
                        alto_txt = aj.caja.alto / c.l.ky;
                    }
                }
            }
            if aj.sobra > 0.0 {
                c.avisos.push(Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(aj.sobra / c.l.ky),
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
            caja_txt = Some((
                aj.caja,
                Caja {
                    x: x_txt,
                    y: y_reglas + din.pad_arriba,
                    w: ancho,
                    h: alto_caja - din.pad_arriba - pad_abajo,
                },
            ));
        } else if let Some((h, caja)) = c.hueco("reglas") {
            let (est, _) = c.estilo_texto(&h);
            let aj = ajustar_reglas(
                &mut c.motor.m,
                &mut c.motor.letras,
                &reglas,
                &sabor,
                &est,
                c.l.x(caja.w),
                c.l.y(caja.h),
                tam_min.min(est.tam),
                paso,
            );
            if aj.sobra > 0.0 {
                c.avisos.push(Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(aj.sobra / c.l.ky),
                });
            }
            caja_txt = Some((aj.caja, caja));
        }
    }

    // ── Ilustración.
    if let Some((_, arte)) = {
        let h = c.estilo.arte.clone();
        let ev = |v: &super::pack::Valor| v.evaluar(&c.nums).unwrap_or(0.0);
        Some((
            h.clone(),
            Caja {
                x: ev(&h.x),
                y: ev(&h.y),
                w: ev(&h.w),
                h: ev(&h.h),
            },
        ))
    } {
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
    if d.style == DesignStyle::FullArt {
        let s = d
            .card
            .color_identity
            .first()
            .cloned()
            .unwrap_or_else(|| "C".into());
        c.simbolo("simbolo", &s);
    }
    if let Some((caja, sitio)) = &caja_txt {
        // En las cajas fijas, el texto va centrado en vertical.
        let libre = if caja_din.is_some() {
            0.0
        } else {
            (sitio.h - caja.alto / c.l.ky).max(0.0) / 2.0
        };
        let tinta = c
            .vars
            .get("tintaCaja")
            .and_then(|t| rgba(t))
            .unwrap_or(BLANCO);
        let raya = [tinta[0], tinta[1], tinta[2], 110];
        let (x, y, w) = (c.l.x(sitio.x), c.l.y(sitio.y + libre), c.l.x(sitio.w));
        c.faltan.extend(pintar_caja(
            &mut c.l.px.as_mut(),
            caja,
            x,
            y,
            w,
            raya,
            rec.simbolos,
        ));
    }
    if let Some(fr) = &fr {
        c.linea("fr", &texto::llano(fr), None, squeeze);
    }
    let original_pie = if en_cinta || e.original_name == OriginalName::Hidden {
        String::new()
    } else {
        original
    };
    let pie = pie_texto(e, lang, &original_pie);
    c.pie(&pie);

    // ── Avisos.
    if lang == DesignLang::Es && d.card.sin_texto_es {
        c.avisos.push(Aviso::SinTextoOficial);
    }
    c.faltan
        .retain(|s| !matches!(s.as_str(), "PINCEL" | "ESTRELLA"));
    c.faltan.sort();
    c.faltan.dedup();
    if !c.faltan.is_empty() {
        c.avisos.push(Aviso::FaltanSimbolos {
            simbolos: c.faltan.clone(),
        });
    }

    // ── Salida.
    let mut px = c.l.px;
    if salida == Salida::Pantalla {
        let r = c.estilo.radio * (ancho as f32 / 63.0);
        if let Some(m) = mascara_redondeada(ancho, alto, r) {
            px.apply_mask(&m);
        }
    }
    let mut img = RgbaImage::new(ancho, alto);
    for (d, s) in img.pixels_mut().zip(px.pixels()) {
        let c = s.demultiply();
        *d = image::Rgba([c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Ok(Render {
        imagen: img,
        avisos: c.avisos,
    })
}

fn mascara_redondeada(w: u32, h: u32, r: f32) -> Option<Mask> {
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
