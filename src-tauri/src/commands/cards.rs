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
    let mut consulta = format!("{}{}", q.trim(), filtro_idioma(&lang));
    if highres_only {
        consulta.push_str(" is:hires");
    }
    let r = state.scryfall.search(&consulta, lang.is_some(), 50).await?;
    Ok(resumir(&state, r))
}

/// Todas las impresiones de una carta (selector de variantes).
#[tauri::command]
pub async fn card_prints(
    state: State<'_, AppState>,
    name: String,
    lang: Option<String>,
    highres_only: bool,
) -> AppResult<Vec<CardSummary>> {
    let mut q = format!("!\"{}\"{}", escapar(&name), filtro_idioma(&lang));
    if highres_only {
        q.push_str(" is:hires");
    }
    let r = state.scryfall.search(&q, true, 175).await?;
    Ok(resumir(&state, r))
}

/// Descarga una impresión concreta y la devuelve como carta del deck (para
/// añadirla o para reemplazar la imagen de otra).
#[tauri::command]
pub async fn card_fetch(state: State<'_, AppState>, scryfall_id: String) -> AppResult<DeckCard> {
    let _marca = state.trabajo();
    let c = state
        .scryfall
        .card(&scryfall_id)
        .await?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("carta", &scryfall_id))?;
    cache::guardar(&state.db, &c)?;
    let bajar = |cara: usize| {
        let c = &c;
        let state = &state;
        async move {
            match c.print_url(cara) {
                None => Ok::<_, AppError>(None),
                Some((url, png)) => {
                    let bytes = state.scryfall.download(&url).await?;
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
        meta: proxy_core::model::CardMeta {
            name: c.name.clone(),
            set: Some(c.set.clone()),
            collector_number: Some(c.collector_number.clone()),
            lang: Some(c.lang.clone()),
            lang_fallback: false,
            layout: Some(c.layout.clone()),
            is_dfc: c.is_dfc(),
            image_status: c.image_status.clone(),
            highres: c.highres_image,
            type_line: c.type_line.clone(),
        },
        design: None,
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
    let code: String = code.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let mut q = format!("e:{code}");
    if let Some(t) = type_filter.filter(|t| !t.is_empty()) {
        let t: String = t.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        q.push_str(&format!(" t:{t}"));
    }
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
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?
        .get(&api)
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
    tracing::info!(host = "archidekt.com", ruta = %format!("/api/decks/{id}/"), estado = r.status().as_u16(), "red");
    if r.status() == reqwest::StatusCode::NOT_FOUND || r.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(AppError::new(ErrorCode::NotFound).with("deck", id));
    }
    if !r.status().is_success() {
        return Err(AppError::new(ErrorCode::Network).with("estado", r.status().as_u16()));
    }
    let texto = r
        .text()
        .await
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
    let i = archidekt::desde_json(&texto, include_sideboard)?;
    Ok(ArchidektImport {
        text: i.texto,
        list: i.lista,
    })
}
