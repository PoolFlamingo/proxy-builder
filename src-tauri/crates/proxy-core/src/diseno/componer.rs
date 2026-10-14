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
    avisos: Vec<Aviso>,
) -> Result<Pieza, DisenoError> {
    let d = &e.design;
    let cu = cuerpo(&d.card, cara);
    let mut avisos = avisos;
    let est = estilo(rec, e, layout_de_cara(e, cara, cu, maq), &mut avisos)?;
    let mut o = Opciones::de(cara, cu, ancho, alto);
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
        // Las reglas puestas aquí no son «editadas a mano».
        if sin_fusionar.is_some() {
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
    })
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
    let cara = cara.min(maq.caras_impresas() - 1);
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
        _ => cara_simple(motor, e, lang, rec, maq, cara, ancho, alto, Vec::new())?,
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
    if salida == Salida::Pantalla {
        let radio = estilo_para(rec, d.style, None, &mut Vec::new()).map_or(3.0, |s| s.radio);
        let r = radio * (ancho as f32 / 63.0);
        if let Some(m) = mascara_redondeada(ancho, alto, r) {
            px.apply_mask(&m);
        }
    }
    let imagen: RgbaImage = de_pixmap(&px);
    Ok(Render { imagen, avisos })
}
