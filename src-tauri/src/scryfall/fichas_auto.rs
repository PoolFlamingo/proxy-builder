//! **Las fichas que crean unas cartas** (`all_parts` con `component ==
//! "token"`), para «Añadir tokens automáticamente» al resolver la lista y para
//! «Detectar fichas» en un deck ya hecho.
//!
//! Scryfall solo da `all_parts` completos en la impresión **inglesa**: en las
//! traducidas viene vacío o a medias (19/10/2026: *Printlifter Ooze* en
//! alemán, francés, italiano o japonés no trae su Ooze; los dragones de
//! Lathliss en español tampoco). Así que de cada carta que no está en inglés
//! se piden las partes de su inglesa.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::model::DeckCard;

use super::client::CardApi;
use super::models::{Identifier, ScryCard};
use crate::diseno::borrador::id_scryfall;
use crate::error::{AppError, AppResult, ErrorCode};

const LOTE: usize = 75;

/// Una ficha y la carta que la crea (la primera que se vio).
#[derive(Debug, Clone, PartialEq)]
pub struct FichaHallada {
    pub ficha: ScryCard,
    pub creada_por: String,
}

fn comprobar(cancelado: &AtomicBool) -> AppResult<()> {
    if cancelado.load(Ordering::SeqCst) {
        Err(AppError::new(ErrorCode::Cancelled))
    } else {
        Ok(())
    }
}

fn partes_de_ficha(c: &ScryCard) -> impl Iterator<Item = &str> {
    c.all_parts
        .iter()
        .filter(|p| p.component == "token" && p.id != c.id)
        .map(|p| p.id.as_str())
}

/// Lo que hace que dos fichas sean «la misma» aunque sean de colecciones
/// distintas: su texto de Oracle (o, sin él, nombre, tipo y F/R).
pub fn clave_de_ficha(c: &ScryCard) -> String {
    c.oracle_id.clone().unwrap_or_else(|| {
        format!(
            "{}|{}|{}/{}",
            c.name,
            c.type_line.as_deref().unwrap_or(""),
            c.power.as_deref().unwrap_or(""),
            c.toughness.as_deref().unwrap_or("")
        )
    })
}

/// Las fichas que crean `cartas`, una por ficha distinta, en el orden en que
/// aparecen. Las cartas que no están en inglés se miran en su impresión
/// inglesa. `progreso(hechas, total)` va por lotes.
pub async fn fichas_de<A: CardApi>(
    api: &A,
    cartas: &[ScryCard],
    cancelado: &AtomicBool,
    mut progreso: impl FnMut(u32, u32) + Send,
) -> AppResult<Vec<FichaHallada>> {
    // 1. Los ids de las fichas, con quién las crea.
    let mut ids: Vec<(String, String)> = Vec::new();
    let mut vistos: HashSet<String> = HashSet::new();
    let mut apuntar = |c: &ScryCard, ids: &mut Vec<(String, String)>| {
        for id in partes_de_ficha(c) {
            if vistos.insert(id.to_string()) {
                ids.push((id.to_string(), c.name.clone()));
            }
        }
    };
    let mut en_ingles: Vec<String> = Vec::new();
    let mut pedidas: HashSet<String> = HashSet::new();
    for c in cartas {
        if c.lang.is_empty() || c.lang == "en" {
            apuntar(c, &mut ids);
        } else {
            // Las que tenga ya, por si la inglesa no aparece.
            apuntar(c, &mut ids);
            let nombre = c.name.split(" // ").next().unwrap_or(&c.name).to_string();
            if pedidas.insert(nombre.clone()) {
                en_ingles.push(nombre);
            }
        }
    }
    let total = (en_ingles.len().div_ceil(LOTE) + ids.len().div_ceil(LOTE).max(1)) as u32;
    let mut hechas = 0u32;
    for lote in en_ingles.chunks(LOTE) {
        comprobar(cancelado)?;
        let pedir: Vec<Identifier> = lote
            .iter()
            .map(|n| Identifier::Name { name: n.clone() })
            .collect();
        let r = api.collection(&pedir).await?;
        for c in &r.data {
            apuntar(c, &mut ids);
        }
        hechas += 1;
        progreso(hechas, total);
    }

    // 2. Las fichas, en el orden pedido; una por ficha distinta.
    let mut fichas: Vec<FichaHallada> = Vec::new();
    let mut claves: HashSet<String> = HashSet::new();
    for lote in ids.chunks(LOTE) {
        comprobar(cancelado)?;
        let pedir: Vec<Identifier> = lote
            .iter()
            .map(|(id, _)| Identifier::Id { id: id.clone() })
            .collect();
        let r = api.collection(&pedir).await?;
        for (id, creador) in lote {
            if let Some(t) = r.data.iter().find(|t| &t.id == id) {
                if claves.insert(clave_de_ficha(t)) {
                    fichas.push(FichaHallada {
                        ficha: t.clone(),
                        creada_por: creador.clone(),
                    });
                }
            }
        }
        hechas += 1;
        progreso(hechas.min(total), total);
    }
    Ok(fichas)
}

/// ¿Es una ficha (por su línea de tipo, en inglés o en español)?
fn es_ficha(type_line: Option<&str>) -> bool {
    type_line.is_some_and(|t| {
        let t = t.to_lowercase();
        t.contains("token") || t.contains("ficha")
    })
}

/// **Las fichas que faltan en un deck ya hecho**: las que crean sus cartas
/// (de Scryfall o propias vinculadas) y que el deck no tiene ya, ni con el
/// mismo id, ni la misma ficha de otra colección, ni una ficha con el mismo
/// nombre (también propia: «Treasure» vinculada o hecha a mano).
/// `de_la_cache`: la carta de la caché local, para no pedir lo que ya se tiene.
pub async fn faltan_en_el_deck<A: CardApi>(
    api: &A,
    deck: &[DeckCard],
    de_la_cache: impl Fn(&str) -> Option<ScryCard>,
    cancelado: &AtomicBool,
) -> AppResult<Vec<FichaHallada>> {
    let mut ids: Vec<String> = Vec::new();
    for c in deck {
        if let Some(id) = id_scryfall(c) {
            if !ids.iter().any(|x| x == id) {
                ids.push(id.to_string());
            }
        }
    }
    // Las cartas: de la caché o, si no están, de Scryfall en lotes.
    let mut cartas: Vec<ScryCard> = Vec::new();
    let mut pedir: Vec<String> = Vec::new();
    for id in &ids {
        match de_la_cache(id) {
            Some(c) => cartas.push(c),
            None => pedir.push(id.clone()),
        }
    }
    for lote in pedir.chunks(LOTE) {
        comprobar(cancelado)?;
        let r = api
            .collection(
                &lote
                    .iter()
                    .map(|id| Identifier::Id { id: id.clone() })
                    .collect::<Vec<_>>(),
            )
            .await?;
        cartas.extend(r.data);
    }

    // Lo que ya hay: ids, la misma ficha de otra colección y nombres de ficha.
    let ya_ids: HashSet<&str> = ids.iter().map(String::as_str).collect();
    let ya_claves: HashSet<String> = cartas
        .iter()
        .filter(|c| c.layout.contains("token") || es_ficha(c.type_line.as_deref()))
        .map(clave_de_ficha)
        .collect();
    let ya_nombres: HashSet<String> = deck
        .iter()
        .filter(|c| es_ficha(c.meta.type_line.as_deref()))
        .map(|c| c.meta.name.to_lowercase())
        .collect();

    let fuentes: Vec<ScryCard> = cartas
        .into_iter()
        .filter(|c| !c.layout.contains("token"))
        .collect();
    Ok(fichas_de(api, &fuentes, cancelado, |_, _| {})
        .await?
        .into_iter()
        .filter(|f| {
            !ya_ids.contains(f.ficha.id.as_str())
                && !ya_claves.contains(&clave_de_ficha(&f.ficha))
                && !ya_nombres.contains(&f.ficha.name.to_lowercase())
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::resolve::tests::{carta, falso};
    use serde_json::json;

    #[tokio::test]
    async fn de_la_impresion_inglesa_aunque_la_traducida_no_las_tenga() {
        let mut api = falso();
        // Dragon Fodder en español, sin `all_parts` (como en Scryfall).
        api.cartas.push(carta(json!({
            "id": "fodder-es", "name": "Dragon Fodder", "lang": "es", "set": "fdn",
            "collector_number": "535", "layout": "normal"
        })));
        let es = api.cartas.last().cloned().unwrap();
        let no = AtomicBool::new(false);
        let f = fichas_de(&api, &[es], &no, |_, _| {}).await.unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].ficha.id, "goblin");
        assert_eq!(f[0].creada_por, "Dragon Fodder");
    }

    #[tokio::test]
    async fn una_por_ficha_distinta() {
        let mut api = falso();
        // Otra carta que crea un Goblin igual, de otra colección.
        api.cartas.push(carta(json!({
            "id": "goblin-2", "oracle_id": "o-goblin", "name": "Goblin", "lang": "en",
            "set": "tm21", "collector_number": "5", "layout": "token"
        })));
        for c in api.cartas.iter_mut().filter(|c| c.id == "goblin") {
            c.oracle_id = Some("o-goblin".into());
        }
        api.cartas.push(carta(json!({
            "id": "krenko", "name": "Krenko's Command", "lang": "en", "set": "m21",
            "collector_number": "152", "layout": "normal",
            "all_parts": [{"id": "goblin-2", "component": "token", "name": "Goblin"}]
        })));
        let cartas: Vec<ScryCard> = api
            .cartas
            .iter()
            .filter(|c| c.id == "fodder" || c.id == "krenko")
            .cloned()
            .collect();
        let no = AtomicBool::new(false);
        let f = fichas_de(&api, &cartas, &no, |_, _| {}).await.unwrap();
        assert_eq!(f.len(), 1);
    }

    fn del_deck(source: proxy_core::model::CardSource, nombre: &str, tipo: &str) -> DeckCard {
        DeckCard {
            id: nombre.into(),
            source,
            front: None,
            back: proxy_core::model::BackSpec::DeckDefault,
            edits: Default::default(),
            locked: false,
            meta: proxy_core::model::CardMeta {
                name: nombre.into(),
                type_line: Some(tipo.into()),
                ..Default::default()
            },
            design: None,
        }
    }

    #[tokio::test]
    async fn las_que_faltan_tambien_de_propias_vinculadas() {
        use proxy_core::model::CardSource;
        let api = falso();
        // Una propia vinculada a Dragon Fodder (inglesa, fuera de la caché).
        let deck = vec![del_deck(
            CardSource::Custom {
                scryfall_id: Some("fodder".into()),
                externa: None,
            },
            "Dragon Fodder",
            "Sorcery",
        )];
        let no = AtomicBool::new(false);
        let f = faltan_en_el_deck(&api, &deck, |_| None, &no).await.unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].ficha.id, "goblin");

        // Con el Goblin ya en el deck (como ficha de Scryfall), nada.
        let mut con = deck.clone();
        con.push(del_deck(
            CardSource::Token {
                scryfall_id: "goblin".into(),
            },
            "Goblin",
            "Token Creature — Goblin",
        ));
        assert!(faltan_en_el_deck(&api, &con, |_| None, &no)
            .await
            .unwrap()
            .is_empty());

        // Ni con una ficha propia que se llama igual.
        let mut propia = deck.clone();
        propia.push(del_deck(
            CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            "Goblin",
            "Ficha de criatura — Trasgo",
        ));
        assert!(faltan_en_el_deck(&api, &propia, |_| None, &no)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn usa_la_cache_antes_que_scryfall() {
        use proxy_core::model::CardSource;
        let api = falso();
        let fodder = api.cartas.iter().find(|c| c.id == "fodder").cloned();
        let deck = vec![del_deck(
            CardSource::Scryfall {
                scryfall_id: "fodder".into(),
            },
            "Dragon Fodder",
            "Sorcery",
        )];
        let no = AtomicBool::new(false);
        let f = faltan_en_el_deck(&api, &deck, |_| fodder.clone(), &no)
            .await
            .unwrap();
        assert_eq!(f.len(), 1);
        // Solo la petición de la ficha.
        assert_eq!(*api.colecciones.lock().unwrap(), 1);
    }
}

/// Prueba en vivo con una copia de la base de datos: `PB_DB=<library.db>
/// PB_DECK=<nombre> cargo test --lib fichas_auto::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;
    use crate::scryfall::cache;
    use crate::scryfall::client::Scryfall;
    use crate::store::{db::Db, decks};

    #[tokio::test]
    #[ignore]
    async fn fichas_de_un_deck_real() {
        let dir = tempfile::tempdir().unwrap();
        let copia = dir.path().join("copia.db");
        std::fs::copy(std::env::var("PB_DB").unwrap(), &copia).unwrap();
        let db = Db::open(&copia).unwrap();
        let nombre = std::env::var("PB_DECK").unwrap();
        let id = decks::list(&db)
            .unwrap()
            .into_iter()
            .find(|d| d.name == nombre)
            .unwrap()
            .id;
        let deck = decks::open(&db, &id).unwrap();
        let s = Scryfall::new().unwrap();
        let f = faltan_en_el_deck(
            &s,
            &deck.cards,
            |id| cache::leer(&db, id).ok().flatten().map(|(c, _)| c),
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
        for x in &f {
            println!(
                "{} ({}) <- {}",
                x.ficha.name,
                x.ficha.type_line.as_deref().unwrap_or(""),
                x.creada_por
            );
        }
    }
}
