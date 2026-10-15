//! **Pintar diseños en la app**: el diseño efectivo (con su colección), su
//! huella y el render con los recursos de la app (símbolos, letras, packs e
//! imágenes del almacén).

use std::collections::HashMap;

use proxy_core::diseno::efectivo::{efectivo, huella, DisenoEfectivo};
use proxy_core::diseno::modelo::{CardDesign, Collection, DesignLang};
use proxy_core::diseno::render::{render, render_capa, CapaVista, Recursos, Render, Salida};
use proxy_core::imaging::pipeline::DisenoRef;
use proxy_core::layout::{Slot, SlotFace};
use proxy_core::model::{BackSpec, CardSize, Deck, DeckCard};

use super::recursos::ImagenesDelAlmacen;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::colecciones;

/// Las colecciones, por id (se leen una vez por trabajo).
pub fn colecciones_por_id(state: &AppState) -> AppResult<HashMap<String, Collection>> {
    Ok(colecciones::list(&state.db)?
        .into_iter()
        .map(|c| (c.id.clone(), c))
        .collect())
}

pub fn efectivo_de(d: &CardDesign, cols: &HashMap<String, Collection>) -> DisenoEfectivo {
    let c = d.collection.as_ref().and_then(|id| cols.get(id));
    efectivo(d, c)
}

/// La huella de un render (diseño efectivo + recursos + cara + idioma).
pub fn huella_de(
    state: &AppState,
    e: &DisenoEfectivo,
    cara: u32,
    lang: DesignLang,
) -> AppResult<String> {
    let inst = state.diseno.instantanea(&e.frame.pack)?;
    Ok(huella(e, lang, cara as usize, &inst.huella))
}

/// Pinta una cara de un diseño.
pub fn pintar(
    state: &AppState,
    e: &DisenoEfectivo,
    cara: u32,
    lang: DesignLang,
    card: CardSize,
    ancho: u32,
    salida: Salida,
) -> AppResult<Render> {
    let inst = state.diseno.instantanea(&e.frame.pack)?;
    let imagenes = ImagenesDelAlmacen {
        db: &state.db,
        blobs: &state.blobs,
        cache: &state.imagenes,
    };
    let rec = Recursos {
        pack: &inst.pack,
        simbolos: &inst.simbolos,
        imagenes: &imagenes,
    };
    state
        .diseno
        .con_motor(|m| render(m, e, cara as usize, lang, &rec, card, ancho, salida))?
        .map_err(|e| AppError::new(ErrorCode::Image).with("detalle", e))
}

/// La carta sin la ilustración y dónde va (ver [`render_capa`]); `None` en
/// las compuestas y las grandes.
pub fn pintar_capa(
    state: &AppState,
    e: &DisenoEfectivo,
    cara: u32,
    lang: DesignLang,
    ancho: u32,
) -> AppResult<Option<CapaVista>> {
    let inst = state.diseno.instantanea(&e.frame.pack)?;
    let imagenes = ImagenesDelAlmacen {
        db: &state.db,
        blobs: &state.blobs,
        cache: &state.imagenes,
    };
    let rec = Recursos {
        pack: &inst.pack,
        simbolos: &inst.simbolos,
        imagenes: &imagenes,
    };
    state
        .diseno
        .con_motor(|m| render_capa(m, e, cara as usize, lang, &rec, CardSize::MTG, ancho))?
        .map_err(|e| AppError::new(ErrorCode::Image).with("detalle", e))
}

/// Qué cara del diseño va en una casilla del plan, si va alguna: el frente
/// de la carta, o la otra cara en las DFC diseñadas (como dorso o como carta
/// aparte).
pub fn cara_de_casilla(card: &DeckCard, slot: &Slot) -> Option<u32> {
    let d = card.design.as_ref()?;
    match slot.face {
        SlotFace::Front => Some(0),
        SlotFace::DfcFace | SlotFace::Back => match &card.back {
            BackSpec::Dfc { .. } if d.faces.len() >= 2 => Some(1),
            _ => None,
        },
    }
}

/// Los diseños de un deck listos para exportar: por id de carta, su diseño
/// efectivo; y por huella, lo que hay que pintar.
pub struct DisenosDelDeck {
    pub efectivos: HashMap<String, DisenoEfectivo>,
    pub lang: DesignLang,
}

impl DisenosDelDeck {
    pub fn de(state: &AppState, deck: &Deck) -> AppResult<DisenosDelDeck> {
        let cols = if deck.cards.iter().any(|c| c.design.is_some()) {
            colecciones_por_id(state)?
        } else {
            HashMap::new()
        };
        Ok(DisenosDelDeck {
            efectivos: deck
                .cards
                .iter()
                .filter_map(|c| {
                    c.design
                        .as_ref()
                        .map(|d| (c.id.clone(), efectivo_de(d, &cols)))
                })
                .collect(),
            lang: deck.settings.design_lang,
        })
    }

    /// La referencia de diseño de una casilla, si es de una carta diseñada.
    pub fn referencia(
        &self,
        state: &AppState,
        card: &DeckCard,
        slot: &Slot,
    ) -> AppResult<Option<DisenoRef>> {
        let Some(cara) = cara_de_casilla(card, slot) else {
            return Ok(None);
        };
        let Some(e) = self.efectivos.get(&card.id) else {
            return Ok(None);
        };
        Ok(Some(DisenoRef {
            huella: huella_de(state, e, cara, self.lang)?,
            cara,
            lang: self.lang,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;
    use crate::store::blobs::tests::png;
    use crate::store::colecciones::tests::nueva;
    use proxy_core::diseno::modelo::{ByLang, CardData, FaceData};
    use proxy_core::layout::RectMm;
    use proxy_core::model::{
        CardEdits, CardMeta, CardSource, FaceImage, ImageOrigin, PrintSettings, DECK_SCHEMA_VERSION,
    };

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    fn imagen(s: &AppState, semilla: u8) -> FaceImage {
        s.blobs
            .put_bytes(&s.db, &png(63, 88, semilla), ImageOrigin::Upload)
            .unwrap()
    }

    fn cara(nombre: &str) -> FaceData {
        FaceData {
            name: nombre.into(),
            mana_cost: "{1}".into(),
            type_line: ByLang {
                es: "Artefacto".into(),
                en: "Artifact".into(),
            },
            ..FaceData::default()
        }
    }

    /// Una carta del deck; con `dfc`, de dos caras y con su dorso propio.
    fn carta(s: &AppState, id: &str, dfc: bool, disenada: bool) -> DeckCard {
        let mut c = DeckCard {
            id: id.into(),
            source: CardSource::Scryfall {
                scryfall_id: "x".into(),
            },
            front: Some(imagen(s, 1)),
            back: if dfc {
                BackSpec::Dfc {
                    image: imagen(s, 2),
                }
            } else {
                BackSpec::DeckDefault
            },
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
            design: None,
        };
        if disenada {
            let datos = CardData {
                scryfall_id: "x".into(),
                layout: if dfc { "transform" } else { "normal" }.into(),
                rarity: "common".into(),
                set: "tst".into(),
                collector_number: "1".into(),
                faces: if dfc {
                    vec![cara("Delante"), cara("Detrás")]
                } else {
                    vec![cara("Sola")]
                },
                ..CardData::default()
            };
            c.design = Some(crate::diseno::borrador::montar(&c, datos, None, &[]).design);
        }
        c
    }

    fn casilla(face: SlotFace) -> Slot {
        let r = RectMm {
            x: 0.0,
            y: 0.0,
            w: 63.0,
            h: 88.0,
        };
        Slot {
            index: 0,
            row: 0,
            col: 0,
            card_id: "c".into(),
            face,
            image: None,
            apply_edits: true,
            trim: r,
            bleed: r,
            bleed_full_mm: 0.0,
            rotate_180: false,
        }
    }

    fn deck(cards: Vec<DeckCard>, lang: DesignLang) -> Deck {
        Deck {
            id: "d".into(),
            name: "Prueba".into(),
            decklist_text: String::new(),
            cards,
            settings: PrintSettings {
                design_lang: lang,
                ..PrintSettings::default()
            },
            default_back: None,
            schema_version: DECK_SCHEMA_VERSION,
        }
    }

    // ── Qué cara va en cada casilla ──

    #[test]
    fn una_carta_sin_diseno_no_tiene_cara_de_diseno() {
        let (_dir, s) = estado();
        let c = carta(&s, "c", true, false);
        for f in [SlotFace::Front, SlotFace::DfcFace, SlotFace::Back] {
            assert_eq!(cara_de_casilla(&c, &casilla(f)), None);
        }
    }

    #[test]
    fn el_frente_es_la_cara_cero_y_el_dorso_la_otra_si_es_dfc() {
        let (_dir, s) = estado();
        let c = carta(&s, "c", true, true);
        assert_eq!(cara_de_casilla(&c, &casilla(SlotFace::Front)), Some(0));
        assert_eq!(cara_de_casilla(&c, &casilla(SlotFace::Back)), Some(1));
        assert_eq!(cara_de_casilla(&c, &casilla(SlotFace::DfcFace)), Some(1));
    }

    #[test]
    fn el_reverso_del_deck_no_es_parte_del_diseno() {
        let (_dir, s) = estado();
        let c = carta(&s, "c", false, true);
        assert_eq!(cara_de_casilla(&c, &casilla(SlotFace::Front)), Some(0));
        assert_eq!(cara_de_casilla(&c, &casilla(SlotFace::Back)), None);
    }

    #[test]
    fn un_dorso_dfc_con_un_diseno_de_una_cara_no_se_pinta() {
        let (_dir, s) = estado();
        let mut c = carta(&s, "c", false, true);
        c.back = BackSpec::Dfc {
            image: imagen(&s, 3),
        };
        assert_eq!(cara_de_casilla(&c, &casilla(SlotFace::Back)), None);
    }

    // ── Diseño efectivo y huella ──

    #[test]
    fn el_diseno_efectivo_busca_su_coleccion_por_id() {
        let (_dir, s) = estado();
        let col = crate::store::colecciones::save(&s.db, &nueva("Tensura")).unwrap();
        let cols = colecciones_por_id(&s).unwrap();
        assert_eq!(cols.len(), 1);
        let mut d = carta(&s, "c", false, true).design.unwrap();
        d.collection = Some(col.id.clone());
        let e = efectivo_de(&d, &cols);
        assert!(!e.coleccion_perdida);
        assert_eq!(e.pie.set_code, "RTS");
        // Una colección borrada: valores de fábrica y aviso.
        d.collection = Some("borrada".into());
        let e = efectivo_de(&d, &cols);
        assert!(e.coleccion_perdida);
        assert_eq!(e.pie.set_code, "TST");
    }

    #[test]
    fn la_huella_cambia_con_la_cara_y_el_idioma() {
        let (_dir, s) = estado();
        let d = carta(&s, "c", true, true).design.unwrap();
        let e = efectivo_de(&d, &HashMap::new());
        let h = |cara, lang| huella_de(&s, &e, cara, lang).unwrap();
        assert_eq!(h(0, DesignLang::Es), h(0, DesignLang::Es));
        assert_ne!(h(0, DesignLang::Es), h(1, DesignLang::Es));
        assert_ne!(h(0, DesignLang::Es), h(0, DesignLang::En));
    }

    #[test]
    fn los_disenos_del_deck_solo_son_los_de_las_cartas_disenadas() {
        let (_dir, s) = estado();
        let d = deck(
            vec![
                carta(&s, "a", false, true),
                carta(&s, "b", false, false),
                carta(&s, "c", true, true),
            ],
            DesignLang::En,
        );
        let dd = DisenosDelDeck::de(&s, &d).unwrap();
        let mut ids: Vec<_> = dd.efectivos.keys().cloned().collect();
        ids.sort();
        assert_eq!(ids, ["a", "c"]);
        assert_eq!(dd.lang, DesignLang::En);
        // Sin cartas diseñadas, nada.
        let vacio = DisenosDelDeck::de(
            &s,
            &deck(vec![carta(&s, "b", false, false)], DesignLang::Es),
        )
        .unwrap();
        assert!(vacio.efectivos.is_empty());
    }

    #[test]
    fn la_referencia_de_una_casilla_lleva_cara_idioma_y_huella() {
        let (_dir, s) = estado();
        let dfc = carta(&s, "c", true, true);
        let sin = carta(&s, "b", false, false);
        let d = deck(vec![dfc.clone(), sin.clone()], DesignLang::En);
        let dd = DisenosDelDeck::de(&s, &d).unwrap();
        let r = dd
            .referencia(&s, &dfc, &casilla(SlotFace::Back))
            .unwrap()
            .unwrap();
        assert_eq!(r.cara, 1);
        assert_eq!(r.lang, DesignLang::En);
        assert_eq!(
            r.huella,
            huella_de(&s, &dd.efectivos["c"], 1, DesignLang::En).unwrap()
        );
        assert!(dd
            .referencia(&s, &sin, &casilla(SlotFace::Front))
            .unwrap()
            .is_none());
        // Una carta diseñada que no estaba en el deck al preparar: sin referencia.
        let mut ajena = dfc.clone();
        ajena.id = "otra".into();
        assert!(dd
            .referencia(&s, &ajena, &casilla(SlotFace::Front))
            .unwrap()
            .is_none());
    }

    #[test]
    fn pintar_una_cara_da_una_imagen_del_ancho_pedido() {
        let (_dir, s) = estado();
        let d = carta(&s, "c", true, true).design.unwrap();
        let e = efectivo_de(&d, &HashMap::new());
        for cara in [0, 1] {
            let r = pintar(
                &s,
                &e,
                cara,
                DesignLang::Es,
                CardSize::MTG,
                80,
                Salida::Pantalla,
            )
            .unwrap();
            assert_eq!(r.imagen.dimensions(), (80, 112));
            assert_eq!(r.imagen.get_pixel(0, 0).0[3], 0, "esquina redondeada");
        }
        let r = pintar(
            &s,
            &e,
            0,
            DesignLang::Es,
            CardSize::MTG,
            80,
            Salida::Impresion,
        )
        .unwrap();
        assert_eq!(r.imagen.get_pixel(0, 0).0[3], 255);
    }
}
