//! Lo que comparten los tests de dorados: arte, emblema y símbolos de prueba
//! (hechos aquí, nada con derechos) y la comparación de imágenes.

#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;

use image::{Rgba, RgbaImage};
use proxy_core::diseno::render::Imagenes;
use proxy_core::diseno::simbolos::Simbolos;
use proxy_core::model::{FaceImage, ImageOrigin};

pub const ANCHO: u32 = 420;

pub struct Artes(pub HashMap<String, Arc<RgbaImage>>);

impl Imagenes for Artes {
    fn imagen(&self, blob: &str) -> Option<Arc<RgbaImage>> {
        self.0.get(blob).cloned()
    }
}

/// Un «arte» de prueba: degradado con círculos, siempre igual.
pub fn arte(semilla: u32) -> RgbaImage {
    RgbaImage::from_fn(600, 840, |x, y| {
        let (fx, fy) = (x as f32 / 600.0, y as f32 / 840.0);
        let s = semilla as f32;
        let cx = 300.0 + 120.0 * (s * 1.7).sin();
        let cy = 380.0 + 90.0 * (s * 2.3).cos();
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
        let anillo = if ((d / 40.0) as u32).is_multiple_of(2) {
            30.0
        } else {
            0.0
        };
        Rgba([
            (60.0 + 150.0 * fx + anillo + s * 10.0) as u8,
            (40.0 + 120.0 * fy + anillo) as u8,
            (120.0 + 90.0 * (1.0 - fx) * fy) as u8,
            255,
        ])
    })
}

/// Un emblema de prueba: un óvalo azul con brillo.
pub fn emblema() -> RgbaImage {
    RgbaImage::from_fn(90, 60, |x, y| {
        let (dx, dy) = ((x as f32 - 45.0) / 44.0, (y as f32 - 30.0) / 28.0);
        if dx * dx + dy * dy <= 1.0 {
            Rgba([90, 160, 240, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
}

/// Símbolos de prueba dibujados aquí: un círculo de color y una marca
/// distinta por símbolo (sin texto, que usvg no pinta sin letras).
pub fn simbolos() -> Simbolos {
    let mut s = Simbolos::nuevos();
    let colores = [
        (
            "W",
            "#f8f6d8",
            "M50 20 L58 42 L80 50 L58 58 L50 80 L42 58 L20 50 L42 42 Z",
        ),
        (
            "U",
            "#aae0fa",
            "M50 18 C66 44 72 58 50 82 C28 58 34 44 50 18 Z",
        ),
        ("B", "#cbc2bf", "M30 35 H70 V65 H30 Z"),
        ("R", "#f9aa8f", "M50 20 L80 75 H20 Z"),
        (
            "G",
            "#9bd3ae",
            "M50 18 L74 50 L60 50 L74 80 H26 L40 50 L26 50 Z",
        ),
        ("C", "#cccccc", "M50 25 L75 50 L50 75 L25 50 Z"),
        (
            "T",
            "#cac5c0",
            "M30 30 C55 20 75 40 70 60 L80 60 L65 78 L52 60 L60 60 C62 45 50 35 30 40 Z",
        ),
        ("1", "#cac5c0", "M46 25 H56 V75 H46 Z"),
        (
            "2",
            "#cac5c0",
            "M35 30 H65 V50 H45 V65 H65 V75 H35 V45 H55 V40 H35 Z",
        ),
        (
            "3",
            "#cac5c0",
            "M35 25 H65 V75 H35 V65 H55 V55 H40 V45 H55 V35 H35 Z",
        ),
    ];
    for (c, fondo, marca) in colores {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="48" fill="{fondo}"/><path d="{marca}" fill="#111111"/></svg>"##
        );
        s.cargar(c, svg.as_bytes()).unwrap();
    }
    s
}

pub fn img(blob: &str) -> FaceImage {
    FaceImage {
        blob: blob.into(),
        width: 1,
        height: 1,
        origin: ImageOrigin::Upload,
    }
}

/// Porcentaje de píxeles con algún canal más de 8/255 distinto.
pub fn diferencia(a: &RgbaImage, b: &RgbaImage) -> f64 {
    if a.dimensions() != b.dimensions() {
        return 100.0;
    }
    let distintos = a
        .pixels()
        .zip(b.pixels())
        .filter(|(p, q)| p.0.iter().zip(q.0.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    distintos as f64 * 100.0 / (a.width() * a.height()) as f64
}
