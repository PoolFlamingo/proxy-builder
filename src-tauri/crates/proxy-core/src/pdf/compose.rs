//! Traducir un `PagePlan` a páginas de krilla.

use krilla::color::rgb;
use krilla::geom::{PathBuilder, Rect, Size, Transform};
use krilla::image::Image;
use krilla::metadata::Metadata;
use krilla::page::PageSettings;
use krilla::paint::{FillRule, LineCap, Stroke};
use krilla::surface::Surface;
use krilla::{Document, SerializeSettings};

use super::{color_hex, PdfError};
use crate::layout::{LineMm, Page, PagePlan, RectMm, Slot};
use crate::model::CutGuides;
use crate::units::{css_px_to_pt, mm_to_pt};

/// Metadatos del documento. Sin fecha: así el mismo deck da los mismos bytes.
#[derive(Debug, Clone)]
pub struct PdfMeta {
    pub title: String,
    /// Identificador estable (p. ej. la huella del plan y de las imágenes).
    pub document_id: String,
}

fn pt(mm: f64) -> f32 {
    mm_to_pt(mm) as f32
}

fn rect(r: &RectMm) -> Option<Rect> {
    Rect::from_xywh(pt(r.x), pt(r.y), pt(r.w), pt(r.h))
}

/// Dibuja una casilla: la imagen (que trae el sangrado completo) recortada a
/// la caja de sangrado de la casilla y, si toca, girada 180°.
pub(crate) fn dibujar_casilla(s: &mut Surface, slot: &Slot, img: Image) -> Result<(), PdfError> {
    let caja = rect(&slot.bleed).ok_or(PdfError::Geometry)?;
    let mut pb = PathBuilder::new();
    pb.push_rect(caja);
    let recorte = pb.finish().ok_or(PdfError::Geometry)?;
    let bf = slot.bleed_full_mm;
    let (x, y) = (pt(slot.trim.x - bf), pt(slot.trim.y - bf));
    let (w, h) = (pt(slot.trim.w + 2.0 * bf), pt(slot.trim.h + 2.0 * bf));
    let tam = Size::from_wh(w, h).ok_or(PdfError::Geometry)?;

    s.push_clip_path(&recorte, &FillRule::NonZero);
    let mut pilas = 1;
    if slot.rotate_180 {
        let cx = pt(slot.trim.x + slot.trim.w / 2.0);
        let cy = pt(slot.trim.y + slot.trim.h / 2.0);
        s.push_transform(&Transform::from_rotate_at(180.0, cx, cy));
        pilas += 1;
    }
    s.push_transform(&Transform::from_translate(x, y));
    pilas += 1;
    s.draw_image(img, tam);
    for _ in 0..pilas {
        s.pop();
    }
    Ok(())
}

pub(crate) fn dibujar_lineas(
    s: &mut Surface,
    lineas: &[LineMm],
    color: (u8, u8, u8),
    grosor_pt: f32,
) -> Result<(), PdfError> {
    if lineas.is_empty() {
        return Ok(());
    }
    let mut pb = PathBuilder::new();
    for l in lineas {
        pb.move_to(pt(l.x1), pt(l.y1));
        pb.line_to(pt(l.x2), pt(l.y2));
    }
    let Some(camino) = pb.finish() else {
        return Ok(());
    };
    s.set_fill(None);
    s.set_stroke(Some(Stroke {
        paint: rgb::Color::new(color.0, color.1, color.2).into(),
        width: grosor_pt.max(0.05),
        line_cap: LineCap::Butt,
        ..Stroke::default()
    }));
    s.draw_path(&camino);
    s.set_stroke(None);
    Ok(())
}

pub(crate) fn documento(meta: &PdfMeta) -> Document {
    let mut doc = Document::new_with(SerializeSettings {
        xmp_metadata: false,
        ..SerializeSettings::default()
    });
    doc.set_metadata(
        Metadata::new()
            .title(meta.title.clone())
            .producer("Proxy Builder".to_string())
            .creator("Proxy Builder".to_string())
            .document_id(meta.document_id.clone()),
    );
    doc
}

pub(crate) fn tam_pagina(p: &Page) -> Result<Size, PdfError> {
    Size::from_wh(pt(p.width_mm), pt(p.height_mm)).ok_or(PdfError::Geometry)
}

/// Escribe el PDF. `imagen(page, slot)` da la imagen ya preparada de cada
/// casilla (o `None` si va en blanco); `pagina_hecha(i)` se llama al terminar
/// cada página y puede pedir que se pare devolviendo `false`.
pub fn compose(
    plan: &PagePlan,
    guides: &CutGuides,
    meta: &PdfMeta,
    mut imagen: impl FnMut(&Page, &Slot) -> Result<Option<Image>, PdfError>,
    mut pagina_hecha: impl FnMut(usize) -> bool,
) -> Result<Vec<u8>, PdfError> {
    let mut doc = documento(meta);
    let color = color_hex(&guides.color);
    let grosor = css_px_to_pt(guides.width_px) as f32;
    for (i, p) in plan.pages.iter().enumerate() {
        let mut hoja = doc.start_page_with(PageSettings::new(tam_pagina(p)?));
        let mut s = hoja.surface();
        for slot in &p.slots {
            if let Some(img) = imagen(p, slot)? {
                dibujar_casilla(&mut s, slot, img)?;
            }
        }
        dibujar_lineas(&mut s, &p.guides, color, grosor)?;
        s.finish();
        hoja.finish();
        if !pagina_hecha(i) {
            return Err(PdfError::Cancelled);
        }
    }
    doc.finish().map_err(|e| PdfError::Write(format!("{e:?}")))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::imaging::encode;
    use crate::layout::plan;
    use crate::model::*;
    use image::{Rgba, RgbaImage};
    use krilla::Data;
    use lopdf::content::Content;
    use lopdf::{Document as Lop, Object};

    pub fn deck(n: usize, s: PrintSettings) -> Deck {
        let img = FaceImage {
            blob: "a".repeat(64),
            width: 10,
            height: 10,
            origin: ImageOrigin::Upload,
        };
        Deck {
            id: "d".into(),
            name: "Prueba".into(),
            decklist_text: String::new(),
            cards: (0..n)
                .map(|i| DeckCard {
                    id: format!("c{i}"),
                    source: CardSource::Custom {
                        scryfall_id: None,
                        externa: None,
                    },
                    front: Some(img.clone()),
                    back: BackSpec::DeckDefault,
                    edits: CardEdits::default(),
                    locked: false,
                    meta: CardMeta::default(),
                    design: None,
                    marca: None,
                })
                .collect(),
            settings: s,
            default_back: Some(img),
            schema_version: DECK_SCHEMA_VERSION,
        }
    }

    pub fn jpeg_de_prueba() -> Vec<u8> {
        encode::jpeg(&RgbaImage::from_pixel(64, 90, Rgba([200, 30, 30, 255])), 90).unwrap()
    }

    fn generar(d: &Deck) -> Vec<u8> {
        let p = plan(d).unwrap();
        let jpg = jpeg_de_prueba();
        let img = Image::from_jpeg(Data::from(jpg), true).unwrap();
        compose(
            &p,
            &d.settings.cut_guides,
            &PdfMeta {
                title: "t".into(),
                document_id: "id".into(),
            },
            |_, _| Ok(Some(img.clone())),
            |_| true,
        )
        .unwrap()
    }

    /// Para cada imagen dibujada, su rectángulo en la página (x, y desde
    /// ARRIBA, w, h), en puntos, a partir de las matrices del contenido.
    pub fn rects_de_imagenes(pdf: &[u8]) -> Vec<Vec<(f32, f32, f32, f32)>> {
        let doc = Lop::load_mem(pdf).unwrap();
        let mut out = Vec::new();
        for (_n, id) in doc.get_pages() {
            let alto = {
                let pagina = doc.get_object(id).unwrap().as_dict().unwrap();
                let mb = pagina.get(b"MediaBox").unwrap().as_array().unwrap();
                mb[3].as_float().unwrap()
            };
            let contenido = Content::decode(&doc.get_page_content(id)).unwrap();
            let mut pila: Vec<[f32; 6]> = vec![[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]];
            let mut rects = Vec::new();
            for op in contenido.operations {
                match op.operator.as_str() {
                    "q" => pila.push(*pila.last().unwrap()),
                    "Q" => {
                        pila.pop();
                    }
                    "cm" => {
                        let v: Vec<f32> = op
                            .operands
                            .iter()
                            .map(|o| match o {
                                Object::Integer(i) => *i as f32,
                                Object::Real(r) => *r,
                                _ => 0.0,
                            })
                            .collect();
                        let m = [v[0], v[1], v[2], v[3], v[4], v[5]];
                        let c = *pila.last().unwrap();
                        let n = [
                            m[0] * c[0] + m[1] * c[2],
                            m[0] * c[1] + m[1] * c[3],
                            m[2] * c[0] + m[3] * c[2],
                            m[2] * c[1] + m[3] * c[3],
                            m[4] * c[0] + m[5] * c[2] + c[4],
                            m[4] * c[1] + m[5] * c[3] + c[5],
                        ];
                        *pila.last_mut().unwrap() = n;
                    }
                    "Do" => {
                        let c = *pila.last().unwrap();
                        let esquinas = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)].map(
                            |(x, y): (f32, f32)| {
                                (c[0] * x + c[2] * y + c[4], c[1] * x + c[3] * y + c[5])
                            },
                        );
                        let xs = esquinas.map(|e| e.0);
                        let ys = esquinas.map(|e| e.1);
                        let (x0, x1) = (
                            xs.iter().cloned().fold(f32::MAX, f32::min),
                            xs.iter().cloned().fold(f32::MIN, f32::max),
                        );
                        let (y0, y1) = (
                            ys.iter().cloned().fold(f32::MAX, f32::min),
                            ys.iter().cloned().fold(f32::MIN, f32::max),
                        );
                        rects.push((x0, alto - y1, x1 - x0, y1 - y0));
                    }
                    _ => {}
                }
            }
            out.push(rects);
        }
        out
    }

    #[test]
    fn diez_cartas_dan_dos_paginas_a4_y_una_sola_imagen_incrustada() {
        let pdf = generar(&deck(10, PrintSettings::default()));
        let doc = Lop::load_mem(&pdf).unwrap();
        assert_eq!(doc.get_pages().len(), 2);
        let (_, id) = doc.get_pages().into_iter().next().unwrap();
        let mb = doc
            .get_object(id)
            .unwrap()
            .as_dict()
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        assert!((mb[2].as_float().unwrap() - 595.2756).abs() < 0.01);
        assert!((mb[3].as_float().unwrap() - 841.8898).abs() < 0.01);
        let imagenes: Vec<_> = doc
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|s| {
                s.dict.get(b"Subtype").ok().and_then(|n| n.as_name().ok())
                    == Some(b"Image".as_slice())
            })
            .collect();
        assert_eq!(imagenes.len(), 1, "las copias iguales se escriben una vez");
        let filtro = imagenes[0].dict.get(b"Filter").unwrap();
        let es_dct = match filtro {
            Object::Name(n) => n == b"DCTDecode",
            Object::Array(a) => a
                .iter()
                .any(|x| x.as_name().ok() == Some(b"DCTDecode".as_slice())),
            _ => false,
        };
        assert!(es_dct);
        assert_eq!(imagenes[0].content.len(), jpeg_de_prueba().len());
    }

    #[test]
    fn dos_veces_lo_mismo_dan_los_mismos_bytes() {
        let d = deck(
            12,
            PrintSettings {
                duplex: Duplex::LongEdge,
                ..Default::default()
            },
        );
        assert_eq!(generar(&d), generar(&d));
    }

    #[test]
    fn cada_imagen_cae_donde_dice_el_plan() {
        for (duplex, landscape) in [
            (Duplex::Off, false),
            (Duplex::LongEdge, false),
            (Duplex::ShortEdge, false),
            (Duplex::LongEdge, true),
        ] {
            let s = PrintSettings {
                duplex,
                landscape,
                ..Default::default()
            };
            let d = deck(9, s);
            let p = plan(&d).unwrap();
            let rects = rects_de_imagenes(&generar(&d));
            for (pagina, rs) in p.pages.iter().zip(&rects) {
                assert_eq!(pagina.slots.len(), rs.len());
                for (slot, r) in pagina.slots.iter().zip(rs) {
                    let bf = slot.bleed_full_mm;
                    let esperado = (
                        pt(slot.trim.x - bf),
                        pt(slot.trim.y - bf),
                        pt(slot.trim.w + 2.0 * bf),
                        pt(slot.trim.h + 2.0 * bf),
                    );
                    let ok = (r.0 - esperado.0).abs() < 0.01
                        && (r.1 - esperado.1).abs() < 0.01
                        && (r.2 - esperado.2).abs() < 0.01
                        && (r.3 - esperado.3).abs() < 0.01;
                    assert!(ok, "{duplex:?} {landscape}: {r:?} ≠ {esperado:?}");
                }
            }
        }
    }

    #[test]
    fn las_guias_se_dibujan_solo_en_el_frente() {
        let mut s = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        s.cut_guides.enabled = true;
        let pdf = generar(&deck(9, s));
        let doc = Lop::load_mem(&pdf).unwrap();
        let paginas: Vec<_> = doc.get_pages().into_values().collect();
        let trazos = |id| {
            Content::decode(&doc.get_page_content(id))
                .unwrap()
                .operations
                .iter()
                .filter(|o| o.operator == "S")
                .count()
        };
        assert!(trazos(paginas[0]) >= 1);
        assert_eq!(trazos(paginas[1]), 0);
    }

    /// Medida en milímetros de lo que sale en el PDF, no del plan: si alguien
    /// toca la geometría o las unidades, la carta impresa no puede cambiar de
    /// tamaño sin que esto falle.
    #[test]
    fn la_carta_sale_en_el_pdf_con_su_tamano_en_milimetros() {
        for card_size in [CardSize::MTG, CardSize::POKER] {
            for duplex in [Duplex::Off, Duplex::LongEdge] {
                let s = PrintSettings {
                    card_size,
                    duplex,
                    bleed_mm: 0.0,
                    ..Default::default()
                };
                let d = deck(9, s);
                for pagina in rects_de_imagenes(&generar(&d)) {
                    for (_, _, w, h) in pagina {
                        let mm = |pts: f32| f64::from(pts) * 25.4 / 72.0;
                        assert!((mm(w) - card_size.width_mm).abs() < 0.01, "{w} pt de ancho");
                        assert!((mm(h) - card_size.height_mm).abs() < 0.01, "{h} pt de alto");
                    }
                }
            }
        }
    }
}
