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
use crate::store::db::Db;

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
    calcular(&state.db, state.scryfall.as_ref(), &cards, kind).await
}

/// Lo que hace `deck_stats`, con la caché y el Scryfall que se le den.
pub(crate) async fn calcular<A: CardApi>(
    db: &Db,
    api: &A,
    cards: &[DeckCard],
    kind: Option<DeckKind>,
) -> AppResult<DeckStats> {
    let mut datos: HashMap<String, ScryCard> = HashMap::new();
    let mut pedir: Vec<String> = Vec::new();
    let ahora = crate::store::ahora_ms();
    for c in cards {
        let Some(id) = id_scryfall(c) else {
            continue;
        };
        if datos.contains_key(id) || pedir.iter().any(|p| p == id) {
            continue;
        }
        match cache::leer(db, id)? {
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
        match api.collection(&ids).await {
            Ok(r) => {
                for nueva in r.data {
                    cache::guardar(db, &nueva)?;
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
        match cache::leer_impresion(db, &clave.0, &clave.1, "en")? {
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
        match api.collection(&ids).await {
            Ok(r) => {
                for c in r.data {
                    cache::guardar(db, &c)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{AppError, ErrorCode};
    use crate::scryfall::models::Collection;
    use crate::scryfall::resolve::tests::carta;
    use proxy_core::model::{BackSpec, CardEdits, CardMeta};
    use serde_json::json;
    use std::sync::Mutex;

    /// Un Scryfall de mentira que solo sabe de `/cards/collection` y apunta
    /// lo que se le pide.
    struct Api {
        cartas: Vec<ScryCard>,
        pedidas: Mutex<Vec<Vec<Identifier>>>,
        sin_red: bool,
    }

    fn api(cartas: Vec<ScryCard>) -> Api {
        Api {
            cartas,
            pedidas: Mutex::new(Vec::new()),
            sin_red: false,
        }
    }

    impl Api {
        fn pedidas(&self) -> Vec<Vec<Identifier>> {
            self.pedidas.lock().unwrap().clone()
        }
    }

    impl CardApi for Api {
        async fn collection(&self, ids: &[Identifier]) -> AppResult<Collection> {
            self.pedidas.lock().unwrap().push(ids.to_vec());
            if self.sin_red {
                return Err(AppError::new(ErrorCode::Network));
            }
            let data = ids
                .iter()
                .filter_map(|i| {
                    self.cartas.iter().find(|c| match i {
                        Identifier::Id { id } => &c.id == id,
                        Identifier::SetNumber {
                            set,
                            collector_number,
                        } => {
                            &c.set == set
                                && &c.collector_number == collector_number
                                && c.lang == "en"
                        }
                        _ => false,
                    })
                })
                .cloned()
                .collect();
            Ok(Collection {
                data,
                not_found: Vec::new(),
            })
        }
        async fn card(&self, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn card_lang(&self, _: &str, _: &str, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn search(&self, _: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            Ok(Vec::new())
        }
        async fn download(&self, _: &str) -> AppResult<Vec<u8>> {
            Err(AppError::new(ErrorCode::Network))
        }
    }

    /// Una carta de Scryfall con lo que miran las estadísticas.
    fn scry(id: &str, tipo: &str, eur: Option<&str>) -> ScryCard {
        carta(json!({
            "id": id, "name": format!("Carta {id}"), "lang": "en", "set": "m11",
            "collector_number": id, "layout": "normal", "type_line": tipo, "cmc": 2.0,
            "mana_cost": "{1}{G}", "colors": ["G"], "keywords": [],
            "prices": {"eur": eur, "usd": null, "eur_foil": null, "usd_foil": null}
        }))
    }

    fn copia(id: &str, source: CardSource) -> DeckCard {
        DeckCard {
            id: id.into(),
            source,
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: format!("Propia {id}"),
                ..CardMeta::default()
            },
            design: None,
        }
    }

    fn de_scryfall(copia_id: &str, id: &str) -> DeckCard {
        copia(
            copia_id,
            CardSource::Scryfall {
                scryfall_id: id.into(),
            },
        )
    }

    fn envejecer(db: &Db, id: &str) {
        db.conn()
            .unwrap()
            .execute(
                "UPDATE scryfall_cards SET fetched_at = 0 WHERE id = ?1",
                [id],
            )
            .unwrap();
    }

    // ── Piezas sueltas ──

    #[test]
    fn que_carta_de_scryfall_cuenta() {
        let id = |s| id_scryfall(&copia("c", s)).map(str::to_string);
        assert_eq!(
            id(CardSource::Scryfall {
                scryfall_id: "a".into()
            }),
            Some("a".into())
        );
        assert_eq!(
            id(CardSource::Token {
                scryfall_id: "t".into()
            }),
            Some("t".into())
        );
        assert_eq!(
            id(CardSource::Custom {
                scryfall_id: Some("v".into())
            }),
            Some("v".into())
        );
        assert_eq!(id(CardSource::Custom { scryfall_id: None }), None);
        assert_eq!(id(CardSource::Blank), None);
    }

    #[test]
    fn sin_precio_normal_vale_el_foil() {
        let p = Prices {
            eur: None,
            eur_foil: Some("9.00".into()),
            usd: Some("1.00".into()),
            usd_foil: Some("5.00".into()),
        };
        assert_eq!(precios(&p), (Some("9.00".into()), Some("1.00".into())));
        assert_eq!(precios(&Prices::default()), (None, None));
    }

    #[test]
    fn la_entrada_completa_lo_que_falta_con_la_primera_cara() {
        let s = carta(json!({
            "id": "dfc", "name": "Delver // Aberration", "lang": "es", "set": "",
            "collector_number": "51", "layout": "transform", "keywords": ["Flying"],
            "card_faces": [
                {"name": "Delver", "colors": ["U"], "power": "1", "toughness": "1", "oracle_text": "A"},
                {"name": "Aberration", "colors": ["U"], "power": "3", "toughness": "2", "oracle_text": "B"}
            ]
        }));
        let en_ingles = Prices {
            eur: Some("0.50".into()),
            ..Prices::default()
        };
        let e = entrada(&copia("c1", CardSource::Blank), &s, Some(&en_ingles), true);
        assert_eq!(e.id, "c1");
        assert_eq!(e.colors, ["U"]);
        assert_eq!(
            (e.power.as_deref(), e.toughness.as_deref()),
            (Some("1"), Some("1"))
        );
        assert_eq!(e.oracle_text, "A\nB");
        assert_eq!(e.keywords, ["Flying"]);
        // Sin precio propio, el de la impresión inglesa.
        assert_eq!(e.price_eur.as_deref(), Some("0.50"));
        // Un set vacío no es un set.
        assert_eq!(e.set, None);
        assert!(e.is_token);
    }

    #[test]
    fn con_precio_propio_no_se_usa_el_ingles() {
        let s = scry("a", "Instant", Some("3.00"));
        let en_ingles = Prices {
            eur: Some("0.10".into()),
            usd: Some("0.20".into()),
            ..Prices::default()
        };
        let e = entrada(&copia("c", CardSource::Blank), &s, Some(&en_ingles), false);
        assert_eq!(e.price_eur.as_deref(), Some("3.00"));
        // Tampoco para la otra moneda: va todo de la misma impresión.
        assert_eq!(e.price_usd, None);
    }

    // ── El cálculo entero ──

    #[tokio::test]
    async fn pide_de_una_vez_solo_lo_que_no_esta_fresco() {
        let db = Db::in_memory().unwrap();
        // «b» está en la caché y fresca; «a» no está.
        cache::guardar(&db, &scry("b", "Instant", Some("1.00"))).unwrap();
        let a = api(vec![
            scry("a", "Creature — Elf", Some("2.00")),
            scry("b", "Instant", Some("1.00")),
        ]);
        let cartas = vec![
            de_scryfall("1", "a"),
            de_scryfall("2", "a"),
            de_scryfall("3", "b"),
        ];
        let st = calcular(&db, &a, &cartas, None).await.unwrap();
        assert_eq!(
            a.pedidas(),
            vec![vec![Identifier::Id { id: "a".into() }]],
            "una petición y sin repetir la copia"
        );
        assert_eq!(st.cards, 3);
        assert_eq!(st.unknown, 0);
        assert!((st.prices.total_eur - 5.0).abs() < 1e-9);
        // Lo pedido queda en la caché: la segunda vez no se pide nada.
        let st2 = calcular(&db, &a, &cartas, None).await.unwrap();
        assert_eq!(a.pedidas().len(), 1);
        assert_eq!(st2.cards, 3);
    }

    #[tokio::test]
    async fn los_precios_viejos_y_la_cache_de_antes_se_vuelven_a_pedir() {
        let db = Db::in_memory().unwrap();
        cache::guardar(&db, &scry("viejo", "Instant", Some("1.00"))).unwrap();
        envejecer(&db, "viejo");
        // Guardada antes de que la app leyera las palabras clave.
        let mut antigua = scry("antigua", "Instant", Some("1.00"));
        antigua.keywords = None;
        cache::guardar(&db, &antigua).unwrap();
        let a = api(vec![
            scry("viejo", "Instant", Some("4.00")),
            scry("antigua", "Instant", Some("1.00")),
        ]);
        let cartas = vec![de_scryfall("1", "viejo"), de_scryfall("2", "antigua")];
        let st = calcular(&db, &a, &cartas, None).await.unwrap();
        let mut pedidas = a.pedidas().concat();
        pedidas.sort_by_key(|i| format!("{i:?}"));
        assert_eq!(
            pedidas,
            vec![
                Identifier::Id {
                    id: "antigua".into()
                },
                Identifier::Id { id: "viejo".into() }
            ]
        );
        // Cuenta el precio nuevo.
        assert!((st.prices.total_eur - 5.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn sin_red_se_cuenta_con_la_cache() {
        let db = Db::in_memory().unwrap();
        cache::guardar(&db, &scry("b", "Instant", Some("1.00"))).unwrap();
        envejecer(&db, "b");
        let mut a = api(Vec::new());
        a.sin_red = true;
        let cartas = vec![de_scryfall("1", "b"), de_scryfall("2", "nueva")];
        let st = calcular(&db, &a, &cartas, None).await.unwrap();
        // Lo viejo vale; lo que nunca se pidió queda «sin datos».
        assert_eq!(st.cards, 1);
        assert_eq!(st.unknown, 1);
        assert!((st.prices.total_eur - 1.0).abs() < 1e-9);
        // Al primer fallo se deja de pedir.
        assert_eq!(a.pedidas().len(), 1);
    }

    #[tokio::test]
    async fn mas_de_setenta_y_cinco_cartas_van_en_varios_lotes() {
        let db = Db::in_memory().unwrap();
        let ids: Vec<String> = (0..80).map(|i| format!("c{i}")).collect();
        let a = api(ids.iter().map(|i| scry(i, "Instant", None)).collect());
        let cartas: Vec<DeckCard> = ids.iter().map(|i| de_scryfall(i, i)).collect();
        let st = calcular(&db, &a, &cartas, None).await.unwrap();
        let tamanos: Vec<usize> = a.pedidas().iter().map(Vec::len).collect();
        assert_eq!(tamanos, [75, 5]);
        assert_eq!(st.cards, 80);
    }

    #[tokio::test]
    async fn una_impresion_sin_precio_usa_el_de_la_inglesa() {
        let db = Db::in_memory().unwrap();
        let mut es = scry("x-es", "Instant", None);
        es.lang = "es".into();
        es.collector_number = "28".into();
        let mut en = scry("x-en", "Instant", Some("2.50"));
        en.collector_number = "28".into();
        let a = api(vec![es, en]);
        let cartas = vec![de_scryfall("1", "x-es"), de_scryfall("2", "x-es")];
        let st = calcular(&db, &a, &cartas, None).await.unwrap();
        assert!((st.prices.total_eur - 5.0).abs() < 1e-9);
        assert_eq!(st.prices.without_eur, 0);
        assert_eq!(
            a.pedidas()[1],
            vec![Identifier::SetNumber {
                set: "m11".into(),
                collector_number: "28".into()
            }],
            "la inglesa, una vez aunque haya dos copias"
        );
        // La inglesa queda en la caché: la próxima vez no se pide.
        calcular(&db, &a, &cartas, None).await.unwrap();
        assert_eq!(a.pedidas().len(), 2);
    }

    #[tokio::test]
    async fn fichas_propias_y_huecos() {
        let db = Db::in_memory().unwrap();
        let a = api(vec![
            scry("goblin", "Token Creature — Goblin", None),
            scry("elfo", "Creature — Elf", None),
        ]);
        let cartas = vec![
            copia(
                "1",
                CardSource::Token {
                    scryfall_id: "goblin".into(),
                },
            ),
            // Una propia vinculada a una ficha también es ficha.
            copia(
                "2",
                CardSource::Custom {
                    scryfall_id: Some("goblin".into()),
                },
            ),
            copia(
                "3",
                CardSource::Custom {
                    scryfall_id: Some("elfo".into()),
                },
            ),
            copia("4", CardSource::Custom { scryfall_id: None }),
            copia("5", CardSource::Blank),
        ];
        let st = calcular(&db, &a, &cartas, None).await.unwrap();
        assert_eq!(st.tokens, 2);
        assert_eq!(st.cards, 1);
        // La propia sin vincular cuenta como «sin datos», con su nombre; el
        // hueco en blanco no cuenta para nada.
        assert_eq!(st.unknown, 1);
        assert_eq!(st.unknown_cards[0].name, "Propia 4");
        assert_eq!(st.deck_size, 2);
    }

    #[tokio::test]
    async fn un_deck_vacio_no_pide_nada() {
        let db = Db::in_memory().unwrap();
        let a = api(Vec::new());
        let st = calcular(&db, &a, &[], None).await.unwrap();
        assert!(a.pedidas().is_empty());
        assert_eq!((st.cards, st.deck_size, st.unknown), (0, 0, 0));
    }
}
