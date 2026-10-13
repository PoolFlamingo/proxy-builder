//! Las estadísticas del deck: Rust lee los datos de Scryfall de la caché (y
//! pide de una vez, por `/cards/collection`, los que falten, los que tengan
//! precios de más de 24 h y los guardados antes de que la app leyera el texto
//! de reglas) y el núcleo cuenta.
//!
//! Scryfall no da precio a las impresiones que no están en inglés: para esas
//! se usa el de la misma impresión en inglés (mismo set y número).

use std::collections::HashMap;

use proxy_core::model::{CardSource, DeckCard};
use proxy_core::stats::{stats, DeckKind, DeckStats, StatInput};
use tauri::State;

use crate::error::AppResult;
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::{Identifier, Prices, ScryCard};
use crate::state::AppState;

/// Los precios se refrescan si tienen más de un día.
const PRECIOS_MS: i64 = 24 * 60 * 60 * 1000;
/// Lo que admite `/cards/collection` por petición.
const POR_LOTE: usize = 75;

/// La carta de Scryfall que cuenta para las estadísticas: la de Scryfall, la
/// del token o la vinculada a una imagen propia.
fn id_scryfall(c: &DeckCard) -> Option<&str> {
    match &c.source {
        CardSource::Scryfall { scryfall_id } | CardSource::Token { scryfall_id } => {
            Some(scryfall_id)
        }
        CardSource::Custom { scryfall_id } => scryfall_id.as_deref(),
        CardSource::Blank => None,
    }
}

/// Sin precio normal, el foil (hay impresiones que solo existen en foil).
fn precios(p: &Prices) -> (Option<String>, Option<String>) {
    (
        p.eur.clone().or_else(|| p.eur_foil.clone()),
        p.usd.clone().or_else(|| p.usd_foil.clone()),
    )
}

fn sin_precio(s: &ScryCard) -> bool {
    let (eur, usd) = precios(&s.prices);
    eur.is_none() && usd.is_none()
}

fn entrada(c: &DeckCard, s: &ScryCard, en_ingles: Option<&Prices>, es_token: bool) -> StatInput {
    let cara = s.card_faces.first();
    let (price_eur, price_usd) = match en_ingles {
        Some(p) if sin_precio(s) => precios(p),
        _ => precios(&s.prices),
    };
    StatInput {
        id: c.id.clone(),
        name: s.name.clone(),
        cmc: s.cmc,
        mana_cost: s.coste(),
        colors: if s.colors.is_empty() {
            cara.map(|f| f.colors.clone()).unwrap_or_default()
        } else {
            s.colors.clone()
        },
        color_identity: s.color_identity.clone(),
        type_line: s.type_line.clone(),
        oracle_text: s.texto_completo(),
        keywords: s.keywords.clone().unwrap_or_default(),
        produced_mana: s.produced_mana.clone(),
        power: s
            .power
            .clone()
            .or_else(|| cara.and_then(|f| f.power.clone())),
        toughness: s
            .toughness
            .clone()
            .or_else(|| cara.and_then(|f| f.toughness.clone())),
        rarity: s.rarity.clone(),
        set: (!s.set.is_empty()).then(|| s.set.clone()),
        set_name: (!s.set_name.is_empty()).then(|| s.set_name.clone()),
        released_at: s.released_at.clone(),
        price_eur,
        price_usd,
        legalities: s.legalities.clone(),
        game_changer: s.game_changer,
        is_token: es_token,
    }
}

#[tauri::command]
pub async fn deck_stats(
    state: State<'_, AppState>,
    cards: Vec<DeckCard>,
    kind: Option<DeckKind>,
) -> AppResult<DeckStats> {
    let mut datos: HashMap<String, ScryCard> = HashMap::new();
    let mut pedir: Vec<String> = Vec::new();
    let ahora = crate::store::ahora_ms();
    for c in &cards {
        let Some(id) = id_scryfall(c) else {
            continue;
        };
        if datos.contains_key(id) || pedir.iter().any(|p| p == id) {
            continue;
        }
        match cache::leer(&state.db, id)? {
            Some((carta, t)) => {
                let fresca = ahora - t < PRECIOS_MS && carta.keywords.is_some();
                if !fresca {
                    pedir.push(id.to_string());
                }
                // Mientras tanto (y si no hay red), lo que haya vale.
                datos.insert(id.to_string(), carta);
            }
            None => pedir.push(id.to_string()),
        }
    }

    for lote in pedir.chunks(POR_LOTE) {
        let ids: Vec<Identifier> = lote
            .iter()
            .map(|id| Identifier::Id { id: id.clone() })
            .collect();
        match state.scryfall.collection(&ids).await {
            Ok(r) => {
                for nueva in r.data {
                    cache::guardar(&state.db, &nueva)?;
                    datos.insert(nueva.id.clone(), nueva);
                }
            }
            // Sin red se cuenta con la caché.
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "estadísticas sin refrescar");
                break;
            }
        }
    }

    // Los precios de las impresiones en otro idioma, de su versión en inglés.
    let mut ingles: HashMap<(String, String), Prices> = HashMap::new();
    let mut pedir_ingles: Vec<(String, String)> = Vec::new();
    for s in datos.values() {
        if s.lang == "en" || !sin_precio(s) || s.set.is_empty() {
            continue;
        }
        let clave = (s.set.clone(), s.collector_number.clone());
        if ingles.contains_key(&clave) || pedir_ingles.contains(&clave) {
            continue;
        }
        match cache::leer_impresion(&state.db, &clave.0, &clave.1, "en")? {
            Some((c, t)) if ahora - t < PRECIOS_MS => {
                ingles.insert(clave, c.prices);
            }
            _ => pedir_ingles.push(clave),
        }
    }
    for lote in pedir_ingles.chunks(POR_LOTE) {
        let ids: Vec<Identifier> = lote
            .iter()
            .map(|(set, numero)| Identifier::SetNumber {
                set: set.clone(),
                collector_number: numero.clone(),
            })
            .collect();
        match state.scryfall.collection(&ids).await {
            Ok(r) => {
                for c in r.data {
                    cache::guardar(&state.db, &c)?;
                    ingles.insert((c.set.clone(), c.collector_number.clone()), c.prices);
                }
            }
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "precios en inglés sin refrescar");
                break;
            }
        }
    }

    let entrada: Vec<StatInput> = cards
        .iter()
        .filter(|c| !matches!(c.source, CardSource::Blank))
        .map(|c| {
            let es_token = matches!(c.source, CardSource::Token { .. });
            let dato = id_scryfall(c).and_then(|id| datos.get(id));
            // Una propia vinculada a una ficha también es una ficha.
            let es_token = es_token
                || dato.is_some_and(|s| {
                    s.type_line
                        .as_deref()
                        .is_some_and(|t| t.starts_with("Token"))
                });
            match dato {
                Some(s) => entrada(
                    c,
                    s,
                    ingles.get(&(s.set.clone(), s.collector_number.clone())),
                    es_token,
                ),
                // Las propias sin vincular cuentan como «sin datos».
                None => StatInput {
                    id: c.id.clone(),
                    name: c.meta.name.clone(),
                    is_token: es_token,
                    ..StatInput::default()
                },
            }
        })
        .collect();
    Ok(stats(&entrada, kind))
}
