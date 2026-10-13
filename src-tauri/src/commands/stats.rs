//! Las estadísticas del deck (Fase 8): Rust lee los datos de Scryfall de la
//! caché (y los pide si faltan o si los precios tienen más de 24 h) y el
//! núcleo cuenta.

use std::collections::HashMap;

use proxy_core::model::{CardSource, DeckCard};
use proxy_core::stats::{stats, DeckStats, StatInput};
use tauri::State;

use crate::error::AppResult;
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;
use crate::state::AppState;

/// Los precios se refrescan si tienen más de un día.
const PRECIOS_MS: i64 = 24 * 60 * 60 * 1000;

#[tauri::command]
pub async fn deck_stats(state: State<'_, AppState>, cards: Vec<DeckCard>) -> AppResult<DeckStats> {
    let mut datos: HashMap<String, Option<ScryCard>> = HashMap::new();
    let ahora = crate::store::ahora_ms();
    for c in &cards {
        let (CardSource::Scryfall { scryfall_id } | CardSource::Token { scryfall_id }) = &c.source
        else {
            continue;
        };
        if datos.contains_key(scryfall_id) {
            continue;
        }
        let en_cache = cache::leer(&state.db, scryfall_id)?;
        let fresca = en_cache
            .as_ref()
            .is_some_and(|(_, t)| ahora - t < PRECIOS_MS);
        let carta = if fresca {
            en_cache.map(|(c, _)| c)
        } else {
            // Sin red, lo que haya en la caché vale.
            match state.scryfall.card(scryfall_id).await {
                Ok(Some(nueva)) => {
                    cache::guardar(&state.db, &nueva)?;
                    Some(nueva)
                }
                _ => en_cache.map(|(c, _)| c),
            }
        };
        datos.insert(scryfall_id.clone(), carta);
    }

    let entrada: Vec<StatInput> = cards
        .iter()
        .filter(|c| !matches!(c.source, CardSource::Blank))
        .map(|c| match &c.source {
            CardSource::Scryfall { scryfall_id } | CardSource::Token { scryfall_id } => {
                let es_token = matches!(c.source, CardSource::Token { .. });
                match datos.get(scryfall_id).and_then(Option::as_ref) {
                    Some(s) => StatInput {
                        name: s.name.clone(),
                        cmc: s.cmc,
                        colors: if s.colors.is_empty() {
                            s.card_faces
                                .first()
                                .map(|f| f.colors.clone())
                                .unwrap_or_default()
                        } else {
                            s.colors.clone()
                        },
                        color_identity: s.color_identity.clone(),
                        type_line: s.type_line.clone(),
                        rarity: s.rarity.clone(),
                        price_eur: s.prices.eur.clone(),
                        legalities: s.legalities.clone(),
                        is_token: es_token,
                    },
                    None => StatInput {
                        name: c.meta.name.clone(),
                        is_token: es_token,
                        ..StatInput::default()
                    },
                }
            }
            // Las propias cuentan como «sin datos».
            _ => StatInput {
                name: c.meta.name.clone(),
                ..StatInput::default()
            },
        })
        .collect();
    Ok(stats(&entrada))
}
