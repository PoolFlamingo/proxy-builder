//! La biblioteca: decks guardados, imágenes subidas y ajustes sueltos.

use std::path::PathBuf;

use proxy_core::model::{Deck, FaceImage};
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::fichero;
use crate::optimizar::{DeckOptimizable, EstimacionOptimizar, OptimizarFin, OptimizarProgreso};
use crate::state::AppState;
use crate::store::blobs::GcReport;
use crate::store::db::Db;
use crate::store::decks::{self, DeckSummary};
use crate::store::etiquetas::{self, Tag};
use crate::store::presets::{self, Preset, PresetKind};
use crate::store::settings;

#[tauri::command]
pub fn deck_list(state: State<'_, AppState>) -> AppResult<Vec<DeckSummary>> {
    decks::list(&state.db)
}

/// Crea un deck vacío con el reverso genérico de la app.
#[tauri::command]
pub fn deck_create(state: State<'_, AppState>, name: String) -> AppResult<Deck> {
    crear_deck(&state, &name)
}

fn crear_deck(state: &AppState, name: &str) -> AppResult<Deck> {
    let reverso = state.blobs.generic_back(&state.db)?;
    decks::create(&state.db, name, Some(reverso))
}

#[tauri::command]
pub fn deck_open(state: State<'_, AppState>, id: String) -> AppResult<Deck> {
    let d = decks::open(&state.db, &id)?;
    decks::marcar_abierto(&state.db, &id)?;
    Ok(d)
}

/// El deck, para mirarlo desde la biblioteca (elegir la portada) sin
/// apuntarlo como abierto.
#[tauri::command]
pub fn deck_peek(state: State<'_, AppState>, id: String) -> AppResult<Deck> {
    decks::open(&state.db, &id)
}

/// Colores, clase, precio, idiomas… de los decks (todos si `ids` va vacío),
/// con la caché de Scryfall y sin red.
#[tauri::command]
pub async fn deck_details(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> AppResult<Vec<crate::biblioteca::DeckDetails>> {
    crate::biblioteca::detalles(&state.db, &state.detalles, &ids).await
}

#[tauri::command]
pub fn tag_list(state: State<'_, AppState>) -> AppResult<Vec<Tag>> {
    etiquetas::list(&state.db)
}

/// Crea una etiqueta (o devuelve la que ya se llama así).
#[tauri::command]
pub fn tag_create(state: State<'_, AppState>, name: String, color: String) -> AppResult<Tag> {
    etiquetas::create(&state.db, &name, &color)
}

#[tauri::command]
pub fn tag_update(
    state: State<'_, AppState>,
    id: String,
    name: String,
    color: String,
) -> AppResult<Tag> {
    etiquetas::update(&state.db, &id, &name, &color)
}

/// El icono de una etiqueta (`None`: sin icono).
#[tauri::command]
pub fn tag_icon_set(state: State<'_, AppState>, id: String, icon: Option<String>) -> AppResult<()> {
    etiquetas::icono(&state.db, &id, icon.as_deref())
}

#[tauri::command]
pub fn tag_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    etiquetas::delete(&state.db, &id)
}

/// Pone y quita etiquetas a varios decks a la vez.
#[tauri::command]
pub fn deck_tags_set(
    state: State<'_, AppState>,
    decks: Vec<String>,
    add: Vec<String>,
    remove: Vec<String>,
) -> AppResult<()> {
    etiquetas::etiquetar(&state.db, &decks, &add, &remove)
}

#[tauri::command]
pub fn deck_pin(state: State<'_, AppState>, decks: Vec<String>, pinned: bool) -> AppResult<()> {
    etiquetas::favorito(&state.db, &decks, pinned)
}

#[tauri::command]
pub fn deck_notes_set(state: State<'_, AppState>, id: String, notes: String) -> AppResult<()> {
    etiquetas::notas(&state.db, &id, &notes)
}

/// El idioma de varios decks elegido a mano (`None`: el que se calcula).
#[tauri::command]
pub fn deck_lang_set(
    state: State<'_, AppState>,
    decks: Vec<String>,
    lang: Option<String>,
) -> AppResult<()> {
    etiquetas::idioma(&state.db, &decks, lang.as_deref())
}

#[tauri::command]
pub fn deck_cover_set(
    state: State<'_, AppState>,
    id: String,
    card: Option<String>,
) -> AppResult<()> {
    etiquetas::portada(&state.db, &id, card.as_deref())
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SaveResult {
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[tauri::command]
pub fn deck_save(state: State<'_, AppState>, deck: Deck) -> AppResult<SaveResult> {
    Ok(SaveResult {
        updated_at: decks::save(&state.db, &deck)?,
    })
}

#[tauri::command]
pub fn deck_rename(state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    decks::rename(&state.db, &id, &name)
}

#[tauri::command]
pub fn deck_duplicate(state: State<'_, AppState>, id: String, name: String) -> AppResult<Deck> {
    decks::duplicate(&state.db, &id, &name)
}

#[tauri::command]
pub fn deck_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    decks::delete(&state.db, &id)
}

/// Lo más que se lee de una imagen elegida en el selector del sistema.
const TOPE_IMAGEN: u64 = 256 * 1024 * 1024;

/// Copia al almacén las imágenes que el usuario ha elegido en el diálogo. En
/// Android son `content://`: se leen por el sistema, sin pasar por rutas.
#[tauri::command]
pub async fn blob_import_files(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> AppResult<Vec<FaceImage>> {
    if !paths.iter().any(|p| fichero::es_uri(p)) {
        let rutas: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        return state.blobs.import_files(&state.db, &rutas);
    }
    paths
        .iter()
        .map(|p| {
            let bytes = fichero::leer_bytes(&app, p, TOPE_IMAGEN)?;
            state
                .blobs
                .put_bytes(&state.db, &bytes, proxy_core::model::ImageOrigin::Upload)
        })
        .collect()
}

/// El reverso genérico de la app (para devolverlo a un deck).
#[tauri::command]
pub fn blob_generic_back(state: State<'_, AppState>) -> AppResult<FaceImage> {
    state.blobs.generic_back(&state.db)
}

/// Borra las imágenes que ya no usa ningún deck.
#[tauri::command]
pub fn blob_gc(state: State<'_, AppState>) -> AppResult<GcReport> {
    state.blobs.gc(&state.db, state.ocupado())
}

/// Los decks con imágenes que se pueden optimizar («Liberar espacio»).
#[tauri::command]
pub fn optimizar_decks(state: State<'_, AppState>) -> AppResult<Vec<DeckOptimizable>> {
    crate::optimizar::optimizables(&state)
}

/// Lo que liberaría optimizar estos decks.
#[tauri::command]
pub fn optimizar_estimar(
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> AppResult<EstimacionOptimizar> {
    crate::optimizar::estimar(&state, &ids)
}

/// Optimiza estos decks, en segundo plano: el avance llega por
/// `optimizar://progress` y el final por `optimizar://done`. Se cancela con
/// `job_cancel` (mientras se hacen los JPEG). No con una partida o un trabajo
/// en marcha: usan estas imágenes.
#[tauri::command]
pub fn optimizar_lanzar(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    ids: Vec<String>,
) -> AppResult<u32> {
    if state.ocupado() {
        return Err(AppError::new(ErrorCode::Busy));
    }
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn_blocking(move || {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let emisor = app.clone();
        let r = crate::optimizar::optimizar(&estado, &ids, &cancelado, |hechas, total| {
            let _ = emisor.emit(
                "optimizar://progress",
                OptimizarProgreso {
                    job_id,
                    hechas,
                    total,
                },
            );
        });
        estado.jobs.baja(job_id);
        let fin = match r {
            Ok(h) => OptimizarFin {
                job_id,
                imagenes: h.imagenes,
                libera: h.libera,
                cancelado: h.cancelado,
                error: None,
            },
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "no se pudieron optimizar las imágenes");
                OptimizarFin {
                    job_id,
                    imagenes: 0,
                    libera: 0,
                    cancelado: false,
                    error: Some(e),
                }
            }
        };
        let _ = app.emit("optimizar://done", fin);
    });
    Ok(job_id)
}

#[tauri::command]
pub fn app_setting_get(state: State<'_, AppState>, key: String) -> AppResult<Option<String>> {
    settings::get(&state.db, &key)
}

#[tauri::command]
pub fn app_setting_set(state: State<'_, AppState>, key: String, value: String) -> AppResult<()> {
    settings::set(&state.db, &key, &value)
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CacheStats {
    pub files: u32,
    #[ts(type = "number")]
    pub bytes: u64,
}

fn recorrer(dir: &std::path::Path, out: &mut CacheStats) {
    let Ok(entradas) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entradas.flatten() {
        let p = e.path();
        if p.is_dir() {
            recorrer(&p, out);
        } else if let Ok(m) = e.metadata() {
            out.files += 1;
            out.bytes += m.len();
        }
    }
}

fn medir(dir: &std::path::Path) -> CacheStats {
    let mut s = CacheStats { files: 0, bytes: 0 };
    recorrer(dir, &mut s);
    s
}

/// Borra la carpeta y devuelve lo que ocupaba.
fn vaciar(dir: &std::path::Path) -> AppResult<CacheStats> {
    let antes = medir(dir);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(antes)
}

/// Lo que ocupa la caché de miniaturas (se puede borrar sin perder nada).
#[tauri::command]
pub fn cache_stats(state: State<'_, AppState>) -> CacheStats {
    medir(&state.carpeta_renders())
}

/// Borra la caché de miniaturas. 🛑 Nunca toca el almacén de imágenes de los decks.
#[tauri::command]
pub fn cache_clear(state: State<'_, AppState>) -> AppResult<CacheStats> {
    vaciar(&state.carpeta_renders())
}

/// Guarda el deck con todas sus imágenes en un `.pbdeck`. En Android `path` es
/// un `content://`: se escribe en la caché y se copia al destino.
#[tauri::command]
pub async fn deck_export_package(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> AppResult<u64> {
    let deck = decks::open(&state.db, &id)?;
    fichero::escribir(&app, &path, Some("deck.pbdeck"), |destino| {
        crate::package::export(&deck, destino, &state.blobs, &state.db)
    })
}

/// Mete en la biblioteca un `.pbdeck` (ids nuevos, imágenes comprobadas). Un
/// `content://` de Android se copia antes a la caché: el ZIP necesita `Seek`.
#[tauri::command]
pub async fn deck_import_package(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> AppResult<Deck> {
    let copia = fichero::leer(&app, &path, Some("deck.pbdeck"))?;
    let f = std::fs::File::open(&copia)?;
    crate::package::import(std::io::BufReader::new(f), &state.blobs, &state.db)
}

#[tauri::command]
pub fn preset_list(state: State<'_, AppState>, kind: PresetKind) -> AppResult<Vec<Preset>> {
    presets::list(&state.db, kind)
}

#[tauri::command]
pub fn preset_save(
    state: State<'_, AppState>,
    kind: PresetKind,
    name: String,
    json: String,
) -> AppResult<Preset> {
    guardar_preset(&state.db, kind, &name, &json)
}

fn guardar_preset(db: &Db, kind: PresetKind, name: &str, json: &str) -> AppResult<Preset> {
    if name.trim().is_empty() {
        return Err(crate::error::AppError::new(
            crate::error::ErrorCode::InvalidInput,
        ));
    }
    presets::save(db, kind, name, json)
}

#[tauri::command]
pub fn preset_delete(state: State<'_, AppState>, id: String) -> AppResult<()> {
    presets::delete(&state.db, &id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorCode;
    use crate::state::Paths;
    use crate::store::blobs::tests::png;
    use proxy_core::model::ImageOrigin;

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    fn escribir(ruta: &std::path::Path, bytes: usize) {
        std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
        std::fs::write(ruta, vec![7u8; bytes]).unwrap();
    }

    #[test]
    fn un_deck_nuevo_nace_vacio_con_el_reverso_generico() {
        let (_dir, s) = estado();
        let d = crear_deck(&s, "Mi deck").unwrap();
        assert_eq!(d.name, "Mi deck");
        assert!(d.cards.is_empty());
        let reverso = s.blobs.generic_back(&s.db).unwrap();
        assert_eq!(d.default_back.as_ref(), Some(&reverso));
        assert_eq!(reverso.origin, ImageOrigin::Generated);
        // El segundo deck comparte la misma imagen de reverso.
        let otro = crear_deck(&s, "Otro").unwrap();
        assert_eq!(otro.default_back, d.default_back);
        assert_ne!(otro.id, d.id);
        assert_eq!(decks::list(&s.db).unwrap().len(), 2);
    }

    #[test]
    fn medir_la_cache_cuenta_ficheros_y_bytes_en_subcarpetas() {
        let dir = tempfile::tempdir().unwrap();
        escribir(&dir.path().join("a.jpg"), 10);
        escribir(&dir.path().join("x/b.jpg"), 20);
        escribir(&dir.path().join("x/y/c.jpg"), 30);
        let m = medir(dir.path());
        assert_eq!((m.files, m.bytes), (3, 60));
        // Una carpeta que no existe ocupa cero, sin error.
        let m = medir(&dir.path().join("no-existe"));
        assert_eq!((m.files, m.bytes), (0, 0));
    }

    #[test]
    fn vaciar_la_cache_no_toca_las_imagenes_de_los_decks() {
        let (_dir, s) = estado();
        let img = s
            .blobs
            .put_bytes(&s.db, &png(4, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let renders = s.carpeta_renders();
        // La caché vive en la carpeta de caché, lejos del almacén.
        assert!(renders.starts_with(&s.paths.cache));
        assert!(!renders.starts_with(&s.paths.data));
        escribir(&renders.join("ab.jpg"), 100);
        escribir(&renders.join("cd.jpg"), 50);

        let antes = vaciar(&renders).unwrap();
        assert_eq!((antes.files, antes.bytes), (2, 150));
        assert!(!renders.exists());
        // La imagen del almacén sigue ahí y se puede leer.
        let (ruta, _) = s.blobs.locate(&s.db, &img.blob).unwrap().unwrap();
        assert!(ruta.exists());
        assert!(s.leer_blob(&img.blob).is_ok());
        // Vaciar otra vez no falla y ya no hay nada.
        let otra = vaciar(&renders).unwrap();
        assert_eq!((otra.files, otra.bytes), (0, 0));
    }

    #[test]
    fn un_preset_sin_nombre_no_se_guarda() {
        let (_dir, s) = estado();
        for nombre in ["", "   ", "\t"] {
            let e = guardar_preset(&s.db, PresetKind::Print, nombre, "{}").unwrap_err();
            assert_eq!(e.code, ErrorCode::InvalidInput);
        }
        assert!(presets::list(&s.db, PresetKind::Print).unwrap().is_empty());
        let p = guardar_preset(&s.db, PresetKind::Print, "  A4 casa ", "{\"a\":1}").unwrap();
        assert_eq!(p.name, "A4 casa");
        assert_eq!(presets::list(&s.db, PresetKind::Print).unwrap(), vec![p]);
    }
}
