//! **Exportar el PDF**: un trabajo largo, en paralelo, con progreso,
//! cancelable y que nunca deja un fichero a medias con el nombre bueno.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use krilla::image::Image;
use krilla::Data;
use proxy_core::imaging::pipeline::{RenderSpec, Rendered};
use proxy_core::layout::{plan, Page, PagePlan};
use proxy_core::model::{CardEdits, Deck, PrintSettings};
use proxy_core::pdf::calibration::{calibration, CalibrationTexts};
use proxy_core::pdf::estimate::bytes_por_imagen;
use proxy_core::pdf::{compose, PdfError, PdfMeta};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use crate::diseno::pintar::{pintar, DisenosDelDeck};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::render::{render_para_pdf, spec_de_casilla};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;
use proxy_core::diseno::efectivo::DisenoEfectivo;
use proxy_core::diseno::render::Salida;
use proxy_core::imaging::frame::trim_px;
use proxy_core::imaging::pipeline::{render_desde_imagen, DisenoRef};
use proxy_core::model::DeckCard;

impl From<PdfError> for AppError {
    fn from(e: PdfError) -> Self {
        match e {
            PdfError::Cancelled => AppError::new(ErrorCode::Cancelled),
            otro => AppError::new(ErrorCode::Pdf).with("detalle", otro),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ExportPhase {
    Prepare,
    Compose,
    Write,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportProgressEvent {
    pub job_id: u32,
    pub phase: ExportPhase,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportResult {
    pub paths: Vec<String>,
    pub pages: u32,
    pub sheets: u32,
    #[ts(type = "number")]
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportDoneEvent {
    pub job_id: u32,
    pub result: Option<ExportResult>,
    pub error: Option<AppError>,
}

/// Qué imagen va en cada casilla (página, índice de casilla) → clave de render.
struct Encargo {
    especificaciones: HashMap<String, RenderSpec>,
    por_casilla: HashMap<(usize, usize), String>,
    /// Huella → diseño efectivo, para las casillas de cartas diseñadas.
    disenos: HashMap<String, DisenoEfectivo>,
}

fn encargo(
    deck: &Deck,
    p: &PagePlan,
    disenos: Option<(&AppState, &DisenosDelDeck)>,
) -> AppResult<Encargo> {
    let cartas: HashMap<&str, &DeckCard> = deck.cards.iter().map(|c| (c.id.as_str(), c)).collect();
    let mut especificaciones = HashMap::new();
    let mut por_casilla = HashMap::new();
    let mut efectivos = HashMap::new();
    let ninguna = CardEdits::default();
    for (i, pagina) in p.pages.iter().enumerate() {
        for (j, slot) in pagina.slots.iter().enumerate() {
            let Some(img) = &slot.image else { continue };
            let carta = cartas.get(slot.card_id.as_str()).copied();
            let ed = carta.map_or(&ninguna, |c| &c.edits);
            let mut spec = spec_de_casilla(slot, img, ed, &deck.settings);
            if let (Some((state, d)), Some(c)) = (disenos, carta) {
                if let Some(r) = d.referencia(state, c, slot)? {
                    // La carta diseñada sale del motor: sin retoques ni oscurecido.
                    spec.edits = CardEdits::default();
                    spec.edge = None;
                    if let Some(e) = d.efectivos.get(&c.id) {
                        efectivos.insert(r.huella.clone(), e.clone());
                    }
                    spec.diseno = Some(r);
                }
            }
            let k = spec.key();
            especificaciones.entry(k.clone()).or_insert(spec);
            por_casilla.insert((i, j), k);
        }
    }
    Ok(Encargo {
        especificaciones,
        por_casilla,
        disenos: efectivos,
    })
}

fn huella_documento(p: &PagePlan, claves: &HashMap<(usize, usize), String>) -> String {
    let mut h = Sha256::new();
    h.update(serde_json::to_vec(p).unwrap_or_default());
    let mut cs: Vec<_> = claves.iter().collect();
    cs.sort();
    for (k, v) in cs {
        h.update(format!("{k:?}{v}").as_bytes());
    }
    h.finalize()
        .iter()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Reparte las páginas en ficheros de `n` hojas.
fn trozos(p: &PagePlan, n: Option<u32>) -> Vec<Vec<usize>> {
    let por_hoja = |pg: &Page| pg.sheet;
    match n.filter(|n| *n > 0) {
        None => vec![(0..p.pages.len()).collect()],
        Some(n) => {
            let mut out: Vec<Vec<usize>> = Vec::new();
            for (i, pg) in p.pages.iter().enumerate() {
                let t = (por_hoja(pg) / n) as usize;
                if out.len() <= t {
                    out.resize(t + 1, Vec::new());
                }
                out[t].push(i);
            }
            out
        }
    }
}

fn ruta_trozo(base: &Path, i: usize, total: usize) -> PathBuf {
    if total <= 1 {
        return base.to_path_buf();
    }
    let tallo = base.file_stem().and_then(|s| s.to_str()).unwrap_or("deck");
    base.with_file_name(format!("{tallo} ({}).pdf", i + 1))
}

/// Una casilla de carta diseñada: el motor la pinta a la medida exacta del
/// corte y a los PPP de la exportación; después, sangrado y codificación.
fn render_diseno_para_pdf(
    state: &AppState,
    e: &DisenoEfectivo,
    d: &DisenoRef,
    spec: &RenderSpec,
) -> AppResult<Rendered> {
    let (tw, _) = trim_px(spec.card, spec.target.into());
    let r = pintar(state, e, d.cara, d.lang, spec.card, tw, Salida::Impresion)?;
    Ok(render_desde_imagen(&r.imagen, spec)?)
}

/// El trabajo entero (síncrono: se lanza en un hilo aparte).
pub fn exportar(
    state: &AppState,
    deck: &Deck,
    destino: &Path,
    cancelado: &AtomicBool,
    progreso: &(dyn Fn(ExportPhase, u32, u32) + Sync),
) -> AppResult<ExportResult> {
    let p = plan(deck)?;
    let disenos = DisenosDelDeck::de(state, deck)?;
    let enc = encargo(deck, &p, Some((state, &disenos)))?;
    let total = enc.especificaciones.len() as u32;
    let hechas = AtomicU32::new(0);
    progreso(ExportPhase::Prepare, 0, total);

    // 1. Preparar cada imagen distinta, en paralelo (como mucho 8 hilos).
    let hilos = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(hilos)
        .build()
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    let lista: Vec<(&String, &RenderSpec)> = enc.especificaciones.iter().collect();
    let preparadas: Vec<AppResult<(String, Rendered)>> = pool.install(|| {
        lista
            .par_iter()
            .map(|(k, spec)| {
                if cancelado.load(Ordering::SeqCst) {
                    return Err(AppError::new(ErrorCode::Cancelled));
                }
                let r = match &spec.diseno {
                    Some(d) => {
                        let e = enc.disenos.get(&d.huella).ok_or_else(|| {
                            AppError::new(ErrorCode::Internal).with("detalle", "diseño perdido")
                        })?;
                        render_diseno_para_pdf(state, e, d, spec)?
                    }
                    None => render_para_pdf(state, spec)?,
                };
                let n = hechas.fetch_add(1, Ordering::SeqCst) + 1;
                progreso(ExportPhase::Prepare, n, total);
                Ok(((*k).clone(), r))
            })
            .collect()
    });
    let mut imagenes: HashMap<String, Image> = HashMap::new();
    for r in preparadas {
        let (k, rendered) = r?;
        let datos = Data::from(rendered.bytes().to_vec());
        let img = if rendered.is_jpeg() {
            Image::from_jpeg(datos, true)
        } else {
            Image::from_png(datos, true)
        }
        .map_err(|e| AppError::new(ErrorCode::Pdf).with("detalle", e))?;
        imagenes.insert(k, img);
    }

    // 2. Componer y escribir cada fichero.
    let trozos = trozos(&p, deck.settings.split_every);
    let id = huella_documento(&p, &enc.por_casilla);
    let total_paginas = p.pages.len() as u32;
    let mut paginas_hechas = 0u32;
    let mut rutas = Vec::new();
    let mut bytes = 0u64;
    for (t, indices) in trozos.iter().enumerate() {
        let sub = PagePlan {
            pages: indices.iter().map(|&i| p.pages[i].clone()).collect(),
            ..p.clone()
        };
        let pdf = compose(
            &sub,
            &deck.settings.cut_guides,
            &PdfMeta {
                title: deck.name.clone(),
                document_id: format!("{id}-{t}"),
            },
            |pagina, slot| {
                let i = indices[sub
                    .pages
                    .iter()
                    .position(|x| std::ptr::eq(x, pagina))
                    .unwrap_or(0)];
                let j = p.pages[i]
                    .slots
                    .iter()
                    .position(|s| s == slot)
                    .unwrap_or(usize::MAX);
                Ok(enc
                    .por_casilla
                    .get(&(i, j))
                    .and_then(|k| imagenes.get(k))
                    .cloned())
            },
            |_| {
                paginas_hechas += 1;
                progreso(ExportPhase::Compose, paginas_hechas, total_paginas);
                !cancelado.load(Ordering::SeqCst)
            },
        )?;
        if cancelado.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::Cancelled));
        }
        progreso(ExportPhase::Write, t as u32, trozos.len() as u32);
        let ruta = ruta_trozo(destino, t, trozos.len());
        escribir_atomico(&ruta, &pdf)?;
        bytes += pdf.len() as u64;
        rutas.push(ruta.to_string_lossy().to_string());
    }
    Ok(ExportResult {
        paths: rutas,
        pages: total_paginas,
        sheets: p.sheets,
        bytes,
    })
}

/// Exporta el deck (la instantánea que tiene la pantalla, no la de la base:
/// el autoguardado puede ir 800 ms por detrás). Avance por `export://progress`
/// y final por `export://done`.
#[tauri::command]
pub fn export_pdf(app: AppHandle, state: State<'_, AppState>, deck: Deck, path: PathBuf) -> u32 {
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn_blocking(move || {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let emisor = app.clone();
        let progreso = move |phase: ExportPhase, done: u32, total: u32| {
            let _ = emisor.emit(
                "export://progress",
                ExportProgressEvent {
                    job_id,
                    phase,
                    done,
                    total,
                },
            );
        };
        let r = exportar(&estado, &deck, &path, &cancelado, &progreso);
        estado.jobs.baja(job_id);
        let (result, error) = match r {
            Ok(x) => (Some(x), None),
            Err(e) => (None, Some(e)),
        };
        let _ = app.emit(
            "export://done",
            ExportDoneEvent {
                job_id,
                result,
                error,
            },
        );
    });
    job_id
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ExportEstimate {
    #[ts(type = "number")]
    pub bytes: u64,
    pub images: u32,
    pub pages: u32,
}

/// Cuánto pesará el PDF, más o menos.
#[tauri::command]
pub fn export_estimate(deck: Deck) -> AppResult<ExportEstimate> {
    let p = plan(&deck)?;
    let enc = encargo(&deck, &p, None)?;
    let s: &PrintSettings = &deck.settings;
    let por_imagen = bytes_por_imagen(
        s.card_size,
        s.quality.dpi(),
        s.bleed_mm,
        s.encoding.format,
        s.encoding.jpeg_quality,
    );
    Ok(ExportEstimate {
        bytes: por_imagen * enc.especificaciones.len() as u64,
        images: enc.especificaciones.len() as u32,
        pages: p.pages.len() as u32,
    })
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CalibrationTextsInput {
    pub title: String,
    pub instructions: Vec<String>,
}

/// Escribe la hoja de calibración dúplex con los ajustes actuales.
#[tauri::command]
pub fn export_calibration(
    settings: PrintSettings,
    path: PathBuf,
    texts: CalibrationTextsInput,
) -> AppResult<String> {
    let pdf = calibration(
        &settings,
        &CalibrationTexts {
            title: texts.title,
            instructions: texts.instructions,
        },
    )?;
    escribir_atomico(&path, &pdf)?;
    Ok(path.to_string_lossy().to_string())
}

/// Solo se abren ficheros que exporta la app (PDF y paquetes) y que existen.
fn exportado(path: &str) -> AppResult<PathBuf> {
    let p = PathBuf::from(path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if !matches!(ext.as_deref(), Some("pdf" | "pbdeck")) || !p.is_file() {
        return Err(AppError::new(ErrorCode::InvalidInput));
    }
    Ok(p)
}

/// Abre un PDF exportado con el visor del sistema.
#[tauri::command]
pub fn export_open(app: AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let p = exportado(&path)?;
    app.opener()
        .open_path(p.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::new(ErrorCode::Io).with("detalle", e))
}

/// Enseña un fichero exportado en su carpeta.
#[tauri::command]
pub fn export_reveal(app: AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let p = exportado(&path)?;
    app.opener()
        .reveal_item_in_dir(p)
        .map_err(|e| AppError::new(ErrorCode::Io).with("detalle", e))
}

#[tauri::command]
pub fn export_cancel(state: State<'_, AppState>, job_id: u32) -> bool {
    state.jobs.cancelar(job_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, Paths};
    use crate::store::blobs::tests::png;
    use proxy_core::model::*;

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    fn deck(s: &AppState, n: usize, ajustes: PrintSettings) -> Deck {
        let img: FaceImage = s
            .blobs
            .put_bytes(&s.db, &png(63, 88, 9), ImageOrigin::Upload)
            .unwrap();
        let reverso = s.blobs.generic_back(&s.db).unwrap();
        Deck {
            id: "d".into(),
            name: "Prueba".into(),
            decklist_text: String::new(),
            cards: (0..n)
                .map(|i| DeckCard {
                    id: format!("c{i}"),
                    source: CardSource::Custom {
                        scryfall_id: None,
                        externa: None,
                    },
                    front: Some(img.clone()),
                    back: BackSpec::DeckDefault,
                    edits: CardEdits::default(),
                    locked: false,
                    meta: CardMeta::default(),
                    design: None,
                })
                .collect(),
            settings: ajustes,
            default_back: Some(reverso),
            schema_version: DECK_SCHEMA_VERSION,
        }
    }

    #[test]
    fn exportar_veinte_cartas_en_duplex() {
        let (dir, s) = estado();
        let d = deck(
            &s,
            20,
            PrintSettings {
                duplex: Duplex::LongEdge,
                quality: Quality::Dpi300,
                ..Default::default()
            },
        );
        let destino = dir.path().join("salida.pdf");
        let pasos = std::sync::Mutex::new(Vec::new());
        let r = exportar(&s, &d, &destino, &AtomicBool::new(false), &|f, a, b| {
            pasos.lock().unwrap().push((f, a, b))
        })
        .unwrap();
        assert_eq!(r.pages, 6);
        assert_eq!(r.sheets, 3);
        assert!(destino.exists());
        let doc = lopdf::Document::load(&destino).unwrap();
        assert_eq!(doc.get_pages().len(), 6);
        let pasos = pasos.into_inner().unwrap();
        let prep: Vec<_> = pasos
            .iter()
            .filter(|p| p.0 == ExportPhase::Prepare)
            .collect();
        assert_eq!(
            prep.last().map(|p| (p.1, p.2)),
            Some((2, 2)),
            "frente y reverso: dos imágenes distintas"
        );
        // Lo mismo dos veces, mismos bytes.
        let otro = dir.path().join("otra.pdf");
        exportar(&s, &d, &otro, &AtomicBool::new(false), &|_, _, _| {}).unwrap();
        assert_eq!(
            std::fs::read(&destino).unwrap(),
            std::fs::read(&otro).unwrap()
        );
    }

    /// Tres cartas diseñadas: el PDF las pinta desde el diseño, a la medida
    /// exacta del corte y a los PPP pedidos, y siempre igual.
    #[test]
    fn exportar_cartas_disenadas() {
        use proxy_core::diseno::modelo::{ByLang, CardData, FaceData};
        let (dir, s) = estado();
        for (ppp, calidad, ancho) in [
            (600, Quality::Dpi600, 1488i64),
            (900, Quality::Dpi900, 2232),
        ] {
            let mut d = deck(
                &s,
                3,
                PrintSettings {
                    quality: calidad,
                    ..Default::default()
                },
            );
            for (i, c) in d.cards.iter_mut().enumerate() {
                let datos = CardData {
                    scryfall_id: format!("x{i}"),
                    layout: "normal".into(),
                    rarity: "rare".into(),
                    set: "tst".into(),
                    collector_number: "1".into(),
                    faces: vec![FaceData {
                        name: format!("Carta {i}"),
                        mana_cost: "{2}".into(),
                        type_line: ByLang {
                            es: "Artefacto".into(),
                            en: "Artifact".into(),
                        },
                        rules: ByLang {
                            es: format!("Carta {i} hace cosas."),
                            en: String::new(),
                        },
                        ..FaceData::default()
                    }],
                    ..CardData::default()
                };
                c.design = Some(crate::diseno::borrador::montar(c, datos, None, &[]).design);
            }
            let destino = dir.path().join(format!("disenadas-{ppp}.pdf"));
            let n = std::sync::Mutex::new(0);
            exportar(&s, &d, &destino, &AtomicBool::new(false), &|f, _, b| {
                if f == ExportPhase::Prepare {
                    *n.lock().unwrap() = b;
                }
            })
            .unwrap();
            assert_eq!(*n.lock().unwrap(), 3, "una imagen por diseño");
            let doc = lopdf::Document::load(&destino).unwrap();
            let anchos: Vec<i64> = doc
                .objects
                .values()
                .filter_map(|o| o.as_stream().ok())
                .filter(|st| {
                    st.dict.get(b"Subtype").and_then(|v| v.as_name()).ok() == Some(&b"Image"[..])
                })
                .filter_map(|st| st.dict.get(b"Width").and_then(|v| v.as_i64()).ok())
                .collect();
            assert!(anchos.contains(&ancho), "{ppp} PPP: {anchos:?}");
            let otro = dir.path().join(format!("otra-{ppp}.pdf"));
            exportar(&s, &d, &otro, &AtomicBool::new(false), &|_, _, _| {}).unwrap();
            assert_eq!(
                std::fs::read(&destino).unwrap(),
                std::fs::read(&otro).unwrap()
            );
        }
    }

    /// Una transformable diseñada en dúplex: la casilla de delante pinta la
    /// cara 0 y la de detrás (en la hoja de dorsos) la cara 1.
    #[test]
    fn una_dfc_disenada_sale_con_cada_cara_en_su_sitio() {
        use proxy_core::diseno::modelo::{CardData, DesignLang};
        use proxy_core::layout::SlotFace;
        let (dir, s) = estado();
        let mut d = deck(
            &s,
            1,
            PrintSettings {
                duplex: Duplex::LongEdge,
                quality: Quality::Dpi300,
                ..Default::default()
            },
        );
        let datos: CardData = serde_json::from_str(include_str!(
            "../../crates/proxy-core/tests/dorados/layouts/datos/transform.json"
        ))
        .unwrap();
        let c = d.cards[0].clone();
        let diseno = crate::diseno::borrador::montar(&c, datos, None, &[]).design;
        d.cards[0] = crate::diseno::vista::aplicar(&s, &c, &diseno, DesignLang::Es).unwrap();
        assert!(matches!(d.cards[0].back, BackSpec::Dfc { .. }));
        let p = plan(&d).unwrap();
        let disenos = DisenosDelDeck::de(&s, &d).unwrap();
        let enc = encargo(&d, &p, Some((&s, &disenos))).unwrap();
        let mut caras = Vec::new();
        for (i, pagina) in p.pages.iter().enumerate() {
            for (j, slot) in pagina.slots.iter().enumerate() {
                let Some(k) = enc.por_casilla.get(&(i, j)) else {
                    continue;
                };
                let cara = enc.especificaciones[k].diseno.as_ref().map(|r| r.cara);
                caras.push((slot.face, cara));
            }
        }
        assert!(caras.contains(&(SlotFace::Front, Some(0))), "{caras:?}");
        assert!(
            caras
                .iter()
                .any(|(f, c)| *f != SlotFace::Front && *c == Some(1)),
            "{caras:?}"
        );
        let destino = dir.path().join("dfc.pdf");
        let r = exportar(&s, &d, &destino, &AtomicBool::new(false), &|_, _, _| {}).unwrap();
        assert_eq!(r.pages, 2);
    }

    /// Un plano diseñado va en su hoja al final y sale a 88,9 × 127 mm:
    /// a 300 PPP, una imagen de 1050 × 1500.
    #[test]
    fn un_plano_disenado_sale_grande_en_su_hoja() {
        use proxy_core::diseno::modelo::{CardData, DesignLang};
        let (dir, s) = estado();
        let mut d = deck(
            &s,
            2,
            PrintSettings {
                quality: Quality::Dpi300,
                ..Default::default()
            },
        );
        let datos: CardData = serde_json::from_str(include_str!(
            "../../crates/proxy-core/tests/dorados/layouts/datos/plano.json"
        ))
        .unwrap();
        let c = d.cards[0].clone();
        let diseno = crate::diseno::borrador::montar(&c, datos, None, &[]).design;
        d.cards[0] = crate::diseno::vista::aplicar(&s, &c, &diseno, DesignLang::Es).unwrap();
        let p = plan(&d).unwrap();
        assert_eq!(p.pages.len(), 2, "una hoja normal y una de planos");
        assert!(p.pages[1].big && p.pages[1].slots[0].card_id == d.cards[0].id);
        let destino = dir.path().join("plano.pdf");
        exportar(&s, &d, &destino, &AtomicBool::new(false), &|_, _, _| {}).unwrap();
        let doc = lopdf::Document::load(&destino).unwrap();
        let medidas: Vec<(i64, i64)> = doc
            .objects
            .values()
            .filter_map(|o| o.as_stream().ok())
            .filter(|st| {
                st.dict.get(b"Subtype").and_then(|v| v.as_name()).ok() == Some(&b"Image"[..])
            })
            .filter_map(|st| {
                Some((
                    st.dict.get(b"Width").and_then(|v| v.as_i64()).ok()?,
                    st.dict.get(b"Height").and_then(|v| v.as_i64()).ok()?,
                ))
            })
            .collect();
        assert!(medidas.contains(&(1050, 1500)), "{medidas:?}");
    }

    #[test]
    fn cancelar_no_toca_el_destino() {
        let (dir, s) = estado();
        let d = deck(
            &s,
            4,
            PrintSettings {
                quality: Quality::Dpi300,
                ..Default::default()
            },
        );
        let destino = dir.path().join("salida.pdf");
        std::fs::write(&destino, b"anterior").unwrap();
        let e = exportar(&s, &d, &destino, &AtomicBool::new(true), &|_, _, _| {}).unwrap_err();
        assert_eq!(e.code, ErrorCode::Cancelled);
        assert_eq!(std::fs::read(&destino).unwrap(), b"anterior");
    }

    #[test]
    fn partir_cada_dos_hojas() {
        let (dir, s) = estado();
        let d = deck(
            &s,
            30,
            PrintSettings {
                split_every: Some(2),
                quality: Quality::Dpi300,
                ..Default::default()
            },
        );
        let r = exportar(
            &s,
            &d,
            &dir.path().join("mazo.pdf"),
            &AtomicBool::new(false),
            &|_, _, _| {},
        )
        .unwrap();
        assert_eq!(r.paths.len(), 2);
        assert!(r.paths[0].ends_with("mazo (1).pdf"));
        assert_eq!(
            lopdf::Document::load(&r.paths[1])
                .unwrap()
                .get_pages()
                .len(),
            2
        );
    }

    #[test]
    fn la_estimacion_cuenta_imagenes_distintas() {
        let (_dir, s) = estado();
        let d = deck(&s, 9, PrintSettings::default());
        let e = export_estimate(d).unwrap();
        assert_eq!(e.images, 1);
        assert!(e.bytes > 500_000);
    }

    #[test]
    fn la_hoja_de_calibracion_se_escribe() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("cal.pdf");
        export_calibration(
            PrintSettings::default(),
            ruta.clone(),
            CalibrationTextsInput {
                title: "c".into(),
                instructions: vec!["x".into()],
            },
        )
        .unwrap();
        assert_eq!(lopdf::Document::load(&ruta).unwrap().get_pages().len(), 2);
    }

    /// De punta a punta con Scryfall de verdad: resolver una lista y exportarla.
    /// `cargo test --release -- --ignored de_punta_a_punta --nocapture`
    /// (deja el PDF en `$PB_PDF_DE_PRUEBA` o en el temporal del sistema).
    #[tokio::test(flavor = "multi_thread")]
    #[ignore]
    async fn de_punta_a_punta() {
        use crate::scryfall::resolve::{resolve, Guardar, ImagePolicy, ResolveOptions};
        struct G<'a>(&'a AppState);
        impl Guardar for G<'_> {
            fn guardar(&self, b: &[u8], o: ImageOrigin) -> AppResult<FaceImage> {
                self.0.blobs.put_bytes(&self.0.db, b, o)
            }
        }
        let (_dir, s) = estado();
        let lista = proxy_core::decklist::parse(
            "1 Sol Ring (CMM) 410\n2 Serra Ascendant\n1 The One Ring\n1 Delver of Secrets\n1 Lightning Bolt\n1 Counterspell\n1 Llanowar Elves\n1 Swords to Plowshares\n1 Dark Ritual\n1 Giant Growth",
        );
        let r = resolve(
            s.scryfall.as_ref(),
            &G(&s),
            &lista,
            &ResolveOptions {
                lang: "en".into(),
                image_policy: ImagePolicy::WithImage,
                add_tokens: false,
            },
            &AtomicBool::new(false),
            |_| {},
        )
        .await
        .unwrap();
        let reverso = s.blobs.generic_back(&s.db).unwrap();
        let mut ajustes = PrintSettings {
            duplex: Duplex::LongEdge,
            ..Default::default()
        };
        ajustes.cut_guides.enabled = true;
        let d = Deck {
            id: "e2e".into(),
            name: "De punta a punta".into(),
            decklist_text: String::new(),
            cards: r.cards,
            settings: ajustes,
            default_back: Some(reverso),
            schema_version: DECK_SCHEMA_VERSION,
        };
        let destino = std::env::var("PB_PDF_DE_PRUEBA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::temp_dir().join("proxy-builder-e2e.pdf"));
        let t = std::time::Instant::now();
        let res = exportar(&s, &d, &destino, &AtomicBool::new(false), &|_, _, _| {}).unwrap();
        println!("{res:?} en {:?}", t.elapsed());
    }
}
