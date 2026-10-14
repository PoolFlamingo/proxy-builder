//! **Pintar diseños en la app**: el diseño efectivo (con su colección), su
//! huella y el render con los recursos de la app (símbolos, letras, packs e
//! imágenes del almacén).

use std::collections::HashMap;

use proxy_core::diseno::efectivo::{efectivo, huella, DisenoEfectivo};
use proxy_core::diseno::modelo::{CardDesign, Collection, DesignLang};
use proxy_core::diseno::render::{render, Recursos, Render, Salida};
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
