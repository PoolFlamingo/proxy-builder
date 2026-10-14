//! **Editor de cartas** (la 2.0): borradores desde Scryfall y colecciones.
//! El motor de render y la vista previa llegan en las fases siguientes.

use proxy_core::diseno::modelo::{ArtRef, Aviso, CardDesign, Collection, DesignDraft, DesignLang};
use proxy_core::model::DeckCard;
use tauri::{AppHandle, Manager, State};

use crate::diseno::borrador;
use crate::diseno::recursos::{LetraInfo, PackInfo, SimbolosInfo};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::colecciones;

/// El borrador de una carta del deck: la instantánea mecánica (inglés y
/// español), los nombres originales como punto de partida y el siguiente
/// número libre del pie (`usados`: los números que ya hay en el deck).
#[tauri::command]
pub async fn design_draft(
    state: State<'_, AppState>,
    card: DeckCard,
    usados: Vec<String>,
    collection: Option<String>,
) -> AppResult<DesignDraft> {
    let id = borrador::id_scryfall(&card)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "sin_vincular"))?
        .to_string();
    let api = state.scryfall.clone();
    let en = borrador::carta_completa(&state.db, api.as_ref(), &id).await?;
    let es = borrador::impresion_es(&state.db, api.as_ref(), &en).await?;
    let fundida = borrador::fundida(&state.db, api.as_ref(), &en).await?;
    let datos = borrador::con_fundida(
        borrador::datos(&en, es.as_ref(), crate::store::ahora_ms()),
        fundida,
    );
    let coleccion = match &collection {
        Some(c) => colecciones::get(&state.db, c)?,
        None => None,
    };
    Ok(borrador::montar(&card, datos, coleccion.as_ref(), &usados))
}

#[tauri::command]
pub fn collection_list(state: State<'_, AppState>) -> AppResult<Vec<Collection>> {
    colecciones::list(&state.db)
}

#[tauri::command]
pub fn collection_save(
    state: State<'_, AppState>,
    collection: Collection,
) -> AppResult<Collection> {
    colecciones::save(&state.db, &collection)
}

#[tauri::command]
pub fn collection_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    colecciones::delete(&state.db, &id)
}

// ── Letras, packs y símbolos ───────────────────────────────────────────────

#[tauri::command]
pub fn fonts_list(state: State<'_, AppState>) -> AppResult<Vec<LetraInfo>> {
    state.diseno.letras()
}

#[tauri::command]
pub fn font_import(state: State<'_, AppState>, path: String) -> AppResult<LetraInfo> {
    state.diseno.importar_letra(std::path::Path::new(&path))
}

#[tauri::command]
pub fn font_remove(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.diseno.quitar_letra(&id)
}

#[tauri::command]
pub fn packs_list(state: State<'_, AppState>) -> AppResult<Vec<PackInfo>> {
    state.diseno.packs()
}

#[tauri::command]
pub fn pack_import(state: State<'_, AppState>, path: String) -> AppResult<PackInfo> {
    state.diseno.importar_pack(std::path::Path::new(&path))
}

#[tauri::command]
pub fn pack_remove(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.diseno.quitar_pack(&id)
}

/// Cómo están los símbolos; si faltan o son viejos, los descarga antes.
#[tauri::command]
pub async fn symbols_status(state: State<'_, AppState>, refresh: bool) -> AppResult<SimbolosInfo> {
    state
        .diseno
        .asegurar_simbolos(state.scryfall.as_ref(), refresh)
        .await?;
    state.diseno.simbolos_info()
}

// ── Vista previa, guardar y quitar ─────────────────────────────────────────

/// Corre `f` en un hilo aparte con el estado de la app (los renders pesan).
async fn en_hilo<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppState) -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?
}

/// Registra el diseño que se edita para la vista previa (`/d/<huella>/<ancho>`).
#[tauri::command]
pub async fn design_register(
    app: AppHandle,
    design: CardDesign,
    cara: u32,
    lang: DesignLang,
) -> AppResult<String> {
    en_hilo(app, move |s| {
        crate::diseno::vista::registrar(s, &design, cara, lang)
    })
    .await
}

/// Los avisos de un diseño: texto que no cabe, reglas a mano, símbolos o
/// letras que faltan…
#[tauri::command]
pub async fn design_fit_report(
    app: AppHandle,
    design: CardDesign,
    lang: DesignLang,
) -> AppResult<Vec<Aviso>> {
    en_hilo(app, move |s| crate::diseno::vista::avisos(s, &design, lang)).await
}

/// Cómo se maqueta una cara (layout y texto partido), para el editor.
#[tauri::command]
pub fn design_structure(
    design: CardDesign,
    cara: u32,
    lang: DesignLang,
) -> proxy_core::diseno::maquetacion::Resumen {
    proxy_core::diseno::maquetacion::resumen(&design, cara as usize, lang)
}

/// Aplica el diseño a la carta (miniatura en el almacén) y la devuelve; la
/// pantalla la pone en el deck.
#[tauri::command]
pub async fn design_save(
    app: AppHandle,
    card: DeckCard,
    design: CardDesign,
    lang: DesignLang,
) -> AppResult<DeckCard> {
    en_hilo(app, move |s| {
        crate::diseno::vista::aplicar(s, &card, &design, lang)
    })
    .await
}

/// Deja la carta como estaba antes del diseño.
#[tauri::command]
pub fn design_remove(card: DeckCard) -> DeckCard {
    crate::diseno::vista::quitar(&card)
}

/// Vuelve a numerar los diseños por el orden del deck; devuelve las cartas
/// que cambian, ya con su miniatura.
#[tauri::command]
pub async fn design_renumber(
    app: AppHandle,
    cards: Vec<DeckCard>,
    lang: DesignLang,
) -> AppResult<Vec<DeckCard>> {
    en_hilo(app, move |s| {
        crate::diseno::vista::renumerar(s, &cards, lang)
    })
    .await
}

/// Rehace las miniaturas desfasadas del deck abierto; devuelve las cartas
/// que cambian (la pantalla las aplica de una vez).
#[tauri::command]
pub async fn design_rerender(
    app: AppHandle,
    cards: Vec<DeckCard>,
    lang: DesignLang,
) -> AppResult<Vec<DeckCard>> {
    en_hilo(app, move |s| crate::diseno::vista::rehacer(s, &cards, lang)).await
}

// ── Ilustración ────────────────────────────────────────────────────────────

/// Una imagen pegada del portapapeles (base64).
#[tauri::command]
pub async fn art_from_clipboard(app: AppHandle, data: String) -> AppResult<ArtRef> {
    en_hilo(app, move |s| crate::diseno::arte::desde_bytes(s, &data)).await
}

/// Un fichero ya importado al almacén (`blob_import_files`).
#[tauri::command]
pub fn art_from_blob(state: State<'_, AppState>, blob: String) -> AppResult<ArtRef> {
    crate::diseno::arte::desde_blob(&state, &blob)
}

/// El `art_crop` de Scryfall.
#[tauri::command]
pub async fn art_from_scryfall(
    state: State<'_, AppState>,
    scryfall_id: String,
) -> AppResult<ArtRef> {
    crate::diseno::arte::desde_scryfall(&state, &scryfall_id).await
}

/// Recorta la ilustración de una carta ya hecha (fracciones de 0 a 1).
#[tauri::command]
pub async fn art_from_card(
    app: AppHandle,
    blob: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> AppResult<ArtRef> {
    en_hilo(app, move |s| {
        crate::diseno::arte::desde_carta(s, &blob, x, y, w, h)
    })
    .await
}
