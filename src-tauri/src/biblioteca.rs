//! **Los detalles de cada deck para la biblioteca**: colores, clase de mazo,
//! precio, idiomas, cartas diseñadas… Salen de las cartas y de la caché de
//! Scryfall, sin red, y se guardan en memoria mientras el deck no cambie
//! (por su fecha de modificación).

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

use proxy_core::model::{CardSource, Deck, DeckCard};
use proxy_core::stats::DeckKind;
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::db::Db;
use crate::store::decks;

/// Lo que se cuenta de un deck para la vista de lista, los filtros y los
/// grupos.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckDetails {
    pub id: String,
    /// Commander, construido o limitado (deducido de las cartas).
    pub kind: DeckKind,
    /// Identidad de color, en orden WUBRG (vacía: incoloro).
    pub color_identity: Vec<String>,
    /// Copias que no son ficha.
    pub deck_size: u32,
    pub lands: u32,
    pub tokens: u32,
    /// Copias sin datos de Scryfall (propias sin vincular, sin caché…).
    pub unknown: u32,
    /// Coste medio sin tierras.
    pub average_cmc: f64,
    /// Valor estimado con los precios de la caché (`None`: ninguna con precio).
    pub price_eur: Option<f64>,
    pub price_usd: Option<f64>,
    /// Copias con diseño del editor y, de ellas, las listas para imprimir.
    pub designed: u32,
    pub designed_ready: u32,
    /// Copias con imagen propia y, de ellas, las vinculadas a Scryfall.
    pub custom: u32,
    pub custom_linked: u32,
    /// Copias de doble cara.
    pub dfc: u32,
    /// Idiomas de las cartas (`es`, `en`…), del más usado al menos.
    pub langs: Vec<String>,
    /// Formatos en los que todas las cartas son legales.
    pub legal_formats: Vec<String>,
    /// Bracket mínimo de Commander que permiten las cartas.
    pub bracket: Option<u8>,
    /// Los nombres de sus cartas, sin repetir (para buscar «decks con…»).
    pub card_names: Vec<String>,
}

/// Lo ya calculado: deck → (fecha de modificación, detalles).
#[derive(Default)]
pub struct CacheDetalles {
    m: Mutex<HashMap<String, (i64, DeckDetails)>>,
}

fn candado<T>(m: &Mutex<T>) -> AppResult<std::sync::MutexGuard<'_, T>> {
    m.lock().map_err(|_| AppError::new(ErrorCode::Internal))
}

const WUBRG: [&str; 5] = ["W", "U", "B", "R", "G"];

/// El idioma con el que cuenta una carta. Las propias, el de su imagen: el
/// de la carta vinculada suele ser el inglés aunque la imagen esté en
/// español, y haría «inglés» un deck que no lo es.
fn idioma_de_la_carta(c: &DeckCard) -> Option<&str> {
    let l = match c.source {
        CardSource::Custom { .. } => c.meta.image_lang.as_deref(),
        _ => c.meta.lang.as_deref(),
    };
    l.filter(|l| !l.is_empty())
}

/// Los detalles de un deck ya abierto.
pub async fn detalles_de(db: &Db, deck: &Deck) -> AppResult<DeckDetails> {
    let s = crate::commands::stats::calcular_sin_red(db, &deck.cards).await?;
    let mut color_identity: Vec<String> = s.color_identity.clone();
    color_identity.sort_by_key(|c| WUBRG.iter().position(|w| w == c).unwrap_or(9));
    let mut idiomas: BTreeMap<String, u32> = BTreeMap::new();
    let mut nombres: Vec<String> = Vec::new();
    let (mut designed, mut ready, mut custom, mut linked, mut dfc) = (0, 0, 0, 0, 0);
    for c in &deck.cards {
        if matches!(c.source, CardSource::Blank) {
            continue;
        }
        if let Some(d) = &c.design {
            designed += 1;
            if d.status == proxy_core::diseno::modelo::DesignStatus::Ready {
                ready += 1;
            }
            *idiomas
                .entry(format!("{:?}", deck.settings.design_lang).to_lowercase())
                .or_default() += 1;
        } else if let Some(l) = idioma_de_la_carta(c) {
            *idiomas.entry(l.to_lowercase()).or_default() += 1;
        }
        if let CardSource::Custom { scryfall_id, .. } = &c.source {
            custom += 1;
            if scryfall_id.is_some() {
                linked += 1;
            }
        }
        if c.meta.is_dfc {
            dfc += 1;
        }
        let n = c.meta.name.trim();
        if !n.is_empty() && !nombres.iter().any(|x| x == n) {
            nombres.push(n.to_string());
        }
    }
    let mut langs: Vec<(String, u32)> = idiomas.into_iter().collect();
    langs.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let con_eur = s.prices.cards.iter().any(|c| c.eur.is_some());
    let con_usd = s.prices.cards.iter().any(|c| c.usd.is_some());
    Ok(DeckDetails {
        id: deck.id.clone(),
        kind: s.detected_kind,
        color_identity,
        deck_size: s.deck_size,
        lands: s.lands,
        tokens: s.tokens,
        unknown: s.unknown,
        average_cmc: s.average_cmc,
        price_eur: con_eur.then_some(s.prices.total_eur),
        price_usd: con_usd.then_some(s.prices.total_usd),
        designed,
        designed_ready: ready,
        custom,
        custom_linked: linked,
        dfc,
        langs: langs.into_iter().map(|(l, _)| l).collect(),
        legal_formats: s
            .legality
            .iter()
            .filter(|l| l.legal)
            .map(|l| l.format.clone())
            .collect(),
        bracket: s.bracket.as_ref().map(|b| b.level),
        card_names: nombres,
    })
}

/// Los detalles de los decks pedidos (todos si `ids` está vacío), de la
/// memoria si el deck no ha cambiado.
pub async fn detalles(
    db: &Db,
    cache: &CacheDetalles,
    ids: &[String],
) -> AppResult<Vec<DeckDetails>> {
    let lista = decks::list(db)?;
    let mut out = Vec::new();
    for d in lista
        .iter()
        .filter(|d| ids.is_empty() || ids.contains(&d.id))
    {
        let ya = candado(&cache.m)?
            .get(&d.id)
            .filter(|(t, _)| *t == d.updated_at)
            .map(|(_, x)| x.clone());
        let det = match ya {
            Some(x) => x,
            None => {
                let deck = decks::open(db, &d.id)?;
                let x = detalles_de(db, &deck).await?;
                candado(&cache.m)?.insert(d.id.clone(), (d.updated_at, x.clone()));
                x
            }
        };
        out.push(det);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::blobs::{tests::png, BlobStore};
    use proxy_core::model::*;

    fn carta(id: &str, source: CardSource, lang: &str, img: &FaceImage) -> DeckCard {
        DeckCard {
            id: id.into(),
            source,
            front: Some(img.clone()),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: format!("Carta {id}"),
                lang: (!lang.is_empty()).then(|| lang.to_string()),
                ..Default::default()
            },
            design: None,
            marca: None,
        }
    }

    #[tokio::test]
    async fn cuenta_idiomas_propias_y_guarda_en_memoria() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::in_memory().unwrap();
        let s = BlobStore::new(dir.path().join("blobs"));
        let img = s
            .put_bytes(&db, &png(3, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let mut deck = decks::create(&db, "Mezcla", None).unwrap();
        let mut propia_es = carta(
            "a",
            CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            "",
            &img,
        );
        propia_es.meta.image_lang = Some("es".into());
        // Vinculada a la impresión inglesa, con la imagen en español.
        let mut vinculada = carta(
            "b",
            CardSource::Custom {
                scryfall_id: Some("x".into()),
                externa: None,
            },
            "en",
            &img,
        );
        vinculada.meta.image_lang = Some("es".into());
        // Vinculada en inglés sin saber el idioma de la imagen: no cuenta.
        let sin_idioma = carta(
            "e",
            CardSource::Custom {
                scryfall_id: Some("z".into()),
                externa: None,
            },
            "en",
            &img,
        );
        deck.cards = vec![
            propia_es,
            vinculada,
            carta(
                "c",
                CardSource::Scryfall {
                    scryfall_id: "y".into(),
                },
                "en",
                &img,
            ),
            carta("d", CardSource::Blank, "", &img),
            sin_idioma,
        ];
        decks::save(&db, &deck).unwrap();
        let cache = CacheDetalles::default();
        let v = detalles(&db, &cache, &[]).await.unwrap();
        assert_eq!(v.len(), 1);
        let d = &v[0];
        assert_eq!(d.langs, vec!["es", "en"]);
        assert_eq!((d.custom, d.custom_linked), (3, 2));
        // Sin caché de Scryfall: nada se sabe de esas cartas.
        assert_eq!(d.unknown, 4);
        assert_eq!(d.price_eur, None);
        assert_eq!(
            d.card_names,
            vec!["Carta a", "Carta b", "Carta c", "Carta e"]
        );
        // Queda en memoria con la fecha del deck.
        assert!(cache.m.lock().unwrap().contains_key(&deck.id));
        // Solo los pedidos.
        assert!(detalles(&db, &cache, &["otro".into()])
            .await
            .unwrap()
            .is_empty());
    }
}
