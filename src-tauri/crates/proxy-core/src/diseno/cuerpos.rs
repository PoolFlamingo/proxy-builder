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
    /// Aventura (o presagio): el libro a la izquierda y la criatura a la
    /// derecha. Preparar: al revés, el hechizo a la derecha.
    Aventura {
        cabecera: Parrafo,
        tipo: Parrafo,
        /// El texto del libro (la otra cara).
        izq: Vec<Parrafo>,
        /// El texto de la carta.
        der: Vec<Parrafo>,
        sabor: Vec<Parrafo>,
        libro_derecha: bool,
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
        /// El ancho del libro y dónde empieza.
        ancho_izq: f32,
        x_libro: f32,
        /// Dónde empieza el texto de la carta y su ancho.
        x_der: f32,
        ancho_der: f32,
        libro_derecha: bool,
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
            Hecho::Aventura {
                x_der,
                der,
                libro_derecha: false,
                ..
            } => x_der + ultima(der),
            // El libro a la derecha llega hasta el borde: la última línea
            // que cuenta es la más a la derecha de las dos columnas.
            Hecho::Aventura {
                x_der,
                der,
                x_libro,
                izq,
                ..
            } => (x_der + ultima(der)).max(x_libro + ultima(izq)),
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
            libro_derecha,
        } => {
            let pad = 0.9 * k;
            let ancho_izq = ancho * 0.48;
            let hueco = 1.2 * k;
            let (x_libro, x_der) = if *libro_derecha {
                (ancho - ancho_izq, 0.0)
            } else {
                (0.0, ancho_izq + hueco)
            };
            let ancho_der = ancho - ancho_izq - hueco;
            let w_izq = ancho_izq - 2.0 * pad;
            let nombre = m.maquetar(letras, cabecera, &negrita(&e), Some(w_izq));
            let tipo_b = m.maquetar(letras, tipo, &e.con_tam(tam * 0.92), Some(w_izq));
            let izq_c = caja_de_reglas(m, letras, izq, &[], &e, w_izq, sep);
            let der_c = caja_de_reglas(m, letras, der, sabor, &e, ancho_der, sep);
            let alto_izq = 2.0 * pad + nombre.alto + tipo_b.alto + 0.25 * tam + izq_c.alto;
            Hecho::Aventura {
                ancho_izq,
                x_libro,
                x_der,
                ancho_der,
                libro_derecha: *libro_derecha,
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
                super::textos_motor::palabra_nivel(lang).to_string()
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

/// El contenido de una aventura: la cara 1 en el libro y la 0 a la derecha
/// (con `libro_derecha`, como las de Preparar: la 0 a la izquierda).
pub fn aventura(
    libro_derecha: bool,
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
        libro_derecha,
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
                    x_libro,
                    x_der,
                    ancho_der,
                    libro_derecha,
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
                // A la izquierda, el libro llena la caja; a la derecha (Preparar)
                // se queda por encima de la F/R, que va en esa esquina.
                let alto_libro = if *libro_derecha {
                    alto_izq.max(ph - 5.2 * self.l.ky)
                } else {
                    alto_izq.max(ph)
                };
                // El libro: una página clara con borde.
                let svg = format!(
                    r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="0.6" fill="{fondo}" fill-opacity="0.16" stroke="{borde}" stroke-opacity="0.7" stroke-width="0.22"/>"##,
                    x = x + x_libro / self.l.kx,
                    y = y,
                    w = ancho_izq / self.l.kx,
                    h = alto_libro / self.l.ky,
                    fondo = super::render::hex(tinta),
                    borde = super::render::hex(tinta),
                );
                self.forma(&svg);
                let mut yy = py + pad;
                let bx = px + x_libro;
                for b in [nombre, tipo] {
                    self.faltan.extend(pintar_bloque(
                        &mut self.l.px.as_mut(),
                        b,
                        bx + pad,
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
                    bx + pad,
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
                    ancho_der.min(pw - x_der),
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
        // La piel del pack, si la trae; si no, negro con el borde del color.
        let piel = self.rec.pack.def.insignias.clone().unwrap_or_default();
        let fondo = piel.fondo.clone().unwrap_or_else(|| "#151515".into());
        let medio = piel.borde.clone().unwrap_or(medio);
        let tinta = piel
            .texto
            .as_deref()
            .and_then(super::pack::rgba)
            .unwrap_or([255, 255, 255, 255]);
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
                match &piel.png_lealtad {
                    Some(ruta) => {
                        let escalado = self.rec.pack.def.escalado;
                        self.png_del_pack(
                            ruta,
                            super::pack::Zona { x: x0, y: y0, w, h },
                            escalado,
                            super::pack::Mezcla::Normal,
                            1.0,
                        );
                    }
                    None => self.forma(&format!(
                        r##"<path d="{d}" fill="{fondo}" stroke="{medio}" stroke-width="0.32" stroke-linejoin="round"/>"##
                    )),
                }
                self.texto_centrado(coste, "fr", 1.75, tinta, cx, cy, w - 0.6);
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
                        r##"<path d="{d}" fill="{fondo}" stroke="{medio}" stroke-width="0.3" stroke-linejoin="round"/>"##
                    ));
                    self.texto_centrado(&a_romano(*n), "fr", 1.3, tinta, cx, ym, w - 0.8);
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
                    r##"<path d="{d}" fill="{fondo}" stroke="{medio}" stroke-width="0.3" stroke-linejoin="round"/>"##
                ));
                if etiqueta.is_empty() {
                    self.texto_centrado(valor, "fr", 1.6, tinta, cx - 0.5, cy, w - 1.6);
                } else {
                    self.texto_centrado(etiqueta, "fr", 0.8, tinta, cx - 0.5, cy - 1.05, w - 1.6);
                    self.texto_centrado(valor, "fr", 1.45, tinta, cx - 0.5, cy + 0.75, w - 1.6);
                }
            }
            Insignia::Prototipo { coste, fr } => {
                let (w, h) = (9.0, 7.2);
                let (x0, y0) = (cx - w / 2.0, cy - h / 2.0);
                self.forma(&format!(
                    r##"<rect x="{x0}" y="{y0}" width="{w}" height="{h}" rx="0.8" fill="{fondo}" stroke="{medio}" stroke-width="0.3"/>"##
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

/// Grosor de las paredes del plano de una mazmorra (mm).
const PARED: f32 = 1.05;
/// Ancho de una puerta (mm).
const PUERTA: f32 = 3.0;
/// Margen del texto dentro de una sala, aparte de la pared (mm).
const MARGEN_SALA: f32 = 0.7;
/// Lado de las losas del suelo (mm).
const LOSA: f32 = 2.2;

/// El texto de una sala maquetado a un tamaño.
struct TextoSala {
    nombre: Bloque,
    reglas: CajaTexto,
    /// Nombre y texto en un solo renglón, uno al lado del otro.
    en_linea: Option<Bloque>,
}

impl TextoSala {
    /// Alto que necesita (px).
    fn alto(&self, hueco: f32) -> f32 {
        match &self.en_linea {
            Some(t) => self.nombre.alto.max(t.alto),
            None => self.nombre.alto + hueco + self.reglas.alto,
        }
    }
}

impl Carta<'_> {
    /// El plano de una mazmorra dentro de la caja (mm), como en las cartas
    /// impresas: salas pegadas unas a otras, paredes del color del marco,
    /// suelo de losas y una puerta (con su flecha) en cada pared que se
    /// cruza; la entrada y la salida quedan abiertas en la pared de fuera.
    /// Todas las salas llevan el mismo tamaño de letra: el mayor con el que
    /// caben (cada fila mide lo que piden sus salas y se reparte lo que sobra).
    /// Si no caben, las salas con poco texto van en un renglón («Dungeon
    /// Level  Scry 1.», como en la carta) antes de bajar la letra.
    pub(super) fn pintar_mazmorra(
        &mut self,
        nota: &[String],
        salas: &[estructura::Sala],
        caja: super::render::Caja,
        est: &Estilo,
        tinta: Color,
    ) {
        let plano = estructura::plano_de_mazmorra(salas);
        if plano.filas == 0 {
            return;
        }
        let (kx, ky) = (self.l.kx, self.l.ky);
        let centro = Estilo {
            alinear: texto::Alinear::Centro,
            ..est.clone()
        };
        // La nota (Undercity) va encima del plano, en cursiva.
        let mut mapa = caja;
        if !nota.is_empty() {
            let parrafos: Vec<Parrafo> = nota
                .iter()
                .map(|l| {
                    vec![Trozo::Texto {
                        texto: l.clone(),
                        cursiva: true,
                    }]
                })
                .collect();
            let c = caja_de_reglas(
                &mut self.motor.m,
                &mut self.motor.letras,
                &parrafos,
                &[],
                &centro.con_tam(centro.tam * 0.9),
                (caja.w - 2.0 * MARGEN_SALA) * kx,
                Separaciones::default(),
            );
            let alto = c.alto / ky + 0.8;
            self.faltan.extend(pintar_caja(
                &mut self.l.px.as_mut(),
                &c,
                (caja.x + MARGEN_SALA) * kx,
                (caja.y + 0.3) * ky,
                (caja.w - 2.0 * MARGEN_SALA) * kx,
                tinta,
                self.rec.simbolos,
            ));
            mapa.y += alto;
            mapa.h -= alto;
        }
        let n = salas.len();
        let sitios = &plano.sitios;
        // Lo que se come la pared de cada lado: entera la de fuera, la mitad
        // la que se comparte.
        let lado = |exterior: bool| if exterior { PARED } else { PARED / 2.0 } + MARGEN_SALA;
        let ancho_util: Vec<f32> = sitios
            .iter()
            .map(|s| (mapa.w * (s.x1 - s.x0) - lado(s.x0 <= 0.0) - lado(s.x1 >= 1.0)).max(2.0) * kx)
            .collect();
        let reglas: Vec<Vec<Parrafo>> = salas.iter().map(|s| texto::reglas(&s.texto)).collect();
        let maquetar = |c: &mut Carta<'_>, tam: f32, en_linea: bool| -> Vec<TextoSala> {
            let nombre_est = negrita(&centro).con_tam(tam * 1.12);
            let reglas_est = centro.con_tam(tam);
            (0..n)
                .map(|i| {
                    let nombre_p = vec![Trozo::Texto {
                        texto: salas[i].nombre.clone(),
                        cursiva: false,
                    }];
                    let nombre = c.motor.m.maquetar(
                        &mut c.motor.letras,
                        &nombre_p,
                        &nombre_est,
                        Some(ancho_util[i]),
                    );
                    let caja = caja_de_reglas(
                        &mut c.motor.m,
                        &mut c.motor.letras,
                        &reglas[i],
                        &[],
                        &reglas_est,
                        ancho_util[i],
                        Separaciones::default(),
                    );
                    let en_linea = (en_linea && reglas[i].len() == 1)
                        .then(|| {
                            let inicio = Estilo {
                                alinear: texto::Alinear::Inicio,
                                ..reglas_est.clone()
                            };
                            let solo = Estilo {
                                alinear: texto::Alinear::Inicio,
                                ..nombre_est.clone()
                            };
                            let a = c
                                .motor
                                .m
                                .maquetar(&mut c.motor.letras, &nombre_p, &solo, None);
                            let b = c.motor.m.maquetar(
                                &mut c.motor.letras,
                                &reglas[i][0],
                                &inicio,
                                None,
                            );
                            (a.ancho + tam * 0.9 + b.ancho <= ancho_util[i]).then_some((a, b))
                        })
                        .flatten();
                    match en_linea {
                        Some((a, b)) => TextoSala {
                            nombre: a,
                            reglas: caja,
                            en_linea: Some(b),
                        },
                        None => TextoSala {
                            nombre,
                            reglas: caja,
                            en_linea: None,
                        },
                    }
                })
                .collect()
        };
        // El alto de cada fila (mm) para unos textos.
        let altos = |textos: &[TextoSala], tam: f32| -> Vec<f32> {
            let mut h = vec![0.0f32; plano.filas];
            let pide = |i: usize| {
                let s = sitios[i];
                textos[i].alto(tam * 0.25) / ky
                    + lado(s.fila0 == 0)
                    + lado(s.fila1 + 1 == plano.filas)
            };
            for (i, s) in sitios.iter().enumerate() {
                if s.fila0 == s.fila1 {
                    h[s.fila0] = h[s.fila0].max(pide(i));
                }
            }
            let minimo = tam * 1.4 / ky + 2.0 * lado(false);
            for v in &mut h {
                *v = v.max(minimo);
            }
            // Las salas de varias filas: si no les basta, se reparte lo que
            // falta entre sus filas.
            for (i, s) in sitios.iter().enumerate() {
                if s.fila0 == s.fila1 {
                    continue;
                }
                let tiene: f32 = h[s.fila0..=s.fila1].iter().sum();
                let falta = pide(i) - tiene;
                if falta > 0.0 {
                    let k = (s.fila1 - s.fila0 + 1) as f32;
                    for v in &mut h[s.fila0..=s.fila1] {
                        *v += falta / k;
                    }
                }
            }
            h
        };
        let cabe = |h: &[f32]| h.iter().sum::<f32>() <= mapa.h;
        let tam0 = centro.tam;
        let tam_min = (tam0 * 0.55).max(4.0);
        let mut elegido = None;
        for en_linea in [false, true] {
            let t = maquetar(self, tam0, en_linea);
            let h = altos(&t, tam0);
            if cabe(&h) {
                elegido = Some((t, h, tam0));
                break;
            }
        }
        if elegido.is_none() {
            // Búsqueda binaria sobre el tamaño, de 0,1 pt en 0,1 pt.
            let paso = 0.1 * super::MM_POR_PT * kx;
            let pasos = ((tam0 - tam_min) / paso).floor().max(1.0) as u32;
            let (mut lo, mut hi) = (1u32, pasos);
            while lo <= hi {
                let mid = (lo + hi) / 2;
                let tam = tam0 - mid as f32 * paso;
                let t = maquetar(self, tam, true);
                let h = altos(&t, tam);
                if cabe(&h) {
                    elegido = Some((t, h, tam));
                    hi = mid - 1;
                } else {
                    lo = mid + 1;
                }
            }
        }
        let (textos, mut alto_fila, tam) = match elegido {
            Some(e) => e,
            None => {
                let t = maquetar(self, tam_min, true);
                let h = altos(&t, tam_min);
                let aviso = super::modelo::Aviso::TextoNoCabe {
                    hueco: "reglas".into(),
                    sobra: f64::from(h.iter().sum::<f32>() - mapa.h),
                };
                if !self.avisos.contains(&aviso) {
                    self.avisos.push(aviso);
                }
                (t, h, tam_min)
            }
        };
        // Lo que sobra, repartido en proporción: las filas de un renglón
        // siguen siendo pasillos y las salas grandes, grandes.
        let total: f32 = alto_fila.iter().sum();
        if total > 0.0 {
            let k = mapa.h / total;
            for v in &mut alto_fila {
                *v *= k;
            }
        }
        let mut ys = vec![mapa.y];
        for h in &alto_fila {
            ys.push(ys[ys.len() - 1] + h);
        }
        let x_de = |f: f32| mapa.x + f * mapa.w;

        // ── Suelo, roca, paredes y puertas.
        let t = super::render::hex(tinta);
        let suelo_claro = super::render::luminancia(tinta) < 0.5;
        let pared = self
            .vars
            .get(if suelo_claro { "oscuro" } else { "medio" })
            .cloned()
            .unwrap_or_else(|| "#2c2c2c".into());
        let mut svg = format!(
            r#"<pattern id="losas" patternUnits="userSpaceOnUse" x="{x}" y="{y}" width="{LOSA}" height="{LOSA}"><path d="M{LOSA},0 V{LOSA} H0" fill="none" stroke="{t}" stroke-opacity="0.10" stroke-width="0.07"/></pattern><rect x="{x}" y="{y}" width="{w}" height="{h}" fill="url(#losas)"/>"#,
            x = mapa.x,
            y = mapa.y,
            w = mapa.w,
            h = mapa.h,
        );
        let mut muro = |x0: f32, y0: f32, x1: f32, y1: f32| {
            if x1 - x0 > 0.01 && y1 - y0 > 0.01 {
                svg.push_str(&format!(
                    r##"<rect x="{x0}" y="{y0}" width="{w}" height="{h}" fill="{pared}" stroke="#000" stroke-opacity="0.45" stroke-width="0.1"/>"##,
                    w = x1 - x0,
                    h = y1 - y0,
                ));
            }
        };
        // Roca: lo que ninguna sala ocupa.
        for r in 0..plano.filas {
            let ocupado: Vec<(f32, f32)> = sitios
                .iter()
                .filter(|s| s.fila0 <= r && s.fila1 >= r)
                .map(|s| (s.x0, s.x1))
                .collect();
            for (a, b) in estructura::tramos_libres(&ocupado) {
                muro(x_de(a), ys[r], x_de(b), ys[r + 1]);
            }
        }
        let e = PARED / 2.0;
        // Paredes entre filas, sin las salas que siguen abajo ni las puertas.
        for r in 0..plano.filas.saturating_sub(1) {
            let mut abierto: Vec<(f32, f32)> = sitios
                .iter()
                .filter(|s| s.fila0 <= r && s.fila1 > r)
                .map(|s| (s.x0, s.x1))
                .collect();
            for p in plano.puertas.iter().filter(|p| p.fila == r) {
                let m = PUERTA / 2.0 / mapa.w;
                abierto.push((p.x - m, p.x + m));
            }
            for (a, b) in estructura::tramos_libres(&abierto) {
                let (xa, xb) = (
                    (x_de(a) - e).max(mapa.x),
                    (x_de(b) + e).min(mapa.x + mapa.w),
                );
                muro(xa, ys[r + 1] - e, xb, ys[r + 1] + e);
            }
        }
        // Paredes entre salas de la misma fila.
        for s in sitios {
            for x in [s.x0, s.x1] {
                if x > 0.001 && x < 0.999 {
                    muro(x_de(x) - e, ys[s.fila0], x_de(x) + e, ys[s.fila1 + 1]);
                }
            }
        }
        // La pared de fuera, abierta en la entrada y en la salida.
        let (x0, y0, x1, y1) = (mapa.x, mapa.y, mapa.x + mapa.w, mapa.y + mapa.h);
        let con_puertas = !plano.puertas.is_empty();
        let abiertas = |fila: usize, ultima: bool| -> Vec<(f32, f32)> {
            if !con_puertas {
                return Vec::new();
            }
            sitios
                .iter()
                .enumerate()
                .filter(|(i, s)| {
                    if ultima {
                        s.fila1 == fila && salas[*i].lleva_a.is_empty()
                    } else {
                        s.fila0 == fila && *i == 0
                    }
                })
                .map(|(_, s)| {
                    let c = (s.x0 + s.x1) / 2.0;
                    let m = PUERTA / 2.0 / mapa.w;
                    (c - m, c + m)
                })
                .collect()
        };
        for (a, b) in estructura::tramos_libres(&abiertas(0, false)) {
            muro(x_de(a), y0, x_de(b), y0 + PARED);
        }
        for (a, b) in estructura::tramos_libres(&abiertas(plano.filas - 1, true)) {
            muro(x_de(a), y1 - PARED, x_de(b), y1);
        }
        muro(x0, y0, x0 + PARED, y1);
        muro(x1 - PARED, y0, x1, y1);
        // Las puertas: una flecha hacia la sala de abajo.
        for p in &plano.puertas {
            let (cx, cy) = (x_de(p.x), ys[p.fila + 1]);
            svg.push_str(&format!(
                r##"<path d="M{a},{b} L{c},{b} L{cx},{d} Z" fill="#f4f4f4" stroke="#000" stroke-opacity="0.8" stroke-width="0.14" stroke-linejoin="round"/>"##,
                a = cx - 0.95,
                b = cy - 0.5,
                c = cx + 0.95,
                d = cy + 0.65,
            ));
        }
        self.forma(&svg);

        // ── Los textos, centrados en cada sala.
        for (i, s) in sitios.iter().enumerate() {
            let ix = x_de(s.x0) + lado(s.x0 <= 0.0);
            let iy = ys[s.fila0] + lado(s.fila0 == 0);
            let iw = ancho_util[i] / kx;
            let ih = ys[s.fila1 + 1] - lado(s.fila1 + 1 == plano.filas) - iy;
            let tx = &textos[i];
            let alto = tx.alto(tam * 0.25) / ky;
            let y = iy + ((ih - alto) / 2.0).max(0.0);
            match &tx.en_linea {
                Some(reglas) => {
                    let ancho = (tx.nombre.ancho + tam * 0.9 + reglas.ancho) / kx;
                    let x = ix + (iw - ancho) / 2.0;
                    // Las dos líneas base a la misma altura.
                    let base = tx.nombre.base.max(reglas.base);
                    let arriba = y * ky + (alto * ky - tx.nombre.alto.max(reglas.alto)) / 2.0;
                    self.faltan.extend(pintar_bloque(
                        &mut self.l.px.as_mut(),
                        &tx.nombre,
                        x * kx,
                        arriba + base - tx.nombre.base,
                        Pincel::default(),
                        self.rec.simbolos,
                    ));
                    self.faltan.extend(pintar_bloque(
                        &mut self.l.px.as_mut(),
                        reglas,
                        x * kx + tx.nombre.ancho + tam * 0.9,
                        arriba + base - reglas.base,
                        Pincel::default(),
                        self.rec.simbolos,
                    ));
                }
                None => {
                    self.faltan.extend(pintar_bloque(
                        &mut self.l.px.as_mut(),
                        &tx.nombre,
                        ix * kx,
                        y * ky,
                        Pincel::default(),
                        self.rec.simbolos,
                    ));
                    self.faltan.extend(pintar_caja(
                        &mut self.l.px.as_mut(),
                        &tx.reglas,
                        ix * kx,
                        y * ky + tx.nombre.alto + tam * 0.25,
                        ancho_util[i],
                        tinta,
                        self.rec.simbolos,
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::render::Motor;

    /// Píxeles por milímetro de las pruebas.
    const K: f32 = 10.0;
    const ANCHO: f32 = 500.0;

    fn estilo() -> Estilo {
        Estilo {
            familia: "Spectral".into(),
            peso: 400.0,
            tam: 26.0,
            interlinea: 1.06,
            color: [0, 0, 0, 255],
            alinear: texto::Alinear::Inicio,
            versalitas: false,
            resalte: None,
            habilidad: None,
        }
    }

    fn hacer(c: &Contenido) -> Hecho {
        let mut m = Motor::de_fabrica().unwrap();
        let e = estilo();
        maquetar(
            c,
            &mut m.m,
            &mut m.letras,
            &e,
            ANCHO,
            e.tam,
            Separaciones::default(),
            K,
        )
    }

    fn ajustado(c: &Contenido, alto_max: f32, tam_min: f32) -> (Hecho, f32) {
        let mut m = Motor::de_fabrica().unwrap();
        ajustar(
            c,
            &mut m.m,
            &mut m.letras,
            &estilo(),
            ANCHO,
            alto_max,
            tam_min,
            0.5,
            K,
        )
    }

    fn seccion(izq: Option<Insignia>, texto: &str, fr: Option<&str>) -> Seccion {
        Seccion {
            izq,
            parrafos: texto::reglas(texto),
            fr: fr.map(Into::into),
            ..Seccion::default()
        }
    }

    fn secciones(s: Vec<Seccion>, sabor: &str) -> Contenido {
        Contenido::Secciones {
            secciones: s,
            sabor: texto::sabor(sabor),
            repartir: false,
            rayas: true,
        }
    }

    fn lealtad() -> Contenido {
        secciones(
            vec![
                seccion(
                    Some(Insignia::Lealtad("+1".into())),
                    "Roba una carta.",
                    None,
                ),
                seccion(
                    Some(Insignia::Lealtad("−3".into())),
                    "Destruye la criatura objetivo. Su controlador gana 3 vidas y roba una carta por cada criatura que controle.",
                    None,
                ),
            ],
            "",
        )
    }

    fn items(h: &Hecho) -> &[SeccionHecha] {
        match h {
            Hecho::Secciones { items, .. } => items,
            _ => panic!("se esperaban secciones"),
        }
    }

    // ── Insignias ──

    #[test]
    fn las_insignias_tienen_su_columna_y_su_alto_minimo() {
        assert_eq!(Insignia::Lealtad("+1".into()).ancho(), 6.4);
        assert_eq!(
            Insignia::Prototipo {
                coste: "{1}".into(),
                fr: "1/1".into()
            }
            .ancho(),
            9.6
        );
        // Un capítulo doble necesita sitio para las dos marcas.
        let uno = Insignia::Capitulos(vec![1]).alto_min();
        let dos = Insignia::Capitulos(vec![1, 2]).alto_min();
        assert!((uno - 4.3).abs() < 1e-4);
        assert!((dos - 8.2).abs() < 1e-4);
    }

    // ── Maquetar ──

    #[test]
    fn las_secciones_se_apilan_con_su_insignia_a_la_izquierda() {
        let h = hacer(&lealtad());
        let it = items(&h);
        assert_eq!(it.len(), 2);
        let izq = 6.4 * K + 0.8 * K;
        for s in it {
            assert!((s.x_txt - izq).abs() < 1e-3);
            assert!((s.ancho_txt - (ANCHO - izq)).abs() < 1e-3);
            assert!(s.alto >= 4.6 * K, "alto mínimo de la insignia");
            assert!(s.alto >= s.alto_txt);
            assert!(!s.con_fr);
        }
        assert_eq!(it[0].y, 0.0);
        assert!((it[1].y - it[0].alto).abs() < 1e-3);
        // El texto largo ocupa más que el corto.
        assert!(it[1].alto_txt > it[0].alto_txt);
        assert!((h.alto() - (it[0].alto + it[1].alto)).abs() < 1e-3);
        assert_eq!(h.tam(), estilo().tam);
    }

    #[test]
    fn una_seccion_sin_insignia_usa_todo_el_ancho() {
        let h = hacer(&secciones(vec![seccion(None, "Vuela.", None)], ""));
        let s = &items(&h)[0];
        assert_eq!(s.x_txt, 0.0);
        assert_eq!(s.ancho_txt, ANCHO);
    }

    #[test]
    fn una_franja_con_fuerza_resistencia_deja_su_hueco_a_la_derecha() {
        let h = hacer(&secciones(
            vec![
                seccion(None, "Subir de nivel {W}.", None),
                seccion(
                    Some(Insignia::Nivel {
                        etiqueta: "NIVEL".into(),
                        valor: "2-6".into(),
                    }),
                    "Daña primero.",
                    Some("3/3"),
                ),
            ],
            "",
        ));
        let s = &items(&h)[1];
        assert!(s.con_fr);
        let esperado = ANCHO - (6.4 * K + 0.8 * K) - (7.0 * K + 0.8 * K);
        assert!((s.ancho_txt - esperado).abs() < 1e-3);
        assert!(s.alto >= 5.2 * K);
        // La última franja lleva F/R: el último renglón «llega» a la derecha.
        assert_eq!(h.fin_ultima_linea(), f32::MAX);
    }

    #[test]
    fn con_sabor_el_ultimo_renglon_es_el_del_sabor() {
        let h = hacer(&secciones(
            vec![seccion(None, "Daña primero.", Some("3/3"))],
            "«Corto.»",
        ));
        let fin = h.fin_ultima_linea();
        assert!(fin > 0.0 && fin < ANCHO / 2.0, "{fin}");
        match &h {
            Hecho::Secciones { sabor, alto, .. } => {
                let (c, y) = sabor.as_ref().unwrap();
                assert!((y + c.alto - alto).abs() < 1e-3, "el sabor va al final");
                assert!(*y > items(&h)[0].alto, "con su separación");
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn el_fin_del_ultimo_renglon_en_texto_y_secciones() {
        let corto = Contenido::Texto {
            reglas: texto::reglas("Vuela."),
            sabor: Vec::new(),
        };
        let largo = Contenido::Texto {
            reglas: texto::reglas("Vuela, prisa, vínculo vital."),
            sabor: Vec::new(),
        };
        let (a, b) = (
            hacer(&corto).fin_ultima_linea(),
            hacer(&largo).fin_ultima_linea(),
        );
        assert!(a > 0.0 && a < b && b <= ANCHO);
        // Sin texto, no llega a ningún sitio.
        let vacio = Contenido::Texto {
            reglas: Vec::new(),
            sabor: Vec::new(),
        };
        assert_eq!(hacer(&vacio).fin_ultima_linea(), 0.0);
        // En secciones sin F/R se cuenta desde la columna de la insignia.
        let h = hacer(&lealtad());
        let ultima = &items(&h)[1];
        assert!(h.fin_ultima_linea() > ultima.x_txt);
        assert!(h.fin_ultima_linea() <= ANCHO);
    }

    #[test]
    fn la_aventura_va_en_dos_columnas() {
        let c = aventura(
            false,
            "Robo en la torre",
            "{1}{U}",
            "Conjuro — Aventura",
            "Roba dos cartas.",
            "Vuela.",
            Vec::new(),
        );
        let h = hacer(&c);
        match &h {
            Hecho::Aventura {
                ancho_izq,
                x_der,
                alto,
                alto_izq,
                der,
                ..
            } => {
                assert!((ancho_izq - ANCHO * 0.48).abs() < 1e-3);
                assert!((x_der - (ancho_izq + 1.2 * K)).abs() < 1e-3);
                assert_eq!(*alto, alto_izq.max(der.alto));
                // El último renglón es el de la columna derecha.
                assert!(h.fin_ultima_linea() > *x_der);
            }
            _ => panic!("se esperaba aventura"),
        }
    }

    #[test]
    fn preparar_pone_el_hechizo_a_la_derecha() {
        let c = aventura(
            true,
            "Relámpago",
            "{R}",
            "Instantáneo",
            "Hace 3 puntos de daño a cualquier objetivo.",
            "Daña primero.",
            Vec::new(),
        );
        match hacer(&c) {
            Hecho::Aventura {
                ancho_izq,
                x_libro,
                x_der,
                ancho_der,
                ..
            } => {
                assert_eq!(x_der, 0.0);
                assert!((x_libro + ancho_izq - ANCHO).abs() < 1e-3);
                // Las dos columnas no se pisan.
                assert!(x_der + ancho_der <= x_libro + 1e-3);
            }
            _ => panic!("se esperaba aventura"),
        }
    }

    #[test]
    fn la_cabecera_de_la_aventura_lleva_nombre_y_coste() {
        let Contenido::Aventura { cabecera, tipo, .. } =
            aventura(false, "Robo", "{1}{U}", "Conjuro", "", "", Vec::new())
        else {
            panic!("se esperaba aventura");
        };
        assert_eq!(
            cabecera,
            vec![
                Trozo::Texto {
                    texto: "Robo  ".into(),
                    cursiva: false
                },
                Trozo::Simbolo("1".into()),
                Trozo::Simbolo("U".into()),
            ]
        );
        assert!(matches!(&tipo[0], Trozo::Texto { cursiva: true, .. }));
    }

    // ── Ajustar ──

    #[test]
    fn lo_que_cabe_no_se_toca() {
        let (h, sobra) = ajustado(&lealtad(), 10_000.0, 12.0);
        assert_eq!(sobra, 0.0);
        assert_eq!(h.tam(), estilo().tam);
    }

    #[test]
    fn lo_que_no_cabe_baja_la_letra_lo_justo() {
        let natural = hacer(&lealtad()).alto();
        let alto_max = natural * 0.85;
        let (h, sobra) = ajustado(&lealtad(), alto_max, 8.0);
        assert_eq!(sobra, 0.0);
        assert!(h.alto() <= alto_max);
        assert!(h.tam() < estilo().tam && h.tam() >= 8.0);
        // Medio punto más ya no cabría (se busca el mayor tamaño que cabe).
        let mut m = Motor::de_fabrica().unwrap();
        let mayor = maquetar(
            &lealtad(),
            &mut m.m,
            &mut m.letras,
            &estilo(),
            ANCHO,
            h.tam() + 0.5,
            Separaciones::default(),
            K,
        );
        assert!(mayor.alto() > alto_max);
    }

    #[test]
    fn lo_que_no_cabe_ni_al_minimo_dice_cuanto_sobra() {
        let (h, sobra) = ajustado(&lealtad(), 5.0, 14.0);
        assert!((h.tam() - 14.0).abs() < 1e-4);
        assert!(sobra > 0.0);
        assert!((sobra - (h.alto() - 5.0)).abs() < 1e-3);
    }

    #[test]
    fn el_texto_normal_se_ajusta_igual() {
        let c = Contenido::Texto {
            reglas: texto::reglas(&"Roba una carta y luego descarta una carta. ".repeat(6)),
            sabor: texto::sabor("«Saber es poder.»"),
        };
        let natural = hacer(&c).alto();
        let (h, sobra) = ajustado(&c, natural * 0.8, 8.0);
        assert_eq!(sobra, 0.0);
        assert!(h.tam() < estilo().tam);
        assert!(matches!(h, Hecho::Texto(_)));
        let (_, sobra) = ajustado(&c, 3.0, 20.0);
        assert!(sobra > 0.0);
    }

    // ── Del texto de la carta al contenido ──

    fn tipo_de(c: &Contenido) -> &'static str {
        match c {
            Contenido::Texto { .. } => "texto",
            Contenido::Secciones { .. } => "secciones",
            Contenido::Aventura { .. } => "aventura",
        }
    }

    #[test]
    fn un_planeswalker_va_por_habilidades_con_bandas_alternas() {
        let c = contenido(
            Cuerpo::Lealtad,
            "Las criaturas tienen +1/+0.\n+1: Roba una carta.\n−2: Destruye una criatura.",
            Vec::new(),
            DesignLang::Es,
        );
        let Contenido::Secciones {
            secciones,
            repartir,
            rayas,
            ..
        } = &c
        else {
            panic!("se esperaban secciones");
        };
        assert!(*repartir && !*rayas);
        assert_eq!(secciones.len(), 3);
        assert_eq!(secciones[0].izq, None, "la estática no lleva escudo");
        assert_eq!(secciones[1].izq, Some(Insignia::Lealtad("+1".into())));
        assert_eq!(
            secciones.iter().map(|s| s.banda).collect::<Vec<_>>(),
            [false, true, false]
        );
        // Sin ninguna habilidad de lealtad, texto normal.
        let c = contenido(Cuerpo::Lealtad, "Vuela.", Vec::new(), DesignLang::Es);
        assert_eq!(tipo_de(&c), "texto");
    }

    #[test]
    fn una_saga_va_por_capitulos_con_su_introduccion() {
        let c = contenido(
            Cuerpo::Capitulos,
            "(En cuanto esta Saga entre…)\nI, II — Roba una carta.\nIII — Gana 3 vidas.",
            Vec::new(),
            DesignLang::Es,
        );
        let Contenido::Secciones { secciones, .. } = &c else {
            panic!("se esperaban secciones");
        };
        assert_eq!(secciones.len(), 3);
        assert_eq!(secciones[0].izq, None);
        assert_eq!(secciones[1].izq, Some(Insignia::Capitulos(vec![1, 2])));
        assert_eq!(secciones[2].izq, Some(Insignia::Capitulos(vec![3])));
        let c = contenido(
            Cuerpo::Capitulos,
            "Sin capítulos.",
            Vec::new(),
            DesignLang::Es,
        );
        assert_eq!(tipo_de(&c), "texto");
    }

    #[test]
    fn el_leveler_pone_la_palabra_de_su_idioma() {
        let reglas =
            "Subir de nivel {W}.\nNIVEL 2-6\n3/3\nDaña primero.\nNIVEL 7+\n4/4\nDaña dos veces.";
        for (lang, palabra) in DesignLang::TODOS
            .iter()
            .map(|l| (*l, super::super::textos_motor::palabra_nivel(*l)))
        {
            let c = contenido(Cuerpo::Leveler, reglas, Vec::new(), lang);
            let Contenido::Secciones {
                secciones,
                repartir,
                ..
            } = &c
            else {
                panic!("se esperaban secciones");
            };
            assert!(!*repartir);
            assert_eq!(
                secciones[1].izq,
                Some(Insignia::Nivel {
                    etiqueta: palabra.into(),
                    valor: "2-6".into()
                })
            );
            assert_eq!(secciones[1].fr.as_deref(), Some("3/3"));
        }
        // La estación va sin palabra.
        let c = contenido(
            Cuerpo::Estacion,
            "When this enters…\nStation (Tap another creature…)\n8+ | Flying\n12+ | 5/5",
            Vec::new(),
            DesignLang::En,
        );
        let Contenido::Secciones { secciones, .. } = &c else {
            panic!("se esperaban secciones");
        };
        assert!(matches!(
            &secciones[1].izq,
            Some(Insignia::Nivel { etiqueta, valor }) if etiqueta.is_empty() && valor == "8+"
        ));
        // Los rangos de cualquier idioma se reconocen («等级2-6», «Lv 7+»).
        for reglas in [
            "升级{W}\n等级2-6\n3/3\n先攻\n等级7+\n4/4\n连击",
            "Lvアップ {W}\nLv 2-6\n3/3\n先制攻撃\nLv 7+\n4/4\n二段攻撃",
            "Stufe aufsteigen {W}\nSTUFE 2-6\n3/3\nErstschlag\nSTUFE 7+\n4/4\nDoppelschlag",
        ] {
            let c = contenido(Cuerpo::Leveler, reglas, Vec::new(), DesignLang::Fr);
            let Contenido::Secciones { secciones, .. } = &c else {
                panic!("se esperaban secciones en «{reglas}»");
            };
            assert_eq!(secciones.len(), 3, "{reglas}");
            assert_eq!(secciones[2].fr.as_deref(), Some("4/4"));
        }
    }

    #[test]
    fn el_prototipo_lleva_su_recuadro_y_el_resto_aparte() {
        let c = contenido(
            Cuerpo::Prototipo,
            "Prototype {1}{B}{B} — 3/3 (You may cast…)\nMenace, lifelink",
            Vec::new(),
            DesignLang::En,
        );
        let Contenido::Secciones { secciones, .. } = &c else {
            panic!("se esperaban secciones");
        };
        assert_eq!(secciones.len(), 2);
        assert_eq!(
            secciones[0].izq,
            Some(Insignia::Prototipo {
                coste: "{1}{B}{B}".into(),
                fr: "3/3".into()
            })
        );
        assert!(secciones[0].banda);
        let c = contenido(Cuerpo::Prototipo, "Flying", Vec::new(), DesignLang::En);
        assert_eq!(tipo_de(&c), "texto");
    }

    #[test]
    fn una_clase_con_un_solo_tramo_es_texto_normal() {
        let c = contenido(
            Cuerpo::Niveles,
            "Te cuesta {1} menos.",
            Vec::new(),
            DesignLang::Es,
        );
        assert_eq!(tipo_de(&c), "texto");
        let c = contenido(
            Cuerpo::Niveles,
            "Te cuesta {1} menos.\n{1}{U}: Nivel 2\nCuando…",
            Vec::new(),
            DesignLang::Es,
        );
        let Contenido::Secciones { secciones, .. } = &c else {
            panic!("se esperaban secciones");
        };
        assert!(secciones[1].cabecera.is_some());
    }

    #[test]
    fn el_texto_y_la_mazmorra_son_texto_normal_con_su_sabor() {
        for cu in [Cuerpo::Texto, Cuerpo::Mazmorra] {
            let c = contenido(
                cu,
                "Vuela.\nPrisa.",
                texto::sabor("«Hola.»"),
                DesignLang::Es,
            );
            let Contenido::Texto { reglas, sabor } = &c else {
                panic!("se esperaba texto");
            };
            assert_eq!(reglas.len(), 2);
            assert_eq!(sabor.len(), 1);
        }
    }
}
