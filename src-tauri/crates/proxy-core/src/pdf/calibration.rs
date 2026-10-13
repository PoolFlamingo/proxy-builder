//! **Hoja de calibración dúplex.**
//!
//! Frente: el contorno de cada casilla, su etiqueta y dos reglas milimetradas
//! (±5 mm) centradas. Dorso: una cruz roja en el centro de cada casilla,
//! colocada con el mismo espejo, giro y desplazamientos que los reversos.
//! Mirando la hoja al trasluz desde el frente se lee en las reglas cuánto se
//! desvía la cruz en cada fila.

use krilla::geom::Point;
use krilla::page::PageSettings;
use krilla::text::{Font, TextDirection};

use super::compose::{dibujar_lineas, documento, tam_pagina, PdfMeta};
use super::{PdfError, LETRA};
use crate::layout::{plan, LineMm, SheetSide};
use crate::model::{
    BackSpec, CardEdits, CardMeta, CardSource, Deck, DeckCard, Duplex, PrintSettings,
    DECK_SCHEMA_VERSION,
};
use crate::units::mm_to_pt;

fn pt(mm: f64) -> f32 {
    mm_to_pt(mm) as f32
}

/// Genera la hoja de calibración para estos ajustes. Si no hay dúplex
/// elegido, se usa borde largo.
pub fn calibration(
    settings: &PrintSettings,
    textos: &CalibrationTexts,
) -> Result<Vec<u8>, PdfError> {
    let mut s = settings.clone();
    if s.duplex == Duplex::Off {
        s.duplex = Duplex::LongEdge;
    }
    s.interleave_dfc = false;
    s.cut_guides.enabled = false;
    // Una hoja llena de cartas sin imagen: solo interesa la geometría.
    let base = Deck {
        id: "calibracion".into(),
        name: String::new(),
        decklist_text: String::new(),
        cards: Vec::new(),
        settings: s.clone(),
        default_back: None,
        schema_version: DECK_SCHEMA_VERSION,
    };
    let cap = plan(&base).map_err(|_| PdfError::Geometry)?.grid.capacity() as usize;
    let deck = Deck {
        cards: (0..cap)
            .map(|i| DeckCard {
                id: format!("c{i}"),
                source: CardSource::Custom,
                front: None,
                back: BackSpec::DeckDefault,
                edits: CardEdits::default(),
                locked: false,
                meta: CardMeta::default(),
            })
            .collect(),
        ..base
    };
    let p = plan(&deck).map_err(|_| PdfError::Geometry)?;
    let fuente = Font::new(LETRA.to_vec().into(), 0).ok_or(PdfError::Write("letra".into()))?;
    let mut doc = documento(&PdfMeta {
        title: textos.title.clone(),
        document_id: "proxy-builder-calibracion".into(),
    });

    for pagina in p.pages.iter().take(2) {
        let mut hoja = doc.start_page_with(PageSettings::new(tam_pagina(pagina)?));
        let mut sup = hoja.surface();
        match pagina.side {
            SheetSide::Front => {
                let mut contornos = Vec::new();
                let mut reglas = Vec::new();
                for slot in &pagina.slots {
                    let t = slot.trim;
                    contornos.extend([
                        LineMm {
                            x1: t.x,
                            y1: t.y,
                            x2: t.right(),
                            y2: t.y,
                        },
                        LineMm {
                            x1: t.right(),
                            y1: t.y,
                            x2: t.right(),
                            y2: t.bottom(),
                        },
                        LineMm {
                            x1: t.right(),
                            y1: t.bottom(),
                            x2: t.x,
                            y2: t.bottom(),
                        },
                        LineMm {
                            x1: t.x,
                            y1: t.bottom(),
                            x2: t.x,
                            y2: t.y,
                        },
                    ]);
                    let (cx, cy) = (t.x + t.w / 2.0, t.y + t.h / 2.0);
                    reglas.push(LineMm {
                        x1: cx - 5.0,
                        y1: cy,
                        x2: cx + 5.0,
                        y2: cy,
                    });
                    reglas.push(LineMm {
                        x1: cx,
                        y1: cy - 5.0,
                        x2: cx,
                        y2: cy + 5.0,
                    });
                    for k in -5i32..=5 {
                        let largo = if k % 5 == 0 { 1.6 } else { 0.8 };
                        let d = f64::from(k);
                        reglas.push(LineMm {
                            x1: cx + d,
                            y1: cy - largo,
                            x2: cx + d,
                            y2: cy + largo,
                        });
                        reglas.push(LineMm {
                            x1: cx - largo,
                            y1: cy + d,
                            x2: cx + largo,
                            y2: cy + d,
                        });
                    }
                    let etiqueta = format!("R{}C{}", slot.row + 1, slot.col + 1);
                    sup.draw_text(
                        Point::from_xy(pt(t.x + 2.0), pt(t.y + 5.0)),
                        fuente.clone(),
                        8.0,
                        &etiqueta,
                        false,
                        TextDirection::Auto,
                    );
                    sup.draw_text(
                        Point::from_xy(pt(cx + 1.0), pt(cy - 2.5)),
                        fuente.clone(),
                        4.5,
                        "+5 →",
                        false,
                        TextDirection::Auto,
                    );
                }
                dibujar_lineas(&mut sup, &contornos, (0, 0, 0), 0.6)?;
                dibujar_lineas(&mut sup, &reglas, (40, 40, 40), 0.3)?;
                let mut y = 4.0;
                for linea in &textos.instructions {
                    sup.draw_text(
                        Point::from_xy(pt(5.0), pt(y)),
                        fuente.clone(),
                        6.5,
                        linea,
                        false,
                        TextDirection::Auto,
                    );
                    y += 2.8;
                }
            }
            SheetSide::Back => {
                let mut cruces = Vec::new();
                for slot in &pagina.slots {
                    let t = slot.trim;
                    let (cx, cy) = (t.x + t.w / 2.0, t.y + t.h / 2.0);
                    cruces.push(LineMm {
                        x1: cx - 6.0,
                        y1: cy,
                        x2: cx + 6.0,
                        y2: cy,
                    });
                    cruces.push(LineMm {
                        x1: cx,
                        y1: cy - 6.0,
                        x2: cx,
                        y2: cy + 6.0,
                    });
                }
                dibujar_lineas(&mut sup, &cruces, (220, 0, 0), 1.0)?;
            }
        }
        sup.finish();
        hoja.finish();
    }
    doc.finish().map_err(|e| PdfError::Write(format!("{e:?}")))
}

/// Los textos de la hoja, en el idioma de la interfaz (los pone la pantalla).
#[derive(Debug, Clone)]
pub struct CalibrationTexts {
    pub title: String,
    pub instructions: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Content;
    use lopdf::Document as Lop;

    fn textos() -> CalibrationTexts {
        CalibrationTexts {
            title: "Calibración".into(),
            instructions: vec!["Imprime a doble cara.".into()],
        }
    }

    #[test]
    fn dos_paginas_y_las_cruces_del_dorso_siguen_el_plan() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let pdf = calibration(&s, &textos()).unwrap();
        let doc = Lop::load_mem(&pdf).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        let dorso = doc.get_pages()[&2];
        let ops = Content::decode(&doc.get_page_content(dorso))
            .unwrap()
            .operations;
        // 9 casillas × 2 segmentos, en un solo trazo.
        assert_eq!(ops.iter().filter(|o| o.operator == "m").count(), 18);
    }

    #[test]
    fn el_desplazamiento_de_dorso_mueve_la_cruz_en_espejo() {
        let x_cruz = |off: f64| {
            let mut s = PrintSettings {
                duplex: Duplex::LongEdge,
                ..Default::default()
            };
            s.row_offsets.back = vec![crate::model::Offset {
                x_mm: off,
                y_mm: 0.0,
            }];
            let d = Deck {
                id: "x".into(),
                name: String::new(),
                decklist_text: String::new(),
                cards: vec![DeckCard {
                    id: "a".into(),
                    source: CardSource::Custom,
                    front: None,
                    back: BackSpec::DeckDefault,
                    edits: CardEdits::default(),
                    locked: false,
                    meta: CardMeta::default(),
                }],
                settings: s,
                default_back: None,
                schema_version: 1,
            };
            let p = plan(&d).unwrap();
            p.pages[1].slots[0].trim.x
        };
        assert!((x_cruz(-0.5) - x_cruz(0.0) - 0.5).abs() < 1e-9);
    }
}
