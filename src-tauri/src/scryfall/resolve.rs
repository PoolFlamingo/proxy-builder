//! **De una lista de texto a cartas con imagen guardada.**
//!
//! 1. Las líneas se piden a `/cards/collection` en lotes de 75, y los
//!    resultados se emparejan por set/número o por nombre — **nunca por
//!    posición**: los `not_found` descolocan el orden.
//! 2. Idioma: si se pidió otro, `/cards/{set}/{nº}/{idioma}`; si no existe,
//!    la impresión en ese idioma (mejor del mismo set); si tampoco, inglés con
//!    `lang_fallback` («↩ EN»).
//! 3. Política de imagen: «con imagen» descarta `missing`/`placeholder`;
//!    «solo alta resolución» exige `highres_image`.
//! 4. Tokens (opcional): los `all_parts` con `component == "token"`.
//! 5. Imágenes: el PNG de cada cara al almacén de blobs, 6 a la vez.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use futures::future::join_all;
use proxy_core::decklist::{LineKind, ParsedDecklist, ParsedLine};
use proxy_core::model::{
    BackSpec, CardEdits, CardMeta, CardSource, DeckCard, FaceImage, ImageOrigin,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::client::CardApi;
use super::models::{Identifier, ScryCard};
use crate::error::{AppError, AppResult, ErrorCode};

const LOTE: usize = 75;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImagePolicy {
    #[default]
    BestAvailable,
    WithImage,
    HighresOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolveOptions {
    /// Código de Scryfall (`en`, `es`…) o `any` para «mejor disponible».
    pub lang: String,
    pub image_policy: ImagePolicy,
    pub add_tokens: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UnresolvedReason {
    NotFound,
    NoImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Unresolved {
    pub line_no: u32,
    pub name: String,
    pub reason: UnresolvedReason,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolveResult {
    pub cards: Vec<DeckCard>,
    pub unresolved: Vec<Unresolved>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ResolvePhase {
    Lookup,
    Language,
    Tokens,
    Images,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolveProgress {
    pub phase: ResolvePhase,
    pub done: u32,
    pub total: u32,
}

/// Dónde se guardan las imágenes descargadas (el almacén de blobs, en la app).
pub trait Guardar: Sync {
    fn guardar(&self, bytes: &[u8], origen: ImageOrigin) -> AppResult<FaceImage>;
    /// Apunta los datos de una carta en la caché local (para estadísticas y
    /// para no volver a pedirlos).
    fn apuntar(&self, _carta: &ScryCard) -> AppResult<()> {
        Ok(())
    }
}

fn comprobar(cancelado: &AtomicBool) -> AppResult<()> {
    if cancelado.load(Ordering::SeqCst) {
        Err(AppError::new(ErrorCode::Cancelled))
    } else {
        Ok(())
    }
}

/// Lo que se pide por cada línea distinta.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Peticion {
    name: String,
    set: Option<String>,
    collector: Option<String>,
}

impl Peticion {
    fn de(l: &ParsedLine) -> Peticion {
        Peticion {
            name: l.name.clone(),
            set: l.set.clone(),
            collector: l.collector.clone(),
        }
    }

    fn identificador(&self) -> Identifier {
        match (&self.set, &self.collector) {
            (Some(s), Some(n)) => Identifier::SetNumber {
                set: s.clone(),
                collector_number: n.clone(),
            },
            (Some(s), None) => Identifier::NameSet {
                name: self.name.clone(),
                set: s.clone(),
            },
            _ => Identifier::Name {
                name: self.name.clone(),
            },
        }
    }

    fn casa(&self, c: &ScryCard) -> bool {
        match (&self.set, &self.collector) {
            (Some(s), Some(n)) => {
                c.set.eq_ignore_ascii_case(s) && c.collector_number.eq_ignore_ascii_case(n)
            }
            (Some(s), None) => c.set.eq_ignore_ascii_case(s) && c.responde_a(&self.name),
            _ => c.responde_a(&self.name),
        }
    }
}

/// Busca la impresión de `c` en `lang`, o `None` si no existe.
async fn en_idioma<A: CardApi>(api: &A, c: &ScryCard, lang: &str) -> AppResult<Option<ScryCard>> {
    if let Some(x) = api.card_lang(&c.set, &c.collector_number, lang).await? {
        if x.lang == lang {
            return Ok(Some(x));
        }
    }
    let q = format!("!\"{}\" lang:{}", c.name.replace('"', ""), lang);
    let impresiones = api.search(&q, true, 60).await?;
    Ok(impresiones
        .iter()
        .find(|x| x.lang == lang && x.set == c.set)
        .or_else(|| impresiones.iter().find(|x| x.lang == lang))
        .cloned())
}

/// Si la carta no cumple la política de imagen, otra impresión que sí.
async fn segun_politica<A: CardApi>(
    api: &A,
    c: ScryCard,
    politica: ImagePolicy,
) -> AppResult<Option<ScryCard>> {
    let cumple = |x: &ScryCard| match politica {
        ImagePolicy::BestAvailable => x.face_uris(0).is_some(),
        ImagePolicy::WithImage => x.tiene_imagen_buena(),
        ImagePolicy::HighresOnly => x.tiene_imagen_buena() && x.highres_image,
    };
    if cumple(&c) {
        return Ok(Some(c));
    }
    let q = format!("!\"{}\"", c.name.replace('"', ""));
    let impresiones = api.search(&q, true, 100).await?;
    let mismo_idioma = |x: &&ScryCard| x.lang == c.lang;
    Ok(impresiones
        .iter()
        .filter(|x| cumple(x))
        .find(|x| mismo_idioma(x) && x.set == c.set)
        .or_else(|| impresiones.iter().filter(|x| cumple(x)).find(mismo_idioma))
        .or_else(|| impresiones.iter().find(|x| cumple(x)))
        .cloned())
}

fn meta_de(c: &ScryCard, fallback: bool) -> CardMeta {
    CardMeta {
        name: c.name.clone(),
        set: Some(c.set.clone()),
        collector_number: Some(c.collector_number.clone()),
        lang: Some(c.lang.clone()),
        lang_fallback: fallback,
        layout: Some(c.layout.clone()),
        is_dfc: c.is_dfc(),
        image_status: c.image_status.clone(),
        highres: c.highres_image,
        type_line: c.type_line.clone(),
    }
}

/// Imágenes de una carta (frente y, si es DFC, la cara trasera).
#[derive(Clone)]
struct Caras {
    frente: Option<FaceImage>,
    dorso: Option<FaceImage>,
}

/// Resuelve una lista entera.
pub async fn resolve<A: CardApi, G: Guardar>(
    api: &A,
    guardar: &G,
    lista: &ParsedDecklist,
    opciones: &ResolveOptions,
    cancelado: &AtomicBool,
    mut progreso: impl FnMut(ResolveProgress) + Send,
) -> AppResult<ResolveResult> {
    let lineas: Vec<&ParsedLine> = lista
        .lines
        .iter()
        .filter(|l| l.kind == LineKind::Card && l.included)
        .collect();

    // 1. Buscar cada petición distinta.
    let mut unicas: Vec<Peticion> = Vec::new();
    let mut vistas = HashSet::new();
    for l in &lineas {
        let p = Peticion::de(l);
        if vistas.insert(p.clone()) {
            unicas.push(p);
        }
    }
    let total = unicas.len() as u32;
    let mut encontradas: HashMap<Peticion, ScryCard> = HashMap::new();
    let mut hechas = 0u32;
    for lote in unicas.chunks(LOTE) {
        comprobar(cancelado)?;
        let ids: Vec<Identifier> = lote.iter().map(Peticion::identificador).collect();
        let r = api.collection(&ids).await?;
        for p in lote {
            if let Some(c) = r.data.iter().find(|c| p.casa(c)) {
                encontradas.insert(p.clone(), c.clone());
            }
        }
        hechas += lote.len() as u32;
        progreso(ResolveProgress {
            phase: ResolvePhase::Lookup,
            done: hechas,
            total,
        });
    }

    // 2 y 3. Idioma y política de imagen, por carta distinta.
    let quiere_idioma = opciones.lang != "any" && !opciones.lang.is_empty();
    let mut finales: HashMap<Peticion, (ScryCard, bool)> = HashMap::new();
    let mut sin_imagen: HashSet<Peticion> = HashSet::new();
    let total = encontradas.len() as u32;
    for (i, (p, c)) in encontradas.into_iter().enumerate() {
        comprobar(cancelado)?;
        let mut c = c;
        if quiere_idioma && c.lang != opciones.lang {
            if let Some(x) = en_idioma(api, &c, &opciones.lang).await? {
                c = x;
            }
        }
        match segun_politica(api, c, opciones.image_policy).await? {
            Some(x) => {
                // «↩ EN» si al final no está en el idioma pedido (no había
                // impresión, o la que había no cumplía la política de imagen).
                let fallback = quiere_idioma && x.lang != opciones.lang;
                finales.insert(p, (x, fallback));
            }
            None => {
                sin_imagen.insert(p);
            }
        }
        progreso(ResolvePhase::Language.paso(i as u32 + 1, total));
    }

    // 4. Tokens.
    let mut tokens: Vec<ScryCard> = Vec::new();
    if opciones.add_tokens {
        let mut ids: Vec<String> = Vec::new();
        let mut vistos = HashSet::new();
        for l in &lineas {
            if let Some((c, _)) = finales.get(&Peticion::de(l)) {
                for parte in c.all_parts.iter().filter(|p| p.component == "token") {
                    if vistos.insert(parte.id.clone()) {
                        ids.push(parte.id.clone());
                    }
                }
            }
        }
        let total = ids.len() as u32;
        for (i, lote) in ids.chunks(LOTE).enumerate() {
            comprobar(cancelado)?;
            let pedir: Vec<Identifier> = lote
                .iter()
                .map(|id| Identifier::Id { id: id.clone() })
                .collect();
            let r = api.collection(&pedir).await?;
            // En el orden pedido.
            for id in lote {
                if let Some(t) = r.data.iter().find(|t| &t.id == id) {
                    tokens.push(t.clone());
                }
            }
            progreso(ResolvePhase::Tokens.paso(((i + 1) * LOTE).min(ids.len()) as u32, total));
        }
    }

    // 5. Imágenes: una descarga por carta distinta (frente y dorso DFC).
    let mut por_id: HashMap<String, ScryCard> = HashMap::new();
    for (c, _) in finales.values() {
        por_id.insert(c.id.clone(), c.clone());
    }
    for t in &tokens {
        por_id.insert(t.id.clone(), t.clone());
    }
    for c in por_id.values() {
        guardar.apuntar(c)?;
    }
    let total = por_id.len() as u32;
    let mut caras: HashMap<String, Caras> = HashMap::new();
    let mut hechas = 0u32;
    let todas: Vec<ScryCard> = por_id.into_values().collect();
    for tanda in todas.chunks(12) {
        comprobar(cancelado)?;
        let resultados = join_all(tanda.iter().map(|c| async move {
            let bajar = |cara: usize| async move {
                match c.print_url(cara) {
                    None => Ok::<_, AppError>(None),
                    Some((url, es_png)) => {
                        let bytes = api.download(&url).await?;
                        let origen = if es_png {
                            ImageOrigin::ScryfallPng
                        } else {
                            ImageOrigin::ScryfallLarge
                        };
                        guardar.guardar(&bytes, origen).map(Some)
                    }
                }
            };
            let frente = bajar(0).await?;
            let dorso = if c.is_dfc() { bajar(1).await? } else { None };
            Ok::<_, AppError>((c.id.clone(), Caras { frente, dorso }))
        }))
        .await;
        for r in resultados {
            let (id, cs) = r?;
            caras.insert(id, cs);
        }
        hechas += tanda.len() as u32;
        progreso(ResolvePhase::Images.paso(hechas, total));
    }
    comprobar(cancelado)?;

    // 6. Las copias, en el orden de la lista.
    let hacer = |c: &ScryCard, fuente: CardSource, fallback: bool| -> DeckCard {
        let cs = caras.get(&c.id).cloned().unwrap_or(Caras {
            frente: None,
            dorso: None,
        });
        DeckCard {
            id: uuid::Uuid::new_v4().to_string(),
            source: fuente,
            front: cs.frente,
            back: match cs.dorso {
                Some(image) => BackSpec::Dfc { image },
                None => BackSpec::DeckDefault,
            },
            edits: CardEdits::default(),
            locked: false,
            meta: meta_de(c, fallback),
        }
    };
    let mut cards = Vec::new();
    let mut unresolved = Vec::new();
    for l in &lineas {
        let p = Peticion::de(l);
        match finales.get(&p) {
            Some((c, fallback)) => {
                for _ in 0..l.qty {
                    cards.push(hacer(
                        c,
                        CardSource::Scryfall {
                            scryfall_id: c.id.clone(),
                        },
                        *fallback,
                    ));
                }
            }
            None => unresolved.push(Unresolved {
                line_no: l.line_no,
                name: l.name.clone(),
                reason: if sin_imagen.contains(&p) {
                    UnresolvedReason::NoImage
                } else {
                    UnresolvedReason::NotFound
                },
            }),
        }
    }
    for t in &tokens {
        cards.push(hacer(
            t,
            CardSource::Token {
                scryfall_id: t.id.clone(),
            },
            false,
        ));
    }
    Ok(ResolveResult { cards, unresolved })
}

impl ResolvePhase {
    fn paso(self, done: u32, total: u32) -> ResolveProgress {
        ResolveProgress {
            phase: self,
            done,
            total,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::scryfall::models::Collection;
    use proxy_core::decklist::parse;
    use serde_json::json;
    use std::sync::Mutex;

    /// Un Scryfall de mentira con un puñado de cartas.
    pub struct Falso {
        pub cartas: Vec<ScryCard>,
        pub descargas: Mutex<Vec<String>>,
        pub colecciones: Mutex<u32>,
    }

    pub fn carta(v: serde_json::Value) -> ScryCard {
        serde_json::from_value(v).unwrap()
    }

    pub fn falso() -> Falso {
        let img = |id: &str| json!({"png": format!("https://img/{id}.png"), "large": format!("https://img/{id}.jpg")});
        Falso {
            cartas: vec![
                carta(
                    json!({"id":"sol-cmm","name":"Sol Ring","lang":"en","set":"cmm","collector_number":"410","layout":"normal","image_uris":img("sol-cmm"),"image_status":"highres_scan","highres_image":true}),
                ),
                carta(
                    json!({"id":"sol-cmm-es","name":"Sol Ring","printed_name":"Anillo solar","lang":"es","set":"cmm","collector_number":"410","layout":"normal","image_uris":img("sol-cmm-es"),"image_status":"highres_scan","highres_image":true}),
                ),
                carta(
                    json!({"id":"serra","name":"Serra Ascendant","lang":"en","set":"m11","collector_number":"28","layout":"normal","image_uris":img("serra"),"image_status":"lowres","highres_image":false}),
                ),
                carta(
                    json!({"id":"serra-hr","name":"Serra Ascendant","lang":"en","set":"2x2","collector_number":"25","layout":"normal","image_uris":img("serra-hr"),"image_status":"highres_scan","highres_image":true}),
                ),
                carta(
                    json!({"id":"delver","name":"Delver of Secrets // Insectile Aberration","lang":"en","set":"isd","collector_number":"51","layout":"transform","image_status":"highres_scan","highres_image":true,
                    "card_faces":[{"name":"Delver of Secrets","image_uris":img("delver-0")},{"name":"Insectile Aberration","image_uris":img("delver-1")}]}),
                ),
                carta(
                    json!({"id":"fodder","name":"Dragon Fodder","lang":"en","set":"fdn","collector_number":"535","layout":"normal","image_uris":img("fodder"),"image_status":"highres_scan","highres_image":true,
                    "all_parts":[{"id":"goblin","component":"token","name":"Goblin"},{"id":"fodder","component":"combo_piece","name":"Dragon Fodder"}]}),
                ),
                carta(
                    json!({"id":"goblin","name":"Goblin","lang":"en","set":"tfdn","collector_number":"1","layout":"token","image_uris":img("goblin"),"image_status":"highres_scan","highres_image":true}),
                ),
            ],
            descargas: Mutex::new(Vec::new()),
            colecciones: Mutex::new(0),
        }
    }

    impl CardApi for Falso {
        async fn collection(&self, ids: &[Identifier]) -> AppResult<Collection> {
            *self.colecciones.lock().unwrap() += 1;
            let mut data = Vec::new();
            for id in ids {
                let c = self
                    .cartas
                    .iter()
                    .filter(|c| c.lang == "en")
                    .find(|c| match id {
                        Identifier::Id { id } => &c.id == id,
                        Identifier::SetNumber {
                            set,
                            collector_number,
                        } => &c.set == set && &c.collector_number == collector_number,
                        Identifier::NameSet { name, set } => &c.set == set && c.responde_a(name),
                        Identifier::Name { name } => c.responde_a(name),
                    });
                if let Some(c) = c {
                    data.push(c.clone());
                }
            }
            Ok(Collection {
                data,
                not_found: vec![],
            })
        }
        async fn card(&self, id: &str) -> AppResult<Option<ScryCard>> {
            Ok(self.cartas.iter().find(|c| c.id == id).cloned())
        }
        async fn card_lang(&self, set: &str, n: &str, lang: &str) -> AppResult<Option<ScryCard>> {
            Ok(self
                .cartas
                .iter()
                .find(|c| c.set == set && c.collector_number == n && c.lang == lang)
                .cloned())
        }
        async fn search(&self, q: &str, _prints: bool, _l: usize) -> AppResult<Vec<ScryCard>> {
            let nombre = q.split('"').nth(1).unwrap_or("");
            let lang = q.split("lang:").nth(1);
            Ok(self
                .cartas
                .iter()
                .filter(|c| c.name == nombre && lang.is_none_or(|l| c.lang == l))
                .cloned()
                .collect())
        }
        async fn download(&self, url: &str) -> AppResult<Vec<u8>> {
            self.descargas.lock().unwrap().push(url.to_string());
            Ok(url.as_bytes().to_vec())
        }
    }

    /// Guarda «imágenes» falsas: la huella es el texto de la URL.
    pub struct GuardarFalso;
    impl Guardar for GuardarFalso {
        fn guardar(&self, bytes: &[u8], origin: ImageOrigin) -> AppResult<FaceImage> {
            Ok(FaceImage {
                blob: String::from_utf8_lossy(bytes).to_string(),
                width: 745,
                height: 1040,
                origin,
            })
        }
    }

    fn opciones(lang: &str) -> ResolveOptions {
        ResolveOptions {
            lang: lang.into(),
            image_policy: ImagePolicy::BestAvailable,
            add_tokens: false,
        }
    }

    async fn resolver(texto: &str, o: ResolveOptions) -> (ResolveResult, Falso) {
        let api = falso();
        let r = resolve(
            &api,
            &GuardarFalso,
            &parse(texto),
            &o,
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap();
        (r, api)
    }

    #[tokio::test]
    async fn la_lista_de_la_captura_da_cuatro_cartas() {
        let (r, api) = resolver(
            "1 Sol Ring (CMM) 410\n2 Serra Ascendant\n1 Nonexistent Card",
            opciones("en"),
        )
        .await;
        assert_eq!(r.cards.len(), 3);
        assert_eq!(r.cards[0].meta.set.as_deref(), Some("cmm"));
        assert_eq!(r.cards[0].meta.collector_number.as_deref(), Some("410"));
        assert_eq!(
            r.cards[0].front.as_ref().unwrap().blob,
            "https://img/sol-cmm.png"
        );
        assert_eq!(
            r.cards[0].front.as_ref().unwrap().origin,
            ImageOrigin::ScryfallPng
        );
        assert_ne!(r.cards[1].id, r.cards[2].id);
        assert_eq!(
            r.unresolved,
            vec![Unresolved {
                line_no: 3,
                name: "Nonexistent Card".into(),
                reason: UnresolvedReason::NotFound
            }]
        );
        // Serra se descarga una vez aunque haya dos copias.
        let d = api.descargas.lock().unwrap();
        assert_eq!(d.iter().filter(|u| u.contains("serra")).count(), 1);
    }

    #[tokio::test]
    async fn una_dfc_trae_frente_y_dorso() {
        let (r, _) = resolver("1 Delver of Secrets", opciones("en")).await;
        assert!(r.cards[0].meta.is_dfc);
        assert!(
            matches!(&r.cards[0].back, BackSpec::Dfc { image } if image.blob.contains("delver-1"))
        );
    }

    #[tokio::test]
    async fn el_idioma_se_busca_y_si_no_hay_cae_a_ingles() {
        let (r, _) = resolver("1 Sol Ring (CMM) 410\n1 Serra Ascendant", opciones("es")).await;
        assert_eq!(r.cards[0].meta.lang.as_deref(), Some("es"));
        assert!(!r.cards[0].meta.lang_fallback);
        assert_eq!(r.cards[1].meta.lang.as_deref(), Some("en"));
        assert!(r.cards[1].meta.lang_fallback);
    }

    #[tokio::test]
    async fn solo_alta_resolucion_cambia_de_impresion() {
        let o = ResolveOptions {
            image_policy: ImagePolicy::HighresOnly,
            ..opciones("en")
        };
        let (r, _) = resolver("1 Serra Ascendant (M11) 28", o).await;
        assert_eq!(r.cards[0].meta.set.as_deref(), Some("2x2"));
        assert!(r.cards[0].meta.highres);
    }

    #[tokio::test]
    async fn los_tokens_se_anaden_al_final_una_vez() {
        let o = ResolveOptions {
            add_tokens: true,
            ..opciones("en")
        };
        let (r, _) = resolver("2 Dragon Fodder", o).await;
        assert_eq!(r.cards.len(), 3);
        assert!(
            matches!(&r.cards[2].source, CardSource::Token { scryfall_id } if scryfall_id == "goblin")
        );
    }

    #[tokio::test]
    async fn cancelar_no_devuelve_cartas() {
        let api = falso();
        let cancelado = AtomicBool::new(true);
        let e = resolve(
            &api,
            &GuardarFalso,
            &parse("1 Sol Ring"),
            &opciones("en"),
            &cancelado,
            |_| {},
        )
        .await
        .unwrap_err();
        assert_eq!(e.code, ErrorCode::Cancelled);
    }

    #[tokio::test]
    async fn mas_de_75_cartas_van_en_varios_lotes() {
        let texto: String = (0..80).map(|i| format!("1 Carta {i}\n")).collect();
        let (r, api) = resolver(&texto, opciones("en")).await;
        assert_eq!(*api.colecciones.lock().unwrap(), 2);
        assert_eq!(r.unresolved.len(), 80);
    }

    #[tokio::test]
    async fn el_progreso_avanza_y_termina_completo() {
        let api = falso();
        let mut pasos = Vec::new();
        resolve(
            &api,
            &GuardarFalso,
            &parse("1 Sol Ring\n1 Serra Ascendant"),
            &opciones("en"),
            &AtomicBool::new(false),
            |p| pasos.push(p),
        )
        .await
        .unwrap();
        let imagenes: Vec<_> = pasos
            .iter()
            .filter(|p| p.phase == ResolvePhase::Images)
            .collect();
        assert_eq!(imagenes.last().map(|p| (p.done, p.total)), Some((2, 2)));
    }

    /// En vivo contra Scryfall con el almacén de verdad:
    /// `cargo test -- --ignored resolver_en_vivo --nocapture`.
    #[tokio::test]
    #[ignore]
    async fn resolver_en_vivo() {
        use crate::store::{blobs::BlobStore, db::Db};
        struct Real(Db, BlobStore);
        impl Guardar for Real {
            fn guardar(&self, b: &[u8], o: ImageOrigin) -> AppResult<FaceImage> {
                self.1.put_bytes(&self.0, b, o)
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let real = Real(
            Db::in_memory().unwrap(),
            BlobStore::new(dir.path().join("b")),
        );
        let api = crate::scryfall::client::Scryfall::new().unwrap();
        let t = std::time::Instant::now();
        let r = resolve(
            &api,
            &real,
            &parse("1 Sol Ring (CMM) 410\n2 Serra Ascendant\n1 The One Ring\n1 Delver of Secrets"),
            &ResolveOptions {
                lang: "es".into(),
                image_policy: ImagePolicy::WithImage,
                add_tokens: true,
            },
            &AtomicBool::new(false),
            |p| println!("{p:?}"),
        )
        .await
        .unwrap();
        println!(
            "{} cartas en {:?}; sin resolver: {:?}",
            r.cards.len(),
            t.elapsed(),
            r.unresolved
        );
        for c in &r.cards {
            println!(
                "  {} {:?} {:?} {:?}",
                c.meta.name,
                c.meta.lang,
                c.front.as_ref().map(|f| (f.width, f.height)),
                c.meta.lang_fallback
            );
        }
        assert!(r.cards.len() >= 5);
        assert!(r.cards.iter().all(|c| c.front.is_some()));
    }
}
