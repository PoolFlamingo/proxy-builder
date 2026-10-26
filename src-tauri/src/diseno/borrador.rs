//! **Borrador de un diseño** a partir de la carta de Scryfall.
//!
//! El inglés sale del Oracle (Scryfall lo trae en cualquier impresión) y cada
//! otro idioma, de la impresión en ese idioma: la misma si la carta del deck
//! ya lo está; si no, la más reciente; si no, la del mismo set y número. Sin
//! ninguna, el Oracle con el aviso `sinTextoOficial`. El español se baja
//! siempre (como antes de haber más idiomas); los demás, solo el idioma de
//! diseño del deck (`anadir_idioma`). Rumano, polaco, neerlandés y ucraniano
//! no tienen cartas: no se busca nada y la línea de tipo sale de las tablas.

use proxy_core::diseno::borrador::{siguiente_numero, tipo_traducido};
use proxy_core::diseno::estructura::limpiar;
use proxy_core::diseno::modelo::*;
use proxy_core::model::{CardSource, DeckCard};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::{CardFace, ScryCard};
use crate::store::db::Db;

use super::traduccion::traducir_tipo;

/// Layouts cuyas caras se diseñan por separado (las de `card_faces`).
fn por_caras(layout: &str) -> bool {
    matches!(
        layout,
        "split"
            | "flip"
            | "transform"
            | "modal_dfc"
            | "adventure"
            | "prepare"
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

/// La impresión española de una carta (`impresion` en español).
pub async fn impresion_es<A: CardApi>(
    db: &Db,
    api: &A,
    c: &ScryCard,
) -> AppResult<Option<ScryCard>> {
    impresion(db, api, c, DesignLang::Es).await
}

/// La impresión de una carta en `lang`, si existe: la de la propia carta si
/// ya está en ese idioma; si no, **la más reciente** (el texto vigente, que
/// en las reediciones corrige plantillas viejas); si no, la del mismo set y
/// número. Si esa impresión no trae la línea de tipo (Scryfall se la deja en
/// blanco en algunas), se toma la de otra carta del idioma con el mismo tipo
/// en inglés, para que no salga «Land» en una carta en español. En inglés y
/// en los idiomas sin cartas impresas no hay nada que buscar.
pub async fn impresion<A: CardApi>(
    db: &Db,
    api: &A,
    c: &ScryCard,
    lang: DesignLang,
) -> AppResult<Option<ScryCard>> {
    if lang == DesignLang::En || !lang.impresas() {
        return Ok(None);
    }
    let mut p = buscar_impresion(db, api, c, lang.codigo()).await?;
    if let Some(e) = p.as_mut() {
        completar_tipo(api, c, e, lang.codigo()).await;
    }
    Ok(p)
}

/// Rellena en `es` (la impresión en el idioma `codigo`) las líneas de tipo
/// que le faltan. Sin red o sin otra carta con ese tipo, se queda como
/// estaba (luego se prueba con las tablas sin red; si no, el inglés).
async fn completar_tipo<A: CardApi>(api: &A, en: &ScryCard, es: &mut ScryCard, codigo: &str) {
    if es.card_faces.is_empty() {
        if es.printed_type_line.is_none() {
            if let Some(tipo) = en.type_line.as_deref() {
                es.printed_type_line = traducir_tipo(api, tipo, codigo).await.ok().flatten();
            }
        }
        return;
    }
    for (i, cara) in es.card_faces.iter_mut().enumerate() {
        if cara.printed_type_line.is_some() {
            continue;
        }
        let tipo = en
            .card_faces
            .get(i)
            .and_then(|f| f.type_line.as_deref())
            .or(en.type_line.as_deref());
        if let Some(tipo) = tipo {
            cara.printed_type_line = traducir_tipo(api, tipo, codigo).await.ok().flatten();
        }
    }
}

async fn buscar_impresion<A: CardApi>(
    db: &Db,
    api: &A,
    c: &ScryCard,
    codigo: &str,
) -> AppResult<Option<ScryCard>> {
    if c.lang == codigo {
        return Ok(Some(c.clone()));
    }
    if let Some(oracle) = &c.oracle_id {
        let mut todas = api
            .search(&format!("oracleid:{oracle} lang:{codigo}"), true, 60)
            .await
            .unwrap_or_default();
        todas.retain(|e| e.lang == codigo && texto_impreso(e));
        todas.sort_by(|a, b| b.released_at.cmp(&a.released_at));
        if let Some(e) = todas.into_iter().next() {
            cache::guardar(db, &e)?;
            return Ok(Some(e));
        }
    }
    if let Some((e, _)) = cache::leer_impresion(db, &c.set, &c.collector_number, codigo)? {
        if e.completa_para_disenar() {
            return Ok(Some(e));
        }
    }
    let e = api.card_lang(&c.set, &c.collector_number, codigo).await?;
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

/// Línea de tipo, reglas y nombre impresos de una cara (o de la carta, con
/// `i = None`) en una impresión.
fn impresos(
    p: Option<&ScryCard>,
    i: Option<usize>,
) -> (Option<String>, Option<String>, Option<String>) {
    match (p, i) {
        (Some(p), Some(i)) => {
            let f = p.card_faces.get(i);
            (
                f.and_then(|f| f.printed_type_line.clone()),
                f.and_then(|f| f.printed_text.as_deref().map(limpiar)),
                // Alguna impresión trae en cada cara el nombre de la carta
                // entera («Dusk // Dawn»): ese no vale para la cara.
                f.and_then(|f| f.printed_name.clone())
                    .filter(|n| !n.contains(" // ")),
            )
        }
        (Some(p), None) => (
            p.printed_type_line.clone(),
            p.printed_text.as_deref().map(limpiar),
            p.printed_name.clone(),
        ),
        (None, _) => (None, None, None),
    }
}

fn cara_de(en: &ScryCard, es: Option<&ScryCard>, i: Option<usize>) -> FaceData {
    let fe: Option<&CardFace> = i.and_then(|i| en.card_faces.get(i));
    let o = |a: Option<&String>, b: Option<&String>| a.or(b).cloned();
    let nombre = fe.map_or(en.name.clone(), |f| f.name.clone());
    let tipo_en =
        o(fe.and_then(|f| f.type_line.as_ref()), en.type_line.as_ref()).unwrap_or_default();
    let reglas_en = match fe {
        Some(f) => f.oracle_text.clone().unwrap_or_default(),
        None => en.oracle_text.clone().unwrap_or_default(),
    };
    let (tipo_es, reglas_es, nombre_es) = impresos(es, i);
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
            es: tipo_es
                .or_else(|| tipo_traducido(&tipo_en, DesignLang::Es))
                .unwrap_or_else(|| tipo_en.clone()),
            en: tipo_en,
            otros: Default::default(),
        },
        rules: ByLang {
            // Una carta sin texto (una tierra básica) tampoco lo tiene en español.
            es: reglas_es.unwrap_or_else(|| reglas_en.clone()),
            en: reglas_en,
            otros: Default::default(),
        },
        printed_name_es: nombre_es,
        printed_names: Default::default(),
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
        sin_texto: Vec::new(),
        oversized: en.oversized.unwrap_or(false),
        hand_modifier: en.hand_modifier.clone(),
        life_modifier: en.life_modifier.clone(),
        meld_arriba: None,
        fetched_at: ahora,
    }
}

/// Las impresiones de las MDFC traen al final de cada cara la pestaña de la
/// otra («Tierra / {T}: Agrega {R}.»); el motor ya la pinta.
fn sin_pestana_mdfc(caras: &mut [FaceData]) {
    sin_pestana_mdfc_en(caras, DesignLang::Es);
}

fn sin_pestana_mdfc_en(caras: &mut [FaceData], lang: DesignLang) {
    let tipos: Vec<String> = caras
        .iter()
        .map(|c| c.type_line.get(lang).clone())
        .collect();
    for (i, c) in caras.iter_mut().enumerate() {
        let Some(otro) = tipos.get(1 - i.min(1)) else {
            continue;
        };
        if !c.rules.tiene(lang) {
            continue;
        }
        let reglas = c.rules.get(lang).clone();
        let lineas: Vec<&str> = reglas.lines().collect();
        if let Some(j) = lineas.iter().position(|l| l.trim() == otro.trim()) {
            if j > 0 {
                *c.rules.get_mut(lang) = lineas[..j].join("\n");
            }
        }
    }
}

/// Añade a la instantánea los textos de `lang` (ni español ni inglés, que
/// hace `datos`) de la impresión `p` en ese idioma. Sin impresión, el
/// idioma queda marcado como sin texto (cae al Oracle) y la línea de tipo
/// sale de las tablas sin red, si se sabe. La carta fundida de un meld (la
/// segunda cara) se queda con el inglés.
pub fn anadir_idioma(d: &mut CardData, en: &ScryCard, lang: DesignLang, p: Option<&ScryCard>) {
    if matches!(lang, DesignLang::Es | DesignLang::En) {
        return;
    }
    let indices: Vec<Option<usize>> = if por_caras(&en.layout) && en.card_faces.len() >= 2 {
        (0..en.card_faces.len()).map(Some).collect()
    } else {
        vec![None]
    };
    for (k, i) in indices.into_iter().enumerate() {
        let Some(f) = d.faces.get_mut(k) else {
            continue;
        };
        let (tipo, reglas, nombre) = impresos(p, i);
        let tipo = tipo
            .or_else(|| tipo_traducido(&f.type_line.en, lang))
            .unwrap_or_else(|| f.type_line.en.clone());
        *f.type_line.get_mut(lang) = tipo;
        if p.is_some() {
            // Una carta sin texto (una tierra básica) tampoco lo tiene en
            // otro idioma.
            *f.rules.get_mut(lang) = reglas.unwrap_or_else(|| f.rules.en.clone());
        } else {
            f.rules.otros.remove(&lang);
        }
        f.poner_printed_name(lang, nombre);
    }
    if en.layout == "modal_dfc" {
        sin_pestana_mdfc_en(&mut d.faces, lang);
    }
    let sin = match p {
        None => true,
        Some(e) => !texto_impreso(e) && !en.oracle_text.as_deref().unwrap_or("").is_empty(),
    };
    d.poner_sin_texto(lang, sin);
}

/// La instantánea completa de una carta para diseñar en `lang`: el inglés,
/// el español, la carta fundida de un meld y, si es otro, ese idioma.
pub async fn datos_de<A: CardApi>(
    db: &Db,
    api: &A,
    en: &ScryCard,
    lang: DesignLang,
) -> AppResult<CardData> {
    let es = impresion_es(db, api, en).await?;
    let fundida = fundida(db, api, en).await?;
    let mut d = con_fundida(datos(en, es.as_ref(), crate::store::ahora_ms()), fundida);
    if !matches!(lang, DesignLang::Es | DesignLang::En) {
        let p = impresion(db, api, en, lang).await?;
        anadir_idioma(&mut d, en, lang, p.as_ref());
    }
    Ok(d)
}

/// Los textos de `lang` (ni español ni inglés) de un diseño que aún no los
/// tiene: el nombre temático, si se puso uno en inglés o en español (si no,
/// el impreso en ese idioma o el inglés), y el sabor.
pub fn poner_textos(d: &mut CardDesign, lang: DesignLang) {
    if matches!(lang, DesignLang::Es | DesignLang::En) {
        return;
    }
    for (i, f) in d.faces.iter_mut().enumerate() {
        if f.texts.tiene(lang) {
            continue;
        }
        let datos = d.card.faces.get(i);
        let oficial_en = datos.map_or("", |x| x.name.as_str());
        let oficial_es = datos
            .and_then(|x| x.printed_name_es.as_deref())
            .unwrap_or(oficial_en);
        let tematico = [
            (f.texts.en.name.trim(), oficial_en),
            (f.texts.es.name.trim(), oficial_es),
        ]
        .into_iter()
        .find(|(n, o)| !n.is_empty() && n != o)
        .map(|(n, _)| n.to_string());
        let nombre = tematico
            .or_else(|| datos.and_then(|x| x.printed_name(lang)).map(str::to_string))
            .unwrap_or_else(|| oficial_en.to_string());
        let sabor = if f.texts.en.flavor.trim().is_empty() {
            f.texts.es.flavor.clone()
        } else {
            f.texts.en.flavor.clone()
        };
        f.texts.otros.insert(
            lang,
            FaceTexts {
                name: nombre,
                flavor: sabor,
                ..FaceTexts::default()
            },
        );
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
                otros: Default::default(),
            },
            ajustes: None,
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

    /// Solo sabe contestar a la búsqueda de una carta española por su tipo.
    struct ApiTipos(Vec<ScryCard>);

    impl CardApi for ApiTipos {
        async fn collection(
            &self,
            _: &[crate::scryfall::models::Identifier],
        ) -> AppResult<crate::scryfall::models::Collection> {
            Err(AppError::new(ErrorCode::Network))
        }
        async fn card(&self, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn card_lang(&self, _: &str, _: &str, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn search(&self, _: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            Ok(self.0.clone())
        }
        async fn download(&self, _: &str) -> AppResult<Vec<u8>> {
            Err(AppError::new(ErrorCode::Network))
        }
    }

    #[tokio::test]
    async fn una_impresion_sin_linea_de_tipo_la_toma_de_otra_carta_con_el_mismo_tipo() {
        let en = carta(serde_json::json!({
            "id":"e","name":"Rogue's Passage","lang":"en","set":"hoc","collector_number":"212",
            "layout":"normal","type_line":"Land","oracle_text":"{T}: Add {C}."
        }));
        // Como la de FIC: tiene el texto impreso pero no el tipo.
        let mut es = en.clone();
        es.lang = "es".into();
        es.printed_text = Some("{T}: Agrega {C}.".into());
        let otra = carta(serde_json::json!({
            "id":"o","name":"Command Tower","lang":"es","set":"x","collector_number":"1",
            "layout":"normal","type_line":"Land","printed_type_line":"Tierra"
        }));
        completar_tipo(&ApiTipos(vec![otra]), &en, &mut es, "es").await;
        assert_eq!(es.printed_type_line.as_deref(), Some("Tierra"));
        let d = datos(&en, Some(&es), 1);
        assert_eq!(d.faces[0].type_line.es, "Tierra");
        assert_eq!(d.faces[0].type_line.en, "Land");

        // Sin ninguna con ese tipo (o sin red) se queda como estaba.
        let mut sola = es.clone();
        sola.printed_type_line = None;
        completar_tipo(&ApiTipos(Vec::new()), &en, &mut sola, "es").await;
        assert_eq!(sola.printed_type_line, None);
    }

    /// Apunta cada búsqueda y contesta con lo que tenga.
    struct ApiApunta(std::sync::Mutex<Vec<String>>, Vec<ScryCard>);

    impl CardApi for ApiApunta {
        async fn collection(
            &self,
            _: &[crate::scryfall::models::Identifier],
        ) -> AppResult<crate::scryfall::models::Collection> {
            Err(AppError::new(ErrorCode::Network))
        }
        async fn card(&self, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn card_lang(&self, _: &str, _: &str, l: &str) -> AppResult<Option<ScryCard>> {
            if let Ok(mut v) = self.0.lock() {
                v.push(format!("card_lang {l}"));
            }
            Ok(None)
        }
        async fn search(&self, q: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            if let Ok(mut v) = self.0.lock() {
                v.push(q.to_string());
            }
            Ok(self.1.clone())
        }
        async fn download(&self, _: &str) -> AppResult<Vec<u8>> {
            Err(AppError::new(ErrorCode::Network))
        }
    }

    fn progenitus() -> ScryCard {
        carta(serde_json::json!({
            "id":"p","oracle_id":"o1","name":"Progenitus","lang":"en","set":"con","collector_number":"121",
            "layout":"normal","type_line":"Legendary Creature — Hydra Avatar",
            "oracle_text":"Protection from everything.","rarity":"mythic"
        }))
    }

    #[tokio::test]
    async fn en_frances_se_busca_la_impresion_francesa_y_en_rumano_nada() {
        let db = Db::in_memory().unwrap();
        let mut fr = progenitus();
        fr.lang = "fr".into();
        fr.printed_name = Some("Progénitus".into());
        fr.printed_type_line = Some("Créature légendaire — hydre et avatar".into());
        fr.printed_text = Some("Protection contre tout.".into());
        let api = ApiApunta(Default::default(), vec![fr]);
        let en = progenitus();
        let p = impresion(&db, &api, &en, DesignLang::Fr).await.unwrap();
        assert_eq!(api.0.lock().unwrap()[0], "oracleid:o1 lang:fr");
        let mut d = datos(&en, None, 1);
        anadir_idioma(&mut d, &en, DesignLang::Fr, p.as_ref());
        let f = &d.faces[0];
        assert_eq!(f.printed_name(DesignLang::Fr), Some("Progénitus"));
        assert_eq!(f.rules.get(DesignLang::Fr), "Protection contre tout.");
        assert_eq!(
            f.type_line.get(DesignLang::Fr),
            "Créature légendaire — hydre et avatar"
        );
        assert!(!d.sin_texto_en(DesignLang::Fr) && d.tiene_idioma(DesignLang::Fr));

        // En rumano no hay cartas: ninguna búsqueda, reglas en inglés y el
        // tipo, si se sabe, de las tablas.
        let api = ApiApunta(Default::default(), Vec::new());
        assert!(impresion(&db, &api, &en, DesignLang::Ro)
            .await
            .unwrap()
            .is_none());
        assert!(api.0.lock().unwrap().is_empty());
        anadir_idioma(&mut d, &en, DesignLang::Ro, None);
        assert!(d.sin_texto_en(DesignLang::Ro) && d.tiene_idioma(DesignLang::Ro));
        assert_eq!(
            d.faces[0].rules.get(DesignLang::Ro),
            "Protection from everything."
        );
        // «Legendary Creature — Hydra Avatar»: el tipo sale de la tabla y los
        // subtipos de criatura, que no se saben sin red, se quedan en inglés.
        assert_eq!(
            d.faces[0].type_line.get(DesignLang::Ro),
            "Creatură legendară — Hydra Avatar"
        );
        let mut tierra = carta(serde_json::json!({
            "id":"t","name":"Plains","lang":"en","set":"x","collector_number":"1",
            "layout":"normal","type_line":"Basic Land — Plains"
        }));
        tierra.oracle_id = None;
        let mut d = datos(&tierra, None, 1);
        anadir_idioma(&mut d, &tierra, DesignLang::Pl, None);
        assert_eq!(
            d.faces[0].type_line.get(DesignLang::Pl),
            "Podstawowa kraina — Równina"
        );
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

    /// *Llanowar Elves* en alemán: nombre y tipo oficiales.
    #[tokio::test]
    #[ignore]
    async fn llanowar_elves_en_aleman() {
        let db = Db::in_memory().unwrap();
        let api = Scryfall::new().unwrap();
        let r = api
            .search("!\"Llanowar Elves\" set:m19", false, 1)
            .await
            .unwrap();
        let en = carta_completa(&db, &api, &r[0].id).await.unwrap();
        let d = datos_de(&db, &api, &en, DesignLang::De).await.unwrap();
        println!("{:#?}", d.faces[0]);
        // El nombre alemán vigente (la impresión más reciente).
        assert_eq!(
            d.faces[0].printed_name(DesignLang::De),
            Some("Llanowarelfen")
        );
        assert_eq!(
            d.faces[0].type_line.get(DesignLang::De),
            "Kreatur — Elf, Druide"
        );
    }

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
