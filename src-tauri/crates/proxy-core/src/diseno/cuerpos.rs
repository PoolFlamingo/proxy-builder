//! **Lo que va dentro de la caja de reglas** según el cuerpo de la cara
//! (Fase 5): texto normal, o secciones con su insignia a la izquierda
//! (lealtad, capítulo, nivel, prototipo), su cabecera (nivel de clase,
//! «Resolver») y su fuerza/resistencia a la derecha (leveler, estación), o
//! las dos columnas de una aventura.
//!
//! Igual que el texto normal, todo se maqueta a un tamaño de letra y se baja
//! de 0,1 pt en 0,1 pt hasta que cabe (`ajustar`).

use tiny_skia::{Paint, Rect, Transform};

use super::estructura::{self, a_romano};
use super::letras::Letras;
use super::maquetacion::Cuerpo;
use super::modelo::DesignLang;
use super::pintar::{pintar_bloque, pintar_caja, Pincel};
use super::render::Carta;
use super::texto::{
    self, ajustar_reglas, caja_de_reglas, Bloque, CajaTexto, Estilo, Maquetador, Parrafo,
    Separaciones, Trozo,
};
use super::Color;

/// La marca de la izquierda de una sección.
#[derive(Debug, Clone, PartialEq)]
pub enum Insignia {
    /// `+1`, `−2`, `0`.
    Lealtad(String),
    /// Los números de un capítulo.
    Capitulos(Vec<u32>),
    /// Un rango de nivel (`NIVEL` / `2-6`) o un umbral de estación (`8+`).
    Nivel { etiqueta: String, valor: String },
    /// El recuadro del prototipo: coste y fuerza/resistencia.
    Prototipo { coste: String, fr: String },
}

impl Insignia {
    /// Ancho de la columna (mm).
    fn ancho(&self) -> f32 {
        match self {
            Insignia::Lealtad(_) => 6.4,
            Insignia::Capitulos(_) => 4.8,
            Insignia::Nivel { .. } => 6.4,
            Insignia::Prototipo { .. } => 9.6,
        }
    }
    /// Alto mínimo de la sección (mm).
    fn alto_min(&self) -> f32 {
        match self {
            Insignia::Lealtad(_) => 4.6,
            Insignia::Capitulos(n) => n.len() as f32 * 3.9 + 0.4,
            Insignia::Nivel { .. } => 5.2,
            Insignia::Prototipo { .. } => 7.8,
        }
    }
}

/// Una sección de la caja de reglas.
#[derive(Debug, Clone, Default)]
pub struct Seccion {
    pub izq: Option<Insignia>,
    pub cabecera: Option<Parrafo>,
    pub parrafos: Vec<Parrafo>,
    /// Fuerza/resistencia del tramo (leveler, estación).
    pub fr: Option<String>,
    /// Lleva banda de fondo (las secciones alternas).
    pub banda: bool,
}

/// El contenido de la caja.
pub enum Contenido {
    Texto {
        reglas: Vec<Parrafo>,
        sabor: Vec<Parrafo>,
    },
    Secciones {
        secciones: Vec<Seccion>,
        sabor: Vec<Parrafo>,
        /// Se reparte el alto que sobra entre las secciones (sagas, clases).
        repartir: bool,
        /// Raya entre secciones.
        rayas: bool,
    },
    /// Aventura (o presagio): el libro a la izquierda y la criatura a la derecha.
    Aventura {
        cabecera: Parrafo,
        tipo: Parrafo,
        izq: Vec<Parrafo>,
        der: Vec<Parrafo>,
        sabor: Vec<Parrafo>,
    },
}

/// Una sección ya maquetada (px).
pub struct SeccionHecha {
    pub y: f32,
    pub alto: f32,
    pub x_txt: f32,
    pub ancho_txt: f32,
    pub cab: Option<Bloque>,
    pub cuerpo: CajaTexto,
    pub alto_txt: f32,
    /// Lleva su caja de fuerza/resistencia a la derecha.
    pub con_fr: bool,
}

/// El contenido maquetado a un tamaño (px).
#[allow(clippy::large_enum_variant)]
pub enum Hecho {
    Texto(CajaTexto),
    Secciones {
        items: Vec<SeccionHecha>,
        sabor: Option<(CajaTexto, f32)>,
        alto: f32,
        tam: f32,
    },
    Aventura {
        ancho_izq: f32,
        x_der: f32,
        nombre: Bloque,
        tipo: Bloque,
        izq: CajaTexto,
        der: CajaTexto,
        alto_izq: f32,
        alto: f32,
        tam: f32,
    },
}

impl Hecho {
    pub fn alto(&self) -> f32 {
        match self {
            Hecho::Texto(c) => c.alto,
            Hecho::Secciones { alto, .. } | Hecho::Aventura { alto, .. } => *alto,
        }
    }
    pub fn tam(&self) -> f32 {
        match self {
            Hecho::Texto(c) => c.tam,
            Hecho::Secciones { tam, .. } | Hecho::Aventura { tam, .. } => *tam,
        }
    }
    /// Hasta dónde llega a lo ancho el último renglón (px desde la izquierda
    /// de la caja), para no pisar la fuerza/resistencia.
    pub fn fin_ultima_linea(&self) -> f32 {
        let ultima = |c: &CajaTexto| {
            c.bloques
                .last()
                .map_or(0.0, |(b, _)| b.ultima_linea_ancho())
        };
        match self {
            Hecho::Texto(c) => ultima(c),
            Hecho::Secciones { items, sabor, .. } => match sabor {
                Some((c, _)) => ultima(c),
                // Una franja con su F/R llega siempre a la derecha.
                None => items.last().map_or(0.0, |s| {
                    if s.con_fr {
                        f32::MAX
                    } else {
                        s.x_txt + ultima(&s.cuerpo)
                    }
                }),
            },
            Hecho::Aventura { x_der, der, .. } => x_der + ultima(der),
        }
    }
}

fn negrita(e: &Estilo) -> Estilo {
    Estilo {
        peso: 700.0,
        ..e.clone()
    }
}

/// Maqueta el contenido a un tamaño de letra.
#[allow(clippy::too_many_arguments)]
pub fn maquetar(
    c: &Contenido,
    m: &mut Maquetador,
    letras: &mut Letras,
    est: &Estilo,
    ancho: f32,
    tam: f32,
    sep: Separaciones,
    k: f32,
) -> Hecho {
    let e = est.con_tam(tam);
    match c {
        Contenido::Texto { reglas, sabor } => {
            Hecho::Texto(caja_de_reglas(m, letras, reglas, sabor, &e, ancho, sep))
        }
        Contenido::Secciones {
            secciones, sabor, ..
        } => {
            let pad = 0.24 * tam;
            let hueco = 0.8 * k;
            let cab_e = negrita(&e);
            let mut y = 0.0;
            let mut items = Vec::new();
            for s in secciones {
                let izq = s.izq.as_ref().map_or(0.0, |i| i.ancho() * k + hueco);
                let der = if s.fr.is_some() { 7.0 * k + hueco } else { 0.0 };
                let w = (ancho - izq - der).max(4.0 * k);
                let cab = s
                    .cabecera
                    .as_ref()
                    .map(|p| m.maquetar(letras, p, &cab_e, Some(w)));
                let cuerpo = caja_de_reglas(m, letras, &s.parrafos, &[], &e, w, sep);
                let entre = if cab.is_some() && !s.parrafos.is_empty() {
                    0.12 * tam
                } else {
                    0.0
                };
                let alto_txt = cab.as_ref().map_or(0.0, |b| b.alto) + entre + cuerpo.alto;
                let minimo = s
                    .izq
                    .as_ref()
                    .map_or(0.0, |i| i.alto_min() * k)
                    .max(if s.fr.is_some() { 4.4 * k } else { 0.0 });
                let alto = (alto_txt + 2.0 * pad).max(minimo);
                items.push(SeccionHecha {
                    y,
                    alto,
                    x_txt: izq,
                    ancho_txt: w,
                    cab,
                    cuerpo,
                    alto_txt,
                    con_fr: s.fr.is_some(),
                });
                y += alto;
            }
            let sabor = if sabor.is_empty() {
                None
            } else {
                y += sep.sabor * tam;
                let c = caja_de_reglas(m, letras, &[], sabor, &e, ancho, sep);
                let y0 = y;
                y += c.alto;
                Some((c, y0))
            };
            Hecho::Secciones {
                items,
                sabor,
                alto: y,
                tam,
            }
        }
        Contenido::Aventura {
            cabecera,
            tipo,
            izq,
            der,
            sabor,
        } => {
            let pad = 0.9 * k;
            let ancho_izq = ancho * 0.48;
            let x_der = ancho_izq + 1.2 * k;
            let w_izq = ancho_izq - 2.0 * pad;
            let nombre = m.maquetar(letras, cabecera, &negrita(&e), Some(w_izq));
            let tipo_b = m.maquetar(letras, tipo, &e.con_tam(tam * 0.92), Some(w_izq));
            let izq_c = caja_de_reglas(m, letras, izq, &[], &e, w_izq, sep);
            let der_c = caja_de_reglas(m, letras, der, sabor, &e, ancho - x_der, sep);
            let alto_izq = 2.0 * pad + nombre.alto + tipo_b.alto + 0.25 * tam + izq_c.alto;
            Hecho::Aventura {
                ancho_izq,
                x_der,
                alto: alto_izq.max(der_c.alto),
                nombre,
                tipo: tipo_b,
                izq: izq_c,
                der: der_c,
                alto_izq,
                tam,
            }
        }
    }
}

/// Baja la letra hasta que el contenido quepa en `alto_max` (igual que
/// `ajustar_reglas`, que es lo que hace el texto normal).
#[allow(clippy::too_many_arguments)]
pub fn ajustar(
    c: &Contenido,
    m: &mut Maquetador,
    letras: &mut Letras,
    est: &Estilo,
    ancho: f32,
    alto_max: f32,
    tam_min: f32,
    paso: f32,
    k: f32,
) -> (Hecho, f32) {
    if let Contenido::Texto { reglas, sabor } = c {
        let a = ajustar_reglas(
            m, letras, reglas, sabor, est, ancho, alto_max, tam_min, paso,
        );
        return (Hecho::Texto(a.caja), a.sobra);
    }
    let sep = Separaciones::default();
    let h = maquetar(c, m, letras, est, ancho, est.tam, sep, k);
    if h.alto() <= alto_max {
        return (h, 0.0);
    }
    let pasos = ((est.tam - tam_min) / paso).floor().max(0.0) as u32;
    let (mut lo, mut hi) = (1u32, pasos);
    let mut mejor = None;
    while lo <= hi {
        let mid = (lo + hi) / 2;
        let tam = est.tam - mid as f32 * paso;
        let h = maquetar(c, m, letras, est, ancho, tam, sep, k);
        if h.alto() <= alto_max {
            mejor = Some(h);
            hi = mid - 1;
        } else {
            lo = mid + 1;
        }
    }
    if let Some(h) = mejor {
        return (h, 0.0);
    }
    let apretado = Separaciones {
        parrafo: 0.12,
        sabor: 0.45,
    };
    let h = maquetar(c, m, letras, est, ancho, tam_min, apretado, k);
    let sobra = (h.alto() - alto_max).max(0.0);
    (h, sobra)
}

// ── Del texto de la carta al contenido ─────────────────────────────────────

fn parrafos(lineas: &[String]) -> Vec<Parrafo> {
    lineas.iter().flat_map(|l| texto::reglas(l)).collect()
}

/// Etiqueta de los niveles del leveler según el idioma.
fn palabra_nivel(lang: DesignLang) -> &'static str {
    match lang {
        DesignLang::Es => "NIVEL",
        DesignLang::En => "LEVEL",
    }
}

/// El contenido de la caja según el cuerpo de la cara. Lo que no sigue el
/// patrón esperado queda como texto normal.
pub fn contenido(
    cuerpo: Cuerpo,
    reglas_txt: &str,
    sabor: Vec<Parrafo>,
    lang: DesignLang,
) -> Contenido {
    let limpio = estructura::limpiar(reglas_txt);
    let texto_normal = |sabor| Contenido::Texto {
        reglas: texto::reglas(&limpio),
        sabor,
    };
    match cuerpo {
        Cuerpo::Texto | Cuerpo::Mazmorra => texto_normal(sabor),
        Cuerpo::Lealtad => {
            let h = estructura::planeswalker(&limpio);
            if h.iter().all(|x| x.coste.is_none()) {
                return texto_normal(sabor);
            }
            let secciones = h
                .into_iter()
                .enumerate()
                .map(|(i, x)| Seccion {
                    izq: x.coste.map(Insignia::Lealtad),
                    cabecera: None,
                    parrafos: texto::reglas(&x.texto),
                    fr: None,
                    banda: i % 2 == 1,
                })
                .collect();
            Contenido::Secciones {
                secciones,
                sabor,
                repartir: true,
                rayas: false,
            }
        }
        Cuerpo::Capitulos => {
            let s = estructura::saga(&limpio);
            if s.capitulos.is_empty() {
                return texto_normal(sabor);
            }
            let mut secciones = Vec::new();
            if !s.intro.is_empty() {
                secciones.push(Seccion {
                    parrafos: parrafos(&s.intro),
                    ..Seccion::default()
                });
            }
            for c in s.capitulos {
                secciones.push(Seccion {
                    izq: Some(Insignia::Capitulos(c.numeros)),
                    parrafos: texto::reglas(&c.texto),
                    ..Seccion::default()
                });
            }
            Contenido::Secciones {
                secciones,
                sabor,
                repartir: true,
                rayas: true,
            }
        }
        Cuerpo::Niveles | Cuerpo::Caso => {
            let t = if cuerpo == Cuerpo::Niveles {
                estructura::clase(&limpio)
            } else {
                estructura::caso(&limpio)
            };
            if t.len() < 2 {
                return texto_normal(sabor);
            }
            let secciones = t
                .into_iter()
                .enumerate()
                .map(|(i, x)| Seccion {
                    izq: None,
                    cabecera: x.cabecera.map(|c| texto::con_simbolos(&c)),
                    parrafos: parrafos(&x.texto),
                    fr: None,
                    banda: i % 2 == 1,
                })
                .collect();
            Contenido::Secciones {
                secciones,
                sabor,
                repartir: true,
                rayas: true,
            }
        }
        Cuerpo::Leveler | Cuerpo::Estacion => {
            let t = if cuerpo == Cuerpo::Leveler {
                estructura::leveler(&limpio)
            } else {
                estructura::estacion(&limpio)
            };
            if t.len() < 2 {
                return texto_normal(sabor);
            }
            let etiqueta = if cuerpo == Cuerpo::Leveler {
                palabra_nivel(lang).to_string()
            } else {
                String::new()
            };
            let secciones = t
                .into_iter()
                .enumerate()
                .map(|(i, x)| Seccion {
                    izq: x.cabecera.map(|valor| Insignia::Nivel {
                        etiqueta: etiqueta.clone(),
                        valor,
                    }),
                    cabecera: None,
                    parrafos: parrafos(&x.texto),
                    fr: x.fr,
                    banda: i % 2 == 1,
                })
                .collect();
            Contenido::Secciones {
                secciones,
                sabor,
                repartir: false,
                rayas: true,
            }
        }
        Cuerpo::Prototipo => match estructura::prototipo(&limpio) {
            Some(p) => {
                let mut secciones = vec![Seccion {
                    izq: Some(Insignia::Prototipo {
                        coste: p.coste,
                        fr: p.fr,
                    }),
                    parrafos: p
                        .recordatorio
                        .map(|r| texto::reglas(&r))
                        .unwrap_or_default(),
                    banda: true,
                    ..Seccion::default()
                }];
                if !p.resto.is_empty() {
                    secciones.push(Seccion {
                        parrafos: parrafos(&p.resto),
                        ..Seccion::default()
                    });
                }
                Contenido::Secciones {
                    secciones,
                    sabor,
                    repartir: false,
                    rayas: false,
                }
            }
            None => texto_normal(sabor),
        },
    }
}

/// El contenido de una aventura: la cara 1 en el libro y la 0 a la derecha.
pub fn aventura(
    nombre: &str,
    coste: &str,
    tipo: &str,
    reglas_aventura: &str,
    reglas_criatura: &str,
    sabor: Vec<Parrafo>,
) -> Contenido {
    let mut cab = vec![Trozo::Texto {
        texto: format!("{nombre}  "),
        cursiva: false,
    }];
    cab.extend(texto::con_simbolos(coste));
    Contenido::Aventura {
        cabecera: cab,
        tipo: vec![Trozo::Texto {
            texto: tipo.to_string(),
            cursiva: true,
        }],
        izq: texto::reglas(&estructura::limpiar(reglas_aventura)),
        der: texto::reglas(&estructura::limpiar(reglas_criatura)),
        sabor,
    }
}

// ── Pintar ─────────────────────────────────────────────────────────────────

fn rect(x: f32, y: f32, w: f32, h: f32) -> Option<Rect> {
    Rect::from_xywh(x, y, w.max(0.5), h.max(0.5))
}

fn pintura(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c[0], c[1], c[2], c[3]);
    p.anti_alias = true;
    p
}

impl Carta<'_> {
    /// Pinta el contenido maquetado en su caja (`x, y, w, h` en mm). `h` es
    /// el alto disponible: las secciones que reparten lo llenan.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn pintar_contenido(
        &mut self,
        c: &Contenido,
        hecho: &Hecho,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        tinta: Color,
    ) {
        let raya = [tinta[0], tinta[1], tinta[2], 110];
        let (px, py, pw, ph) = (self.l.x(x), self.l.y(y), self.l.x(w), self.l.y(h));
        match (c, hecho) {
            (_, Hecho::Texto(caja)) => {
                let simbolos = self.rec.simbolos;
                self.faltan.extend(pintar_caja(
                    &mut self.l.px.as_mut(),
                    caja,
                    px,
                    py,
                    pw,
                    raya,
                    simbolos,
                ));
            }
            (
                Contenido::Secciones {
                    secciones,
                    repartir,
                    rayas,
                    ..
                },
                Hecho::Secciones {
                    items, sabor, alto, ..
                },
            ) => {
                let n = items.len().max(1) as f32;
                let alto_sabor = sabor.as_ref().map_or(0.0, |(c, _)| c.alto);
                let extra = if *repartir {
                    ((ph - alto).max(0.0)) / n
                } else {
                    0.0
                };
                let mut y_act = py;
                for (i, (s, it)) in secciones.iter().zip(items).enumerate() {
                    let alto_s = it.alto + extra;
                    if s.banda {
                        if let Some(r) = rect(px - 0.5 * self.l.kx, y_act, pw + self.l.kx, alto_s) {
                            self.l.px.fill_rect(
                                r,
                                &pintura([tinta[0], tinta[1], tinta[2], 22]),
                                Transform::identity(),
                                None,
                            );
                        }
                    }
                    if *rayas && i > 0 {
                        let g = (0.14 * self.l.kx).max(1.0);
                        if let Some(r) = rect(px, y_act - g / 2.0, pw, g) {
                            self.l
                                .px
                                .fill_rect(r, &pintura(raya), Transform::identity(), None);
                        }
                    }
                    let y_txt = y_act + (alto_s - it.alto_txt) / 2.0;
                    let mut yy = y_txt;
                    if let Some(cab) = &it.cab {
                        self.faltan.extend(pintar_bloque(
                            &mut self.l.px.as_mut(),
                            cab,
                            px + it.x_txt,
                            yy,
                            Pincel::default(),
                            self.rec.simbolos,
                        ));
                        yy += cab.alto
                            + if s.parrafos.is_empty() {
                                0.0
                            } else {
                                0.12 * hecho.tam()
                            };
                    }
                    let simbolos = self.rec.simbolos;
                    self.faltan.extend(pintar_caja(
                        &mut self.l.px.as_mut(),
                        &it.cuerpo,
                        px + it.x_txt,
                        yy,
                        it.ancho_txt,
                        raya,
                        simbolos,
                    ));
                    let cy_mm = (y_act + alto_s / 2.0) / self.l.ky;
                    if let Some(ins) = &s.izq {
                        self.insignia(ins, x + ins.ancho() / 2.0, cy_mm, alto_s / self.l.ky);
                    }
                    if let Some(fr) = &s.fr {
                        self.caja_fr_tramo(fr, x + w - 3.5, cy_mm);
                    }
                    y_act += alto_s;
                }
                if let Some((caja, _)) = sabor {
                    let ys = py + ph.max(*alto) - alto_sabor;
                    let g = (caja.tam * 0.06).max(1.0);
                    let r_y = ys - Separaciones::default().sabor * caja.tam / 2.0;
                    if let Some(r) = rect(px + pw * 0.02, r_y - g / 2.0, pw * 0.96, g) {
                        self.l
                            .px
                            .fill_rect(r, &pintura(raya), Transform::identity(), None);
                    }
                    let simbolos = self.rec.simbolos;
                    self.faltan.extend(pintar_caja(
                        &mut self.l.px.as_mut(),
                        caja,
                        px,
                        ys,
                        pw,
                        raya,
                        simbolos,
                    ));
                }
            }
            (
                Contenido::Aventura { .. },
                Hecho::Aventura {
                    ancho_izq,
                    x_der,
                    nombre,
                    tipo,
                    izq,
                    der,
                    alto_izq,
                    tam,
                    ..
                },
            ) => {
                let pad = 0.9 * self.l.kx;
                let alto_libro = alto_izq.max(ph);
                // El libro: una página clara con borde.
                let svg = format!(
                    r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="0.6" fill="{fondo}" fill-opacity="0.16" stroke="{borde}" stroke-opacity="0.7" stroke-width="0.22"/>"##,
                    x = x,
                    y = y,
                    w = ancho_izq / self.l.kx,
                    h = alto_libro / self.l.ky,
                    fondo = super::render::hex(tinta),
                    borde = super::render::hex(tinta),
                );
                self.forma(&svg);
                let mut yy = py + pad;
                for b in [nombre, tipo] {
                    self.faltan.extend(pintar_bloque(
                        &mut self.l.px.as_mut(),
                        b,
                        px + pad,
                        yy,
                        Pincel::default(),
                        self.rec.simbolos,
                    ));
                    yy += b.alto;
                }
                yy += 0.25 * tam;
                let simbolos = self.rec.simbolos;
                self.faltan.extend(pintar_caja(
                    &mut self.l.px.as_mut(),
                    izq,
                    px + pad,
                    yy,
                    ancho_izq - 2.0 * pad,
                    raya,
                    simbolos,
                ));
                self.faltan.extend(pintar_caja(
                    &mut self.l.px.as_mut(),
                    der,
                    px + x_der,
                    py,
                    pw - x_der,
                    raya,
                    simbolos,
                ));
            }
            _ => {}
        }
    }

    /// Una insignia centrada en `(cx, cy)` (mm), en una sección de `alto` mm.
    fn insignia(&mut self, ins: &Insignia, cx: f32, cy: f32, alto: f32) {
        let medio = self.vars.get("medio").cloned().unwrap_or_default();
        match ins {
            Insignia::Lealtad(coste) => {
                let (w, h) = (5.8, 4.2);
                let (x0, x1, y0, y1) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let d = if coste.starts_with('+') {
                    format!(
                        "M{x0},{a} L{cx},{y0} L{x1},{a} L{x1},{y1} L{x0},{y1} Z",
                        a = y0 + 1.1
                    )
                } else if coste.starts_with('−') {
                    format!(
                        "M{x0},{y0} L{x1},{y0} L{x1},{b} L{cx},{y1} L{x0},{b} Z",
                        b = y1 - 1.1
                    )
                } else {
                    format!("M{x0},{y0} L{x1},{y0} L{x1},{y1} L{x0},{y1} Z")
                };
                self.forma(&format!(
                    r##"<path d="{d}" fill="#151515" stroke="{medio}" stroke-width="0.32" stroke-linejoin="round"/>"##
                ));
                self.texto_centrado(coste, "fr", 1.75, [255, 255, 255, 255], cx, cy, w - 0.6);
            }
            Insignia::Capitulos(nums) => {
                let (w, h, sep) = (4.3, 3.5, 0.4);
                let total = nums.len() as f32 * h + (nums.len().saturating_sub(1)) as f32 * sep;
                let mut y = cy - total / 2.0;
                let _ = alto;
                for n in nums {
                    let (x0, x1, ym, y1) = (cx - w / 2.0, cx + w / 2.0, y + h / 2.0, y + h);
                    let d = format!(
                        "M{x0},{ym} L{a},{y} L{b},{y} L{x1},{ym} L{b},{y1} L{a},{y1} Z",
                        a = cx - w / 4.0,
                        b = cx + w / 4.0
                    );
                    self.forma(&format!(
                        r##"<path d="{d}" fill="#151515" stroke="{medio}" stroke-width="0.3" stroke-linejoin="round"/>"##
                    ));
                    self.texto_centrado(
                        &a_romano(*n),
                        "fr",
                        1.3,
                        [255, 255, 255, 255],
                        cx,
                        ym,
                        w - 0.8,
                    );
                    y += h + sep;
                }
            }
            Insignia::Nivel { etiqueta, valor } => {
                let (w, h) = (6.0, 4.6);
                let (x0, x1, y0, y1) = (cx - w / 2.0, cx + w / 2.0, cy - h / 2.0, cy + h / 2.0);
                let d = format!(
                    "M{x0},{y0} L{a},{y0} L{x1},{cy} L{a},{y1} L{x0},{y1} Z",
                    a = x1 - 1.2
                );
                self.forma(&format!(
                    r##"<path d="{d}" fill="#151515" stroke="{medio}" stroke-width="0.3" stroke-linejoin="round"/>"##
                ));
                if etiqueta.is_empty() {
                    self.texto_centrado(
                        valor,
                        "fr",
                        1.6,
                        [255, 255, 255, 255],
                        cx - 0.5,
                        cy,
                        w - 1.6,
                    );
                } else {
                    self.texto_centrado(
                        etiqueta,
                        "fr",
                        0.8,
                        [255, 255, 255, 255],
                        cx - 0.5,
                        cy - 1.05,
                        w - 1.6,
                    );
                    self.texto_centrado(
                        valor,
                        "fr",
                        1.45,
                        [255, 255, 255, 255],
                        cx - 0.5,
                        cy + 0.75,
                        w - 1.6,
                    );
                }
            }
            Insignia::Prototipo { coste, fr } => {
                let (w, h) = (9.0, 7.2);
                let (x0, y0) = (cx - w / 2.0, cy - h / 2.0);
                self.forma(&format!(
                    r##"<rect x="{x0}" y="{y0}" width="{w}" height="{h}" rx="0.8" fill="#151515" stroke="{medio}" stroke-width="0.3"/>"##
                ));
                self.simbolos_en_fila(coste, cx, y0 + 2.0, 2.3, w - 1.0);
                self.caja_fr_tramo(fr, cx, y0 + h - 1.95);
            }
        }
    }

    /// La caja pequeña de fuerza/resistencia de un tramo, centrada en (cx, cy).
    fn caja_fr_tramo(&mut self, fr: &str, cx: f32, cy: f32) {
        let (w, h) = (6.4, 3.2);
        self.forma(&format!(
            r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="0.6" fill="url(#panelfr)" stroke="#000" stroke-opacity="0.6" stroke-width="0.15"/>"##,
            x = cx - w / 2.0,
            y = cy - h / 2.0
        ));
        let tinta = self
            .vars
            .get("tintaFr")
            .and_then(|t| super::pack::rgba(t))
            .unwrap_or([255, 255, 255, 255]);
        self.texto_centrado(fr, "fr", 1.6, tinta, cx, cy, w - 0.6);
    }
}

impl Carta<'_> {
    /// Las salas de una mazmorra en filas, unidas por flechas, dentro de la
    /// caja (mm). Cada sala lleva su nombre y su texto, que se ajusta solo.
    pub(super) fn pintar_mazmorra(
        &mut self,
        salas: &[estructura::Sala],
        caja: super::render::Caja,
        est: &Estilo,
        tinta: Color,
    ) {
        let filas = estructura::filas_de_mazmorra(salas);
        let n = filas.len().max(1) as f32;
        let hueco_y = 1.9;
        let alto_fila = (caja.h - hueco_y * (n - 1.0)) / n;
        let hueco_x = 1.2;
        // Dónde queda cada sala (mm).
        let mut sitio = vec![(0.0f32, 0.0f32, 0.0f32, 0.0f32); salas.len()];
        for (r, fila) in filas.iter().enumerate() {
            let k = fila.len().max(1) as f32;
            let w = ((caja.w - hueco_x * (k - 1.0)) / k).min(caja.w * 0.72);
            let total = w * k + hueco_x * (k - 1.0);
            let x0 = caja.x + (caja.w - total) / 2.0;
            let y = caja.y + r as f32 * (alto_fila + hueco_y);
            for (j, &i) in fila.iter().enumerate() {
                sitio[i] = (x0 + j as f32 * (w + hueco_x), y, w, alto_fila);
            }
        }
        let t = super::render::hex(tinta);
        // Flechas primero, por debajo de las salas.
        let mut flechas = String::new();
        for (i, s) in salas.iter().enumerate() {
            let (x, y, w, h) = sitio[i];
            for d in &s.lleva_a {
                let Some(j) = salas.iter().position(|o| &o.nombre == d) else {
                    continue;
                };
                let (xj, yj, wj, _) = sitio[j];
                let (ax, ay, bx, by) = (x + w / 2.0, y + h, xj + wj / 2.0, yj);
                flechas.push_str(&format!(
                    r#"<line x1="{ax}" y1="{ay}" x2="{bx}" y2="{b2}" stroke="{t}" stroke-opacity="0.85" stroke-width="0.32"/><path d="M{l},{p} L{bx},{by} L{r},{p} Z" fill="{t}" fill-opacity="0.9"/>"#,
                    b2 = by - 0.5,
                    l = bx - 0.55,
                    r = bx + 0.55,
                    p = by - 0.75,
                ));
            }
        }
        self.forma(&flechas);
        let mut cajas = String::new();
        for &(x, y, w, h) in &sitio {
            cajas.push_str(&format!(
                r#"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="0.8" fill="{t}" fill-opacity="0.10" stroke="{t}" stroke-opacity="0.65" stroke-width="0.22"/>"#
            ));
        }
        self.forma(&cajas);
        let centro = Estilo {
            alinear: texto::Alinear::Centro,
            ..est.clone()
        };
        let pad = 0.5;
        for (i, s) in salas.iter().enumerate() {
            let (x, y, w, h) = sitio[i];
            let (px, py, pw) = (
                self.l.x(x + pad),
                self.l.y(y + pad * 0.6),
                self.l.x(w - 2.0 * pad),
            );
            let nombre = vec![Trozo::Texto {
                texto: s.nombre.clone(),
                cursiva: false,
            }];
            let b =
                self.motor
                    .m
                    .maquetar(&mut self.motor.letras, &nombre, &negrita(&centro), Some(pw));
            let alto_nombre = b.alto;
            self.faltan.extend(pintar_bloque(
                &mut self.l.px.as_mut(),
                &b,
                px,
                py,
                Pincel::default(),
                self.rec.simbolos,
            ));
            let reglas = texto::reglas(&s.texto);
            let disponible = self.l.y(h - pad * 1.2) - alto_nombre;
            let a = ajustar_reglas(
                &mut self.motor.m,
                &mut self.motor.letras,
                &reglas,
                &[],
                &centro,
                pw,
                disponible,
                centro.tam * 0.6,
                self.l.x(0.1 * super::MM_POR_PT),
            );
            if a.sobra > 0.0 {
                let aviso = super::modelo::Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(a.sobra / self.l.ky),
                };
                if !self.avisos.contains(&aviso) {
                    self.avisos.push(aviso);
                }
            }
            let libre = ((disponible - a.caja.alto) / 2.0).max(0.0);
            let simbolos = self.rec.simbolos;
            self.faltan.extend(pintar_caja(
                &mut self.l.px.as_mut(),
                &a.caja,
                px,
                py + alto_nombre + libre,
                pw,
                tinta,
                simbolos,
            ));
        }
    }
}
