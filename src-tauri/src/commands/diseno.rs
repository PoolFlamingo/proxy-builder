//! **Editor de cartas** (la 2.0): borradores desde Scryfall y colecciones.
//! El motor de render y la vista previa llegan en las fases siguientes.

use proxy_core::diseno::modelo::{ArtRef, Aviso, CardDesign, Collection, DesignDraft, DesignLang};
use proxy_core::model::DeckCard;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::diseno::borrador;
use crate::diseno::letras_libres::{self, LetraLibre};
use crate::diseno::original::{self, OriginalInfo, OriginalProgressEvent};
use crate::diseno::recursos::{LetraInfo, PackInfo, SimbolosInfo};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::colecciones;

/// El borrador de una carta del deck: la instantánea mecánica (inglés,
/// español y el idioma de diseño del deck, `lang`), los nombres originales
/// como punto de partida y el siguiente número libre del pie (`usados`: los
/// números que ya hay en el deck).
#[tauri::command]
pub async fn design_draft(
    state: State<'_, AppState>,
    card: DeckCard,
    usados: Vec<String>,
    collection: Option<String>,
    lang: Option<DesignLang>,
) -> AppResult<DesignDraft> {
    let id = borrador::id_scryfall(&card)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "sin_vincular"))?
        .to_string();
    let lang = lang.unwrap_or_default();
    let api = state.scryfall.clone();
    let en = borrador::carta_completa(&state.db, api.as_ref(), &id).await?;
    let datos = borrador::datos_de(&state.db, api.as_ref(), &en, lang).await?;
    let coleccion = match &collection {
        Some(c) => colecciones::get(&state.db, c)?,
        None => None,
    };
    let mut b = borrador::montar(&card, datos, coleccion.as_ref(), &usados);
    borrador::poner_textos(&mut b.design, lang);
    Ok(b)
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

/// Las letras libres que se pueden descargar y si ya están.
#[tauri::command]
pub fn fonts_free_list(state: State<'_, AppState>) -> AppResult<Vec<LetraLibre>> {
    letras_libres::lista(&state)
}

/// Descarga e importa una letra libre del catálogo.
#[tauri::command]
pub async fn font_free_download(state: State<'_, AppState>, id: String) -> AppResult<LetraLibre> {
    letras_libres::descargar(&state, &id).await
}

/// Cómo están los marcos «Original» (descargados o no).
#[tauri::command]
pub fn original_info(state: State<'_, AppState>) -> AppResult<OriginalInfo> {
    original::info(&state)
}

/// Descarga los marcos «Original» que falten; avisa por `original://progress`.
#[tauri::command]
pub async fn original_download(app: AppHandle, state: State<'_, AppState>) -> AppResult<PackInfo> {
    original::descargar(&state, &|done, total| {
        let _ = app.emit("original://progress", OriginalProgressEvent { done, total });
    })
    .await
}

/// Los packs de marcos descargables (Balatro…) y cómo están.
#[tauri::command]
pub fn packs_downloadable_list(
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::diseno::descargables::DescargableInfo>> {
    crate::diseno::descargables::lista(&state)
}

/// Descarga un pack de marcos del repositorio de packs; avisa por
/// `packs://progress`.
#[tauri::command]
pub async fn pack_downloadable_download(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> AppResult<PackInfo> {
    let id2 = id.clone();
    crate::diseno::descargables::descargar(&state, &id, &move |done, total| {
        let _ = app.emit(
            "packs://progress",
            crate::diseno::descargables::DescargaProgressEvent {
                id: id2.clone(),
                done,
                total,
            },
        );
    })
    .await
}

/// Un extra de un pack descargable (reverso, ficha, carta hecha) al almacén
/// de imágenes.
#[tauri::command]
pub fn pack_extra_use(
    state: State<'_, AppState>,
    id: String,
    ruta: String,
) -> AppResult<proxy_core::model::FaceImage> {
    crate::diseno::descargables::usar_extra(&state, &id, &ruta)
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

/// La carta sin la ilustración y dónde va, para moverla en directo
/// (`pbimg://…/c/<huella>/<ancho>`). `None` en las compuestas y las grandes.
#[tauri::command]
pub async fn design_overlay(
    app: AppHandle,
    design: CardDesign,
    cara: u32,
    lang: DesignLang,
    ancho: u32,
) -> AppResult<Option<crate::diseno::vista::CapaInfo>> {
    if !crate::protocol::ANCHOS.contains(&ancho) {
        return Err(AppError::new(ErrorCode::InvalidInput).with("ancho", ancho));
    }
    en_hilo(app, move |s| {
        crate::diseno::vista::capa(s, &design, cara, lang, ancho)
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

/// «Diseñar el deck»: borradores de las cartas sin diseño, en orden. Cada
/// carta hecha llega por `design://progress`; el final, por
/// `design://done`. Se cancela con `job_cancel`.
#[tauri::command]
pub fn design_drafts_for_deck(
    app: AppHandle,
    state: State<'_, AppState>,
    cards: Vec<DeckCard>,
    usados: Vec<String>,
    collection: Option<String>,
    lang: DesignLang,
) -> AppResult<u32> {
    let coleccion = match &collection {
        Some(c) => colecciones::get(&state.db, c)?,
        None => None,
    };
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let r = crate::diseno::lote::borradores(
            &app, job_id, cards, usados, coleccion, lang, &cancelado,
        )
        .await;
        estado.jobs.baja(job_id);
        let _ = tauri::Emitter::emit(
            &app,
            "design://done",
            crate::diseno::lote::DesignDoneEvent {
                job_id,
                error: r.err(),
            },
        );
    });
    Ok(job_id)
}

/// Completa el idioma de diseño `lang` en las cartas diseñadas del deck: baja
/// sus letras si es japonés, coreano o chino, y la impresión de cada carta a
/// la que le falte. Cada carta hecha llega por `designlang://progress`; el
/// final, por `designlang://done`. Se cancela con `job_cancel`.
#[tauri::command]
pub fn design_complete_lang(
    app: AppHandle,
    state: State<'_, AppState>,
    cards: Vec<DeckCard>,
    lang: DesignLang,
) -> AppResult<u32> {
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let r = crate::diseno::idioma::completar_deck(&app, job_id, cards, lang, &cancelado).await;
        estado.jobs.baja(job_id);
        let _ = tauri::Emitter::emit(
            &app,
            "designlang://done",
            crate::diseno::lote::DesignDoneEvent {
                job_id,
                error: r.err(),
            },
        );
    });
    Ok(job_id)
}

/// Exporta las cartas diseñadas como PNG a una carpeta, cada idioma en la
/// suya (`es/`, `en/`, `fr/`…). El
/// avance llega por `designpng://progress`; el final, por `designpng://done`.
#[tauri::command]
pub fn design_export_png(
    app: AppHandle,
    state: State<'_, AppState>,
    cards: Vec<DeckCard>,
    folder: String,
    card_size: proxy_core::model::CardSize,
    dpi: u32,
    langs: Vec<DesignLang>,
) -> AppResult<u32> {
    if !matches!(dpi, 300 | 600 | 900) || langs.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "png"));
    }
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn_blocking(move || {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let r = crate::diseno::png::exportar(
            &estado,
            &cards,
            std::path::Path::new(&folder),
            card_size,
            dpi,
            &langs,
            &cancelado,
            &|done, total| {
                let _ = tauri::Emitter::emit(
                    &app,
                    "designpng://progress",
                    crate::diseno::png::DesignPngProgressEvent {
                        job_id,
                        done,
                        total,
                    },
                );
            },
        );
        estado.jobs.baja(job_id);
        let (files, error) = match r {
            Ok(n) => (n, None),
            Err(e) => (0, Some(e)),
        };
        let _ = tauri::Emitter::emit(
            &app,
            "designpng://done",
            crate::diseno::png::DesignPngDoneEvent {
                job_id,
                files,
                error,
            },
        );
    });
    Ok(job_id)
}

/// Traducción sugerida (experimental) de una cara sin texto en `lang` (un
/// idioma con cartas impresas): frases oficiales idénticas de otras cartas y
/// su línea de tipo. Nunca se aplica sola: la pantalla la enseña y el
/// usuario decide.
#[tauri::command]
pub async fn design_suggest_translation(
    state: State<'_, AppState>,
    design: CardDesign,
    cara: u32,
    lang: DesignLang,
) -> AppResult<crate::diseno::traduccion::Sugerencia> {
    if lang == DesignLang::En || !lang.impresas() {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "idioma"));
    }
    let f = design
        .card
        .faces
        .get(cara as usize)
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "cara"))?;
    crate::diseno::traduccion::sugerir(
        state.scryfall.as_ref(),
        &f.name,
        &f.type_line.en,
        &f.rules.en,
        lang.codigo(),
    )
    .await
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

/// La imagen del portapapeles del sistema; `None` si no hay ninguna.
#[tauri::command]
pub async fn art_from_clipboard(app: AppHandle) -> AppResult<Option<ArtRef>> {
    let Some(img) = crate::diseno::portapapeles::imagen(&app).await? else {
        return Ok(None);
    };
    en_hilo(app, move |s| {
        crate::diseno::arte::desde_imagen(s, &img).map(Some)
    })
    .await
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
    cara: Option<u32>,
) -> AppResult<ArtRef> {
    crate::diseno::arte::desde_scryfall(&state, &scryfall_id, cara.unwrap_or(0) as usize).await
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
