//! Cartas: resolver una lista contra Scryfall, buscar, elegir variante,
//! importar de Archidekt.

use proxy_core::decklist::ParsedDecklist;
use proxy_core::model::{BackSpec, CardEdits, CardSource, DeckCard, FaceImage, ImageOrigin};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use crate::archidekt;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::scryfall::models::{ScryCard, SetInfo};
use crate::scryfall::resolve::{self, Guardar, ResolveOptions, ResolveProgress, ResolveResult};
use crate::scryfall::{cache, client::Scryfall};
use crate::state::AppState;

/// Guarda en el almacén de la app y apunta en la caché de Scryfall.
struct EnLaApp<'a>(&'a AppState);

impl Guardar for EnLaApp<'_> {
    fn guardar(&self, bytes: &[u8], origen: ImageOrigin) -> AppResult<FaceImage> {
        self.0.blobs.put_bytes(&self.0.db, bytes, origen)
    }
    fn apuntar(&self, c: &ScryCard) -> AppResult<()> {
        cache::guardar(&self.0.db, c)
    }
    async fn nombre_ingles(&self, nombre: &str, idioma: &str) -> Option<String> {
        crate::fuentes::nombres_ingleses(self.0, nombre, idioma)
            .await
            .into_iter()
            .next()
    }
    async fn externa_por_nombre(&self, nombre: &str) -> AppResult<Option<DeckCard>> {
        let c = crate::fuentes::cockatrice::catalogo(&self.0.externo, &self.0.paths.cache).await?;
        let Some(f) = c
            .exactas(nombre)
            .into_iter()
            .find(|f| !f.imagenes.is_empty())
        else {
            return Ok(None);
        };
        crate::fuentes::carta_cockatrice(self.0, &f.id)
            .await
            .map(Some)
    }
    async fn externas_creadas_por(&self, cartas: &[String]) -> AppResult<Vec<DeckCard>> {
        let c = crate::fuentes::cockatrice::catalogo(&self.0.externo, &self.0.paths.cache).await?;
        let mut ids: Vec<String> = Vec::new();
        for n in cartas {
            for f in c.creadas_por(n) {
                if !f.imagenes.is_empty() && !ids.contains(&f.id) {
                    ids.push(f.id.clone());
                }
            }
        }
        let mut out = Vec::new();
        for id in ids {
            match crate::fuentes::carta_cockatrice(self.0, &id).await {
                Ok(carta) => out.push(carta),
                Err(e) => {
                    tracing::warn!(codigo = ?e.code, ficha = %id, "ficha de Cockatrice sin imagen")
                }
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolveProgressEvent {
    pub job_id: u32,
    pub progress: ResolveProgress,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResolveDoneEvent {
    pub job_id: u32,
    pub result: Option<ResolveResult>,
    pub error: Option<AppError>,
}

/// Empieza a resolver una lista. Devuelve el id del trabajo; el avance llega
/// por `resolve://progress` y el final por `resolve://done`.
#[tauri::command]
pub fn resolve_decklist(
    app: AppHandle,
    state: State<'_, AppState>,
    list: ParsedDecklist,
    options: ResolveOptions,
) -> u32 {
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let api: &Scryfall = &estado.scryfall;
        let emisor = app.clone();
        let r = resolve::resolve(api, &EnLaApp(&estado), &list, &options, &cancelado, |p| {
            let _ = emisor.emit(
                "resolve://progress",
                ResolveProgressEvent {
                    job_id,
                    progress: p,
                },
            );
        })
        .await;
        estado.jobs.baja(job_id);
        let (result, error) = match r {
            Ok(x) => (Some(x), None),
            Err(e) => (None, Some(e)),
        };
        let _ = app.emit(
            "resolve://done",
            ResolveDoneEvent {
                job_id,
                result,
                error,
            },
        );
    });
    job_id
}

#[tauri::command]
pub fn job_cancel(state: State<'_, AppState>, job_id: u32) -> bool {
    state.jobs.cancelar(job_id)
}

/// Una impresión, para los resultados de búsqueda y el selector de variantes.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardSummary {
    pub id: String,
    pub name: String,
    pub printed_name: Option<String>,
    pub lang: String,
    pub set: String,
    pub set_name: String,
    pub collector_number: String,
    /// Huella para pedir la miniatura al protocolo (`pbimg://…/t/<huella>`).
    pub thumb: Option<String>,
    pub highres: bool,
    pub image_status: Option<String>,
    pub is_dfc: bool,
    pub type_line: Option<String>,
    /// De qué fuente es, si no es de Scryfall (`id` es entonces el de allí).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fuente: Option<proxy_core::model::FuenteCarta>,
}

/// Apunta la URL de una miniatura para el protocolo (`/t/<huella>`).
pub(crate) fn registrar_miniatura(state: &AppState, url: &str) -> String {
    let h = huella_url(url);
    if let Ok(mut r) = state.miniaturas.lock() {
        if r.len() > 5_000 {
            r.clear();
        }
        r.insert(h.clone(), url.to_string());
    }
    h
}

fn huella_url(url: &str) -> String {
    Sha256::digest(url.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub(crate) fn resumir(state: &AppState, cartas: Vec<ScryCard>) -> Vec<CardSummary> {
    let mut registro = state.miniaturas.lock().ok();
    cartas
        .into_iter()
        .map(|c| {
            let url = c
                .face_uris(0)
                .and_then(|u| u.normal.clone().or_else(|| u.small.clone()));
            let thumb = url.map(|u| {
                let h = huella_url(&u);
                if let Some(r) = registro.as_mut() {
                    if r.len() > 5_000 {
                        r.clear();
                    }
                    r.insert(h.clone(), u);
                }
                h
            });
            CardSummary {
                id: c.id.clone(),
                printed_name: c.printed_name.clone(),
                lang: c.lang.clone(),
                set: c.set.clone(),
                set_name: c.set_name.clone(),
                collector_number: c.collector_number.clone(),
                thumb,
                highres: c.highres_image,
                image_status: c.image_status.clone(),
                is_dfc: c.is_dfc(),
                type_line: c.type_line.clone(),
                name: c.name,
                fuente: None,
            }
        })
        .collect()
}

fn filtro_idioma(lang: &Option<String>) -> String {
    match lang.as_deref() {
        Some(l) if !l.is_empty() && l != "any" => format!(" lang:{l}"),
        _ => String::new(),
    }
}

fn escapar(nombre: &str) -> String {
    nombre.replace('"', "")
}

/// La consulta de `card_search`.
fn consulta_busqueda(q: &str, lang: &Option<String>, highres_only: bool) -> String {
    let mut consulta = format!("{}{}", q.trim(), filtro_idioma(lang));
    if highres_only {
        consulta.push_str(" is:hires");
    }
    consulta
}

/// La consulta de `card_prints`: todas las impresiones de un nombre exacto.
fn consulta_impresiones(nombre: &str, lang: &Option<String>, highres_only: bool) -> String {
    let mut q = format!("!\"{}\"{}", escapar(nombre), filtro_idioma(lang));
    if highres_only {
        q.push_str(" is:hires");
    }
    q
}

/// La consulta de `set_cards`: el código y el tipo, sin nada que no sean
/// letras (y números en el código), para que no se cuele otra consulta.
fn consulta_set(code: &str, type_filter: Option<String>) -> String {
    let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let mut q = format!("e:{code}");
    // Se limpia antes de mirar si está vacío: «-» no puede dejar un `t:` suelto.
    let tipo = type_filter.map(|t| {
        t.chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>()
    });
    if let Some(t) = tipo.filter(|t| !t.is_empty()) {
        q.push_str(&format!(" t:{t}"));
    }
    q
}

#[tauri::command]
pub async fn card_autocomplete(state: State<'_, AppState>, q: String) -> AppResult<Vec<String>> {
    if q.trim().len() < 2 {
        return Ok(Vec::new());
    }
    state.scryfall.autocomplete(q.trim()).await
}

/// Busca cartas (máx. 50) con la sintaxis de Scryfall.
#[tauri::command]
pub async fn card_search(
    state: State<'_, AppState>,
    q: String,
    lang: Option<String>,
    highres_only: bool,
) -> AppResult<Vec<CardSummary>> {
    let consulta = consulta_busqueda(&q, &lang, highres_only);
    let r = state.scryfall.search(&consulta, lang.is_some(), 50).await?;
    let mut v = resumir(&state, r);
    // Las fichas que Scryfall no tiene (solo en inglés y sin pedir alta
    // resolución: las de Cockatrice no lo son). Detrás, salvo si se llama
    // exactamente así y en Scryfall no hay ninguna que se llame así.
    if lang.as_deref().is_none_or(|l| l == "en") && !highres_only {
        let externas =
            crate::fuentes::buscar_cockatrice(&state, std::slice::from_ref(&q), false).await;
        juntar_externas(&mut v, externas, &q);
    }
    Ok(v)
}

/// Añade las de otras fuentes detrás, o delante si una se llama exactamente
/// como lo buscado y de Scryfall ninguna.
fn juntar_externas(v: &mut Vec<CardSummary>, externas: Vec<CardSummary>, q: &str) {
    use crate::commands::identify::normalizar;
    let q = normalizar(q);
    let exacta_en_scryfall = v.iter().any(|c| normalizar(&c.name) == q);
    let (delante, detras): (Vec<_>, Vec<_>) = externas
        .into_iter()
        .partition(|c| !exacta_en_scryfall && normalizar(&c.name) == q);
    v.splice(0..0, delante);
    v.extend(detras);
}

/// Todas las impresiones de una carta (selector de variantes).
#[tauri::command]
pub async fn card_prints(
    state: State<'_, AppState>,
    name: String,
    lang: Option<String>,
    highres_only: bool,
) -> AppResult<Vec<CardSummary>> {
    let q = consulta_impresiones(&name, &lang, highres_only);
    // Sin resultados en Scryfall (una ficha que no tiene) no es un error.
    let r = state
        .scryfall
        .search(&q, true, 175)
        .await
        .unwrap_or_default();
    let mut v = resumir(&state, r);
    if lang.as_deref().is_none_or(|l| l == "en") && !highres_only {
        let externas =
            crate::fuentes::buscar_cockatrice(&state, std::slice::from_ref(&name), true).await;
        v.extend(externas);
    }
    Ok(v)
}

/// Trae una carta de otra fuente como carta del deck (una propia con su imagen
/// y el vínculo), para añadirla o para reemplazar otra.
#[tauri::command]
pub async fn card_fetch_externa(
    state: State<'_, AppState>,
    fuente: proxy_core::model::FuenteCarta,
    id: String,
) -> AppResult<DeckCard> {
    let _marca = state.trabajo();
    match fuente {
        proxy_core::model::FuenteCarta::Cockatrice => {
            crate::fuentes::carta_cockatrice(&state, &id).await
        }
    }
}

/// Una ficha que falta en el deck y la carta que la crea.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FichaPropuesta {
    pub carta: CardSummary,
    pub creada_por: String,
}

/// **Detectar fichas** en un deck ya hecho: las que crean sus cartas (también
/// las propias vinculadas) y que no tiene. Solo las propone; se añaden con
/// `card_fetch` las que se elijan.
#[tauri::command]
pub async fn deck_tokens_detect(
    state: State<'_, AppState>,
    cards: Vec<DeckCard>,
) -> AppResult<Vec<FichaPropuesta>> {
    let _marca = state.trabajo();
    let db = &state.db;
    let halladas = crate::scryfall::fichas_auto::faltan_en_el_deck(
        state.scryfall.as_ref(),
        &cards,
        |id| cache::leer(db, id).ok().flatten().map(|(c, _)| c),
        &std::sync::atomic::AtomicBool::new(false),
    )
    .await?;
    for f in &halladas {
        cache::guardar(db, &f.ficha)?;
    }
    let creadores: Vec<String> = halladas.iter().map(|f| f.creada_por.clone()).collect();
    let resumen = resumir(&state, halladas.into_iter().map(|f| f.ficha).collect());
    let mut out: Vec<FichaPropuesta> = resumen
        .into_iter()
        .zip(creadores)
        .map(|(carta, creada_por)| FichaPropuesta { carta, creada_por })
        .collect();
    // Las fichas que Scryfall no tiene (Cockatrice) y que crean las cartas
    // del deck, si no están ya (por nombre: también una propia hecha a mano).
    if let Ok(cat) = crate::fuentes::cockatrice::catalogo(&state.externo, &state.paths.cache).await
    {
        let mut ya: std::collections::HashSet<String> = cards
            .iter()
            .map(|c| c.meta.name.to_lowercase())
            .chain(out.iter().map(|p| p.carta.name.to_lowercase()))
            .collect();
        for c in &cards {
            for f in cat.creadas_por(&c.meta.name) {
                if !f.imagenes.is_empty() && ya.insert(f.nombre.to_lowercase()) {
                    out.push(FichaPropuesta {
                        carta: crate::fuentes::resumen_cockatrice(&state, f),
                        creada_por: c.meta.name.clone(),
                    });
                }
            }
        }
    }
    Ok(out)
}

/// Descarga una impresión concreta y la devuelve como carta del deck (para
/// añadirla o para reemplazar la imagen de otra).
#[tauri::command]
pub async fn card_fetch(state: State<'_, AppState>, scryfall_id: String) -> AppResult<DeckCard> {
    let _marca = state.trabajo();
    traer_carta(state.scryfall.as_ref(), &state, &scryfall_id).await
}

/// Lo que hace `card_fetch`, con el Scryfall que se le dé.
pub(crate) async fn traer_carta<A: CardApi>(
    api: &A,
    state: &AppState,
    scryfall_id: &str,
) -> AppResult<DeckCard> {
    let c = api
        .card(scryfall_id)
        .await?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("carta", scryfall_id))?;
    cache::guardar(&state.db, &c)?;
    let bajar = |cara: usize| {
        let c = &c;
        let state = &state;
        async move {
            match c.print_url(cara) {
                None => Ok::<_, AppError>(None),
                Some((url, png)) => {
                    let bytes = api.download(&url).await?;
                    let origen = if png {
                        ImageOrigin::ScryfallPng
                    } else {
                        ImageOrigin::ScryfallLarge
                    };
                    state.blobs.put_bytes(&state.db, &bytes, origen).map(Some)
                }
            }
        }
    };
    let front = bajar(0).await?;
    let back = if c.is_dfc() {
        bajar(1).await?.map(|image| BackSpec::Dfc { image })
    } else {
        None
    };
    Ok(DeckCard {
        id: uuid::Uuid::new_v4().to_string(),
        source: if c.layout == "token" || c.layout == "double_faced_token" {
            CardSource::Token {
                scryfall_id: c.id.clone(),
            }
        } else {
            CardSource::Scryfall {
                scryfall_id: c.id.clone(),
            }
        },
        front,
        back: back.unwrap_or(BackSpec::DeckDefault),
        edits: CardEdits::default(),
        locked: false,
        meta: crate::scryfall::resolve::meta_de(&c, false),
        design: None,
        marca: None,
    })
}

#[tauri::command]
pub async fn sets_list(state: State<'_, AppState>) -> AppResult<Vec<SetInfo>> {
    state.scryfall.sets().await
}

/// Las cartas de un set, opcionalmente de un tipo (Creature, Instant…).
#[tauri::command]
pub async fn set_cards(
    state: State<'_, AppState>,
    code: String,
    type_filter: Option<String>,
) -> AppResult<Vec<CardSummary>> {
    let q = consulta_set(&code, type_filter);
    let r = state.scryfall.search(&q, false, 175).await?;
    Ok(resumir(&state, r))
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArchidektImport {
    /// El texto equivalente, para el cuadro de la lista.
    pub text: String,
    pub list: ParsedDecklist,
}

/// Importa un deck público de Archidekt por su URL.
#[tauri::command]
pub async fn import_archidekt(url: String, include_sideboard: bool) -> AppResult<ArchidektImport> {
    let id = archidekt::id_de_url(&url)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "bad_url"))?;
    let api = format!("https://archidekt.com/api/decks/{id}/");
    let r = reqwest::Client::builder()
        .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(crate::error::red)?
        .get(&api)
        .send()
        .await
        .map_err(crate::error::red)?;
    tracing::info!(host = "archidekt.com", ruta = %format!("/api/decks/{id}/"), estado = r.status().as_u16(), "red");
    if r.status() == reqwest::StatusCode::NOT_FOUND || r.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(AppError::new(ErrorCode::NotFound).with("deck", id));
    }
    if !r.status().is_success() {
        return Err(crate::error::estado_http(r.status()));
    }
    let texto = r.text().await.map_err(crate::error::red)?;
    let i = archidekt::desde_json(&texto, include_sideboard)?;
    Ok(ArchidektImport {
        text: i.texto,
        list: i.lista,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::models::Collection;
    use crate::scryfall::resolve::tests::carta;
    use crate::state::Paths;
    use crate::store::blobs::tests::png;
    use serde_json::json;
    use std::sync::Mutex;

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    /// Un Scryfall de mentira: da las cartas que tiene y descarga PNG de
    /// verdad (distintos por URL).
    struct Api {
        cartas: Vec<ScryCard>,
        descargas: Mutex<Vec<String>>,
    }

    fn api(cartas: Vec<ScryCard>) -> Api {
        Api {
            cartas,
            descargas: Mutex::new(Vec::new()),
        }
    }

    impl CardApi for Api {
        async fn collection(
            &self,
            _: &[crate::scryfall::models::Identifier],
        ) -> AppResult<Collection> {
            Ok(Collection::default())
        }
        async fn card(&self, id: &str) -> AppResult<Option<ScryCard>> {
            Ok(self.cartas.iter().find(|c| c.id == id).cloned())
        }
        async fn card_lang(&self, _: &str, _: &str, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn search(&self, _: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            Ok(Vec::new())
        }
        async fn download(&self, url: &str) -> AppResult<Vec<u8>> {
            self.descargas.lock().unwrap().push(url.to_string());
            let semilla = url.bytes().fold(0u8, |a, b| a.wrapping_add(b));
            Ok(png(6, 8, semilla))
        }
    }

    // ── Consultas ──

    #[test]
    fn la_huella_de_una_url_es_estable_y_distinta() {
        let a = huella_url("https://img/a.jpg");
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a, huella_url("https://img/a.jpg"));
        assert_ne!(a, huella_url("https://img/b.jpg"));
    }

    #[test]
    fn el_filtro_de_idioma() {
        assert_eq!(filtro_idioma(&None), "");
        assert_eq!(filtro_idioma(&Some(String::new())), "");
        assert_eq!(filtro_idioma(&Some("any".into())), "");
        assert_eq!(filtro_idioma(&Some("es".into())), " lang:es");
    }

    #[test]
    fn la_busqueda_recorta_y_anade_idioma_y_alta_resolucion() {
        assert_eq!(consulta_busqueda("  goblin  ", &None, false), "goblin");
        assert_eq!(
            consulta_busqueda("t:goblin", &Some("es".into()), true),
            "t:goblin lang:es is:hires"
        );
    }

    #[test]
    fn las_impresiones_piden_el_nombre_exacto_sin_comillas_sueltas() {
        assert_eq!(
            consulta_impresiones("Sol Ring", &None, false),
            "!\"Sol Ring\""
        );
        // Unas comillas en el nombre cerrarían la consulta antes de tiempo.
        assert_eq!(
            consulta_impresiones("Kongming, \"Sleeping Dragon\"", &Some("any".into()), true),
            "!\"Kongming, Sleeping Dragon\" is:hires"
        );
    }

    #[test]
    fn el_set_y_el_tipo_no_dejan_colar_otra_consulta() {
        assert_eq!(consulta_set("neo", None), "e:neo");
        assert_eq!(consulta_set("neo) or (e:dmu", None), "e:neooredmu");
        assert_eq!(
            consulta_set("neo", Some("creature".into())),
            "e:neo t:creature"
        );
        assert_eq!(
            consulta_set("neo", Some("creature or t:land".into())),
            "e:neo t:creatureortland"
        );
        // Un tipo vacío, o que se queda vacío al limpiarlo, no filtra.
        assert_eq!(consulta_set("neo", Some(String::new())), "e:neo");
        assert_eq!(consulta_set("neo", Some("-!".into())), "e:neo");
    }

    // ── Resúmenes ──

    #[test]
    fn el_resumen_apunta_las_miniaturas_para_el_protocolo() {
        let (_dir, s) = estado();
        let cartas = vec![
            carta(
                json!({"id":"a","name":"A","lang":"en","set":"m11","collector_number":"1",
                "image_uris":{"normal":"https://img/a-n.jpg","small":"https://img/a-s.jpg"}}),
            ),
            carta(
                json!({"id":"b","name":"B","lang":"es","set":"m11","collector_number":"2",
                "image_uris":{"small":"https://img/b-s.jpg"}}),
            ),
            carta(
                json!({"id":"c","name":"C // D","lang":"en","set":"isd","collector_number":"3","layout":"transform",
                "card_faces":[{"name":"C","image_uris":{"normal":"https://img/c0.jpg"}},
                              {"name":"D","image_uris":{"normal":"https://img/c1.jpg"}}]}),
            ),
            carta(json!({"id":"d","name":"D","lang":"en","set":"m11","collector_number":"4"})),
        ];
        let r = resumir(&s, cartas);
        assert_eq!(r.len(), 4);
        assert_eq!(r[0].thumb, Some(huella_url("https://img/a-n.jpg")));
        // Sin la normal, la pequeña.
        assert_eq!(r[1].thumb, Some(huella_url("https://img/b-s.jpg")));
        // Una DFC, la de delante.
        assert_eq!(r[2].thumb, Some(huella_url("https://img/c0.jpg")));
        assert!(r[2].is_dfc && !r[0].is_dfc);
        assert_eq!(r[3].thumb, None);
        assert_eq!(r[1].lang, "es");
        let reg = s.miniaturas.lock().unwrap();
        assert_eq!(reg.len(), 3);
        assert_eq!(
            reg.get(&huella_url("https://img/c0.jpg"))
                .map(String::as_str),
            Some("https://img/c0.jpg")
        );
    }

    #[test]
    fn el_registro_de_miniaturas_no_crece_sin_fin() {
        let (_dir, s) = estado();
        {
            let mut reg = s.miniaturas.lock().unwrap();
            for i in 0..5_001 {
                reg.insert(format!("h{i}"), format!("u{i}"));
            }
        }
        let c = carta(json!({"id":"a","name":"A","image_uris":{"normal":"https://img/a.jpg"}}));
        resumir(&s, vec![c]);
        let reg = s.miniaturas.lock().unwrap();
        assert_eq!(reg.len(), 1, "se vacía antes de apuntar la nueva");
        assert!(reg.contains_key(&huella_url("https://img/a.jpg")));
    }

    // ── Traer una impresión ──

    #[tokio::test]
    async fn una_carta_normal_trae_su_png_y_el_reverso_del_deck() {
        let (_dir, s) = estado();
        let a = api(vec![carta(json!({
            "id":"sol","name":"Sol Ring","lang":"en","set":"cmm","collector_number":"410",
            "layout":"normal","type_line":"Artifact","highres_image":true,"image_status":"highres_scan",
            "image_uris":{"png":"https://img/sol.png","large":"https://img/sol.jpg"}
        }))]);
        let c = traer_carta(&a, &s, "sol").await.unwrap();
        assert_eq!(
            c.source,
            CardSource::Scryfall {
                scryfall_id: "sol".into()
            }
        );
        let f = c.front.as_ref().unwrap();
        assert_eq!(f.origin, ImageOrigin::ScryfallPng);
        assert_eq!((f.width, f.height), (6, 8));
        assert!(s.blobs.locate(&s.db, &f.blob).unwrap().is_some());
        assert_eq!(c.back, BackSpec::DeckDefault);
        assert_eq!(*a.descargas.lock().unwrap(), ["https://img/sol.png"]);
        assert_eq!(c.meta.name, "Sol Ring");
        assert_eq!(c.meta.set.as_deref(), Some("cmm"));
        assert_eq!(c.meta.collector_number.as_deref(), Some("410"));
        assert_eq!(c.meta.layout.as_deref(), Some("normal"));
        assert_eq!(c.meta.type_line.as_deref(), Some("Artifact"));
        assert!(c.meta.highres && !c.meta.is_dfc && !c.locked);
        // Queda en la caché de Scryfall.
        assert!(cache::leer(&s.db, "sol").unwrap().is_some());
        // Cada vez es una copia nueva del deck.
        let otra = traer_carta(&a, &s, "sol").await.unwrap();
        assert_ne!(otra.id, c.id);
        assert_eq!(otra.front, c.front);
    }

    #[tokio::test]
    async fn sin_png_baja_la_grande() {
        let (_dir, s) = estado();
        let a = api(vec![carta(json!({
            "id":"x","name":"X","layout":"normal","image_uris":{"large":"https://img/x.jpg"}
        }))]);
        let c = traer_carta(&a, &s, "x").await.unwrap();
        assert_eq!(c.front.unwrap().origin, ImageOrigin::ScryfallLarge);
    }

    #[tokio::test]
    async fn una_dfc_trae_las_dos_caras() {
        let (_dir, s) = estado();
        let a = api(vec![carta(json!({
            "id":"delver","name":"Delver // Aberration","layout":"transform",
            "card_faces":[{"name":"Delver","image_uris":{"png":"https://img/d0.png"}},
                          {"name":"Aberration","image_uris":{"png":"https://img/d1.png"}}]
        }))]);
        let c = traer_carta(&a, &s, "delver").await.unwrap();
        let BackSpec::Dfc { image } = &c.back else {
            panic!("se esperaba el dorso de la DFC: {:?}", c.back);
        };
        assert_ne!(Some(image), c.front.as_ref());
        assert!(c.meta.is_dfc);
        assert_eq!(a.descargas.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn las_fichas_entran_como_fichas() {
        let (_dir, s) = estado();
        let a = api(vec![
            carta(json!({"id":"t1","name":"Goblin","layout":"token",
                "image_uris":{"png":"https://img/t1.png"}})),
            carta(
                json!({"id":"t2","name":"Goblin // Elf","layout":"double_faced_token",
                "card_faces":[{"name":"Goblin","image_uris":{"png":"https://img/t2a.png"}},
                              {"name":"Elf","image_uris":{"png":"https://img/t2b.png"}}]}),
            ),
        ]);
        for id in ["t1", "t2"] {
            let c = traer_carta(&a, &s, id).await.unwrap();
            assert_eq!(
                c.source,
                CardSource::Token {
                    scryfall_id: id.into()
                }
            );
        }
    }

    #[tokio::test]
    async fn una_carta_sin_imagen_entra_sin_frente_y_sin_descargar() {
        let (_dir, s) = estado();
        let a = api(vec![carta(
            json!({"id":"n","name":"Nada","layout":"normal"}),
        )]);
        let c = traer_carta(&a, &s, "n").await.unwrap();
        assert!(c.front.is_none());
        assert!(a.descargas.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn una_carta_que_no_existe_es_no_encontrada() {
        let (_dir, s) = estado();
        let e = traer_carta(&api(Vec::new()), &s, "nadie")
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
        assert_eq!(e.params.get("carta").map(String::as_str), Some("nadie"));
    }
}
