//! **El deck de mesa**: de un `Deck` de la biblioteca al `DeckDeMesa` que
//! recibe el motor, con la imagen final de cada cara preparada
//! (`imagenes.rs`) y lo que el motor necesita saber de cada carta
//! (`InfoCarta`, de la caché de Scryfall; lo que falte se pide como en las
//! estadísticas).
//!
//! Las caras ya preparadas se reconocen por su clave (la huella del diseño o
//! la del render): la segunda partida con el mismo deck no pinta nada.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::LazyLock;

use mesa::lista::{comandante_por_texto, sin_limite_por_texto, CartaDeLista, ListaDeJuego};
use mesa::modelo::{InfoCarta, OrigenCarta, RefImagen};
use mesa::preparar::{candidatas_de_lista, secciones_de_mesa, CartaDeMesa, DeckDeMesa};
use proxy_core::diseno::modelo::CardData;
use proxy_core::diseno::render::Salida;
use proxy_core::imaging::pipeline::{edge_for, render_rgba, RenderSpec, SpecTarget};
use proxy_core::model::{
    BackSpec, CardEdits, CardSource, Deck, DeckCard, FaceImage, ImageFormat, ImageOrigin,
    MarcaCarta,
};
use rayon::prelude::*;
use regex::Regex;

use super::imagenes::{self, ANCHO_MESA};
use crate::diseno::pintar::{colecciones_por_id, efectivo_de, huella_de, pintar};
use crate::error::AppResult;
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::{Identifier, ScryCard};
use crate::state::AppState;

/// Cuántas caras se han pintado de verdad (para los tests: la segunda vez,
/// ninguna).
pub static PINTADAS: AtomicU32 = AtomicU32::new(0);

/// El id de Scryfall que da los datos de juego de una carta.
fn id_scryfall(c: &DeckCard) -> Option<&str> {
    let id = match &c.source {
        CardSource::Scryfall { scryfall_id } | CardSource::Token { scryfall_id } => {
            Some(scryfall_id.as_str())
        }
        CardSource::Custom { scryfall_id, .. } => scryfall_id.as_deref(),
        CardSource::Blank => None,
    };
    id.or_else(|| {
        c.design
            .as_ref()
            .map(|d| d.card.scryfall_id.as_str())
            .filter(|s| !s.is_empty())
    })
}

/// Las cartas de Scryfall de un deck: de la caché y, lo que falte, de la red
/// (sin red, se juega con lo que haya).
pub async fn datos_de<A: CardApi>(
    state: &AppState,
    api: &A,
    cards: &[&DeckCard],
) -> AppResult<HashMap<String, ScryCard>> {
    let mut datos = HashMap::new();
    let mut pedir: Vec<String> = Vec::new();
    for c in cards {
        let Some(id) = id_scryfall(c) else { continue };
        if datos.contains_key(id) || pedir.iter().any(|p| p == id) {
            continue;
        }
        match cache::leer(&state.db, id)? {
            Some((s, _)) if s.keywords.is_some() => {
                datos.insert(id.to_string(), s);
            }
            Some((s, _)) => {
                datos.insert(id.to_string(), s);
                pedir.push(id.to_string());
            }
            None => pedir.push(id.to_string()),
        }
    }
    for lote in pedir.chunks(75) {
        let ids: Vec<Identifier> = lote
            .iter()
            .map(|id| Identifier::Id { id: id.clone() })
            .collect();
        match api.collection(&ids).await {
            Ok(r) => {
                for s in r.data {
                    cache::guardar(&state.db, &s)?;
                    datos.insert(s.id.clone(), s);
                }
            }
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "mesa sin datos de Scryfall");
                break;
            }
        }
    }
    Ok(datos)
}

static BONO: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)(?:equipped|enchanted) creature gets \+(\d+)/\+(\d+)").ok());

/// El «+N/+N» fijo de un aura o un equipo (nunca «por cada»).
fn bono_fijo(texto: &str) -> Option<(i32, i32)> {
    let re = BONO.as_ref()?;
    let m = re.captures(texto)?;
    let resto = &texto[m.get(0)?.end()..];
    let frase = resto.split(['.', '\n']).next().unwrap_or("");
    if frase.to_lowercase().contains("for each") {
        return None;
    }
    Some((
        m.get(1)?.as_str().parse().ok()?,
        m.get(2)?.as_str().parse().ok()?,
    ))
}

/// Lo que el motor sabe de una carta.
pub fn info_de(c: &DeckCard, s: Option<&ScryCard>) -> InfoCarta {
    let tipo = s
        .and_then(|s| s.type_line.clone())
        .or_else(|| c.meta.type_line.clone());
    let t = tipo.clone().unwrap_or_default().to_lowercase();
    let delante = t.split(" // ").next().unwrap_or(&t).to_string();
    let Some(s) = s else {
        // Sin Scryfall en la caché, una carta diseñada trae sus datos.
        if let Some(d) = &c.design {
            return info_de_diseno(&d.card);
        }
        return InfoCarta {
            es_tierra: delante.contains("land"),
            es_criatura: delante.contains("creature"),
            es_legendaria: delante.contains("legendary"),
            tipo,
            ..InfoCarta::default()
        };
    };
    let cara = s.card_faces.first();
    let oracle = s.texto_completo();
    let keywords = s.keywords.clone().unwrap_or_default();
    let es_tierra = delante.contains("land");
    let funciones = proxy_core::stats::funciones::funciones(&proxy_core::stats::funciones::Texto {
        oracle: &oracle,
        keywords: &keywords,
        produced_mana: &s.produced_mana,
        es_tierra,
    })
    .into_iter()
    .map(str::to_string)
    .collect();
    let colores = if s.colors.is_empty() {
        cara.map(|f| f.colors.clone()).unwrap_or_default()
    } else {
        s.colors.clone()
    };
    InfoCarta {
        tipo,
        cmc_decimas: s.cmc.map(|x| (x * 10.0).round().clamp(0.0, 9_999.0) as u16),
        coste: s.coste(),
        colores: colores.iter().filter_map(|x| x.chars().next()).collect(),
        es_tierra,
        es_criatura: delante.contains("creature"),
        es_legendaria: delante.contains("legendary"),
        fuerza: s
            .power
            .clone()
            .or_else(|| cara.and_then(|f| f.power.clone())),
        resistencia: s
            .toughness
            .clone()
            .or_else(|| cara.and_then(|f| f.toughness.clone())),
        lealtad: s.loyalty.clone(),
        palabras_clave: keywords,
        funciones,
        bono_fijo: bono_fijo(&oracle),
        es_comandante: false,
    }
}

/// Lo que hace falta de cada copia elegible de un deck para hacer una lista
/// para jugar: de Scryfall (la caché o la red) o, si no, de su diseño.
pub async fn cartas_de_lista<A: CardApi>(
    state: &AppState,
    api: &A,
    deck: &Deck,
) -> AppResult<Vec<CartaDeLista>> {
    let cartas = candidatas_de_lista(deck);
    let datos = datos_de(state, api, &cartas).await?;
    Ok(cartas
        .iter()
        .map(|c| carta_de_lista(c, id_scryfall(c).and_then(|id| datos.get(id))))
        .collect())
}

fn carta_de_lista(c: &DeckCard, s: Option<&ScryCard>) -> CartaDeLista {
    let (nombre, tipo, oracle, keywords, identidad) = match (s, &c.design) {
        (Some(s), _) => (
            s.name.clone(),
            s.type_line.clone(),
            s.texto_completo(),
            s.keywords.clone().unwrap_or_default(),
            s.color_identity.clone(),
        ),
        (None, Some(d)) => {
            let caras = &d.card.faces;
            let unir = |f: &dyn Fn(&proxy_core::diseno::modelo::FaceData) -> String, sep: &str| {
                caras.iter().map(f).collect::<Vec<_>>().join(sep)
            };
            (
                if c.meta.name.is_empty() {
                    unir(&|f| f.name.clone(), " // ")
                } else {
                    c.meta.name.clone()
                },
                (!caras.is_empty()).then(|| unir(&|f| f.type_line.en.clone(), " // ")),
                unir(&|f| f.rules.en.clone(), "\n"),
                d.card.keywords.clone(),
                d.card.color_identity.clone(),
            )
        }
        (None, None) => (
            c.meta.name.clone(),
            c.meta.type_line.clone(),
            String::new(),
            Vec::new(),
            Vec::new(),
        ),
    };
    let tipo_l = tipo.clone().unwrap_or_default();
    let (puede, pareja) = comandante_por_texto(&tipo_l, &oracle, &keywords);
    let basica = tipo_l
        .split(" // ")
        .next()
        .is_some_and(|t| t.to_lowercase().contains("basic"));
    CartaDeLista {
        id: c.id.clone(),
        nombre,
        tipo,
        identidad: identidad.iter().filter_map(|x| x.chars().next()).collect(),
        puede_ser_comandante: puede,
        con_pareja: pareja,
        sin_limite: basica || sin_limite_por_texto(&oracle),
        imagen: c.front.as_ref().map(|f| f.blob.clone()),
        marca: c.marca.map(|m| {
            match m {
                MarcaCarta::Variante => "variante",
                MarcaCarta::Opcional => "opcional",
                MarcaCarta::Banquillo => "banquillo",
                MarcaCarta::Copia => "copia",
            }
            .to_string()
        }),
    }
}

/// La información de juego de los datos de un diseño (los mismos de Scryfall
/// guardados en la carta), para cuando la caché no los tiene.
fn info_de_diseno(d: &CardData) -> InfoCarta {
    let cara = d.faces.first();
    let tipo = (!d.faces.is_empty()).then(|| {
        d.faces
            .iter()
            .map(|f| f.type_line.en.clone())
            .collect::<Vec<_>>()
            .join(" // ")
    });
    let delante = cara
        .map(|f| f.type_line.en.to_lowercase())
        .unwrap_or_default();
    let oracle = d
        .faces
        .iter()
        .map(|f| f.rules.en.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let es_tierra = delante.contains("land");
    let funciones = proxy_core::stats::funciones::funciones(&proxy_core::stats::funciones::Texto {
        oracle: &oracle,
        keywords: &d.keywords,
        produced_mana: &[],
        es_tierra,
    })
    .into_iter()
    .map(str::to_string)
    .collect();
    let coste = cara.map(|f| f.mana_cost.clone()).filter(|m| !m.is_empty());
    InfoCarta {
        tipo,
        cmc_decimas: Some((d.cmc * 10.0).round().clamp(0.0, 9_999.0) as u16),
        coste,
        colores: d.colors.iter().filter_map(|x| x.chars().next()).collect(),
        es_tierra,
        es_criatura: delante.contains("creature"),
        es_legendaria: delante.contains("legendary"),
        fuerza: cara.and_then(|f| f.power.clone()),
        resistencia: cara.and_then(|f| f.toughness.clone()),
        lealtad: cara.and_then(|f| f.loyalty.clone()),
        palabras_clave: d.keywords.clone(),
        funciones,
        bono_fijo: bono_fijo(&oracle),
        es_comandante: false,
    }
}

/// Lo que hace falta para pintar las caras de un deck.
struct Contexto<'a> {
    state: &'a AppState,
    deck: &'a Deck,
    datos: &'a HashMap<String, ScryCard>,
    colecciones: HashMap<String, proxy_core::diseno::modelo::Collection>,
}

fn spec(img: &FaceImage, edits: CardEdits, deck: &Deck, dorso: bool) -> RenderSpec {
    let s = &deck.settings;
    RenderSpec {
        blob: img.blob.clone(),
        origin: img.origin,
        edits,
        edge: edge_for(&s.edge_darkening, img.origin, dorso),
        card: s.card_size,
        bleed_mm: 0.0,
        bleed_mode: s.bleed_mode,
        fill_corners: s.fill_corners,
        target: SpecTarget::Width(ANCHO_MESA),
        format: ImageFormat::Jpeg,
        jpeg_quality: 85,
        diseno: None,
    }
}

impl Contexto<'_> {
    /// Una imagen del almacén, con sus retoques, como en la vista previa
    /// (sin sangrado).
    fn de_blob(&self, img: &FaceImage, edits: CardEdits, dorso: bool) -> AppResult<RefImagen> {
        let sp = spec(img, edits, self.deck, dorso);
        let clave = format!("r-{}", sp.key());
        if let Some(r) = imagenes::de_indice(self.state, &clave) {
            return Ok(r);
        }
        let (bytes, _) = self.state.leer_blob(&img.blob)?;
        let rgba = render_rgba(&bytes, &sp)?;
        PINTADAS.fetch_add(1, Ordering::Relaxed);
        let r = imagenes::guardar_propia(self.state, &rgba)?;
        imagenes::al_indice(self.state, &clave, &r);
        Ok(r)
    }

    /// Una cara de una carta diseñada, pintada por el motor.
    fn de_diseno(&self, c: &DeckCard, cara: u32) -> AppResult<Option<RefImagen>> {
        let Some(d) = &c.design else { return Ok(None) };
        let s = &self.deck.settings;
        let e = efectivo_de(d, &self.colecciones);
        let clave = format!("d-{}", huella_de(self.state, &e, cara, s.design_lang)?);
        if let Some(r) = imagenes::de_indice(self.state, &clave) {
            return Ok(Some(r));
        }
        let pintada = pintar(
            self.state,
            &e,
            cara,
            s.design_lang,
            s.card_size,
            ANCHO_MESA,
            Salida::Impresion,
        )?;
        PINTADAS.fetch_add(1, Ordering::Relaxed);
        let r = imagenes::guardar_propia(self.state, &pintada.imagen)?;
        imagenes::al_indice(self.state, &clave, &r);
        Ok(Some(r))
    }

    /// Una cara de Scryfall sin retoques: la referencia para que los demás
    /// la bajen, y la copia local.
    fn de_scryfall(&self, c: &DeckCard, img: &FaceImage, cara: u8) -> AppResult<Option<RefImagen>> {
        let CardSource::Scryfall { scryfall_id } = &c.source else {
            return Ok(None);
        };
        if c.design.is_some()
            || !c.edits.is_identity()
            || !matches!(
                img.origin,
                ImageOrigin::ScryfallPng | ImageOrigin::ScryfallLarge
            )
            || !imagenes::es_id_scryfall(scryfall_id)
        {
            return Ok(None);
        }
        let Some(url) = self
            .datos
            .get(scryfall_id)
            .and_then(|s| s.print_url(usize::from(cara)))
            .map(|(u, _)| u)
            .filter(|u| u.starts_with("https://cards.scryfall.io/"))
        else {
            return Ok(None);
        };
        if !imagenes::ruta_scryfall(self.state, scryfall_id, cara).exists() {
            let sp = spec(img, CardEdits::default(), self.deck, false);
            let (bytes, _) = self.state.leer_blob(&img.blob)?;
            let rgba = render_rgba(&bytes, &sp)?;
            PINTADAS.fetch_add(1, Ordering::Relaxed);
            imagenes::guardar_scryfall(self.state, scryfall_id, cara, &rgba)?;
        }
        Ok(Some(RefImagen::Scryfall {
            id: scryfall_id.clone(),
            cara,
            url,
        }))
    }

    fn cara(&self, c: &DeckCard, img: Option<&FaceImage>, cara: u8) -> AppResult<RefImagen> {
        if let Some(r) = self.de_diseno(c, u32::from(cara))? {
            return Ok(r);
        }
        let Some(img) = img else {
            return Ok(RefImagen::Generica);
        };
        if let Some(r) = self.de_scryfall(c, img, cara)? {
            return Ok(r);
        }
        // Los retoques son del frente (como en la hoja).
        let edits = if cara == 0 {
            c.edits
        } else {
            CardEdits::default()
        };
        self.de_blob(img, edits, false)
    }

    fn origen(&self, c: &DeckCard, dorso_deck: &RefImagen) -> AppResult<OrigenCarta> {
        let mut caras = vec![self.cara(c, c.front.as_ref(), 0)?];
        let dorso = match &c.back {
            BackSpec::Dfc { image } => {
                caras.push(self.cara(c, Some(image), 1)?);
                dorso_deck.clone()
            }
            BackSpec::Custom { image } => self.de_blob(image, CardEdits::default(), true)?,
            BackSpec::DeckDefault => dorso_deck.clone(),
        };
        let s = id_scryfall(c).and_then(|id| self.datos.get(id));
        let nombre = if c.meta.name.trim().is_empty() {
            s.map(|s| s.name.clone()).unwrap_or_default()
        } else {
            c.meta.name.clone()
        };
        Ok(OrigenCarta {
            carta_deck: c.id.clone(),
            nombre,
            caras,
            dorso,
            scryfall_id: id_scryfall(c).map(str::to_string),
            info: info_de(c, s),
        })
    }
}

/// El deck de mesa de un deck de la biblioteca. `progreso(hechas, total)`
/// se llama por cada carta preparada.
pub async fn deck_de_mesa<A: CardApi>(
    state: &AppState,
    api: &A,
    deck: &Deck,
    lista: Option<&ListaDeJuego>,
    progreso: &(dyn Fn(u32, u32) + Sync),
) -> AppResult<DeckDeMesa> {
    let secciones = secciones_de_mesa(deck, lista);
    let cartas: Vec<&DeckCard> = secciones.iter().map(|(c, _)| *c).collect();
    let datos = datos_de(state, api, &cartas).await?;
    let ctx = Contexto {
        state,
        deck,
        datos: &datos,
        colecciones: colecciones_por_id(state)?,
    };
    // Sin reverso propio, el de la app (el mismo que sale en las hojas), no
    // uno dibujado por la pantalla.
    let dorso = match &deck.default_back {
        Some(img) => ctx.de_blob(img, CardEdits::default(), true)?,
        None => match state.blobs.generic_back(&state.db) {
            Ok(img) => ctx.de_blob(&img, CardEdits::default(), true)?,
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "sin el reverso de la app para la mesa");
                RefImagen::Generica
            }
        },
    };
    let total = secciones.len() as u32;
    let hechas = AtomicU32::new(0);
    let cartas: Vec<AppResult<CartaDeMesa>> = secciones
        .par_iter()
        .map(|(c, seccion)| {
            let origen = ctx.origen(c, &dorso)?;
            progreso(hechas.fetch_add(1, Ordering::Relaxed) + 1, total);
            Ok(CartaDeMesa {
                carta_deck: c.id.clone(),
                seccion: *seccion,
                origen,
            })
        })
        .collect();
    Ok(DeckDeMesa {
        nombre: deck.name.clone(),
        cartas: cartas.into_iter().collect::<AppResult<_>>()?,
        tapete: None,
        dorso,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::scryfall::models::Collection;
    use crate::state::Paths;
    use crate::store::blobs::tests::png;
    use proxy_core::diseno::modelo::{ByLang, CardData, FaceData};
    use proxy_core::model::{CardMeta, PrintSettings, DECK_SCHEMA_VERSION};

    /// Un Scryfall sin red: todo sale de la caché.
    pub struct SinRed;

    fn no<T>() -> AppResult<T> {
        Err(crate::error::AppError::new(
            crate::error::ErrorCode::Network,
        ))
    }

    impl CardApi for SinRed {
        async fn collection(&self, _: &[Identifier]) -> AppResult<Collection> {
            no()
        }
        async fn card(&self, _: &str) -> AppResult<Option<ScryCard>> {
            no()
        }
        async fn card_lang(&self, _: &str, _: &str, _: &str) -> AppResult<Option<ScryCard>> {
            no()
        }
        async fn search(&self, _: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            no()
        }
        async fn download(&self, _: &str) -> AppResult<Vec<u8>> {
            no()
        }
    }

    const ID: &str = "0000579f-7b35-4ed3-b44c-db2a538066fe";

    fn carta(id: &str, front: FaceImage) -> DeckCard {
        DeckCard {
            id: id.into(),
            source: CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            front: Some(front),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: id.into(),
                ..CardMeta::default()
            },
            design: None,
            marca: None,
        }
    }

    #[tokio::test]
    async fn tres_cartas_tres_imagenes_y_la_segunda_vez_nada() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        // De Scryfall sin retoques, con su carta en la caché.
        let sf: ScryCard = serde_json::from_value(serde_json::json!({
            "id": ID, "name": "Grizzly Bears", "lang": "en", "set": "lea",
            "collector_number": "1", "layout": "normal", "cmc": 2.0,
            "type_line": "Creature — Bear", "mana_cost": "{1}{G}",
            "oracle_text": "", "keywords": [], "power": "2", "toughness": "2",
            "colors": ["G"],
            "image_uris": { "png": "https://cards.scryfall.io/png/front/x.png" }
        }))
        .unwrap();
        crate::scryfall::cache::guardar(&s.db, &sf).unwrap();
        let mut oso = carta(
            "oso",
            s.blobs
                .put_bytes(&s.db, &png(745, 1040, 1), ImageOrigin::ScryfallPng)
                .unwrap(),
        );
        oso.source = CardSource::Scryfall {
            scryfall_id: ID.into(),
        };
        // Subida y recortada.
        let mut propia = carta(
            "propia",
            s.blobs
                .put_bytes(&s.db, &png(630, 880, 2), ImageOrigin::Upload)
                .unwrap(),
        );
        propia.edits.crop.zoom = 1.3;
        // Diseñada.
        let mut disenada = carta(
            "disenada",
            s.blobs
                .put_bytes(&s.db, &png(63, 88, 3), ImageOrigin::Upload)
                .unwrap(),
        );
        let datos = CardData {
            scryfall_id: "x".into(),
            layout: "normal".into(),
            rarity: "common".into(),
            set: "tst".into(),
            collector_number: "1".into(),
            faces: vec![FaceData {
                name: "Sola".into(),
                mana_cost: "{1}".into(),
                type_line: ByLang {
                    es: "Artefacto".into(),
                    en: "Artifact".into(),
                    otros: Default::default(),
                },
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        disenada.design = Some(crate::diseno::borrador::montar(&disenada, datos, None, &[]).design);
        let deck = Deck {
            id: "d".into(),
            name: "Prueba".into(),
            decklist_text: String::new(),
            cards: vec![oso, propia, disenada],
            settings: PrintSettings::default(),
            default_back: None,
            schema_version: DECK_SCHEMA_VERSION,
        };
        let antes = PINTADAS.load(Ordering::SeqCst);
        let m = deck_de_mesa(&s, &SinRed, &deck, None, &|_, _| {})
            .await
            .unwrap();
        // Las tres caras y el reverso de la app (el deck no tiene uno propio).
        assert_eq!(PINTADAS.load(Ordering::SeqCst) - antes, 4);
        assert!(matches!(m.dorso, RefImagen::Propia { .. }), "{:?}", m.dorso);
        let caras: Vec<&RefImagen> = m.cartas.iter().map(|c| &c.origen.caras[0]).collect();
        assert!(
            matches!(caras[0], RefImagen::Scryfall { id, cara: 0, url } if id == ID && url.starts_with("https://cards.scryfall.io/"))
        );
        for r in &caras[1..] {
            let RefImagen::Propia { sha256, ancho, .. } = r else {
                panic!("{r:?}")
            };
            assert_eq!(*ancho, ANCHO_MESA);
            assert!(imagenes::ruta_propia(&s, sha256).exists());
        }
        assert!(imagenes::ruta_scryfall(&s, ID, 0).exists());
        assert_eq!(m.cartas[0].origen.info.cmc_decimas, Some(20));
        assert!(m.cartas[0].origen.info.es_criatura);
        // La segunda vez, nada que pintar y las mismas referencias.
        let antes = PINTADAS.load(Ordering::SeqCst);
        let m2 = deck_de_mesa(&s, &SinRed, &deck, None, &|_, _| {})
            .await
            .unwrap();
        assert_eq!(PINTADAS.load(Ordering::SeqCst), antes);
        assert_eq!(m, m2);
        // Y se sirven reducidas.
        let RefImagen::Propia { sha256, .. } = caras[1] else {
            unreachable!()
        };
        let b = imagenes::servir(&s, &imagenes::ImagenMesa::Propia(sha256.clone()), 244).unwrap();
        assert_eq!(
            proxy_core::imaging::decode::decode(&b).unwrap().width(),
            244
        );
    }

    #[test]
    fn una_disenada_sin_scryfall_trae_su_fuerza_y_su_tipo() {
        let mut c = carta(
            "oso",
            FaceImage {
                blob: "0".repeat(64),
                origin: ImageOrigin::Upload,
                width: 63,
                height: 88,
            },
        );
        let datos = CardData {
            cmc: 2.0,
            colors: vec!["G".into()],
            keywords: vec!["Trample".into()],
            faces: vec![FaceData {
                name: "Oso".into(),
                mana_cost: "{1}{G}".into(),
                type_line: ByLang {
                    es: "Criatura — Oso".into(),
                    en: "Creature — Bear".into(),
                    otros: Default::default(),
                },
                power: Some("2".into()),
                toughness: Some("2".into()),
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        c.design = Some(crate::diseno::borrador::montar(&c, datos, None, &[]).design);
        let i = info_de(&c, None);
        assert_eq!(i.fuerza.as_deref(), Some("2"));
        assert_eq!(i.resistencia.as_deref(), Some("2"));
        assert!(i.es_criatura && !i.es_tierra);
        assert_eq!(i.tipo.as_deref(), Some("Creature — Bear"));
        assert_eq!(i.coste.as_deref(), Some("{1}{G}"));
        assert_eq!(i.cmc_decimas, Some(20));
        assert_eq!(i.colores, vec!['G']);
        assert_eq!(i.palabras_clave, vec!["Trample".to_string()]);
    }

    #[test]
    fn una_disenada_sabe_si_puede_ser_comandante() {
        let mut c = carta(
            "kruphix",
            FaceImage {
                blob: "1".repeat(64),
                origin: ImageOrigin::Upload,
                width: 63,
                height: 88,
            },
        );
        c.marca = Some(MarcaCarta::Variante);
        let datos = CardData {
            color_identity: vec!["G".into(), "U".into()],
            faces: vec![FaceData {
                name: "Kruphix, God of Horizons".into(),
                type_line: ByLang {
                    es: "Criatura legendaria — Dios".into(),
                    en: "Legendary Creature — God".into(),
                    otros: Default::default(),
                },
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        c.design = Some(crate::diseno::borrador::montar(&c, datos, None, &[]).design);
        let l = carta_de_lista(&c, None);
        assert!(l.puede_ser_comandante && !l.con_pareja && !l.sin_limite);
        assert_eq!(l.identidad, vec!['G', 'U']);
        assert_eq!(l.imagen.as_deref(), Some("1".repeat(64).as_str()));
        assert_eq!(l.marca.as_deref(), Some("variante"));
    }

    #[test]
    fn el_bono_fijo_de_auras_y_equipos() {
        assert_eq!(
            bono_fijo("Equipped creature gets +2/+2.\nEquip {2}"),
            Some((2, 2))
        );
        assert_eq!(
            bono_fijo("Enchant creature\nEnchanted creature gets +1/+0 and has flying."),
            Some((1, 0))
        );
        assert_eq!(
            bono_fijo("Equipped creature gets +1/+1 for each Equipment you control."),
            None
        );
        assert_eq!(bono_fijo("Flying"), None);
    }
}
