//! **El plan de páginas** (`PagePlan`): dónde va cada carta en cada cara de
//! cada hoja, en milímetros.
//!
//! 🛑 Es la ÚNICA geometría del producto. La vista previa la pinta y el PDF la
//! dibuja; ninguno de los dos calcula posiciones por su cuenta.
//!
//! ## Modos de DFC (tabla de verdad del plan, Step 1.3)
//!
//! | dúplex | pares DFC | qué sale para una DFC                         | reverso de una no-DFC |
//! |--------|-----------|-----------------------------------------------|-----------------------|
//! | no     | no        | solo la cara 0 (aviso)                        | —                     |
//! | no     | sí        | cara 0 y cara 1 en casillas seguidas          | —                     |
//! | sí     | no        | cara 0 delante, cara 1 en su reverso          | reverso del deck      |
//! | sí     | sí        | caras seguidas; ambas con el reverso del deck | reverso del deck      |
//!
//! `DfcFilter::HideDfc` quita las DFC del plan.
//!
//! ## Dúplex
//!
//! Tras cada hoja frontal va su hoja trasera (F1, B1, F2, B2…). Eje de
//! volteo:
//!
//! | orientación | borde      | reverso                                  |
//! |-------------|------------|------------------------------------------|
//! | vertical    | largo      | espejo horizontal (`x' = W − x − w`)      |
//! | vertical    | corto      | espejo vertical (`y' = H − y − h`) + 180° |
//! | apaisada    | largo      | espejo vertical + 180°                    |
//! | apaisada    | corto      | espejo horizontal                         |
//!
//! Los desplazamientos del reverso van en el marco frontal y se suman ANTES
//! del espejo (`model::RowOffsets`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::grid::{grid, Grid, RectMm};
use super::LayoutError;
use crate::model::{BackSpec, Deck, DfcFilter, Duplex, FaceImage, PrintSettings};

/// Qué cara de la hoja es una página.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SheetSide {
    Front,
    Back,
}

/// Qué cara de la carta enseña una casilla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum SlotFace {
    /// El frente de la carta (con sus retoques).
    Front,
    /// La cara trasera de una DFC puesta como carta aparte (pares DFC).
    DfcFace,
    /// El dorso de la carta en una hoja trasera.
    Back,
}

/// Una casilla ocupada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Slot {
    /// Posición en el orden de las casillas frontales de la página
    /// (fila a fila, de izquierda a derecha).
    pub index: u32,
    /// Fila FRONTAL a la que pertenece (también en las hojas traseras).
    pub row: u32,
    pub col: u32,
    pub card_id: String,
    pub face: SlotFace,
    /// `None` = casilla sin imagen (hueco, imagen pendiente o sin reverso).
    pub image: Option<FaceImage>,
    /// Si se aplican los retoques de la carta (solo al frente).
    pub apply_edits: bool,
    /// Rectángulo de corte.
    pub trim: RectMm,
    /// Hasta dónde se pinta la imagen (trim + sangrado recortado).
    pub bleed: RectMm,
    /// Sangrado completo que trae el render por cada lado (mm).
    pub bleed_full_mm: f64,
    pub rotate_180: bool,
}

/// Un segmento de guía de corte, en milímetros.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LineMm {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Page {
    /// Índice de la página en el PDF (0, 1, 2…).
    pub index: u32,
    /// Hoja física a la que pertenece (0, 1, 2…).
    pub sheet: u32,
    pub side: SheetSide,
    pub width_mm: f64,
    pub height_mm: f64,
    pub slots: Vec<Slot>,
    pub guides: Vec<LineMm>,
}

/// Algo que conviene avisar al usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum PlanWarning {
    /// Sin dúplex ni pares: estas DFC salen sin su cara trasera.
    DfcWithoutBack { count: u32 },
    /// Dúplex sin reverso por defecto: estas cartas salen con el dorso en blanco.
    MissingDefaultBack { count: u32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PagePlan {
    pub grid: Grid,
    /// Trims de todas las casillas frontales sin desplazamientos (para pintar
    /// los huecos vacíos de la última página).
    pub cells: Vec<RectMm>,
    pub pages: Vec<Page>,
    pub sheets: u32,
    /// Cartas colocadas (cada casilla frontal ocupada cuenta una).
    pub placed: u32,
    pub warnings: Vec<PlanWarning>,
}

/// Lo que va en una casilla frontal antes de colocarlo.
struct Item {
    card_id: String,
    face: SlotFace,
    image: Option<FaceImage>,
    apply_edits: bool,
    back: Option<FaceImage>,
    /// La carta tiene dorso que imprimir (los huecos en blanco no).
    has_back: bool,
}

/// Construye la secuencia de casillas frontales según la tabla de modos DFC.
fn items(deck: &Deck, warnings: &mut Vec<PlanWarning>) -> Vec<Item> {
    let s = &deck.settings;
    let duplex = s.duplex != Duplex::Off;
    let mut out = Vec::with_capacity(deck.cards.len());
    let mut dfc_sin_dorso = 0u32;
    let mut sin_reverso = 0u32;
    let reverso_deck = deck.default_back.clone();

    for c in &deck.cards {
        let dfc_face = match &c.back {
            BackSpec::Dfc { image } => Some(image.clone()),
            _ => None,
        };
        let es_dfc = dfc_face.is_some();
        if es_dfc && s.dfc_filter == DfcFilter::HideDfc {
            continue;
        }
        let es_hueco = matches!(c.source, crate::model::CardSource::Blank);
        let dorso_normal = match &c.back {
            BackSpec::Custom { image } => Some(image.clone()),
            _ => reverso_deck.clone(),
        };

        if es_dfc && s.interleave_dfc {
            for (face, image, edits) in [
                (SlotFace::Front, c.front.clone(), true),
                (SlotFace::DfcFace, dfc_face.clone(), false),
            ] {
                if duplex && dorso_normal.is_none() {
                    sin_reverso += 1;
                }
                out.push(Item {
                    card_id: c.id.clone(),
                    face,
                    image,
                    apply_edits: edits,
                    back: dorso_normal.clone(),
                    has_back: true,
                });
            }
            continue;
        }

        let back = if es_dfc { dfc_face } else { dorso_normal };
        if es_dfc && !duplex {
            dfc_sin_dorso += 1;
        }
        if duplex && !es_hueco && back.is_none() {
            sin_reverso += 1;
        }
        out.push(Item {
            card_id: c.id.clone(),
            face: SlotFace::Front,
            image: c.front.clone(),
            apply_edits: true,
            back,
            has_back: !es_hueco,
        });
    }
    if dfc_sin_dorso > 0 {
        warnings.push(PlanWarning::DfcWithoutBack {
            count: dfc_sin_dorso,
        });
    }
    if sin_reverso > 0 {
        warnings.push(PlanWarning::MissingDefaultBack { count: sin_reverso });
    }
    out
}

/// El eje de volteo del reverso: `true` = espejo horizontal (mirror x), sin
/// giro; `false` = espejo vertical (mirror y) con giro de 180°.
fn espejo_horizontal(duplex: Duplex, landscape: bool) -> bool {
    match (duplex, landscape) {
        (Duplex::LongEdge, false) | (Duplex::ShortEdge, true) => true,
        (Duplex::ShortEdge, false) | (Duplex::LongEdge, true) => false,
        (Duplex::Off, _) => true,
    }
}

/// Lados del sangrado (izquierda, derecha, arriba, abajo) de la casilla
/// `(row, col)` en el marco frontal: completo hacia el exterior de la rejilla,
/// `min(bleed, gap/2)` hacia otra carta.
fn lados_de_sangrado(g: &Grid, row: u32, col: u32, bleed: f64, gap: f64) -> [f64; 4] {
    let interior = bleed.min(gap / 2.0);
    let l = if col == 0 { bleed } else { interior };
    let r = if col + 1 == g.cols { bleed } else { interior };
    let t = if row == 0 { bleed } else { interior };
    let b = if row + 1 == g.rows { bleed } else { interior };
    [l, r, t, b]
}

fn ampliar(trim: RectMm, [l, r, t, b]: [f64; 4], ancho: f64, alto: f64) -> RectMm {
    let x0 = (trim.x - l).max(0.0);
    let y0 = (trim.y - t).max(0.0);
    let x1 = (trim.right() + r).min(ancho);
    let y1 = (trim.bottom() + b).min(alto);
    RectMm {
        x: x0,
        y: y0,
        w: (x1 - x0).max(0.0),
        h: (y1 - y0).max(0.0),
    }
}

/// Guías de corte de una página frontal: en los márgenes, a la altura de cada
/// línea de corte. Las de arriba siguen a la primera fila, las de abajo a la
/// última, y las laterales a cada fila (que puede llevar su desplazamiento).
fn guias(
    slots_por_fila: &[Vec<RectMm>],
    ancho: f64,
    alto: f64,
    offset: f64,
    largo: f64,
) -> Vec<LineMm> {
    let mut out = Vec::new();
    if largo <= 0.0 {
        return out;
    }
    let clamp_y = |y: f64| y.clamp(0.0, alto);
    let clamp_x = |x: f64| x.clamp(0.0, ancho);
    let filas: Vec<&Vec<RectMm>> = slots_por_fila.iter().filter(|f| !f.is_empty()).collect();
    let (Some(primera), Some(ultima)) = (filas.first(), filas.last()) else {
        return out;
    };
    let arriba = primera.iter().map(|r| r.y).fold(f64::INFINITY, f64::min);
    let abajo = ultima
        .iter()
        .map(|r| r.bottom())
        .fold(f64::NEG_INFINITY, f64::max);
    let mut verticales = |fila: &Vec<RectMm>, y_desde: f64, hacia_arriba: bool| {
        for r in fila {
            for x in [r.x, r.right()] {
                let (y1, y2) = if hacia_arriba {
                    (clamp_y(y_desde - offset - largo), clamp_y(y_desde - offset))
                } else {
                    (clamp_y(y_desde + offset), clamp_y(y_desde + offset + largo))
                };
                if y2 > y1 {
                    out.push(LineMm {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                    });
                }
            }
        }
    };
    verticales(primera, arriba, true);
    verticales(ultima, abajo, false);
    for fila in &filas {
        let izq = fila.iter().map(|r| r.x).fold(f64::INFINITY, f64::min);
        let der = fila
            .iter()
            .map(|r| r.right())
            .fold(f64::NEG_INFINITY, f64::max);
        let y_top = fila[0].y;
        let y_bot = fila[0].bottom();
        for y in [y_top, y_bot] {
            let (a1, a2) = (clamp_x(izq - offset - largo), clamp_x(izq - offset));
            if a2 > a1 {
                out.push(LineMm {
                    x1: a1,
                    y1: y,
                    x2: a2,
                    y2: y,
                });
            }
            let (b1, b2) = (clamp_x(der + offset), clamp_x(der + offset + largo));
            if b2 > b1 {
                out.push(LineMm {
                    x1: b1,
                    y1: y,
                    x2: b2,
                    y2: y,
                });
            }
        }
    }
    out
}

/// Calcula el plan de páginas de un deck.
pub fn plan(deck: &Deck) -> Result<PagePlan, LayoutError> {
    let s: &PrintSettings = &deck.settings;
    let (ancho, alto) = s.paper.oriented_mm(s.landscape);
    let (cw, ch) = (s.card_size.width_mm, s.card_size.height_mm);
    let g = grid(ancho, alto, cw, ch, s.gap_mm, s.min_margin_mm, s.bleed_mm)?;
    let cap = g.capacity() as usize;

    let cells: Vec<RectMm> = (0..g.rows)
        .flat_map(|r| (0..g.cols).map(move |c| (r, c)))
        .map(|(r, c)| g.cell(r, c, cw, ch, s.gap_mm))
        .collect();

    let mut warnings = Vec::new();
    let items = items(deck, &mut warnings);
    let duplex = s.duplex != Duplex::Off;
    let espejo_x = espejo_horizontal(s.duplex, s.landscape);

    let mut pages = Vec::new();
    let hojas: Vec<&[Item]> = items.chunks(cap.max(1)).collect();
    for (hoja, trozo) in hojas.iter().enumerate() {
        let hoja = hoja as u32;
        let mut frontales = Vec::with_capacity(trozo.len());
        let mut por_fila: Vec<Vec<RectMm>> = vec![Vec::new(); g.rows as usize];
        let mut traseras = Vec::new();

        for (i, it) in trozo.iter().enumerate() {
            let (row, col) = ((i as u32) / g.cols, (i as u32) % g.cols);
            let base = cells[i];
            let lados = lados_de_sangrado(&g, row, col, s.bleed_mm, s.gap_mm);

            // Frente.
            let off = s.row_offsets.front_row(row as usize);
            let trim = RectMm {
                x: base.x + off.x_mm,
                y: base.y + off.y_mm,
                ..base
            };
            por_fila[row as usize].push(trim);
            frontales.push(Slot {
                index: i as u32,
                row,
                col,
                card_id: it.card_id.clone(),
                face: it.face,
                image: it.image.clone(),
                apply_edits: it.apply_edits,
                trim,
                bleed: ampliar(trim, lados, ancho, alto),
                bleed_full_mm: s.bleed_mm,
                rotate_180: false,
            });

            // Dorso.
            if duplex && it.has_back {
                let off = s.row_offsets.back_row(row as usize);
                let pre = RectMm {
                    x: base.x + off.x_mm,
                    y: base.y + off.y_mm,
                    ..base
                };
                let [l, r, t, b] = if s.bleed_on_backs { lados } else { [0.0; 4] };
                let (trim_b, lados_b, giro) = if espejo_x {
                    (
                        RectMm {
                            x: ancho - pre.x - pre.w,
                            ..pre
                        },
                        [r, l, t, b],
                        false,
                    )
                } else {
                    (
                        RectMm {
                            y: alto - pre.y - pre.h,
                            ..pre
                        },
                        [l, r, b, t],
                        true,
                    )
                };
                let bleed_full = if s.bleed_on_backs { s.bleed_mm } else { 0.0 };
                traseras.push(Slot {
                    index: i as u32,
                    row,
                    col,
                    card_id: it.card_id.clone(),
                    face: SlotFace::Back,
                    image: it.back.clone(),
                    apply_edits: false,
                    trim: trim_b,
                    bleed: ampliar(trim_b, lados_b, ancho, alto),
                    bleed_full_mm: bleed_full,
                    rotate_180: giro,
                });
            }
        }

        let guides = if s.cut_guides.enabled {
            guias(
                &por_fila,
                ancho,
                alto,
                s.cut_guides.offset_mm,
                s.cut_guides.length_mm,
            )
        } else {
            Vec::new()
        };

        pages.push(Page {
            index: pages.len() as u32,
            sheet: hoja,
            side: SheetSide::Front,
            width_mm: ancho,
            height_mm: alto,
            slots: frontales,
            guides,
        });
        if duplex {
            pages.push(Page {
                index: pages.len() as u32,
                sheet: hoja,
                side: SheetSide::Back,
                width_mm: ancho,
                height_mm: alto,
                slots: traseras,
                // Las guías van solo en el frente: en el dorso, al trasluz, se
                // confundirían con las del otro lado.
                guides: Vec::new(),
            });
        }
    }

    Ok(PagePlan {
        sheets: hojas.len() as u32,
        placed: items.len() as u32,
        grid: g,
        cells,
        pages,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::paper::PaperFormat;

    fn img(n: &str) -> FaceImage {
        FaceImage {
            blob: n.repeat(64).chars().take(64).collect(),
            width: 745,
            height: 1040,
            origin: ImageOrigin::ScryfallPng,
        }
    }

    fn carta(id: &str) -> DeckCard {
        DeckCard {
            id: id.into(),
            source: CardSource::Scryfall {
                scryfall_id: id.into(),
            },
            front: Some(img("a")),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
        }
    }

    fn dfc(id: &str) -> DeckCard {
        DeckCard {
            back: BackSpec::Dfc { image: img("d") },
            ..carta(id)
        }
    }

    fn deck(cards: Vec<DeckCard>, s: PrintSettings) -> Deck {
        Deck {
            id: "d".into(),
            name: "t".into(),
            decklist_text: String::new(),
            cards,
            settings: s,
            default_back: Some(img("b")),
            schema_version: DECK_SCHEMA_VERSION,
        }
    }

    fn n(k: usize) -> Vec<DeckCard> {
        (0..k).map(|i| carta(&format!("c{i}"))).collect()
    }

    fn cerca(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn diez_cartas_en_a4_son_dos_hojas() {
        let p = plan(&deck(n(10), PrintSettings::default())).unwrap();
        assert_eq!(p.sheets, 2);
        assert_eq!(p.pages.len(), 2);
        assert_eq!(p.pages[0].slots.len(), 9);
        assert_eq!(p.pages[1].slots.len(), 1);
        assert!(p.pages.iter().all(|pg| pg.side == SheetSide::Front));
        assert!(cerca(p.pages[0].slots[0].trim.x, 8.5) && cerca(p.pages[0].slots[0].trim.y, 14.5));
    }

    #[test]
    fn en_duplex_las_paginas_van_frente_dorso_frente_dorso() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let p = plan(&deck(n(10), s)).unwrap();
        let lados: Vec<SheetSide> = p.pages.iter().map(|pg| pg.side).collect();
        use SheetSide::*;
        assert_eq!(lados, vec![Front, Back, Front, Back]);
    }

    #[test]
    fn borde_largo_vertical_refleja_las_columnas() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let p = plan(&deck(n(9), s)).unwrap();
        let frente = &p.pages[0].slots[0];
        let dorso = &p.pages[1].slots[0];
        // (fila 0, col 0) → su dorso cae donde está la columna 2.
        assert!(cerca(dorso.trim.x, p.cells[2].x), "{dorso:?}");
        assert!(cerca(dorso.trim.y, frente.trim.y));
        assert!(!dorso.rotate_180);
        assert_eq!(dorso.image, Some(img("b")));
    }

    #[test]
    fn borde_corto_vertical_refleja_las_filas_y_gira() {
        let s = PrintSettings {
            duplex: Duplex::ShortEdge,
            ..Default::default()
        };
        let p = plan(&deck(n(9), s)).unwrap();
        let dorso = &p.pages[1].slots[0];
        assert!(cerca(dorso.trim.y, p.cells[6].y), "{dorso:?}");
        assert!(cerca(dorso.trim.x, p.cells[0].x));
        assert!(dorso.rotate_180);
    }

    #[test]
    fn apaisado_borde_largo_gira_y_borde_corto_no() {
        for (d, gira) in [(Duplex::LongEdge, true), (Duplex::ShortEdge, false)] {
            let s = PrintSettings {
                duplex: d,
                landscape: true,
                ..Default::default()
            };
            let p = plan(&deck(n(1), s)).unwrap();
            assert_eq!(p.pages[1].slots[0].rotate_180, gira, "{d:?}");
        }
    }

    #[test]
    fn el_desplazamiento_del_dorso_se_mide_desde_el_frente() {
        // Borde largo: +1 mm en x (marco frontal) mueve el dorso −1 mm en la página trasera.
        let mut s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let sin = plan(&deck(n(1), s.clone())).unwrap().pages[1].slots[0].trim;
        s.row_offsets.back = vec![Offset {
            x_mm: 1.0,
            y_mm: 0.0,
        }];
        let con = plan(&deck(n(1), s)).unwrap().pages[1].slots[0].trim;
        assert!(cerca(con.x, sin.x - 1.0), "{sin:?} → {con:?}");

        // Borde corto: +1 mm en y mueve el dorso −1 mm en y.
        let mut s = PrintSettings {
            duplex: Duplex::ShortEdge,
            ..Default::default()
        };
        let sin = plan(&deck(n(1), s.clone())).unwrap().pages[1].slots[0].trim;
        s.row_offsets.back = vec![Offset {
            x_mm: 0.0,
            y_mm: 1.0,
        }];
        let con = plan(&deck(n(1), s)).unwrap().pages[1].slots[0].trim;
        assert!(cerca(con.y, sin.y - 1.0), "{sin:?} → {con:?}");
    }

    #[test]
    fn el_desplazamiento_del_frente_mueve_solo_su_fila() {
        let s = PrintSettings {
            row_offsets: RowOffsets {
                front: vec![
                    Offset::default(),
                    Offset {
                        x_mm: 0.5,
                        y_mm: -0.25,
                    },
                ],
                back: vec![],
            },
            ..Default::default()
        };
        let p = plan(&deck(n(9), s)).unwrap();
        let sl = &p.pages[0].slots;
        assert!(cerca(sl[0].trim.x, p.cells[0].x));
        assert!(cerca(sl[3].trim.x, p.cells[3].x + 0.5));
        assert!(cerca(sl[3].trim.y, p.cells[3].y - 0.25));
        assert!(cerca(sl[6].trim.x, p.cells[6].x));
    }

    #[test]
    fn dos_sangrados_contiguos_se_tocan_sin_pisarse() {
        let p = plan(&deck(n(9), PrintSettings::default())).unwrap();
        let s = &p.pages[0].slots;
        assert!(cerca(s[0].bleed.right(), s[1].bleed.x));
        assert!(cerca(s[0].bleed.bottom(), s[3].bleed.y));
        // Hacia fuera, el sangrado completo.
        assert!(cerca(s[0].bleed.x, s[0].trim.x - 1.0));
    }

    #[test]
    fn sin_hueco_no_hay_sangrado_interior() {
        let s = PrintSettings {
            gap_mm: 0.0,
            ..Default::default()
        };
        let p = plan(&deck(n(9), s)).unwrap();
        let sl = &p.pages[0].slots;
        assert!(cerca(sl[0].bleed.right(), sl[0].trim.right()));
        assert!(cerca(sl[0].bleed.x, sl[0].trim.x - 1.0));
    }

    #[test]
    fn ningun_sangrado_sale_del_papel_en_ningun_formato() {
        for paper in PaperFormat::TODOS {
            for landscape in [false, true] {
                for duplex in [Duplex::Off, Duplex::LongEdge, Duplex::ShortEdge] {
                    let s = PrintSettings {
                        paper,
                        landscape,
                        duplex,
                        bleed_mm: 3.0,
                        ..Default::default()
                    };
                    let p = plan(&deck(n(20), s)).unwrap();
                    for pg in &p.pages {
                        for sl in &pg.slots {
                            let b = sl.bleed;
                            assert!(b.x >= -1e-9 && b.y >= -1e-9, "{paper:?}");
                            assert!(b.right() <= pg.width_mm + 1e-9, "{paper:?}");
                            assert!(b.bottom() <= pg.height_mm + 1e-9, "{paper:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn el_espejo_aplicado_dos_veces_vuelve_al_sitio() {
        for (duplex, landscape) in [
            (Duplex::LongEdge, false),
            (Duplex::ShortEdge, false),
            (Duplex::LongEdge, true),
            (Duplex::ShortEdge, true),
        ] {
            let s = PrintSettings {
                duplex,
                landscape,
                ..Default::default()
            };
            let p = plan(&deck(n(9), s.clone())).unwrap();
            let (w, h) = s.paper.oriented_mm(landscape);
            for (f, b) in p.pages[0].slots.iter().zip(&p.pages[1].slots) {
                let vuelta = if espejo_horizontal(duplex, landscape) {
                    RectMm {
                        x: w - b.trim.x - b.trim.w,
                        ..b.trim
                    }
                } else {
                    RectMm {
                        y: h - b.trim.y - b.trim.h,
                        ..b.trim
                    }
                };
                assert!(cerca(vuelta.x, f.trim.x) && cerca(vuelta.y, f.trim.y));
            }
        }
    }

    // ── Tabla de modos DFC ──────────────────────────────────────────────

    #[test]
    fn dfc_sin_duplex_ni_pares_sale_solo_el_frente_y_avisa() {
        let p = plan(&deck(vec![dfc("x"), carta("y")], PrintSettings::default())).unwrap();
        assert_eq!(p.placed, 2);
        assert_eq!(p.pages.len(), 1);
        assert!(p
            .warnings
            .contains(&PlanWarning::DfcWithoutBack { count: 1 }));
    }

    #[test]
    fn dfc_con_pares_sin_duplex_salen_las_dos_caras_seguidas() {
        let s = PrintSettings {
            interleave_dfc: true,
            ..Default::default()
        };
        let p = plan(&deck(vec![dfc("x"), carta("y")], s)).unwrap();
        let sl = &p.pages[0].slots;
        assert_eq!(sl.len(), 3);
        assert_eq!(
            (sl[0].face, sl[1].face, sl[2].face),
            (SlotFace::Front, SlotFace::DfcFace, SlotFace::Front)
        );
        assert_eq!(sl[1].image, Some(img("d")));
        assert!(!sl[1].apply_edits);
        assert!(p.warnings.is_empty());
    }

    #[test]
    fn dfc_con_duplex_lleva_su_cara_trasera_al_dorso() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let p = plan(&deck(vec![dfc("x"), carta("y")], s)).unwrap();
        let dorsos = &p.pages[1].slots;
        assert_eq!(dorsos[0].image, Some(img("d")));
        assert_eq!(dorsos[1].image, Some(img("b")));
    }

    #[test]
    fn dfc_con_duplex_y_pares_lleva_el_reverso_del_deck_en_las_dos() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            interleave_dfc: true,
            ..Default::default()
        };
        let p = plan(&deck(vec![dfc("x")], s)).unwrap();
        assert_eq!(p.pages[0].slots.len(), 2);
        let dorsos = &p.pages[1].slots;
        assert_eq!(dorsos.len(), 2);
        assert!(dorsos.iter().all(|d| d.image == Some(img("b"))));
    }

    #[test]
    fn ocultar_dfc_las_quita_del_plan() {
        let s = PrintSettings {
            dfc_filter: DfcFilter::HideDfc,
            ..Default::default()
        };
        let p = plan(&deck(vec![dfc("x"), carta("y")], s)).unwrap();
        assert_eq!(p.placed, 1);
        assert_eq!(p.pages[0].slots[0].card_id, "y");
    }

    #[test]
    fn duplex_sin_reverso_del_deck_avisa_y_deja_el_dorso_en_blanco() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let mut d = deck(n(2), s);
        d.default_back = None;
        let p = plan(&d).unwrap();
        assert!(p
            .warnings
            .contains(&PlanWarning::MissingDefaultBack { count: 2 }));
        assert!(p.pages[1].slots.iter().all(|s| s.image.is_none()));
    }

    #[test]
    fn las_guias_solo_van_en_el_frente_y_dentro_del_papel() {
        let mut s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        s.cut_guides.enabled = true;
        let p = plan(&deck(n(9), s)).unwrap();
        let g = &p.pages[0].guides;
        // 3 columnas × 2 bordes × (arriba + abajo) + 3 filas × 2 bordes × (izq + der)
        assert_eq!(g.len(), 12 + 12);
        assert!(p.pages[1].guides.is_empty());
        for l in g {
            assert!(l.x1 >= 0.0 && l.x2 <= 210.0 && l.y1 >= 0.0 && l.y2 <= 297.0);
        }
        // Las de arriba terminan justo encima del borde de la primera fila.
        assert!(g.iter().any(|l| cerca(l.y2, 14.5) && cerca(l.y1, 9.5)));
    }

    #[test]
    fn un_hueco_en_blanco_ocupa_casilla_pero_no_lleva_dorso() {
        let s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        let mut cartas = n(2);
        cartas.insert(
            1,
            DeckCard {
                source: CardSource::Blank,
                front: None,
                ..carta("hueco")
            },
        );
        let p = plan(&deck(cartas, s)).unwrap();
        assert_eq!(p.pages[0].slots.len(), 3);
        assert_eq!(p.pages[1].slots.len(), 2);
        assert!(p.warnings.is_empty());
    }
}
