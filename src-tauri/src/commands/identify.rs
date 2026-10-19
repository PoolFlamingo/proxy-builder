//! **Vincular una carta propia con la real de Scryfall**: identificarla (por
//! su nombre de fichero y, si se pide, por OCR de la carta entera: título,
//! pie y el resto, contra el catálogo de nombres de Scryfall) y traer los
//! datos de la elegida para la info de carta y las estadísticas.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::model::CardMeta;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::cards::{resumir, CardSummary};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::ocr;
use crate::originales::{self, PieLeido};
use crate::scryfall::cache;
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;
use crate::scryfall::resolve::meta_de;
use crate::scryfall::{fichas, nombres};
use crate::state::AppState;

/// Cuántos textos se prueban como mucho (pista + pie + título del OCR).
const MAX_TEXTOS: usize = 6;
/// Cuántas cartas se proponen.
const MAX_CANDIDATAS: usize = 12;
/// Cuántas trae como mucho la búsqueda de lo escrito en el diálogo («waste» →
/// todas las que lo llevan en el nombre): cuatro páginas de Scryfall.
const MAX_ESCRITO: usize = 700;
/// Lo que se añade a cada búsqueda: los tokens (Scryfall no los da sin
/// `include:extras`), pero no las cartas de ilustración (art series) ni las
/// portadas de los sobres de Jumpstart.
const EXTRAS: &str = "include:extras -layout:art_series -layout:front_card";

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
    /// Lo que se ha leído: los nombres del pie y luego el título (vacío sin OCR).
    pub lines: Vec<String>,
    /// Cartas propuestas, la más probable primero.
    pub candidates: Vec<CardSummary>,
    /// La propuesta (id de Scryfall), si alguna se parece lo bastante.
    pub best: Option<String>,
    pub confidence: Confidence,
    /// La colección, el número y el idioma leídos en el pie (con OCR). El
    /// idioma es el de la imagen: se guarda al vincular.
    pub pie: Option<PieLeido>,
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

/// Parecido entre 0 y 1 (1 − distancia de edición relativa). Dos letras
/// cambiadas de sitio cuentan como un solo error («Menmarch» por «Memnarch»):
/// es el fallo típico de quien escribe el nombre a mano en la carta.
pub fn parecido(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let (n, m) = (a.len(), b.len());
    // Distancia de Damerau (alineamiento óptimo) con tres filas.
    let mut previa: Vec<usize> = vec![0; m + 1];
    let mut anterior: Vec<usize> = (0..=m).collect();
    let mut fila: Vec<usize> = vec![0; m + 1];
    for i in 1..=n {
        fila[0] = i;
        for j in 1..=m {
            let coste = usize::from(a[i - 1] != b[j - 1]);
            let mut d = (anterior[j] + 1)
                .min(fila[j - 1] + 1)
                .min(anterior[j - 1] + coste);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d = d.min(previa[j - 2] + 1);
            }
            fila[j] = d;
        }
        std::mem::swap(&mut previa, &mut anterior);
        std::mem::swap(&mut anterior, &mut fila);
    }
    1.0 - anterior[m] as f64 / n.max(m) as f64
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
    // A igual parecido gana la carta que se llama así entera frente a la que
    // solo tiene una cara con ese nombre: «Lightning Bolt» es Lightning Bolt,
    // no «Emeritus of Conflict // Lightning Bolt».
    let mut mejor: Option<(&CardSummary, f64, bool)> = None;
    for c in candidatas {
        let mut nombres: Vec<(String, bool)> = vec![(c.name.clone(), true)];
        nombres.extend(c.printed_name.clone().map(|n| (n, true)));
        if c.name.contains("//") {
            nombres.extend(c.name.split("//").map(|s| (s.trim().to_string(), false)));
        }
        for (n, entero) in nombres.iter().map(|(n, e)| (normalizar(n), *e)) {
            for t in &textos {
                let p = parecido(&n, t);
                let mejora = match mejor {
                    None => true,
                    Some((_, m, e)) => p > m + 1e-9 || ((p - m).abs() <= 1e-9 && entero && !e),
                };
                if mejora {
                    mejor = Some((c, p, entero));
                }
            }
        }
    }
    match mejor {
        Some((c, p, _)) if p >= 0.999 => (Some(c.id.clone()), Confidence::Exact),
        Some((c, p, _)) if p >= UMBRAL_PROBABLE => (Some(c.id.clone()), Confidence::Probable),
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
        .search(&format!("!\"{limpio}\" {EXTRAS}"), false, 4)
        .await
        .unwrap_or_default();
    if !exacta.is_empty() {
        return Ok(exacta);
    }
    Ok(buscar_en_el_nombre(state, &limpio, 6).await)
}

/// Las cartas que llevan el texto en el nombre, en inglés o impreso en
/// cualquier idioma: una por carta (con impresiones, las de la primera por
/// orden alfabético llenaban la lista: «waste» daba seis Adarkar Wastes).
async fn buscar_en_el_nombre(state: &AppState, texto: &str, max: usize) -> Vec<ScryCard> {
    let limpio = texto.replace('"', "");
    state
        .scryfall
        .search_multilingual(&format!("lang:any \"{limpio}\" {EXTRAS}"), max)
        .await
        .unwrap_or_default()
}

/// Lo que más se parece a lo escrito, primero: el nombre igual, luego el que
/// empieza así, luego el que tiene una palabra que empieza así y el resto
/// (en cada grupo, el orden de Scryfall, alfabético). A igual parecido, la
/// carta que se llama así entera antes que la que solo tiene una cara así:
/// «Treasure» antes que «Dinosaur // Treasure».
fn ordenar_por_lo_escrito(cartas: &mut [ScryCard], escrito: &str) {
    let e = normalizar(escrito);
    let rango = |nombre: &str| {
        let n = normalizar(nombre);
        if n == e {
            0
        } else if n.starts_with(&e) {
            1
        } else if n.split(' ').any(|p| p.starts_with(&e)) {
            2
        } else {
            3
        }
    };
    cartas.sort_by_key(|c| {
        let impreso = c.printed_name.as_deref().map_or(3, rango);
        let entera = rango(&c.name).min(impreso);
        let cara = c.name.split("//").map(rango).min().unwrap_or(3);
        (2 * entera).min(2 * cara + 1)
    });
}

#[tauri::command]
pub async fn card_identify(
    state: State<'_, AppState>,
    blob: String,
    hint: Option<String>,
    ocr: bool,
) -> AppResult<CardIdentify> {
    identificar(&state, &blob, hint.as_deref(), ocr, true).await
}

/// `escrito`: la pista la ha escrito quien busca (el diálogo), así que además
/// se buscan todas las cartas que la llevan en el nombre, no solo la exacta.
async fn identificar(
    state: &AppState,
    blob: &str,
    hint: Option<&str>,
    ocr: bool,
    escrito: bool,
) -> AppResult<CardIdentify> {
    let pista: Vec<String> = hint
        .map(ocr::limpiar)
        .filter(|h| h.len() >= 3)
        .into_iter()
        .collect();
    let mut leido = ocr::Lectura::default();
    if ocr {
        let datos = state.paths.data.clone();
        if !ocr::modelos_listos(&datos) {
            ocr::descargar_modelos(&datos).await?;
        }
        let (bytes, _) = state.leer_blob(blob)?;
        let img = proxy_core::imaging::decode::decode(&bytes)?;
        leido = tauri::async_runtime::spawn_blocking(move || ocr::leer_carta(&datos, &img))
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))??;
    }
    // El pie primero: cuando trae el nombre original, es el inglés exacto (el
    // título de una carta propia suele ser inventado).
    let principales: Vec<String> = pista
        .iter()
        .chain(&leido.pie)
        .chain(&leido.titulo)
        .cloned()
        .collect();

    // Los nombres reales que aparecen en la carta, buscados en local en el
    // catálogo de Scryfall: la pista (el nombre del fichero o lo escrito) y,
    // en lo leído, el nombre original esté donde esté (`originales`). Si
    // nada, lo más parecido a la pista (una letra mal escrita).
    let mut seguros: Vec<String> = Vec::new();
    let mut aproximados: Vec<String> = Vec::new();
    match nombres::indice(&state.scryfall, &state.paths.cache).await {
        Ok(indice) => {
            for t in &pista {
                seguros.extend(indice.exacto(t).or_else(|| indice.al_principio(t)));
            }
            for a in originales::buscar(&leido.lineas, &indice) {
                if a.aproximado {
                    aproximados.push(a.nombre);
                } else {
                    seguros.push(a.nombre);
                }
            }
            if seguros.is_empty() && aproximados.is_empty() {
                aproximados.extend(
                    pista
                        .iter()
                        .filter_map(|t| indice.parecido(t))
                        .map(|h| h.nombre),
                );
            }
        }
        Err(e) => tracing::warn!(codigo = ?e.code, "sin catálogo de nombres"),
    }

    // La colección y el número del pie dan la carta exacta.
    let pie = if leido.lineas.is_empty() {
        None
    } else {
        match nombres::codigos_de_sets(&state.scryfall, &state.paths.cache).await {
            Ok(sets) => originales::pie_leido(&leido.lineas, &sets),
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "sin la lista de sets");
                None
            }
        }
    };

    let mut vistas = HashSet::new();
    let mut probados = HashSet::new();
    let mut cartas = Vec::new();

    let mut del_pie: Option<String> = None;
    if let Some(p) = &pie {
        if let Ok(Some(c)) = state.scryfall.card_lang(&p.set, &p.numero, "en").await {
            del_pie = Some(c.id.clone());
            vistas.insert(c.id.clone());
            cartas.push(c);
        }
    }

    // Las fichas: Scryfall no las devuelve sin `include:extras` y casi no las
    // tiene en otros idiomas, así que se buscan por su tipo en inglés
    // («Ficha de criatura — Guerrero» → `Warrior`).
    let fichas = if leido.ficha {
        fichas::candidatas(leido.titulo.iter().chain(&leido.resto))
    } else {
        Vec::new()
    };
    let mut primera_ficha: Option<String> = None;
    for en in &fichas {
        let q = format!("!\"{en}\" t:token include:extras");
        let mut halladas = state
            .scryfall
            .search(&q, false, 6)
            .await
            .unwrap_or_default();
        // Primero las que se llaman así, no las de dos caras («Dinosaur // Treasure»).
        halladas.sort_by_key(|c| c.name != *en);
        for c in halladas {
            primera_ficha.get_or_insert_with(|| c.id.clone());
            if vistas.insert(c.id.clone()) {
                cartas.push(c);
            }
        }
    }

    let a_buscar = seguros.iter().chain(&aproximados).chain(&principales);
    for t in a_buscar {
        if probados.len() >= MAX_TEXTOS || !probados.insert(normalizar(t)) {
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
    // Nada en Scryfall: puede ser el nombre impreso en otro idioma de una
    // carta que Scryfall no tiene en ese idioma (las antiguas en español):
    // su nombre inglés por otras fuentes, y ese a Scryfall.
    if cartas.is_empty() {
        let idioma = pie
            .as_ref()
            .and_then(|p| p.idioma.clone())
            .unwrap_or_else(|| "es".into());
        let textos: Vec<String> = pista.iter().chain(&leido.titulo).cloned().collect();
        for t in textos.iter().take(2) {
            for en in crate::fuentes::nombres_ingleses(state, t, &idioma).await {
                for c in buscar(state, &en).await? {
                    if vistas.insert(c.id.clone()) {
                        cartas.push(c);
                    }
                }
                seguros.push(en);
            }
        }
        cartas.truncate(MAX_CANDIDATAS);
    }
    // Lo escrito, en todo el nombre: «waste» trae Wastes, Snow-Covered
    // Wastes, Lay Waste…, lo más parecido primero.
    if escrito {
        for t in &pista {
            let mut halladas = buscar_en_el_nombre(state, t, MAX_ESCRITO).await;
            ordenar_por_lo_escrito(&mut halladas, t);
            for c in halladas {
                if vistas.insert(c.id.clone()) {
                    cartas.push(c);
                }
            }
        }
    }
    let mut candidates = resumir(state, cartas);
    // Las fichas que Scryfall no tiene (Cockatrice): las que se llaman como
    // algo leído o escrito; con lo escrito, también las que lo llevan.
    let leidos: Vec<String> = seguros
        .iter()
        .chain(&pista)
        .chain(&leido.titulo)
        .cloned()
        .collect();
    let mut externas = crate::fuentes::buscar_cockatrice(state, &leidos, true).await;
    if escrito {
        externas.extend(crate::fuentes::buscar_cockatrice(state, &pista, false).await);
    }
    for e in externas {
        if !candidates.iter().any(|c| c.id == e.id) {
            candidates.push(e);
        }
    }
    // Los aproximados no puntúan: la propuesta se mide contra lo leído, y así
    // una letra mal leída sale como «probable» para que se revise.
    let textos: Vec<String> = seguros.iter().chain(&principales).cloned().collect();
    let (mut best, mut confidence) = puntuar(&candidates, &textos);
    // Una ficha encontrada por su tipo traducido no se parece a lo leído
    // («Guerrero» no es «Warrior»), pero es la propuesta; hay varias versiones
    // de cada ficha, así que se deja como probable para revisarla.
    // La del pie, si lo leído no da otra: puede ser un pie copiado de otra
    // carta (las plantillas lo traen), así que solo probable.
    if confidence == Confidence::None {
        if let Some(id) = del_pie {
            best = Some(id);
            confidence = Confidence::Probable;
        }
    }
    if confidence == Confidence::None {
        if let Some(id) = primera_ficha {
            best = Some(id);
            confidence = Confidence::Probable;
        }
    }
    // La propuesta, la primera de la lista.
    if let Some(i) = best
        .as_ref()
        .and_then(|b| candidates.iter().position(|c| &c.id == b))
    {
        let c = candidates.remove(i);
        candidates.insert(0, c);
    }
    Ok(CardIdentify {
        lines: leido.pie.into_iter().chain(leido.titulo).collect(),
        candidates,
        best,
        confidence,
        pie,
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
        let result = identificar(estado, &it.blob, it.hint.as_deref(), ocr, false).await;
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

/// Lo que sabe de sí una carta propia vinculada a una de otra fuente.
#[tauri::command]
pub async fn card_link_externa(
    state: State<'_, AppState>,
    fuente: proxy_core::model::FuenteCarta,
    id: String,
) -> AppResult<CardMeta> {
    match fuente {
        proxy_core::model::FuenteCarta::Cockatrice => {
            let c =
                crate::fuentes::cockatrice::catalogo(&state.externo, &state.paths.cache).await?;
            let f = c
                .por_id(&id)
                .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("ficha", &id))?;
            Ok(crate::fuentes::meta_cockatrice(f))
        }
    }
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
            fuente: None,
        }
    }

    #[test]
    fn lo_escrito_ordena_por_parecido() {
        let carta = |nombre: &str, impreso: Option<&str>| {
            serde_json::from_value::<ScryCard>(serde_json::json!({
                "id": nombre, "name": nombre, "printed_name": impreso, "lang": "en",
                "set": "x", "set_name": "X", "collector_number": "1", "layout": "normal",
            }))
            .unwrap()
        };
        let mut cs = vec![
            carta("Adarkar Wastes", None),
            carta("Lay Waste", None),
            carta("Wasteland", None),
            carta("Biowaste Blob", None),
            carta("Wastes", None),
            carta("Ball Lightning", Some("Waste")),
            carta("Dinosaur // Waste", None),
        ];
        ordenar_por_lo_escrito(&mut cs, "waste");
        let nombres: Vec<_> = cs.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            nombres,
            [
                "Ball Lightning",
                "Dinosaur // Waste",
                "Wasteland",
                "Wastes",
                "Adarkar Wastes",
                "Lay Waste",
                "Biowaste Blob"
            ]
        );
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
        // Dos letras cambiadas de sitio: un error.
        assert_eq!(parecido("menmarch", "memnarch"), 0.875);
        assert_eq!(parecido("", "abc"), 0.0);
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

    #[test]
    fn a_igual_nombre_gana_la_carta_entera_y_no_la_cara_de_otra() {
        let t = vec!["Lightning Bolt".to_string()];
        // La de doble cara llega primero (así la devolvía Scryfall).
        let cs = vec![
            carta("dfc", "Emeritus of Conflict // Lightning Bolt", None),
            carta("bolt", "Lightning Bolt", None),
        ];
        assert_eq!(puntuar(&cs, &t), (Some("bolt".into()), Confidence::Exact));
        // Y en el otro orden, igual.
        let cs: Vec<_> = cs.into_iter().rev().collect();
        assert_eq!(puntuar(&cs, &t).0.as_deref(), Some("bolt"));
        // Por su nombre impreso también cuenta como entera.
        let cs = vec![
            carta("dfc", "Emeritus of Conflict // Lightning Bolt", None),
            carta("es", "Lightning Bolt", Some("Relámpago")),
        ];
        let t = vec!["Relámpago".to_string()];
        assert_eq!(puntuar(&cs, &t).0.as_deref(), Some("es"));
    }
}

/// Prueba en vivo (red y modelos del OCR): identifica las imágenes de una
/// carpeta. `PB_OCR_DATOS=<carpeta con modelos/> PB_ID_DIR=<carpeta con
/// imágenes> cargo test --release --lib identify::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;
    use crate::state::Paths;

    #[tokio::test]
    #[ignore]
    async fn identifica_una_carpeta() {
        let tmp = std::env::temp_dir().join("pb-identify");
        let _ = std::fs::remove_dir_all(&tmp);
        let datos = tmp.join("datos");
        std::fs::create_dir_all(&datos).unwrap();
        let origen = std::path::PathBuf::from(std::env::var("PB_OCR_DATOS").unwrap());
        std::os::unix::fs::symlink(origen.join("modelos"), datos.join("modelos")).unwrap();
        let state = AppState::open(Paths {
            data: datos,
            cache: tmp.join("cache"),
        })
        .unwrap();
        let mut rutas: Vec<_> = std::fs::read_dir(std::env::var("PB_ID_DIR").unwrap())
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "png"))
            .collect();
        rutas.sort();
        for (ruta, img) in rutas
            .iter()
            .zip(state.blobs.import_files(&state.db, &rutas).unwrap())
        {
            let r = identificar(&state, &img.blob, None, true, false)
                .await
                .unwrap();
            let elegida = r
                .best
                .as_ref()
                .and_then(|b| r.candidates.iter().find(|c| &c.id == b));
            println!(
                "{:?}: {:?} → {:?} ({:?})",
                ruta.file_name().unwrap(),
                r.lines,
                elegida.map(|c| (&c.name, &c.set)),
                r.confidence
            );
        }
    }
}
