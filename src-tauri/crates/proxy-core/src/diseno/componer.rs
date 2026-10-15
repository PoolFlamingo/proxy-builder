//! **Cartas compuestas** (Fase 5): las que no son una cara normal.
//!
//! - Partidas, Fusionar y Habitación: dos minicartas giradas 90°.
//! - Secuela: la mitad de arriba normal y la de abajo girada.
//! - Flip: dos medias cartas, la de abajo girada 180°, con el arte en medio.
//! - Batalla: la cara de delante apaisada y girada.
//! - Meld: el dorso es media carta fundida, girada.
//! - Cartas grandes: planos (apaisados), planes y Vanguard.
//!
//! Todas se montan con [`pintar_cara`] sobre lienzos de la medida que toca y
//! se pegan giradas: un solo motor para todo.

use image::RgbaImage;
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform};

use super::efectivo::DisenoEfectivo;
use super::maquetacion::{cuerpo, maquetacion, medida_grande, Cuerpo, Maquetacion};
use super::modelo::{Aviso, DesignLang};
use super::pack::EstiloDef;
use super::render::{
    de_pixmap, estilo_para, mascara_redondeada, pie_texto, pintar_cara, Indicador, Motor, Opciones,
    Pieza, Recursos, Render, Salida,
};
use super::DisenoError;
use crate::model::CardSize;

/// Cómo se gira una pieza al pegarla.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Giro {
    Nada,
    /// 90° a la izquierda: lo de arriba de la pieza queda a la izquierda.
    Izquierda,
    Media,
}

/// Pega `src` en el rectángulo `(x, y, w, h)` (px) de `dest`, girada y
/// encajada sin deformar, centrada.
fn pegar(dest: &mut Pixmap, src: &Pixmap, x: f32, y: f32, w: f32, h: f32, giro: Giro) {
    let (sw, sh) = (src.width() as f32, src.height() as f32);
    let (rw, rh) = match giro {
        Giro::Izquierda => (sh, sw),
        _ => (sw, sh),
    };
    let s = (w / rw).min(h / rh);
    let angulo = match giro {
        Giro::Nada => 0.0,
        Giro::Izquierda => -90.0,
        Giro::Media => 180.0,
    };
    let t = Transform::from_translate(x + w / 2.0, y + h / 2.0)
        .pre_rotate(angulo)
        .pre_scale(s, s)
        .pre_translate(-sw / 2.0, -sh / 2.0);
    dest.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &PixmapPaint {
            quality: if (s - 1.0).abs() < 1e-3 && giro != Giro::Nada {
                FilterQuality::Nearest
            } else {
                FilterQuality::Bicubic
            },
            ..PixmapPaint::default()
        },
        t,
        None,
    );
}

fn estilo(
    rec: &Recursos<'_>,
    e: &DisenoEfectivo,
    layout: Option<&str>,
    avisos: &mut Vec<Aviso>,
) -> Result<EstiloDef, DisenoError> {
    estilo_para(rec, e.design.style, layout, avisos)
}

/// El layout de una cara simple según su cuerpo.
fn layout_de_cara(
    e: &DisenoEfectivo,
    cara: usize,
    cuerpo: Cuerpo,
    maq: Maquetacion,
) -> Option<&'static str> {
    if cuerpo == Cuerpo::Capitulos {
        return Some("saga");
    }
    if cuerpo == Cuerpo::Mazmorra {
        return Some("mazmorra");
    }
    let _ = cara;
    let ficha = e.design.card.es_ficha()
        || matches!(maq, Maquetacion::Emblema | Maquetacion::FichaDosCaras);
    if ficha {
        return Some("ficha");
    }
    // Los cuerpos con secciones necesitan más caja.
    matches!(
        cuerpo,
        Cuerpo::Lealtad | Cuerpo::Niveles | Cuerpo::Caso | Cuerpo::Leveler | Cuerpo::Estacion
    )
    .then_some("alto")
}

/// Pinta una cara «simple» (con su cuerpo y, en las de doble cara, su
/// indicador) a la medida dada.
#[allow(clippy::too_many_arguments)]
fn cara_simple(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    maq: Maquetacion,
    cara: usize,
    ancho: u32,
    alto: u32,
    capa: bool,
) -> Result<Pieza, DisenoError> {
    let d = &e.design;
    let cu = cuerpo(&d.card, cara);
    let mut avisos = Vec::new();
    let est = estilo(rec, e, layout_de_cara(e, cara, cu, maq), &mut avisos)?;
    let mut o = Opciones::de(cara, cu, ancho, alto);
    // La capa: todo menos la ilustración, sobre transparente.
    o.transparente = capa;
    o.sin_arte = capa;
    match maq {
        Maquetacion::Transformable | Maquetacion::Batalla | Maquetacion::Meld => {
            o.indicador = Some(if cara == 0 {
                Indicador::Sol
            } else {
                Indicador::Luna
            });
        }
        Maquetacion::Mdfc => {
            o.indicador = Some(if cara == 0 {
                Indicador::Delante
            } else {
                Indicador::Detras
            });
            let otra = d
                .card
                .faces
                .get(1 - cara.min(1))
                .cloned()
                .unwrap_or_default();
            o.otra_cara = Some((otra.type_line.get(lang).clone(), otra.mana_cost));
        }
        Maquetacion::Aventura => o.aventura = Some(1),
        _ => {}
    }
    pintar_cara(motor, e, lang, rec, est, avisos, &o)
}

/// Dos minicartas giradas: partidas, Fusionar y Habitación. En vertical, la
/// primera mitad abajo y la segunda arriba (girando la carta a la derecha
/// se leen de izquierda a derecha).
fn partida(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    maq: Maquetacion,
    ancho: u32,
    alto: u32,
) -> Result<Pieza, DisenoError> {
    let mut lienzo = Pixmap::new(ancho, alto).ok_or(DisenoError::Lienzo)?;
    lienzo.fill(tiny_skia::Color::BLACK);
    let (aw, ah) = (ancho as f32, alto as f32);
    let k = aw / 63.0;
    // Fusionar: la franja común va a la derecha (abajo, en apaisado).
    let franja = if maq == Maquetacion::Fusionar {
        6.0 * k
    } else {
        0.0
    };
    let margen = 0.8 * k;
    let mitad_alto = (ah - 3.0 * margen) / 2.0;
    let region_w = aw - franja - 2.0 * margen;
    // La minicarta en vertical: su ancho es el alto de la mitad.
    let (pw, ph) = {
        let w = mitad_alto;
        let h = w * 88.0 / 63.0;
        if h <= region_w {
            (w, h)
        } else {
            (region_w * 63.0 / 88.0, region_w)
        }
    };
    let mut avisos = Vec::new();
    let mut faltan = Vec::new();
    // Fusionar: la línea de Fusionar va en la franja, no en cada mitad.
    let sin_fusionar = (maq == Maquetacion::Fusionar).then(|| {
        let mut e2 = e.clone();
        for (i, f) in e2.design.faces.iter_mut().enumerate() {
            for l in [DesignLang::Es, DesignLang::En] {
                let reglas = super::borrador::reglas_efectivas(&e.design, i, l);
                let limpio: Vec<&str> = reglas
                    .lines()
                    .filter(|x| !x.starts_with("Fuse") && !x.starts_with("Fusionar"))
                    .collect();
                let t = match l {
                    DesignLang::Es => &mut f.texts.es,
                    DesignLang::En => &mut f.texts.en,
                };
                if t.rules_override.is_none() {
                    t.rules_override = Some(limpio.join("\n"));
                }
            }
        }
        e2
    });
    let e_mitades = sin_fusionar.as_ref().unwrap_or(e);
    for (i, cara) in [0usize, 1].into_iter().enumerate() {
        if e.design.card.faces.len() <= cara {
            break;
        }
        let mut av = Vec::new();
        let cu = cuerpo(&e.design.card, cara);
        let est = estilo(rec, e, None, &mut av)?;
        let o = Opciones::de(cara, cu, pw.round() as u32, ph.round() as u32);
        let mut p = pintar_cara(motor, e_mitades, lang, rec, est, av, &o)?;
        // Las reglas puestas aquí no son «editadas a mano» (salvo que ya lo
        // estuvieran).
        let a_mano = e.design.faces.get(cara).is_some_and(|f| {
            f.texts.es.rules_override.is_some() || f.texts.en.rules_override.is_some()
        });
        if sin_fusionar.is_some() && !a_mano {
            p.avisos
                .retain(|a| !matches!(a, Aviso::ReglasEditadas { .. }));
        }
        let y = if i == 0 {
            2.0 * margen + mitad_alto
        } else {
            margen
        };
        pegar(
            &mut lienzo,
            &p.px,
            margen,
            y,
            region_w,
            mitad_alto,
            Giro::Izquierda,
        );
        avisos.extend(p.avisos);
        faltan.extend(p.faltan);
    }
    if franja > 0.0 {
        franja_fusionar(
            motor,
            e,
            lang,
            rec,
            &mut lienzo,
            aw - franja,
            franja,
            &mut faltan,
        )?;
    }
    Ok(Pieza {
        px: lienzo,
        avisos,
        faltan,
        arte: None,
    })
}

/// La franja de Fusionar: una pieza apaisada con el recordatorio.
#[allow(clippy::too_many_arguments)]
fn franja_fusionar(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    lienzo: &mut Pixmap,
    x: f32,
    w: f32,
    faltan: &mut Vec<String>,
) -> Result<(), DisenoError> {
    let mut av = Vec::new();
    let Some(est) = rec.pack.layout("fusionar").cloned() else {
        return Ok(());
    };
    let alto_px = lienzo.height();
    let [wm, hm] = est.tamano.unwrap_or([88.0, 6.0]);
    // La pieza va apaisada (88 × 6) y se gira para quedar en vertical.
    let ancho_px = (alto_px as f32 * wm / 88.0).round() as u32;
    let alto_pieza = (ancho_px as f32 * hm / wm).round() as u32;
    let mut o = Opciones::de(0, Cuerpo::Texto, ancho_px, alto_pieza);
    o.sin_pie = true;
    o.sin_arte = true;
    o.transparente = true;
    // El texto de la franja es la línea de Fusionar de la primera mitad.
    let texto = super::borrador::reglas_efectivas(&e.design, 0, lang);
    let linea = texto
        .lines()
        .find(|l| l.starts_with("Fuse") || l.starts_with("Fusionar"))
        .unwrap_or(match lang {
            DesignLang::Es => {
                "Fusionar (Puedes lanzar una o las dos mitades de esta carta desde tu mano.)"
            }
            DesignLang::En => "Fuse (You may cast one or both halves of this card from your hand.)",
        })
        .to_string();
    let mut e2 = e.clone();
    if let Some(f) = e2.design.faces.get_mut(0) {
        f.texts.es.rules_override = Some(linea.clone());
        f.texts.en.rules_override = Some(linea);
        f.texts.es.flavor.clear();
        f.texts.en.flavor.clear();
    }
    let p = pintar_cara(motor, &e2, lang, rec, est, std::mem::take(&mut av), &o)?;
    faltan.extend(p.faltan);
    pegar(lienzo, &p.px, x, 0.0, w, alto_px as f32, Giro::Izquierda);
    Ok(())
}

/// Secuela: la primera mitad arriba, normal; la segunda abajo, girada.
fn secuela(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    ancho: u32,
    alto: u32,
) -> Result<Pieza, DisenoError> {
    let mut lienzo = Pixmap::new(ancho, alto).ok_or(DisenoError::Lienzo)?;
    lienzo.fill(tiny_skia::Color::BLACK);
    let (aw, ah) = (ancho as f32, alto as f32);
    let k = ah / 88.0;
    let mut avisos = Vec::new();
    let mut faltan = Vec::new();
    // Arriba: un lienzo de 63 × 51.
    let mut av = Vec::new();
    let arriba = estilo(rec, e, Some("secuelaArriba"), &mut av)?;
    let [wa, ha] = arriba.tamano.unwrap_or([63.0, 51.0]);
    let alto_a = (aw * ha / wa).round() as u32;
    let mut o = Opciones::de(0, cuerpo(&e.design.card, 0), ancho, alto_a);
    o.sin_pie = true;
    let p = pintar_cara(motor, e, lang, rec, arriba, av, &o)?;
    pegar(&mut lienzo, &p.px, 0.0, 0.0, aw, alto_a as f32, Giro::Nada);
    avisos.extend(p.avisos);
    faltan.extend(p.faltan);
    // Abajo: un lienzo vertical que se gira.
    let mut av = Vec::new();
    let abajo = estilo(rec, e, Some("secuelaAbajo"), &mut av)?;
    let [wb, hb] = abajo.tamano.unwrap_or([33.0, 63.0]);
    let y0 = alto_a as f32;
    let region_h = ah - y0 - 4.6 * k;
    let pw = region_h.round() as u32;
    let ph = (region_h * hb / wb).round() as u32;
    let mut o = Opciones::de(1, cuerpo(&e.design.card, 1), pw, ph);
    o.sin_pie = true;
    o.sin_arte = true;
    let p = pintar_cara(motor, e, lang, rec, abajo, av, &o)?;
    pegar(&mut lienzo, &p.px, 0.0, y0, aw, region_h, Giro::Izquierda);
    avisos.extend(p.avisos);
    faltan.extend(p.faltan);
    pie_suelto(motor, e, lang, rec, &mut lienzo, &mut faltan)?;
    Ok(Pieza {
        px: lienzo,
        avisos,
        faltan,
        arte: None,
    })
}

/// El pie de la carta entera (para las compuestas), con el layout `pie`.
fn pie_suelto(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    lienzo: &mut Pixmap,
    faltan: &mut Vec<String>,
) -> Result<(), DisenoError> {
    let Some(est) = rec.pack.layout("pie").cloned() else {
        return Ok(());
    };
    let (w, h) = (lienzo.width(), lienzo.height());
    let mut o = Opciones::de(0, Cuerpo::Texto, w, h);
    o.transparente = true;
    o.sin_arte = true;
    let mut e2 = e.clone();
    // Solo el pie: sin nombre ni reglas.
    for f in &mut e2.design.faces {
        for t in [&mut f.texts.es, &mut f.texts.en] {
            t.rules_override = Some(String::new());
            t.flavor.clear();
        }
    }
    let _ = pie_texto;
    let p = pintar_cara(motor, &e2, lang, rec, est, Vec::new(), &o)?;
    faltan.extend(p.faltan);
    pegar(lienzo, &p.px, 0.0, 0.0, w as f32, h as f32, Giro::Nada);
    Ok(())
}

/// Flip: media carta arriba y la otra girada 180°, con el arte en medio.
/// Las dos mitades se pintan sobre la carta entera (el layout `flip` solo
/// usa la mitad de arriba) y la segunda se gira.
fn flip(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    ancho: u32,
    alto: u32,
) -> Result<Pieza, DisenoError> {
    let mut lienzo = Pixmap::new(ancho, alto).ok_or(DisenoError::Lienzo)?;
    lienzo.fill(tiny_skia::Color::BLACK);
    let (aw, ah) = (ancho as f32, alto as f32);
    let mut avisos = Vec::new();
    let mut faltan = Vec::new();
    for cara in [0usize, 1] {
        let mut av = Vec::new();
        let est = estilo(rec, e, Some("flip"), &mut av)?;
        let mut o = Opciones::de(cara, Cuerpo::Texto, ancho, alto);
        o.sin_pie = true;
        o.transparente = cara == 1;
        o.sin_arte = cara == 1;
        let p = pintar_cara(motor, e, lang, rec, est, av, &o)?;
        let giro = if cara == 0 { Giro::Nada } else { Giro::Media };
        pegar(&mut lienzo, &p.px, 0.0, 0.0, aw, ah, giro);
        avisos.extend(p.avisos);
        faltan.extend(p.faltan);
    }
    pie_suelto(motor, e, lang, rec, &mut lienzo, &mut faltan)?;
    Ok(Pieza {
        px: lienzo,
        avisos,
        faltan,
        arte: None,
    })
}

/// Una pieza apaisada (batalla, plano) pintada a lo ancho y girada a la
/// izquierda para la carta en vertical.
#[allow(clippy::too_many_arguments)]
fn apaisada(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    layout: &str,
    cara: usize,
    ancho: u32,
    alto: u32,
    indicador: Option<Indicador>,
) -> Result<Pieza, DisenoError> {
    let mut av = Vec::new();
    let est = estilo(rec, e, Some(layout), &mut av)?;
    // Apaisada: su ancho es el alto de la carta.
    let mut o = Opciones::de(cara, cuerpo(&e.design.card, cara), alto, ancho);
    o.indicador = indicador;
    let p = pintar_cara(motor, e, lang, rec, est, av, &o)?;
    let mut lienzo = Pixmap::new(ancho, alto).ok_or(DisenoError::Lienzo)?;
    lienzo.fill(tiny_skia::Color::BLACK);
    pegar(
        &mut lienzo,
        &p.px,
        0.0,
        0.0,
        ancho as f32,
        alto as f32,
        Giro::Izquierda,
    );
    Ok(Pieza {
        px: lienzo,
        avisos: p.avisos,
        faltan: p.faltan,
        arte: None,
    })
}

/// El dorso de una carta meld: media carta fundida (la cara 1 del diseño),
/// girada. La fundida es una carta vertical del tamaño de dos.
fn meld_dorso(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    lang: DesignLang,
    rec: &Recursos<'_>,
    ancho: u32,
    alto: u32,
) -> Result<Pieza, DisenoError> {
    let mut av = Vec::new();
    let est = estilo(rec, e, None, &mut av)?;
    // La fundida: tan ancha como alta es la carta y el doble de alta que ancha.
    let (fw, fh) = (alto, ancho * 2);
    let mut o = Opciones::de(1, cuerpo(&e.design.card, 1), fw, fh);
    o.indicador = None;
    let p = pintar_cara(motor, e, lang, rec, est, av, &o)?;
    let arriba = e.design.card.meld_arriba.unwrap_or(true);
    let mut mitad = Pixmap::new(fw, fh / 2).ok_or(DisenoError::Lienzo)?;
    let dy = if arriba { 0.0 } else { -((fh / 2) as f32) };
    mitad.draw_pixmap(
        0,
        0,
        p.px.as_ref(),
        &PixmapPaint::default(),
        Transform::from_translate(0.0, dy),
        None,
    );
    let mut lienzo = Pixmap::new(ancho, alto).ok_or(DisenoError::Lienzo)?;
    lienzo.fill(tiny_skia::Color::BLACK);
    pegar(
        &mut lienzo,
        &mitad,
        0.0,
        0.0,
        ancho as f32,
        alto as f32,
        Giro::Izquierda,
    );
    Ok(Pieza {
        px: lienzo,
        avisos: p.avisos,
        faltan: p.faltan,
        arte: None,
    })
}

/// ¿La cara `cara` se pinta como una cara normal (sin piezas giradas ni
/// medidas grandes)?
fn es_simple(e: &DisenoEfectivo, maq: Maquetacion, cara: usize) -> bool {
    !(maq.es_partida()
        || matches!(
            maq,
            Maquetacion::Secuela
                | Maquetacion::Flip
                | Maquetacion::Plano
                | Maquetacion::Plan
                | Maquetacion::Vanguard
        )
        || (maq == Maquetacion::Batalla && cara == 0)
        || (maq == Maquetacion::Meld && cara == 1 && e.design.card.faces.len() > 1))
}

/// La carta sin la ilustración, sobre transparente, y dónde va la
/// ilustración: la vista previa la mueve ella sola por debajo mientras se
/// arrastra, sin esperar a un render entero por cada paso.
pub struct CapaVista {
    pub imagen: RgbaImage,
    /// El hueco de la ilustración, en fracciones de la carta (x, y, ancho,
    /// alto).
    pub arte: [f32; 4],
    /// El radio de las esquinas, en fracciones del ancho y del alto.
    pub radio: [f32; 2],
    /// La cara del diseño cuya ilustración va en el hueco.
    pub cara: usize,
}

/// La [`CapaVista`] de una cara, si es una cara normal con hueco para la
/// ilustración (las compuestas y las grandes, no: esas se pintan enteras).
#[allow(clippy::too_many_arguments)]
pub fn render_capa(
    motor: &mut Motor,
    e: &DisenoEfectivo,
    cara: usize,
    lang: DesignLang,
    rec: &Recursos<'_>,
    card: CardSize,
    ancho_px: u32,
) -> Result<Option<CapaVista>, DisenoError> {
    let d = &e.design;
    let (maq, _) = maquetacion(&d.card);
    let cara = cara.min(super::maquetacion::caras_impresas(&d.card) - 1);
    if !es_simple(e, maq, cara) {
        return Ok(None);
    }
    let (w_mm, h_mm) = medida(e, card);
    let ancho = ancho_px.max(16);
    let alto = (ancho as f64 * h_mm / w_mm).round() as u32;
    let pieza = cara_simple(motor, e, lang, rec, maq, cara, ancho, alto, true)?;
    let Some(arte) = pieza.arte else {
        return Ok(None);
    };
    let mut px = pieza.px;
    let radio = e.retoques.card_radius.map_or_else(
        || estilo_para(rec, d.style, None, &mut Vec::new()).map_or(3.0, |s| s.radio),
        |r| r as f32,
    );
    let r = radio * (ancho as f32 / w_mm as f32);
    if radio > 0.0 {
        if let Some(m) = mascara_redondeada(ancho, alto, r) {
            px.apply_mask(&m);
        }
    }
    Ok(Some(CapaVista {
        imagen: de_pixmap(&px),
        arte,
        radio: [r / ancho as f32, r / alto as f32],
        cara,
    }))
}

/// La medida (mm) a la que sale la carta.
pub fn medida(e: &DisenoEfectivo, card: CardSize) -> (f64, f64) {
    let (maq, _) = maquetacion(&e.design.card);
    if maq.es_grande() && e.design.card.oversized {
        medida_grande()
    } else {
        (card.width_mm, card.height_mm)
    }
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
    let (maq, desconocido) = maquetacion(&d.card);
    let mut avisos = Vec::new();
    if desconocido {
        avisos.push(Aviso::LayoutDesconocido {
            layout: d.card.layout.clone(),
        });
    }
    let (w_mm, h_mm) = medida(e, card);
    let ancho = ancho_px.max(16);
    let alto = (ancho as f64 * h_mm / w_mm).round() as u32;
    let cara = cara.min(super::maquetacion::caras_impresas(&d.card) - 1);
    let pieza = match maq {
        m if m.es_partida() => partida(motor, e, lang, rec, m, ancho, alto)?,
        Maquetacion::Secuela => secuela(motor, e, lang, rec, ancho, alto)?,
        Maquetacion::Flip => flip(motor, e, lang, rec, ancho, alto)?,
        Maquetacion::Batalla if cara == 0 => apaisada(
            motor,
            e,
            lang,
            rec,
            "batalla",
            0,
            ancho,
            alto,
            Some(Indicador::Sol),
        )?,
        Maquetacion::Meld if cara == 1 && d.card.faces.len() > 1 => {
            meld_dorso(motor, e, lang, rec, ancho, alto)?
        }
        Maquetacion::Plano => apaisada(motor, e, lang, rec, "plano", 0, ancho, alto, None)?,
        Maquetacion::Plan | Maquetacion::Vanguard => {
            let nombre = if maq == Maquetacion::Plan {
                "plan"
            } else {
                "vanguard"
            };
            let mut av = Vec::new();
            let est = estilo(rec, e, Some(nombre), &mut av)?;
            let o = Opciones::de(0, Cuerpo::Texto, ancho, alto);
            pintar_cara(motor, e, lang, rec, est, av, &o)?
        }
        _ => cara_simple(motor, e, lang, rec, maq, cara, ancho, alto, false)?,
    };
    avisos.extend(pieza.avisos);
    let mut faltan = pieza.faltan;
    faltan.retain(|s| !matches!(s.as_str(), "PINCEL" | "ESTRELLA"));
    faltan.sort();
    faltan.dedup();
    if !faltan.is_empty() {
        avisos.push(Aviso::FaltanSimbolos { simbolos: faltan });
    }
    // Sin repetir (las piezas de una compuesta dan los mismos avisos).
    let mut vistos = Vec::new();
    avisos.retain(|a| {
        if vistos.contains(a) {
            false
        } else {
            vistos.push(a.clone());
            true
        }
    });

    let mut px = pieza.px;
    // Las esquinas: las del ajuste fino (en pantalla y al imprimir) o, sin
    // él, las del marco en pantalla y en ángulo recto al imprimir.
    let radio = match e.retoques.card_radius {
        Some(r) => Some(r as f32),
        None if salida == Salida::Pantalla => {
            Some(estilo_para(rec, d.style, None, &mut Vec::new()).map_or(3.0, |s| s.radio))
        }
        None => None,
    };
    if let Some(radio) = radio.filter(|r| *r > 0.0) {
        let r = radio * (ancho as f32 / w_mm as f32);
        if let Some(m) = mascara_redondeada(ancho, alto, r) {
            px.apply_mask(&m);
        }
    }
    let imagen: RgbaImage = de_pixmap(&px);
    Ok(Render { imagen, avisos })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::efectivo::efectivo;
    use crate::diseno::modelo::tests::diseno_de_prueba;
    use crate::diseno::pack::Pack;
    use crate::diseno::render::SinImagenes;
    use crate::diseno::simbolos::Simbolos;

    /// Una carta normal de una cara (la de prueba es partida).
    fn normal() -> DisenoEfectivo {
        let mut d = diseno_de_prueba();
        d.card.layout = "normal".into();
        d.card.faces.truncate(1);
        d.faces.truncate(1);
        efectivo(&d, None)
    }

    fn con_layout(layout: &str, caras: usize) -> DisenoEfectivo {
        let mut e = normal();
        e.design.card.layout = layout.into();
        while e.design.card.faces.len() < caras {
            let mut f = e.design.card.faces[0].clone();
            f.name = format!("Cara {}", e.design.card.faces.len());
            e.design.card.faces.push(f);
            e.design.faces.push(Default::default());
        }
        e
    }

    fn simbolos(codigos: &[&str]) -> Simbolos {
        let mut s = Simbolos::nuevos();
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="48" fill="#cccccc"/></svg>"##;
        for c in codigos {
            s.cargar(c, svg).unwrap();
        }
        s
    }

    fn pintar(e: &DisenoEfectivo, cara: usize, ancho: u32, s: &Simbolos, salida: Salida) -> Render {
        let pack = Pack::de_fabrica().unwrap();
        let rec = Recursos {
            pack: &pack,
            simbolos: s,
            imagenes: &SinImagenes,
        };
        let mut motor = Motor::de_fabrica().unwrap();
        render(
            &mut motor,
            e,
            cara,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            ancho,
            salida,
        )
        .unwrap()
    }

    // ── Ajustes finos ──

    /// Las filas (en mm) donde dos renders se diferencian.
    fn filas_distintas(a: &RgbaImage, b: &RgbaImage) -> Option<(f32, f32)> {
        let k = a.height() as f32 / 88.0;
        let mut r: Option<(f32, f32)> = None;
        for (x, y, p) in a.enumerate_pixels() {
            let q = b.get_pixel(x, y);
            if (0..4).any(|i| p[i].abs_diff(q[i]) > 8) {
                let mm = y as f32 / k;
                r = Some(r.map_or((mm, mm), |(a, b)| (a.min(mm), b.max(mm))));
            }
        }
        r
    }

    #[test]
    fn los_ajustes_finos_mueven_solo_lo_suyo() {
        use crate::diseno::modelo::FineTuning;
        let s = simbolos(&["2", "W"]);
        let base = normal();
        let con = |f: FineTuning| {
            let mut e = base.clone();
            e.retoques = f.limitado();
            pintar(&e, 0, 315, &s, Salida::Impresion).imagen
        };
        let antes = pintar(&base, 0, 315, &s, Salida::Impresion).imagen;
        // El título, 1 mm abajo: cambia solo arriba.
        let (a, b) = filas_distintas(
            &antes,
            &con(FineTuning {
                title: 1.0,
                ..FineTuning::default()
            }),
        )
        .unwrap();
        assert!(a > 2.0 && b < 12.0, "título: {a}–{b}");
        // El pie, 1 mm arriba: solo abajo.
        let (a, _) = filas_distintas(
            &antes,
            &con(FineTuning {
                footer: -1.0,
                ..FineTuning::default()
            }),
        )
        .unwrap();
        assert!(a > 80.0, "pie desde {a}");
        // Las reglas: dentro de la caja.
        let (a, b) = filas_distintas(
            &antes,
            &con(FineTuning {
                rules: 0.5,
                ..FineTuning::default()
            }),
        )
        .unwrap();
        assert!(a > 55.0 && b < 84.0, "reglas: {a}–{b}");
        // La caja (clara, para que se note sobre el fondo oscuro) más opaca:
        // solo cambia la caja.
        let clara = |f: FineTuning| {
            let mut e = base.clone();
            e.frame.rules_box = crate::diseno::modelo::RulesBox::Light;
            e.retoques = f;
            pintar(&e, 0, 315, &s, Salida::Impresion).imagen
        };
        let (a, b) = filas_distintas(
            &clara(FineTuning::default()),
            &clara(FineTuning {
                rules_box_opacity: 1.0,
                ..FineTuning::default()
            }),
        )
        .unwrap();
        assert!(a > 50.0 && b < 84.0, "caja: {a}–{b}");
        // Más radio en la ventana de la ilustración: las esquinas de abajo.
        let (a, b) = filas_distintas(
            &antes,
            &con(FineTuning {
                art_radius: Some(14.0),
                ..FineTuning::default()
            }),
        )
        .unwrap();
        assert!(a > 68.0 && b < 83.5, "ventana: {a}–{b}");
        // Lo que se pase de los límites se queda en ellos.
        let f = FineTuning {
            title: 50.0,
            rules_box_opacity: 7.0,
            card_radius: Some(-2.0),
            ..FineTuning::default()
        }
        .limitado();
        assert_eq!(
            (f.title, f.rules_box_opacity, f.card_radius),
            (3.0, 1.0, Some(0.0))
        );
    }

    /// Un marco con la curva de la ventana pintada en una capa (como los
    /// Original): con un radio propio, en las zonas `repintar` vuelve la
    /// ilustración por encima de esa capa.
    #[test]
    fn el_radio_de_la_ilustracion_rehace_la_curva_de_una_capa() {
        use crate::diseno::modelo::FineTuning;
        let mut pack = Pack::de_fabrica().unwrap();
        pack.ficheros.insert(
            "prueba/curva.svg".into(),
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 63 88" width="63" height="88"><rect x="0" y="70" width="8" height="13" fill="#000"/></svg>"##.into(),
        );
        let est = pack.def.estilos.get_mut("borderless").unwrap();
        est.capas.insert(
            0,
            serde_json::from_str(r#"{"svg":"prueba/curva.svg"}"#).unwrap(),
        );
        let v = est.ventana_arte.as_mut().unwrap();
        v.repintar = vec![[0.0, 60.0, 8.0, 23.0]];
        v.repintar_tras = Some(1);
        let s = simbolos(&["2", "W"]);
        let rec = Recursos {
            pack: &pack,
            simbolos: &s,
            imagenes: &Roja,
        };
        let mut motor = Motor::de_fabrica().unwrap();
        let mut pinta = |radio: Option<f64>, capa: bool| {
            let mut e = normal();
            e.retoques = FineTuning {
                art_radius: radio,
                ..FineTuning::default()
            };
            let img = if capa {
                render_capa(&mut motor, &e, 0, DesignLang::Es, &rec, CardSize::MTG, 630)
                    .unwrap()
                    .unwrap()
                    .imagen
            } else {
                render(
                    &mut motor,
                    &e,
                    0,
                    DesignLang::Es,
                    &rec,
                    CardSize::MTG,
                    630,
                    Salida::Impresion,
                )
                .unwrap()
                .imagen
            };
            // En la esquina de abajo a la izquierda de la ventana (1, 82 mm).
            *img.get_pixel(10, 820)
        };
        // Sin radio propio: la curva de la capa (negro).
        assert_eq!(pinta(None, false).0[..3], [0, 0, 0]);
        // A 0 mm: la ilustración llega a la esquina.
        assert_eq!(pinta(Some(0.0), false).0[..3], [220, 30, 30]);
        // En la capa de la vista en directo, ahí queda hueco para la ilustración.
        assert_eq!(pinta(Some(0.0), true).0[3], 0);
        // Con un radio grande, negro otra vez.
        assert_eq!(pinta(Some(10.0), false).0[..3], [0, 0, 0]);
    }

    #[test]
    fn el_radio_de_la_carta_vale_en_pantalla_y_al_imprimir() {
        use crate::diseno::modelo::FineTuning;
        let s = simbolos(&["2", "W"]);
        let esquina = |radio: Option<f64>, salida: Salida| {
            let mut e = normal();
            e.retoques = FineTuning {
                card_radius: radio,
                ..FineTuning::default()
            };
            pintar(&e, 0, 200, &s, salida).imagen.get_pixel(0, 0)[3]
        };
        // Sin pedirlo: redondeada en pantalla, en ángulo recto al imprimir.
        assert_eq!(esquina(None, Salida::Pantalla), 0);
        assert_eq!(esquina(None, Salida::Impresion), 255);
        // Con 0 mm, recta en las dos; con 3 mm, redondeada en las dos.
        assert_eq!(esquina(Some(0.0), Salida::Pantalla), 255);
        assert_eq!(esquina(Some(3.0), Salida::Impresion), 0);
    }

    // ── Capa para mover la ilustración ──

    /// Una ilustración de un solo color, para cualquier blob.
    struct Roja;
    impl crate::diseno::render::Imagenes for Roja {
        fn imagen(&self, _: &str) -> Option<std::sync::Arc<RgbaImage>> {
            Some(std::sync::Arc::new(RgbaImage::from_pixel(
                300,
                200,
                image::Rgba([220, 30, 30, 255]),
            )))
        }
    }

    #[test]
    fn la_capa_sobre_negro_y_la_ilustracion_es_la_carta_entera() {
        use crate::diseno::modelo::DesignStyle;
        for estilo in [
            DesignStyle::Borderless,
            DesignStyle::Extended,
            DesignStyle::Classic,
            DesignStyle::FullArt,
        ] {
            let mut e = normal();
            e.design.style = estilo;
            assert!(e.design.faces[0].art.is_some());
            let pack = Pack::de_fabrica().unwrap();
            let s = simbolos(&["2", "W"]);
            let rec = Recursos {
                pack: &pack,
                simbolos: &s,
                imagenes: &Roja,
            };
            let mut motor = Motor::de_fabrica().unwrap();
            let ancho = 400;
            let entera = render(
                &mut motor,
                &e,
                0,
                DesignLang::Es,
                &rec,
                CardSize::MTG,
                ancho,
                Salida::Pantalla,
            )
            .unwrap()
            .imagen;
            let capa = render_capa(
                &mut motor,
                &e,
                0,
                DesignLang::Es,
                &rec,
                CardSize::MTG,
                ancho,
            )
            .unwrap()
            .unwrap();
            assert_eq!(capa.cara, 0);
            assert_eq!(capa.imagen.dimensions(), entera.dimensions());
            let (w, h) = entera.dimensions();
            let [ax, ay, aw, ah] = capa.arte;
            assert!(
                aw > 0.5 && ah > 0.2 && ax >= 0.0 && ay >= 0.0,
                "{:?}",
                capa.arte
            );
            // Montada como la monta la pantalla: negro, la ilustración (roja) en
            // su hueco y la capa encima.
            let (x0, y0) = ((ax * w as f32).round(), (ay * h as f32).round());
            let (x1, y1) = (x0 + (aw * w as f32).round(), y0 + (ah * h as f32).round());
            let mut dif = 0u64;
            for (x, y, p) in capa.imagen.enumerate_pixels() {
                let (fx, fy) = (x as f32, y as f32);
                let fondo = if fx >= x0 && fx < x1 && fy >= y0 && fy < y1 {
                    [220.0, 30.0, 30.0]
                } else {
                    [0.0, 0.0, 0.0]
                };
                let a = p[3] as f32 / 255.0;
                let q = entera.get_pixel(x, y);
                // Las esquinas: transparentes en las dos.
                if q[3] == 0 {
                    continue;
                }
                for k in 0..3 {
                    let m = p[k] as f32 * a + fondo[k] * (1.0 - a);
                    dif += (m - q[k] as f32).abs() as u64;
                }
            }
            let media = dif as f64 / (w * h * 3) as f64;
            assert!(media < 1.0, "{estilo:?}: diferencia media {media}");
        }
        let pack = Pack::de_fabrica().unwrap();
        let s = simbolos(&["2", "W"]);
        let rec = Recursos {
            pack: &pack,
            simbolos: &s,
            imagenes: &Roja,
        };
        let mut motor = Motor::de_fabrica().unwrap();
        let ancho = 400;
        // En las compuestas no hay capa: se pintan enteras.
        let partida = con_layout("split", 2);
        assert!(render_capa(
            &mut motor,
            &partida,
            0,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            ancho
        )
        .unwrap()
        .is_none());
    }

    // ── Medida ──

    #[test]
    fn una_carta_normal_sale_a_la_medida_del_deck() {
        let e = normal();
        let poker = CardSize {
            width_mm: 63.5,
            height_mm: 88.9,
        };
        assert_eq!(medida(&e, CardSize::MTG), (63.0, 88.0));
        assert_eq!(medida(&e, poker), (63.5, 88.9));
        // «oversized» solo cuenta en las cartas grandes.
        let mut e = normal();
        e.design.card.oversized = true;
        assert_eq!(medida(&e, CardSize::MTG), (63.0, 88.0));
    }

    #[test]
    fn las_grandes_salen_grandes_solo_si_se_imprimieron_grandes() {
        for layout in ["planar", "scheme", "vanguard"] {
            let mut e = con_layout(layout, 1);
            assert_eq!(medida(&e, CardSize::MTG), (63.0, 88.0), "{layout}");
            e.design.card.oversized = true;
            assert_eq!(medida(&e, CardSize::MTG), medida_grande(), "{layout}");
        }
    }

    // ── Qué layout pide cada cara ──

    #[test]
    fn el_layout_de_cada_cuerpo() {
        let e = normal();
        let l = |cu, m| layout_de_cara(&e, 0, cu, m);
        assert_eq!(l(Cuerpo::Texto, Maquetacion::Normal), None);
        assert_eq!(l(Cuerpo::Capitulos, Maquetacion::Saga), Some("saga"));
        assert_eq!(l(Cuerpo::Mazmorra, Maquetacion::Mazmorra), Some("mazmorra"));
        assert_eq!(l(Cuerpo::Texto, Maquetacion::Emblema), Some("ficha"));
        assert_eq!(l(Cuerpo::Texto, Maquetacion::FichaDosCaras), Some("ficha"));
        for cu in [
            Cuerpo::Lealtad,
            Cuerpo::Niveles,
            Cuerpo::Caso,
            Cuerpo::Leveler,
            Cuerpo::Estacion,
        ] {
            assert_eq!(l(cu, Maquetacion::Normal), Some("alto"), "{cu:?}");
        }
        assert_eq!(l(Cuerpo::Prototipo, Maquetacion::Prototipo), None);
        // Una ficha es ficha aunque su cuerpo pida caja alta, pero una saga
        // sigue siendo saga.
        let mut ficha = normal();
        ficha.design.card.layout = "token".into();
        assert_eq!(
            layout_de_cara(&ficha, 0, Cuerpo::Lealtad, Maquetacion::Ficha),
            Some("ficha")
        );
        assert_eq!(
            layout_de_cara(&ficha, 0, Cuerpo::Capitulos, Maquetacion::Ficha),
            Some("saga")
        );
    }

    // ── Pegar piezas giradas ──

    /// Una pieza de 40 × 20: arriba roja y abajo azul.
    fn pieza() -> Pixmap {
        let mut p = Pixmap::new(40, 20).unwrap();
        p.fill(tiny_skia::Color::from_rgba8(0, 0, 255, 255));
        let mut rojo = tiny_skia::Paint::default();
        rojo.set_color_rgba8(255, 0, 0, 255);
        p.fill_rect(
            tiny_skia::Rect::from_xywh(0.0, 0.0, 40.0, 10.0).unwrap(),
            &rojo,
            Transform::identity(),
            None,
        );
        p
    }

    fn rojo(p: &Pixmap, x: u32, y: u32) -> bool {
        let c = p.pixel(x, y).unwrap();
        c.red() > 200 && c.blue() < 50
    }

    fn azul(p: &Pixmap, x: u32, y: u32) -> bool {
        let c = p.pixel(x, y).unwrap();
        c.blue() > 200 && c.red() < 50
    }

    #[test]
    fn girar_a_la_izquierda_deja_lo_de_arriba_a_la_izquierda() {
        let mut d = Pixmap::new(20, 40).unwrap();
        pegar(&mut d, &pieza(), 0.0, 0.0, 20.0, 40.0, Giro::Izquierda);
        assert!(rojo(&d, 2, 20));
        assert!(azul(&d, 17, 20));
    }

    #[test]
    fn media_vuelta_deja_lo_de_arriba_abajo() {
        let mut d = Pixmap::new(40, 20).unwrap();
        pegar(&mut d, &pieza(), 0.0, 0.0, 40.0, 20.0, Giro::Media);
        assert!(azul(&d, 20, 2));
        assert!(rojo(&d, 20, 17));
    }

    #[test]
    fn sin_giro_encaja_centrada_sin_deformar() {
        // Hueco de 80 × 80: la pieza (2:1) se dobla a 80 × 40 y queda en medio.
        let mut d = Pixmap::new(80, 80).unwrap();
        pegar(&mut d, &pieza(), 0.0, 0.0, 80.0, 80.0, Giro::Nada);
        assert_eq!(d.pixel(40, 5).unwrap().alpha(), 0, "franja libre arriba");
        assert!(rojo(&d, 40, 25));
        assert!(azul(&d, 40, 55));
        assert_eq!(d.pixel(40, 75).unwrap().alpha(), 0, "franja libre abajo");
    }

    // ── El render entero ──

    #[test]
    fn el_alto_sale_de_la_proporcion_y_el_ancho_tiene_minimo() {
        let s = simbolos(&["2", "W"]);
        let r = pintar(&normal(), 0, 126, &s, Salida::Impresion);
        assert_eq!(r.imagen.dimensions(), (126, 176));
        let r = pintar(&normal(), 0, 3, &s, Salida::Impresion);
        assert_eq!(r.imagen.dimensions(), (16, 22));
    }

    #[test]
    fn una_cara_fuera_de_rango_pinta_la_ultima_que_hay() {
        let s = simbolos(&["2", "W"]);
        let a = pintar(&normal(), 0, 90, &s, Salida::Impresion);
        let b = pintar(&normal(), 7, 90, &s, Salida::Impresion);
        assert_eq!(a.imagen, b.imagen);
        assert_eq!(a.avisos, b.avisos);
    }

    #[test]
    fn un_meld_sin_la_carta_fundida_pinta_su_cara() {
        let s = simbolos(&["2", "W"]);
        let e = con_layout("meld", 1);
        let delante = pintar(&e, 0, 90, &s, Salida::Impresion);
        let detras = pintar(&e, 1, 90, &s, Salida::Impresion);
        assert_eq!(delante.imagen, detras.imagen);
    }

    #[test]
    fn el_dorso_de_un_meld_es_media_carta_fundida() {
        let s = simbolos(&["2", "W"]);
        let mut e = con_layout("meld", 2);
        let delante = pintar(&e, 0, 90, &s, Salida::Impresion);
        let arriba = pintar(&e, 1, 90, &s, Salida::Impresion);
        assert_eq!(arriba.imagen.dimensions(), delante.imagen.dimensions());
        assert_ne!(arriba.imagen, delante.imagen);
        // La otra mitad de la fundida es otra imagen.
        e.design.card.meld_arriba = Some(false);
        let abajo = pintar(&e, 1, 90, &s, Salida::Impresion);
        assert_ne!(abajo.imagen, arriba.imagen);
    }

    #[test]
    fn una_carta_sin_caras_se_pinta_sin_romper() {
        let mut e = normal();
        e.design.card.faces.clear();
        e.design.faces.clear();
        let r = pintar(&e, 3, 90, &Simbolos::nuevos(), Salida::Impresion);
        assert_eq!(r.imagen.dimensions(), (90, 126));
        // Sin coste no falta ningún símbolo (el pincel del pie no cuenta).
        assert!(!r
            .avisos
            .iter()
            .any(|a| matches!(a, Aviso::FaltanSimbolos { .. })));
    }

    #[test]
    fn un_coste_larguisimo_no_rompe_el_titulo() {
        // 24 símbolos se comen todo el hueco del título (ancho negativo).
        let mut e = normal();
        e.design.card.faces[0].mana_cost = "{W}".repeat(24);
        let r = pintar(&e, 0, 90, &simbolos(&["W"]), Salida::Impresion);
        assert_eq!(r.imagen.dimensions(), (90, 126));
    }

    #[test]
    fn un_layout_desconocido_avisa_y_se_pinta_como_normal() {
        let s = simbolos(&["2", "W"]);
        let e = con_layout("inventado", 1);
        let r = pintar(&e, 0, 90, &s, Salida::Impresion);
        assert!(r.avisos.contains(&Aviso::LayoutDesconocido {
            layout: "inventado".into()
        }));
        assert_eq!(
            r.imagen,
            pintar(&normal(), 0, 90, &s, Salida::Impresion).imagen
        );
    }

    #[test]
    fn los_simbolos_que_faltan_salen_ordenados_y_sin_repetir() {
        // {2}{W}{W}: el pincel del pie no cuenta como símbolo que falta.
        let r = pintar(&normal(), 0, 90, &Simbolos::nuevos(), Salida::Impresion);
        let faltan: Vec<_> = r
            .avisos
            .iter()
            .filter_map(|a| match a {
                Aviso::FaltanSimbolos { simbolos } => Some(simbolos.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(faltan, vec![vec!["2".to_string(), "W".to_string()]]);
        // Con los símbolos cargados no hay aviso.
        let r = pintar(&normal(), 0, 90, &simbolos(&["2", "W"]), Salida::Impresion);
        assert!(!r
            .avisos
            .iter()
            .any(|a| matches!(a, Aviso::FaltanSimbolos { .. })));
    }

    #[test]
    fn una_partida_no_repite_avisos() {
        let e = efectivo(&diseno_de_prueba(), None);
        let r = pintar(&e, 0, 90, &Simbolos::nuevos(), Salida::Impresion);
        for (i, a) in r.avisos.iter().enumerate() {
            assert!(!r.avisos[i + 1..].contains(a), "repetido: {a:?}");
        }
        // La segunda mitad no tiene nombre temático: un aviso, una vez.
        assert!(r.avisos.contains(&Aviso::FaltaNombre {
            cara: 1,
            lang: DesignLang::Es
        }));
    }

    #[test]
    fn en_pantalla_las_esquinas_son_transparentes_y_al_imprimir_no() {
        let s = simbolos(&["2", "W"]);
        let p = pintar(&normal(), 0, 90, &s, Salida::Pantalla);
        let i = pintar(&normal(), 0, 90, &s, Salida::Impresion);
        let (w, h) = p.imagen.dimensions();
        for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            assert_eq!(p.imagen.get_pixel(x, y).0[3], 0);
            assert_eq!(i.imagen.get_pixel(x, y).0[3], 255);
        }
        assert_eq!(p.imagen.get_pixel(w / 2, h / 2).0[3], 255);
    }

    #[test]
    fn un_plano_grande_sale_apaisado_girado_a_su_medida() {
        let s = simbolos(&["2", "W"]);
        let mut e = con_layout("planar", 1);
        e.design.card.oversized = true;
        let r = pintar(&e, 0, 89, &s, Salida::Impresion);
        assert_eq!(r.imagen.dimensions(), (89, 127));
    }

    #[test]
    fn el_mismo_diseno_da_los_mismos_pixeles() {
        let s = simbolos(&["2", "W"]);
        let e = efectivo(&diseno_de_prueba(), None);
        let a = pintar(&e, 0, 90, &s, Salida::Pantalla);
        let b = pintar(&e, 0, 90, &s, Salida::Pantalla);
        assert_eq!(a.imagen, b.imagen);
    }
}
