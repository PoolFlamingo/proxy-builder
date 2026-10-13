//! **Vincular una carta propia con la real de Scryfall**: identificarla (por
//! su nombre de fichero y, si se pide, por OCR del título) y traer los datos
//! de la elegida para la info de carta y las estadísticas.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::model::CardMeta;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::cards::{resumir, CardSummary};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::ocr;
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;
use crate::scryfall::resolve::meta_de;
use crate::state::AppState;

/// Cuántos textos se prueban como mucho (pista + líneas del OCR).
const MAX_TEXTOS: usize = 4;
/// Cuántas cartas se proponen.
const MAX_CANDIDATAS: usize = 12;

/// Lo seguro que es la carta propuesta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Confidence {
    /// El nombre (o el nombre impreso) es exactamente uno de los textos.
    Exact,
    /// Se parece mucho (una letra mal leída por el OCR, una tilde…).
    Probable,
    /// Hay candidatas, pero ninguna se parece lo bastante.
    None,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardIdentify {
    /// Lo que se ha leído en el título (vacío sin OCR).
    pub lines: Vec<String>,
    /// Cartas propuestas, la más probable primero.
    pub candidates: Vec<CardSummary>,
    /// La propuesta (id de Scryfall), si alguna se parece lo bastante.
    pub best: Option<String>,
    pub confidence: Confidence,
}

/// Para comparar nombres: minúsculas, sin tildes ni signos.
pub fn normalizar(s: &str) -> String {
    let sin_tildes: String = s
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    sin_tildes.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parecido entre 0 y 1 (1 − distancia de edición relativa).
pub fn parecido(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut fila: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut anterior = fila[0];
        fila[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let tmp = fila[j + 1];
            fila[j + 1] = if ca == cb {
                anterior
            } else {
                1 + anterior.min(fila[j]).min(fila[j + 1])
            };
            anterior = tmp;
        }
    }
    1.0 - fila[b.len()] as f64 / a.len().max(b.len()) as f64
}

/// Desde cuánto parecido una propuesta es «probable».
const UMBRAL_PROBABLE: f64 = 0.85;

/// Elige la mejor candidata para estos textos y dice lo segura que es.
pub fn puntuar(candidatas: &[CardSummary], textos: &[String]) -> (Option<String>, Confidence) {
    let textos: Vec<String> = textos
        .iter()
        .map(|t| normalizar(t))
        .filter(|t| t.len() >= 3)
        .collect();
    let mut mejor: Option<(&CardSummary, f64)> = None;
    for c in candidatas {
        let mut nombres = vec![c.name.clone()];
        nombres.extend(c.printed_name.clone());
        nombres.extend(c.name.split("//").map(|s| s.trim().to_string()));
        for n in nombres.iter().map(|n| normalizar(n)) {
            for t in &textos {
                let p = parecido(&n, t);
                if mejor.is_none_or(|(_, m)| p > m) {
                    mejor = Some((c, p));
                }
            }
        }
    }
    match mejor {
        Some((c, p)) if p >= 0.999 => (Some(c.id.clone()), Confidence::Exact),
        Some((c, p)) if p >= UMBRAL_PROBABLE => (Some(c.id.clone()), Confidence::Probable),
        _ => (None, Confidence::None),
    }
}

/// ¿Están descargados los modelos del OCR? (Para avisar de la primera vez.)
#[tauri::command]
pub fn ocr_ready(state: State<'_, AppState>) -> bool {
    ocr::modelos_listos(&state.paths.data)
}

/// Busca un texto como nombre en inglés y, si no, como nombre impreso en
/// cualquier idioma.
async fn buscar(state: &AppState, texto: &str) -> AppResult<Vec<ScryCard>> {
    let limpio = texto.replace('"', "");
    let exacta = state
        .scryfall
        .search(&format!("!\"{limpio}\""), false, 4)
        .await
        .unwrap_or_default();
    if !exacta.is_empty() {
        return Ok(exacta);
    }
    Ok(state
        .scryfall
        .search(&format!("lang:any \"{limpio}\""), true, 6)
        .await
        .unwrap_or_default())
}

#[tauri::command]
pub async fn card_identify(
    state: State<'_, AppState>,
    blob: String,
    hint: Option<String>,
    ocr: bool,
) -> AppResult<CardIdentify> {
    identificar(&state, &blob, hint.as_deref(), ocr).await
}

async fn identificar(
    state: &AppState,
    blob: &str,
    hint: Option<&str>,
    ocr: bool,
) -> AppResult<CardIdentify> {
    let mut textos: Vec<String> = Vec::new();
    if let Some(h) = hint.map(ocr::limpiar).filter(|h| h.len() >= 3) {
        textos.push(h);
    }
    let mut lineas = Vec::new();
    if ocr {
        let datos = state.paths.data.clone();
        if !ocr::modelos_listos(&datos) {
            ocr::descargar_modelos(&datos).await?;
        }
        let (bytes, _) = state.leer_blob(blob)?;
        let img = proxy_core::imaging::decode::decode(&bytes)?;
        lineas = tauri::async_runtime::spawn_blocking(move || ocr::lineas_del_titulo(&datos, &img))
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))??;
        textos.extend(lineas.iter().cloned());
    }

    let mut vistas = HashSet::new();
    let mut probados = HashSet::new();
    let mut cartas = Vec::new();
    for t in &textos {
        if probados.len() >= MAX_TEXTOS || !probados.insert(t.to_lowercase()) {
            continue;
        }
        for c in buscar(state, t).await? {
            if vistas.insert(c.id.clone()) {
                cartas.push(c);
            }
        }
        if cartas.len() >= MAX_CANDIDATAS {
            break;
        }
    }
    cartas.truncate(MAX_CANDIDATAS);
    let mut candidates = resumir(state, cartas);
    let (best, confidence) = puntuar(&candidates, &textos);
    // La propuesta, la primera de la lista.
    if let Some(i) = best
        .as_ref()
        .and_then(|b| candidates.iter().position(|c| &c.id == b))
    {
        let c = candidates.remove(i);
        candidates.insert(0, c);
    }
    Ok(CardIdentify {
        lines: lineas,
        candidates,
        best,
        confidence,
    })
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IdentifyRequest {
    pub blob: String,
    /// El nombre que ya tiene la carta (el del fichero).
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IdentifyProgressEvent {
    pub job_id: u32,
    pub done: u32,
    pub total: u32,
    pub blob: String,
    /// `None` si esa carta falló (la imagen no se pudo leer…): se sigue con las demás.
    pub result: Option<CardIdentify>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct IdentifyDoneEvent {
    pub job_id: u32,
    pub error: Option<AppError>,
}

/// Identifica muchas cartas propias de una vez (la revisión en bloque). Cada
/// resultado llega por `identify://progress`; el final, por `identify://done`.
/// Se cancela con `job_cancel`.
#[tauri::command]
pub fn identify_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    items: Vec<IdentifyRequest>,
    ocr: bool,
) -> u32 {
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let r = lote(&app, &estado, job_id, &items, ocr, &cancelado).await;
        estado.jobs.baja(job_id);
        let _ = app.emit(
            "identify://done",
            IdentifyDoneEvent {
                job_id,
                error: r.err(),
            },
        );
    });
    job_id
}

async fn lote(
    app: &AppHandle,
    estado: &AppState,
    job_id: u32,
    items: &[IdentifyRequest],
    ocr: bool,
    cancelado: &AtomicBool,
) -> AppResult<()> {
    // Los modelos, antes de empezar (y una sola vez).
    if ocr && !ocr::modelos_listos(&estado.paths.data) {
        ocr::descargar_modelos(&estado.paths.data).await?;
    }
    let total = items.len() as u32;
    for (i, it) in items.iter().enumerate() {
        if cancelado.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::Cancelled));
        }
        let result = identificar(estado, &it.blob, it.hint.as_deref(), ocr).await;
        if let Err(e) = &result {
            // Sin red no tiene sentido seguir carta a carta.
            if e.code == ErrorCode::Network {
                return Err(e.clone());
            }
            tracing::warn!(blob = %it.blob, codigo = ?e.code, "no se pudo identificar");
        }
        let _ = app.emit(
            "identify://progress",
            IdentifyProgressEvent {
                job_id,
                done: i as u32 + 1,
                total,
                blob: it.blob.clone(),
                result: result.ok(),
            },
        );
    }
    Ok(())
}

/// Los datos de la carta elegida (de la caché si están; si no, de Scryfall,
/// y se guardan para que las estadísticas funcionen sin red).
#[tauri::command]
pub async fn card_link(state: State<'_, AppState>, scryfall_id: String) -> AppResult<CardMeta> {
    if let Some((c, _)) = cache::leer(&state.db, &scryfall_id)? {
        return Ok(meta_de(&c, false));
    }
    let c = state
        .scryfall
        .card(&scryfall_id)
        .await?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("carta", &scryfall_id))?;
    cache::guardar(&state.db, &c)?;
    Ok(meta_de(&c, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carta(id: &str, nombre: &str, impreso: Option<&str>) -> CardSummary {
        CardSummary {
            id: id.into(),
            name: nombre.into(),
            printed_name: impreso.map(Into::into),
            lang: "en".into(),
            set: "x".into(),
            set_name: "X".into(),
            collector_number: "1".into(),
            thumb: None,
            highres: true,
            image_status: None,
            is_dfc: false,
            type_line: None,
        }
    }

    #[test]
    fn normalizar_quita_tildes_y_signos() {
        assert_eq!(normalizar("  Anillo  Solar!"), "anillo solar");
        assert_eq!(normalizar("Lim-Dûl's Vault"), "lim dul s vault");
        assert_eq!(normalizar("Ñandú, Ágil"), "nandu agil");
    }

    #[test]
    fn el_parecido() {
        assert_eq!(parecido("abc", "abc"), 1.0);
        assert!(parecido("birds of paradise", "birds of paradlse") > 0.9);
        assert!(parecido("sol ring", "city of brass") < 0.5);
    }

    #[test]
    fn exacta_probable_o_ninguna() {
        let cs = vec![
            carta("a", "City of Brass", None),
            carta("b", "Sol Ring", Some("Anillo solar")),
        ];
        let t = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert_eq!(
            puntuar(&cs, &t(&["Castillo Real", "City of Brass"])),
            (Some("a".into()), Confidence::Exact)
        );
        assert_eq!(
            puntuar(&cs, &t(&["Anillo Solar"])),
            (Some("b".into()), Confidence::Exact)
        );
        assert_eq!(
            puntuar(&cs, &t(&["Anil1o solar"])),
            (Some("b".into()), Confidence::Probable)
        );
        assert_eq!(puntuar(&cs, &t(&["Dragón rojo"])), (None, Confidence::None));
    }

    #[test]
    fn una_dfc_casa_por_cualquiera_de_sus_caras() {
        let cs = vec![carta(
            "d",
            "Delver of Secrets // Insectile Aberration",
            None,
        )];
        let t = vec!["Insectile Aberration".to_string()];
        assert_eq!(puntuar(&cs, &t).1, Confidence::Exact);
    }
}
