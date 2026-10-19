//! **Borrador de un diseño** a partir de la carta de Scryfall.
//!
//! El inglés sale del Oracle (Scryfall lo trae en cualquier impresión) y el
//! español, de la impresión española: la misma si la carta del deck ya está
//! en español; si no, la del mismo set y número; si no, la más reciente
//! en español. Sin ninguna, el Oracle con el aviso `sinTextoOficial`.

use proxy_core::diseno::borrador::siguiente_numero;
use proxy_core::diseno::estructura::limpiar;
use proxy_core::diseno::modelo::*;
use proxy_core::model::{CardSource, DeckCard};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::{CardFace, ScryCard};
use crate::store::db::Db;

/// Layouts cuyas caras se diseñan por separado (las de `card_faces`).
fn por_caras(layout: &str) -> bool {
    matches!(
        layout,
        "split"
            | "flip"
            | "transform"
            | "modal_dfc"
            | "adventure"
            | "meld"
            | "battle"
            | "double_faced_token"
            | "reversible_card"
    )
}

/// La carta de Scryfall vinculada a una del deck.
pub fn id_scryfall(c: &DeckCard) -> Option<&str> {
    match &c.source {
        CardSource::Scryfall { scryfall_id } | CardSource::Token { scryfall_id } => {
            Some(scryfall_id)
        }
        CardSource::Custom { scryfall_id, .. } => scryfall_id.as_deref(),
        CardSource::Blank => None,
    }
}

/// La carta con los campos del editor: de la caché si los tiene; si no, se
/// pide otra vez y se guarda.
pub async fn carta_completa<A: CardApi>(db: &Db, api: &A, id: &str) -> AppResult<ScryCard> {
    if let Some((c, _)) = cache::leer(db, id)? {
        if c.completa_para_disenar() {
            return Ok(c);
        }
    }
    let c = api
        .card(id)
        .await?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("carta", id))?;
    cache::guardar(db, &c)?;
    Ok(c)
}

/// La impresión española de una carta, si existe: la de la propia carta si
/// ya está en español; si no, **la más reciente** (el texto vigente, que en
/// las reediciones corrige plantillas viejas); si no, la del mismo set y
/// número.
pub async fn impresion_es<A: CardApi>(
    db: &Db,
    api: &A,
    c: &ScryCard,
) -> AppResult<Option<ScryCard>> {
    if c.lang == "es" {
        return Ok(Some(c.clone()));
    }
    if let Some(oracle) = &c.oracle_id {
        let mut todas = api
            .search(&format!("oracleid:{oracle} lang:es"), true, 60)
            .await
            .unwrap_or_default();
        todas.retain(|e| e.lang == "es" && texto_impreso(e));
        todas.sort_by(|a, b| b.released_at.cmp(&a.released_at));
        if let Some(e) = todas.into_iter().next() {
            cache::guardar(db, &e)?;
            return Ok(Some(e));
        }
    }
    if let Some((e, _)) = cache::leer_impresion(db, &c.set, &c.collector_number, "es")? {
        if e.completa_para_disenar() {
            return Ok(Some(e));
        }
    }
    let e = api.card_lang(&c.set, &c.collector_number, "es").await?;
    if let Some(e) = &e {
        cache::guardar(db, e)?;
    }
    Ok(e)
}

fn texto_impreso(c: &ScryCard) -> bool {
    c.printed_text.is_some()
        || c.card_faces.iter().any(|f| f.printed_text.is_some())
        || c.printed_type_line.is_some()
}

fn cara_de(en: &ScryCard, es: Option<&ScryCard>, i: Option<usize>) -> FaceData {
    let fe: Option<&CardFace> = i.and_then(|i| en.card_faces.get(i));
    let fs: Option<&CardFace> = match (i, es) {
        (Some(i), Some(es)) => es.card_faces.get(i),
        _ => None,
    };
    let o = |a: Option<&String>, b: Option<&String>| a.or(b).cloned();
    let nombre = fe.map_or(en.name.clone(), |f| f.name.clone());
    let tipo_en =
        o(fe.and_then(|f| f.type_line.as_ref()), en.type_line.as_ref()).unwrap_or_default();
    let reglas_en = match fe {
        Some(f) => f.oracle_text.clone().unwrap_or_default(),
        None => en.oracle_text.clone().unwrap_or_default(),
    };
    let (tipo_es, reglas_es, nombre_es) = match es {
        Some(es) => match i {
            Some(_) => (
                fs.and_then(|f| f.printed_type_line.clone()),
                fs.and_then(|f| f.printed_text.as_deref().map(limpiar)),
                // Alguna impresión trae en cada cara el nombre de la carta
                // entera («Dusk // Dawn»): ese no vale para la cara.
                fs.and_then(|f| f.printed_name.clone())
                    .filter(|n| !n.contains(" // ")),
            ),
            None => (
                es.printed_type_line.clone(),
                es.printed_text.as_deref().map(limpiar),
                es.printed_name.clone(),
            ),
        },
        None => (None, None, None),
    };
    let colores = fe
        .map(|f| f.colors.clone())
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| en.colors.clone());
    FaceData {
        name: nombre,
        // Las caras traen su coste (la de detrás de una transformable, vacío);
        // las cartas de una cara, el de arriba.
        mana_cost: match fe {
            Some(f) => f.mana_cost.clone().unwrap_or_default(),
            None => en.mana_cost.clone().unwrap_or_default(),
        },
        type_line: ByLang {
            es: tipo_es.unwrap_or_else(|| tipo_en.clone()),
            en: tipo_en,
        },
        rules: ByLang {
            // Una carta sin texto (una tierra básica) tampoco lo tiene en español.
            es: reglas_es.unwrap_or_else(|| reglas_en.clone()),
            en: reglas_en,
        },
        printed_name_es: nombre_es,
        colors: colores,
        power: o(fe.and_then(|f| f.power.as_ref()), en.power.as_ref()),
        toughness: o(fe.and_then(|f| f.toughness.as_ref()), en.toughness.as_ref()),
        loyalty: o(fe.and_then(|f| f.loyalty.as_ref()), en.loyalty.as_ref()),
        defense: o(fe.and_then(|f| f.defense.as_ref()), en.defense.as_ref()),
    }
}

/// La instantánea mecánica.
pub fn datos(en: &ScryCard, es: Option<&ScryCard>, ahora: i64) -> CardData {
    let caras = if por_caras(&en.layout) && en.card_faces.len() >= 2 {
        let mut caras: Vec<FaceData> = (0..en.card_faces.len())
            .map(|i| cara_de(en, es, Some(i)))
            .collect();
        if en.layout == "modal_dfc" {
            sin_pestana_mdfc(&mut caras);
        }
        caras
    } else {
        vec![cara_de(en, es, None)]
    };
    let sin_es = match es {
        None => true,
        Some(e) => !texto_impreso(e) && !en.oracle_text.as_deref().unwrap_or("").is_empty(),
    };
    CardData {
        scryfall_id: en.id.clone(),
        oracle_id: en.oracle_id.clone(),
        layout: en.layout.clone(),
        keywords: en.keywords.clone().unwrap_or_default(),
        rarity: en.rarity.clone().unwrap_or_else(|| "common".into()),
        frame_effects: en.frame_effects.clone(),
        colors: en.colors.clone(),
        color_identity: en.color_identity.clone(),
        cmc: en.cmc.unwrap_or(0.0),
        set: en.set.clone(),
        collector_number: en.collector_number.clone(),
        artist: en
            .artist
            .clone()
            .or_else(|| en.card_faces.first().and_then(|f| f.artist.clone())),
        faces: caras,
        sin_texto_es: sin_es,
        oversized: en.oversized.unwrap_or(false),
        hand_modifier: en.hand_modifier.clone(),
        life_modifier: en.life_modifier.clone(),
        meld_arriba: None,
        fetched_at: ahora,
    }
}

/// Las impresiones españolas de las MDFC traen al final de cada cara la
/// pestaña de la otra («Tierra / {T}: Agrega {R}.»); el motor ya la pinta.
fn sin_pestana_mdfc(caras: &mut [FaceData]) {
    let tipos: Vec<String> = caras.iter().map(|c| c.type_line.es.clone()).collect();
    for (i, c) in caras.iter_mut().enumerate() {
        let Some(otro) = tipos.get(1 - i.min(1)) else {
            continue;
        };
        let lineas: Vec<&str> = c.rules.es.lines().collect();
        if let Some(j) = lineas.iter().position(|l| l.trim() == otro.trim()) {
            if j > 0 {
                c.rules.es = lineas[..j].join("\n");
            }
        }
    }
}

/// Meld: la carta fundida de esta parte (de Scryfall, con su española si la
/// hay) y si su mitad de arriba va en el dorso de esta.
pub async fn fundida<A: CardApi>(
    db: &Db,
    api: &A,
    en: &ScryCard,
) -> AppResult<Option<(ScryCard, Option<ScryCard>, bool)>> {
    if en.layout != "meld" {
        return Ok(None);
    }
    let Some(parte) = en.all_parts.iter().find(|p| p.component == "meld_result") else {
        return Ok(None);
    };
    if parte.id == en.id {
        return Ok(None);
    }
    let res = carta_completa(db, api, &parte.id).await?;
    let res_es = impresion_es(db, api, &res).await?;
    // La parte que comparte número con la fundida (15a ↔ 15b) lleva la
    // mitad de arriba.
    let num = |s: &str| {
        s.trim_end_matches(|c: char| c.is_ascii_alphabetic())
            .to_string()
    };
    let arriba = num(&res.collector_number) == num(&en.collector_number);
    Ok(Some((res, res_es, arriba)))
}

/// Añade la carta fundida como segunda cara.
pub fn con_fundida(mut d: CardData, f: Option<(ScryCard, Option<ScryCard>, bool)>) -> CardData {
    if let Some((res, res_es, arriba)) = f {
        d.faces.push(cara_de(&res, res_es.as_ref(), None));
        d.meld_arriba = Some(arriba);
    }
    d
}

/// El estilo de partida: paisaje para las tierras básicas; si no, el de la
/// colección o el despejado.
fn estilo_inicial(d: &CardData, coleccion: Option<&Collection>) -> DesignStyle {
    let basica = d
        .faces
        .first()
        .is_some_and(|f| f.type_line.en.starts_with("Basic Land"));
    if basica {
        DesignStyle::FullArt
    } else {
        coleccion.map_or(DesignStyle::Borderless, |c| c.default_style)
    }
}

/// Monta el borrador de una carta del deck.
pub fn montar(
    card: &DeckCard,
    datos: CardData,
    coleccion: Option<&Collection>,
    usados: &[String],
) -> DesignDraft {
    let mut avisos = Vec::new();
    if datos.sin_texto_es {
        avisos.push(Aviso::SinTextoOficial);
    }
    let caras = datos
        .faces
        .iter()
        .map(|f| FaceDesign {
            art: None,
            texts: ByLang {
                es: FaceTexts {
                    name: f.printed_name_es.clone().unwrap_or_else(|| f.name.clone()),
                    ..FaceTexts::default()
                },
                en: FaceTexts {
                    name: f.name.clone(),
                    ..FaceTexts::default()
                },
            },
        })
        .collect();
    let numero = siguiente_numero(usados.iter().map(String::as_str), datos.es_ficha());
    let design = CardDesign {
        version: VERSION_DISENO,
        status: DesignStatus::Draft,
        collection: coleccion.map(|c| c.id.clone()),
        style: estilo_inicial(&datos, coleccion),
        // El estilo propio del pack de la colección (no en las básicas, que
        // van en paisaje).
        estilo_pack: (estilo_inicial(&datos, coleccion) != DesignStyle::FullArt)
            .then(|| coleccion.and_then(|c| c.estilo_pack_defecto.clone()))
            .flatten(),
        variante_pack: None,
        frame: None,
        faces: caras,
        before: BeforeDesign {
            source: card.source.clone(),
            front: card.front.clone(),
            back: card.back.clone(),
            edits: card.edits,
        },
        card: datos,
        footer: FooterOptions {
            number: numero,
            ..FooterOptions::default()
        },
        options: DesignOptions {
            show_flavor: true,
            ..DesignOptions::default()
        },
        emblem: EmblemChoice::Inherit,
        thumb_key: None,
    };
    DesignDraft { design, avisos }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::resolve::tests::carta;

    fn dusk_es() -> ScryCard {
        serde_json::from_str(include_str!("../scryfall/fixtures/dusk-dawn-es.json")).unwrap()
    }

    #[test]
    fn dusk_dawn_en_espanol_da_dos_caras_con_secuela() {
        let es = dusk_es();
        // La impresión española trae también el Oracle en inglés.
        let d = datos(&es, Some(&es), 1);
        assert_eq!(d.faces.len(), 2);
        assert!(!d.sin_texto_es);
        let alba = &d.faces[1];
        assert_eq!(alba.name, "Dawn");
        assert_eq!(alba.printed_name_es.as_deref(), Some("Alba"));
        assert!(alba.rules.es.starts_with("Secuela."));
        assert!(alba.rules.en.starts_with("Aftermath"));
        assert_eq!(alba.mana_cost, "{3}{W}{W}");
        assert_eq!(alba.type_line.es, "Conjuro");
    }

    #[test]
    fn sin_impresion_espanola_avisa() {
        let en = carta(serde_json::json!({
            "id":"p","name":"Progenitus","lang":"en","set":"con","collector_number":"121",
            "layout":"normal","mana_cost":"{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}",
            "type_line":"Legendary Creature — Hydra Avatar","oracle_text":"Protection from everything.",
            "power":"10","toughness":"10","rarity":"mythic","full_art":false
        }));
        let d = datos(&en, None, 1);
        assert!(d.sin_texto_es);
        assert_eq!(d.faces[0].rules.es, "Protection from everything.");
        assert_eq!(d.faces[0].power.as_deref(), Some("10"));
        let card = DeckCard {
            id: "c".into(),
            source: CardSource::Scryfall {
                scryfall_id: "p".into(),
            },
            front: None,
            back: proxy_core::model::BackSpec::DeckDefault,
            edits: Default::default(),
            locked: false,
            meta: Default::default(),
            design: None,
            marca: None,
        };
        let b = montar(&card, d, None, &["0001".into(), "T001".into()]);
        assert_eq!(b.avisos, vec![Aviso::SinTextoOficial]);
        assert_eq!(b.design.footer.number, "0002");
        assert_eq!(b.design.style, DesignStyle::Borderless);
        assert_eq!(b.design.faces[0].texts.en.name, "Progenitus");
    }

    #[test]
    fn una_basica_va_en_paisaje() {
        let en = carta(serde_json::json!({
            "id":"f","name":"Plains","lang":"en","set":"x","collector_number":"1",
            "layout":"normal","type_line":"Basic Land — Plains","rarity":"common","full_art":true
        }));
        let d = datos(&en, Some(&en), 1);
        assert_eq!(estilo_inicial(&d, None), DesignStyle::FullArt);
    }
}

/// Prueba en vivo (red): `cargo test --lib borrador::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;
    use crate::scryfall::client::Scryfall;

    /// Las cartas de prueba de los dorados de layouts (Fase 5): baja sus
    /// datos de Scryfall y los deja en `proxy-core/tests/dorados/layouts/datos`.
    /// `cargo test --lib borrador::en_vivo::datos_de_layouts -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn datos_de_layouts() {
        let db = Db::in_memory().unwrap();
        let api = Scryfall::new().unwrap();
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("crates/proxy-core/tests/dorados/layouts/datos");
        std::fs::create_dir_all(&dir).unwrap();
        let cartas = [
            ("pw-3", "!\"Liliana of the Veil\""),
            ("pw-4", "!\"Jace, the Mind Sculptor\""),
            ("saga-urza", "!\"Urza's Saga\""),
            ("saga", "!\"The Eldest Reborn\""),
            ("clase", "!\"Artificer Class\""),
            ("caso", "!\"Case of the Locked Hothouse\""),
            ("partida", "!\"Fire // Ice\""),
            ("fusionar", "!\"Turn // Burn\""),
            ("secuela", "!\"Dusk // Dawn\""),
            ("habitacion", "!\"Bottomless Pool // Locker Room\""),
            ("aventura", "!\"Bonecrusher Giant\""),
            ("presagio", "!\"Bloomvine Regent\""),
            ("flip", "!\"Akki Lavarunner\""),
            ("transform", "!\"Delver of Secrets\""),
            ("mdfc", "!\"Valakut Awakening\""),
            ("batalla", "!\"Invasion of Zendikar\""),
            ("meld", "!\"Bruna, the Fading Light\""),
            ("reversible", "!\"Ajani Goldmane\" layout:reversible_card"),
            (
                "ficha2",
                "layout:double_faced_token t:angel t:demon include:extras",
            ),
            ("leveler", "!\"Student of Warfare\""),
            ("prototipo", "!\"Phyrexian Fleshgorger\""),
            ("mutar", "!\"Gemrazer\""),
            ("estacion", "!\"Atmospheric Greenhouse\""),
            (
                "emblema",
                "!\"Ajani, Adversary of Tyrants Emblem\" include:extras",
            ),
            ("mazmorra", "!\"Dungeon of the Mad Mage\""),
            ("plano", "!\"Akoum\" is:oversized"),
            ("plan", "!\"All in Good Time\" include:extras"),
            ("vanguard", "!\"Ashnod\" layout:vanguard"),
        ];
        for (archivo, q) in cartas {
            let r = api.search(q, false, 1).await.unwrap();
            let Some(c) = r.first() else {
                println!("{archivo}: no está");
                continue;
            };
            let en = carta_completa(&db, &api, &c.id).await.unwrap();
            let es = impresion_es(&db, &api, &en).await.unwrap();
            let f = fundida(&db, &api, &en).await.unwrap();
            let d = con_fundida(datos(&en, es.as_ref(), 0), f);
            let json = serde_json::to_string_pretty(&d).unwrap();
            std::fs::write(dir.join(format!("{archivo}.json")), json).unwrap();
            println!("{archivo}: {} ({} caras)", d.layout, d.faces.len());
        }
    }

    #[tokio::test]
    #[ignore]
    async fn progenitus_y_arcades_en_espanol() {
        let db = Db::in_memory().unwrap();
        let api = Scryfall::new().unwrap();
        // Progenitus (Conflux, en inglés): busca la impresión española.
        let busq = api
            .search("!\"Progenitus\" set:con", false, 1)
            .await
            .unwrap();
        let t = std::time::Instant::now();
        let en = carta_completa(&db, &api, &busq[0].id).await.unwrap();
        println!("carta: {:?}", t.elapsed());
        let t = std::time::Instant::now();
        let es = impresion_es(&db, &api, &en).await.unwrap();
        println!("española: {:?}", t.elapsed());
        let d = datos(&en, es.as_ref(), 0);
        println!("{:#?}", d.faces[0]);
        assert!(!d.sin_texto_es);
        assert!(d.faces[0].rules.es.contains("Progenitus"));
        assert!(d.faces[0].type_line.es.starts_with("Criatura legendaria"));
        // La caché vieja (sin `full_art`) se vuelve a pedir.
        assert!(cache::leer(&db, &en.id)
            .unwrap()
            .unwrap()
            .0
            .completa_para_disenar());
    }
}
