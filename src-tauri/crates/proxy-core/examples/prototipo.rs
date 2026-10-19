//! **Prototipo de la Fase 0** (Paso 0.3 del plan de la 2.0): compone tres
//! cartas (despejado, extendido y paisaje) con marcos SVG provisionales y
//! varias combinaciones de letras, y mide tiempos de render.
//!
//! No es código de la app: es la puerta visual antes de construir el motor
//! de verdad (Fase 2), que tomará de aquí lo aprobado.
//!
//! `cargo run --release -p proxy-core --example prototipo -- <letras> <simbolos> <artes> <salida>`

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_arguments)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use proxy_core::diseno::letras::Letras;
use proxy_core::diseno::pintar::{pintar_bloque, pintar_caja, Pincel};
use proxy_core::diseno::simbolos::Simbolos;
use proxy_core::diseno::texto::{
    self, ajustar_reglas, linea_ajustada, Alinear, Estilo, Maquetador, Parrafo,
};
use proxy_core::diseno::{Color, MM_POR_PT};
use resvg::usvg;
use tiny_skia::{IntSize, Pixmap, PixmapPaint, Transform};

// ── Combinaciones de letras ────────────────────────────────────────────────

struct Combo {
    id: &'static str,
    nombre: &'static str,
    titulo: (&'static str, f32),
    reglas: &'static str,
    /// Altura de mayúscula / cuerpo de cada letra (para igualar tamaños).
    cap_titulo: f32,
    cap_reglas: f32,
    ficheros: &'static [&'static str],
}

const COMBOS: &[Combo] = &[
    Combo {
        id: "a",
        nombre: "A · Vollkorn + Crimson Pro",
        titulo: ("Vollkorn", 700.0),
        reglas: "Crimson Pro",
        cap_titulo: 0.676,
        cap_reglas: 0.573,
        ficheros: &[
            "Vollkorn[wght].ttf",
            "CrimsonPro[wght].ttf",
            "CrimsonPro-Italic[wght].ttf",
        ],
    },
    Combo {
        id: "b",
        nombre: "B · Spectral + Spectral",
        titulo: ("Spectral", 700.0),
        reglas: "Spectral",
        cap_titulo: 0.66,
        cap_reglas: 0.66,
        ficheros: &[
            "Spectral-Bold.ttf",
            "Spectral-Regular.ttf",
            "Spectral-Italic.ttf",
        ],
    },
    Combo {
        id: "c",
        nombre: "C · Libre Baskerville + Source Serif",
        titulo: ("Libre Baskerville", 700.0),
        reglas: "Source Serif 4",
        cap_titulo: 0.77,
        cap_reglas: 0.67,
        ficheros: &[
            "LibreBaskerville[wght].ttf",
            "SourceSerif4[opsz,wght].ttf",
            "SourceSerif4-Italic[opsz,wght].ttf",
        ],
    },
];

const PIE: &str = "Montserrat";

// ── Paletas de marco ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Paleta {
    claro: &'static str,
    medio: &'static str,
    oscuro: &'static str,
    /// Tinte de la caja de reglas (RGB).
    tinte: (u8, u8, u8),
}

const ORO: Paleta = Paleta {
    claro: "#f6dc8a",
    medio: "#d9ab3c",
    oscuro: "#8a6417",
    tinte: (46, 36, 18),
};
const AZUL: Paleta = Paleta {
    claro: "#6fb2ec",
    medio: "#1f6fb5",
    oscuro: "#0b3e72",
    tinte: (8, 28, 52),
};
const BLANCO: Paleta = Paleta {
    claro: "#ffffff",
    medio: "#d9d6cf",
    oscuro: "#8d8a82",
    tinte: (40, 40, 40),
};

// ── Lienzo ─────────────────────────────────────────────────────────────────

struct Lienzo {
    px: Pixmap,
    /// Píxeles por milímetro.
    k: f32,
}

impl Lienzo {
    fn nuevo(ancho_px: u32) -> Lienzo {
        let k = ancho_px as f32 / 63.0;
        let alto = (88.0 * k).round() as u32;
        let mut px = Pixmap::new(ancho_px, alto).unwrap();
        px.fill(tiny_skia::Color::BLACK);
        Lienzo { px, k }
    }
    fn mm(&self, v: f32) -> f32 {
        v * self.k
    }
    fn pt(&self, pt: f32) -> f32 {
        pt * MM_POR_PT * self.k
    }
    /// Cuerpo en px para que la mayúscula mida `cap_mm`.
    fn cap(&self, cap_mm: f32, proporcion: f32) -> f32 {
        cap_mm / proporcion * self.k
    }

    /// Pinta un SVG con unidades en mm (viewBox 0 0 63 88).
    fn svg(&mut self, svg: &str) {
        let arbol = usvg::Tree::from_str(svg, &usvg::Options::default()).unwrap();
        let s = self.k * 63.0 / arbol.size().width();
        resvg::render(&arbol, Transform::from_scale(s, s), &mut self.px.as_mut());
    }

    /// La ilustración cubriendo `(x, y, w, h)` en mm.
    fn arte(&mut self, ruta: &Path, x: f32, y: f32, w: f32, h: f32) {
        let img = image::open(ruta).unwrap().to_rgba8();
        let (tw, th) = (self.mm(w).round() as u32, self.mm(h).round() as u32);
        let esc = (tw as f32 / img.width() as f32).max(th as f32 / img.height() as f32);
        let (rw, rh) = (
            (img.width() as f32 * esc).round() as u32,
            (img.height() as f32 * esc).round() as u32,
        );
        let r = image::imageops::resize(&img, rw, rh, image::imageops::FilterType::Lanczos3);
        let cx = (rw - tw.min(rw)) / 2;
        let rec = image::imageops::crop_imm(&r, cx, 0, tw.min(rw), th.min(rh)).to_image();
        let pm = Pixmap::from_vec(
            premultiplicar(rec.as_raw()),
            IntSize::from_wh(rec.width(), rec.height()).unwrap(),
        )
        .unwrap();
        self.px.draw_pixmap(
            self.mm(x).round() as i32,
            self.mm(y).round() as i32,
            pm.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }

    fn imagen(&mut self, ruta: &Path, x: f32, y: f32, w: f32, h: f32) {
        let img = image::open(ruta).unwrap().to_rgba8();
        let esc = (self.mm(w) / img.width() as f32).min(self.mm(h) / img.height() as f32);
        let (rw, rh) = (
            (img.width() as f32 * esc).round() as u32,
            (img.height() as f32 * esc).round() as u32,
        );
        let r = image::imageops::resize(&img, rw, rh, image::imageops::FilterType::Lanczos3);
        let pm = Pixmap::from_vec(
            premultiplicar(r.as_raw()),
            IntSize::from_wh(rw, rh).unwrap(),
        )
        .unwrap();
        let dx = (self.mm(w) - rw as f32) / 2.0;
        let dy = (self.mm(h) - rh as f32) / 2.0;
        self.px.draw_pixmap(
            (self.mm(x) + dx).round() as i32,
            (self.mm(y) + dy).round() as i32,
            pm.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

fn mm(k: f32, v: f32) -> f32 {
    v * k
}

fn premultiplicar(rgba: &[u8]) -> Vec<u8> {
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let a = p[3] as u16;
            [
                (p[0] as u16 * a / 255) as u8,
                (p[1] as u16 * a / 255) as u8,
                (p[2] as u16 * a / 255) as u8,
                p[3],
            ]
        })
        .collect()
}

// ── Utilidades de texto ────────────────────────────────────────────────────

struct Ctx {
    letras: Letras,
    m: Maquetador,
    simbolos: Simbolos,
    combo: &'static Combo,
}

impl Ctx {
    fn estilo(&self, familia: &str, peso: f32, tam: f32, color: Color) -> Estilo {
        Estilo {
            familia: familia.into(),
            peso,
            tam,
            interlinea: 1.06,
            color,
            alinear: Alinear::Inicio,
            versalitas: false,
            resalte: None,
            habilidad: None,
        }
    }

    /// Una línea centrada en vertical en `cy` (mm), desde `x` (mm), con
    /// estrechado si no cabe en `ancho` (mm). Con sombra opcional.
    fn linea(
        &mut self,
        l: &mut Lienzo,
        p: &Parrafo,
        e: &Estilo,
        x: f32,
        cy: f32,
        ancho: f32,
        alinear: Alinear,
        sombra: bool,
    ) -> f32 {
        let (b, f, _) = linea_ajustada(&mut self.m, &mut self.letras, p, e, l.mm(ancho), 0.65);
        let w = b.ancho * f;
        let x0 = match alinear {
            Alinear::Inicio => l.mm(x),
            Alinear::Centro => l.mm(x) + (l.mm(ancho) - w) / 2.0,
            Alinear::Fin => l.mm(x + ancho) - w,
        };
        let y0 = l.mm(cy) - b.base + e.tam * 0.34;
        if sombra {
            let d = (e.tam * 0.05).max(1.0);
            pintar_bloque(
                &mut l.px.as_mut(),
                &b,
                x0 + d,
                y0 + d,
                Pincel {
                    escala_x: f,
                    color: Some([0, 0, 0, 170]),
                },
                &self.simbolos,
            );
        }
        pintar_bloque(
            &mut l.px.as_mut(),
            &b,
            x0,
            y0,
            Pincel {
                escala_x: f,
                color: None,
            },
            &self.simbolos,
        );
        w / l.k
    }

    /// El coste de maná, alineado a la derecha en `x_fin`; devuelve dónde empieza.
    fn coste(&self, l: &mut Lienzo, coste: &str, x_fin: f32, cy: f32, lado: f32) -> f32 {
        let simbolos: Vec<&str> = coste
            .split('}')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_start_matches('{'))
            .collect();
        let hueco = 0.12;
        let total = simbolos.len() as f32 * lado + (simbolos.len().max(1) - 1) as f32 * hueco;
        let mut x = x_fin - total;
        let x0 = x;
        for s in simbolos {
            // Sombra bajo cada símbolo, como en las cartas impresas.
            proxy_core::diseno::simbolos::circulo(
                &mut l.px.as_mut(),
                mm(l.k, x + lado / 2.0 - 0.12),
                mm(l.k, cy + 0.18),
                mm(l.k, lado / 2.0),
                [0, 0, 0, 200],
            );
            self.simbolos.pintar(
                s,
                &mut l.px.as_mut(),
                mm(l.k, x),
                mm(l.k, cy - lado / 2.0),
                mm(l.k, lado),
            );
            x += lado + hueco;
        }
        x0
    }

    fn pie(&mut self, l: &mut Lienzo, lineas: &[(f32, f32, &str)], color: Color) {
        for (x, cy, t) in lineas {
            let e = self.estilo(PIE, 500.0, l.pt(4.5), color);
            let p = texto::con_simbolos(t);
            self.linea(l, &p, &e, *x, *cy, 52.0, Alinear::Inicio, false);
        }
    }
}

// ── Carta de prueba ────────────────────────────────────────────────────────

struct Carta {
    nombre: &'static str,
    original: &'static str,
    coste: &'static str,
    tipo: &'static str,
    reglas: &'static str,
    sabor: &'static str,
    fr: Option<&'static str>,
}

const VELDANAVA: Carta = Carta {
    nombre: "Veldanava, Dragón de la Creación",
    original: "Progenitus",
    coste: "{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}",
    tipo: "Criatura legendaria — Avatar Hidra",
    reglas: "Protección contra todo.\nSi Veldanava, Dragón de la Creación fuera a ir a un cementerio desde cualquier lugar, revela a Veldanava, Dragón de la Creación y barájalo en la biblioteca de su propietario en su lugar.",
    sabor: "“Te daré mi destrucción y la llamaré una bendición.”",
    fr: Some("10/10"),
};

const MILIM: Carta = Carta {
    nombre: "Milim, Señora Demonio Dragonoide",
    original: "Arcades, the Strategist",
    coste: "{1}{G}{W}{U}",
    tipo: "Criatura legendaria — Dragón anciano",
    reglas: "Vuela, vigilancia.\nSiempre que una criatura con defensor entre al campo de batalla bajo tu control, roba una carta.\nCada criatura que controlas con defensor asigna una cantidad de daño de combate igual a su resistencia en vez de su fuerza, y puede atacar como si no tuviera defensor.",
    sabor: "",
    fr: Some("3/5"),
};

// ── Despejado ──────────────────────────────────────────────────────────────

fn despejado(cx: &mut Ctx, l: &mut Lienzo, c: &Carta, arte: &Path, emblema: &Path, pal: Paleta) {
    let blanco = [255, 255, 255, 255];
    l.arte(arte, 0.0, 0.0, 63.0, 83.0);

    // Reglas: la caja crece hasta la altura estándar y luego baja la letra.
    let x_txt = 5.95;
    let ancho_txt = 56.15 - x_txt;
    let fondo = 83.0;
    let pad_arriba = 0.95;
    let pad_abajo = 0.75;
    let reglas_max = 27.4; // caja de reglas estándar (sin la barra de tipo)
    let reglas_min = 12.0;
    let e = cx.estilo(
        cx.combo.reglas,
        400.0,
        l.cap(1.78, cx.combo.cap_reglas),
        blanco,
    );
    let r = texto::reglas(c.reglas);
    let s = texto::sabor(c.sabor);
    let aj = ajustar_reglas(
        &mut cx.m,
        &mut cx.letras,
        &r,
        &s,
        &e,
        l.mm(ancho_txt),
        l.mm(reglas_max - pad_arriba - pad_abajo),
        l.pt(6.0),
        l.pt(0.1),
    );
    let alto_txt = aj.caja.alto / l.k;
    let alto_caja = (alto_txt + pad_arriba + pad_abajo).clamp(reglas_min, reglas_max);
    let y_reglas = fondo - alto_caja;
    let y_tipo = y_reglas - 6.0;

    let (tr, tg, tb) = pal.tinte;
    let legendaria = c.tipo.contains("legendaria");
    let corona = if legendaria {
        format!(
            r##"<path d="M7.6,4.5 C9.6,4.5 10.2,2.75 12.3,2.75 C13.8,2.75 14.2,3.45 15.5,3.45 L24.3,3.45 C25.9,3.45 26.3,2.05 28.2,2.05 C29.5,2.05 30.1,0.95 31.5,0.95 C32.9,0.95 33.5,2.05 34.8,2.05 C36.7,2.05 37.1,3.45 38.7,3.45 L47.5,3.45 C48.8,3.45 49.2,2.75 50.7,2.75 C52.8,2.75 53.4,4.5 55.4,4.5 Z" fill="url(#marco)" stroke="#000" stroke-opacity="0.6" stroke-width="0.12"/>
            <path d="M10.6,4.3 C11.3,4.2 11.6,3.25 12.4,3.25 C13.6,3.25 14.1,3.95 15.5,3.95 L24.5,3.95 C26.2,3.95 26.8,2.55 28.4,2.55 C29.6,2.55 30.3,1.5 31.5,1.5 C32.7,1.5 33.4,2.55 34.6,2.55 C36.2,2.55 36.8,3.95 38.5,3.95 L47.5,3.95 C48.9,3.95 49.4,3.25 50.6,3.25 C51.4,3.25 51.7,4.2 52.4,4.3 Z" fill="{o}" opacity="0.75"/>"##,
            o = pal.oscuro
        )
    } else {
        String::new()
    };
    let fr = c.fr.map_or(String::new(), |_| {
        r##"<rect x="47.8" y="79.65" width="10.2" height="4.45" rx="1.1" fill="url(#marco)" stroke="#000" stroke-opacity="0.6" stroke-width="0.12"/>
           <rect x="48.35" y="80.15" width="9.1" height="3.45" rx="0.75" fill="url(#panelfr)"/>"##
            .to_string()
    });
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 63 88" width="63" height="88">
<defs>
 <linearGradient id="marco" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="{claro}"/><stop offset="0.45" stop-color="{medio}"/><stop offset="1" stop-color="{oscuro}"/></linearGradient>
 <linearGradient id="panel" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="{oscuro}"/><stop offset="1" stop-color="rgb({tr},{tg},{tb})"/></linearGradient>
 <linearGradient id="panelfr" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="{claro}"/><stop offset="1" stop-color="{medio}"/></linearGradient>
</defs>
<!-- máscara del arte: esquinas de abajo redondeadas, pie en negro -->
<path fill-rule="evenodd" fill="#000" d="M0,0 H63 V88 H0 Z M0,0 H63 V77 A6,6 0 0 1 57,83 H6 A6,6 0 0 1 0,77 Z"/>
{corona}
<!-- barra del título -->
<rect x="2.0" y="3.9" width="57.0" height="5.55" rx="1.6" fill="url(#marco)" stroke="#000" stroke-opacity="0.55" stroke-width="0.12"/>
<rect x="3.1" y="4.3" width="54.8" height="4.75" rx="1.1" fill="url(#panel)"/>
<rect x="3.1" y="4.3" width="54.8" height="4.75" rx="1.1" fill="none" stroke="#000" stroke-opacity="0.5" stroke-width="0.1"/>
<!-- caja de reglas, translúcida y teñida -->
<rect x="4.45" y="{yr}" width="52.8" height="{hr}" fill="rgb({tr},{tg},{tb})" fill-opacity="0.80" stroke="url(#marco)" stroke-width="0.38"/>
<!-- barra de tipo -->
<rect x="3.55" y="{yt}" width="54.6" height="6.0" rx="1.25" fill="url(#marco)" stroke="#000" stroke-opacity="0.55" stroke-width="0.12"/>
<rect x="4.15" y="{yt2}" width="53.4" height="5.2" rx="0.9" fill="rgb({tr},{tg},{tb})" fill-opacity="0.94"/>
{fr}
</svg>"##,
        claro = pal.claro,
        medio = pal.medio,
        oscuro = pal.oscuro,
        yr = y_reglas,
        hr = alto_caja,
        yt = y_tipo,
        yt2 = y_tipo + 0.4,
    );
    l.svg(&svg);

    // Título y coste.
    let cy_titulo = 6.68;
    let x_coste = cx.coste(l, c.coste, 57.1, cy_titulo, 2.35);
    let (fam, peso) = cx.combo.titulo;
    let et = cx.estilo(fam, peso, l.cap(2.15, cx.combo.cap_titulo), blanco);
    cx.linea(
        l,
        &texto::llano(c.nombre),
        &et,
        4.55,
        cy_titulo,
        x_coste - 4.55 - 0.7,
        Alinear::Inicio,
        true,
    );

    // Tipo y emblema.
    let cy_tipo = y_tipo + 3.0;
    let ety = cx.estilo(fam, peso, l.cap(1.95, cx.combo.cap_titulo), blanco);
    cx.linea(
        l,
        &texto::llano(c.tipo),
        &ety,
        5.95,
        cy_tipo,
        51.1 - 5.95,
        Alinear::Inicio,
        true,
    );
    l.imagen(emblema, 52.1, cy_tipo - 1.6, 4.7, 3.2);

    // Reglas.
    pintar_caja(
        &mut l.px.as_mut(),
        &aj.caja,
        mm(l.k, x_txt),
        mm(l.k, y_reglas + pad_arriba),
        mm(l.k, ancho_txt),
        [255, 255, 255, 110],
        &cx.simbolos,
    );

    // Fuerza/resistencia.
    if let Some(fr) = c.fr {
        let e = cx.estilo(
            fam,
            peso,
            l.cap(2.5, cx.combo.cap_titulo),
            [25, 20, 10, 255],
        );
        cx.linea(
            l,
            &texto::llano(fr),
            &e,
            48.35,
            81.88,
            9.1,
            Alinear::Centro,
            false,
        );
    }

    // Pie.
    let gris = [235, 235, 235, 255];
    cx.pie(
        l,
        &[
            (2.1, 84.35, "0002"),
            (11.9, 84.35, "S"),
            (14.3, 84.35, c.original),
            (2.1, 85.95, "RTS{ESTRELLA}ES"),
            (11.9, 85.95, "{PINCEL} S.J."),
        ],
        gris,
    );
}

// ── Extendido ──────────────────────────────────────────────────────────────

fn extendido(cx: &mut Ctx, l: &mut Lienzo, c: &Carta, arte: &Path, emblema: &Path, pal: Paleta) {
    l.arte(arte, 0.0, 0.0, 63.0, 52.5);
    let tinta = [24, 20, 14, 255];
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 63 88" width="63" height="88">
<defs>
 <linearGradient id="marco" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="{claro}"/><stop offset="0.45" stop-color="{medio}"/><stop offset="1" stop-color="{oscuro}"/></linearGradient>
 <linearGradient id="lateral" x1="0" y1="0" x2="1" y2="1">
  <stop offset="0" stop-color="{claro}"/><stop offset="0.3" stop-color="{medio}"/><stop offset="0.55" stop-color="{claro}"/><stop offset="1" stop-color="{oscuro}"/></linearGradient>
 <linearGradient id="crema" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#f7efda"/><stop offset="1" stop-color="#eadcb8"/></linearGradient>
</defs>
<!-- marco inferior -->
<rect x="0" y="51.4" width="63" height="36.6" fill="#000"/>
<rect x="2.1" y="51.5" width="58.8" height="31.6" rx="1.0" fill="url(#lateral)"/>
<rect x="3.55" y="56.25" width="55.75" height="26.6" fill="url(#crema)" stroke="{oscuro}" stroke-width="0.25"/>
<!-- barra de tipo -->
<rect x="2.35" y="51.45" width="58.3" height="4.75" rx="1.4" fill="url(#marco)"/>
<rect x="2.95" y="51.85" width="57.1" height="3.95" rx="1.0" fill="#141414"/>
<!-- título -->
<rect x="2.05" y="2.65" width="59.0" height="6.3" rx="1.8" fill="url(#marco)" stroke="#000" stroke-opacity="0.5" stroke-width="0.12"/>
<rect x="3.55" y="3.55" width="56.0" height="4.6" rx="1.2" fill="url(#crema)" stroke="{oscuro}" stroke-width="0.12"/>
<!-- cinta del nombre original -->
<path d="M6.8,8.9 H56.5 L55.2,11.6 H8.1 Z" fill="url(#marco)"/>
<path d="M7.6,8.95 H55.7 L54.6,11.2 H8.7 Z" fill="#141414"/>
<!-- fuerza/resistencia -->
<rect x="50.5" y="81.2" width="9.8" height="4.25" rx="1.1" fill="url(#marco)" stroke="#000" stroke-opacity="0.5" stroke-width="0.12"/>
<rect x="51.05" y="81.7" width="8.7" height="3.25" rx="0.75" fill="url(#crema)"/>
</svg>"##,
        claro = pal.claro,
        medio = pal.medio,
        oscuro = pal.oscuro,
    );
    l.svg(&svg);

    let (fam, peso) = cx.combo.titulo;
    let x_coste = cx.coste(l, c.coste, 58.9, 5.85, 2.3);
    let et = cx.estilo(fam, peso, l.cap(2.15, cx.combo.cap_titulo), tinta);
    cx.linea(
        l,
        &texto::llano(c.nombre),
        &et,
        4.6,
        5.85,
        x_coste - 4.6 - 0.6,
        Alinear::Inicio,
        false,
    );
    let ec = cx.estilo(
        cx.combo.reglas,
        400.0,
        l.cap(1.3, cx.combo.cap_reglas),
        [240, 240, 240, 255],
    );
    let cinta: Parrafo = vec![texto::Trozo::Texto {
        texto: c.original.into(),
        cursiva: true,
    }];
    cx.linea(l, &cinta, &ec, 8.7, 10.05, 46.0, Alinear::Centro, false);

    let ety = cx.estilo(
        fam,
        peso,
        l.cap(1.85, cx.combo.cap_titulo),
        [255, 255, 255, 255],
    );
    cx.linea(
        l,
        &texto::llano(c.tipo),
        &ety,
        4.5,
        53.85,
        49.0,
        Alinear::Inicio,
        false,
    );
    l.imagen(emblema, 54.4, 52.35, 4.6, 3.0);

    let x_txt = 5.9;
    let ancho_txt = 57.6 - x_txt;
    let e = cx.estilo(
        cx.combo.reglas,
        400.0,
        l.cap(1.72, cx.combo.cap_reglas),
        tinta,
    );
    let r = texto::reglas(c.reglas);
    let s = texto::sabor(c.sabor);
    let aj = ajustar_reglas(
        &mut cx.m,
        &mut cx.letras,
        &r,
        &s,
        &e,
        l.mm(ancho_txt),
        l.mm(26.6 - 1.4 - 2.6),
        l.pt(6.0),
        l.pt(0.1),
    );
    // Centrado en vertical en la caja, como las cartas impresas.
    let libre = (26.6 - 1.4 - 2.6) - aj.caja.alto / l.k;
    pintar_caja(
        &mut l.px.as_mut(),
        &aj.caja,
        mm(l.k, x_txt),
        mm(l.k, 56.25 + 1.0 + libre / 2.0),
        mm(l.k, ancho_txt),
        [60, 50, 30, 160],
        &cx.simbolos,
    );
    if let Some(fr) = c.fr {
        let e = cx.estilo(fam, peso, l.cap(2.4, cx.combo.cap_titulo), tinta);
        cx.linea(
            l,
            &texto::llano(fr),
            &e,
            51.05,
            83.33,
            8.7,
            Alinear::Centro,
            false,
        );
    }
    cx.pie(
        l,
        &[(
            3.3,
            84.45,
            "2024  ·  MTG  ·  ES  ·  {PINCEL}  Bandai Namco (Isekai Memories)",
        )],
        [235, 235, 235, 255],
    );
}

// ── Paisaje ────────────────────────────────────────────────────────────────

fn paisaje(cx: &mut Ctx, l: &mut Lienzo, arte: &Path, emblema: &Path) {
    l.arte(arte, 0.0, 0.0, 63.0, 81.4);
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 63 88" width="63" height="88">
<defs>
 <linearGradient id="barra" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#5a5a5a"/><stop offset="0.5" stop-color="#2a2a2a"/><stop offset="1" stop-color="#3c3c3c"/></linearGradient>
 <linearGradient id="bisel" x1="0" y1="0" x2="0" y2="1">
  <stop offset="0" stop-color="#ffffff"/><stop offset="0.5" stop-color="#bdbdbd"/><stop offset="1" stop-color="#f0f0f0"/></linearGradient>
</defs>
<path fill-rule="evenodd" fill="#000" d="M0,0 H63 V88 H0 Z M0,0 H63 V77.4 A4,4 0 0 1 59,81.4 H4 A4,4 0 0 1 0,77.4 Z"/>
<rect x="8.6" y="73.9" width="51.1" height="6.0" rx="0.9" fill="url(#bisel)"/>
<rect x="9.1" y="74.35" width="50.1" height="5.1" rx="0.6" fill="url(#barra)"/>
<circle cx="5.95" cy="76.9" r="4.15" fill="url(#bisel)"/>
<circle cx="5.95" cy="76.9" r="3.6" fill="#1e1e1e"/>
</svg>"##;
    l.svg(svg);
    cx.simbolos.pintar(
        "W",
        &mut l.px.as_mut(),
        mm(l.k, 5.95 - 2.7),
        mm(l.k, 76.9 - 2.7),
        mm(l.k, 5.4),
    );
    let (fam, peso) = cx.combo.titulo;
    let e = cx.estilo(
        fam,
        peso,
        l.cap(2.1, cx.combo.cap_titulo),
        [245, 245, 245, 255],
    );
    cx.linea(
        l,
        &texto::llano("Reino Orco de Orbic"),
        &e,
        10.5,
        76.9,
        42.0,
        Alinear::Centro,
        true,
    );
    l.imagen(emblema, 53.4, 75.4, 4.6, 3.0);
    cx.pie(
        l,
        &[
            (4.05, 82.9, "0076"),
            (11.9, 82.9, "S"),
            (14.3, 82.9, "Plains"),
            (4.05, 84.5, "RTS • ES"),
            (11.9, 84.5, "{PINCEL} S.J."),
        ],
        [235, 235, 235, 255],
    );
}

// ── Principal ──────────────────────────────────────────────────────────────

fn cargar_simbolos(dir: &Path) -> Simbolos {
    let mut s = Simbolos::nuevos();
    for f in std::fs::read_dir(dir).unwrap().flatten() {
        let p = f.path();
        if p.extension().is_some_and(|e| e == "svg") {
            let nombre = p.file_stem().unwrap().to_string_lossy().replace('_', "/");
            let _ = s.cargar(&nombre, &std::fs::read(&p).unwrap());
        }
    }
    // Los iconos del pie, propios.
    s.cargar("ESTRELLA", br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path fill="#ebebeb" d="M50 8 L61 38 L93 38 L67 57 L77 89 L50 70 L23 89 L33 57 L7 38 L39 38 Z"/></svg>"##).unwrap();
    s.cargar("PINCEL", br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path fill="#ebebeb" d="M2 46 C14 32 34 30 46 42 L94 22 C100 20 102 30 96 32 L52 58 C46 76 22 82 2 70 C12 64 14 54 2 46 Z"/></svg>"##).unwrap();
    s
}

fn main() {
    let args: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let (letras_dir, simb_dir, artes, salida) = (&args[0], &args[1], &args[2], &args[3]);
    std::fs::create_dir_all(salida).unwrap();
    let emblema = artes.join("emblema.png");

    for combo in COMBOS {
        let mut letras = Letras::nuevas();
        for f in combo.ficheros.iter().chain(["Montserrat[wght].ttf"].iter()) {
            letras
                .cargar(std::fs::read(letras_dir.join(f)).unwrap())
                .unwrap();
        }
        let mut cx = Ctx {
            letras,
            m: Maquetador::nuevo(),
            simbolos: cargar_simbolos(simb_dir),
            combo,
        };
        for (ancho, sufijo) in [(1060u32, ""), (2232, "-900ppp")] {
            let mut l = Lienzo::nuevo(ancho);
            despejado(
                &mut cx,
                &mut l,
                &VELDANAVA,
                &artes.join("veldanava.png"),
                &emblema,
                ORO,
            );
            l.px.save_png(salida.join(format!("{}-veldanava{sufijo}.png", combo.id)))
                .unwrap();
            let mut l = Lienzo::nuevo(ancho);
            extendido(
                &mut cx,
                &mut l,
                &MILIM,
                &artes.join("milim.png"),
                &emblema,
                ORO,
            );
            l.px.save_png(salida.join(format!("{}-milim{sufijo}.png", combo.id)))
                .unwrap();
            let mut l = Lienzo::nuevo(ancho);
            paisaje(&mut cx, &mut l, &artes.join("reino.png"), &emblema);
            l.px.save_png(salida.join(format!("{}-reino{sufijo}.png", combo.id)))
                .unwrap();
        }
        println!("{} listo", combo.nombre);

        // Tiempos (solo con la primera combinación).
        if combo.id == "a" {
            for ancho in [600u32, 2232] {
                let t = Instant::now();
                let n = 5;
                for _ in 0..n {
                    let mut l = Lienzo::nuevo(ancho);
                    despejado(
                        &mut cx,
                        &mut l,
                        &VELDANAVA,
                        &artes.join("veldanava.png"),
                        &emblema,
                        ORO,
                    );
                }
                println!("despejado a {ancho} px: {:?} por carta", t.elapsed() / n);
            }
            // Solo el texto (sin decodificar el arte), que es lo que mide el plan.
            for ancho in [600u32, 2232] {
                let l = Lienzo::nuevo(ancho);
                let e = cx.estilo(
                    combo.reglas,
                    400.0,
                    l.cap(1.78, combo.cap_reglas),
                    [255, 255, 255, 255],
                );
                let r = texto::reglas(VELDANAVA.reglas);
                let s = texto::sabor(VELDANAVA.sabor);
                let t = Instant::now();
                let mut lienzo = Lienzo::nuevo(ancho);
                for _ in 0..10 {
                    let aj = ajustar_reglas(
                        &mut cx.m,
                        &mut cx.letras,
                        &r,
                        &s,
                        &e,
                        l.mm(50.2),
                        l.mm(25.7),
                        l.pt(6.0),
                        l.pt(0.1),
                    );
                    pintar_caja(
                        &mut lienzo.px.as_mut(),
                        &aj.caja,
                        10.0,
                        10.0,
                        l.mm(50.2),
                        [255, 255, 255, 110],
                        &cx.simbolos,
                    );
                }
                println!("caja de reglas a {ancho} px: {:?}", t.elapsed() / 10);
            }
            let _ = &VELDANAVA;
        }
        let _ = BLANCO;
        let _ = AZUL;
    }
}
