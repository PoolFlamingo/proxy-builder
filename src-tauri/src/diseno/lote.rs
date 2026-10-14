//! **«Diseñar el deck»**: borradores en bloque (Fase 6, paso 6.1 del plan de
//! la 2.0). Para cada carta de Scryfall o propia vinculada **sin diseño**, un
//! borrador con la colección elegida, el arte de Scryfall de provisional y
//! su número por el orden del deck. Las copias de una carta comparten diseño.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::diseno::maquetacion::maquetacion;
use proxy_core::diseno::modelo::{CardDesign, Collection, DesignLang};
use proxy_core::model::DeckCard;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::{arte, borrador, vista};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DesignProgressEvent {
    pub job_id: u32,
    pub done: u32,
    pub total: u32,
    /// La carta ya con su borrador; `None` si se saltó o falló.
    pub card: Option<DeckCard>,
    /// Esta carta no se pudo preparar (se sigue con las demás).
    pub failed: bool,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DesignDoneEvent {
    pub job_id: u32,
    pub error: Option<AppError>,
}

/// El borrador de una carta, con el arte de Scryfall en cada cara impresa.
async fn borrador_de(
    estado: &AppState,
    card: &DeckCard,
    id: &str,
    coleccion: Option<&Collection>,
    usados: &[String],
) -> AppResult<CardDesign> {
    let api = estado.scryfall.clone();
    let en = borrador::carta_completa(&estado.db, api.as_ref(), id).await?;
    let es = borrador::impresion_es(&estado.db, api.as_ref(), &en).await?;
    let fundida = borrador::fundida(&estado.db, api.as_ref(), &en).await?;
    let datos = borrador::con_fundida(
        borrador::datos(&en, es.as_ref(), crate::store::ahora_ms()),
        fundida,
    );
    let mut d = borrador::montar(card, datos, coleccion, usados).design;
    let impresas = maquetacion(&d.card).0.caras_impresas();
    for i in 0..d.faces.len() {
        // Las mitades y la aventura comparten la ilustración de la carta;
        // la carta fundida de un meld no tiene la suya aquí.
        let cara = if i < impresas { i } else { 0 };
        if d.card.meld_arriba.is_some() && i == 1 {
            continue;
        }
        match arte::desde_scryfall(estado, id, cara).await {
            Ok(a) => {
                if let Some(f) = d.faces.get_mut(i) {
                    f.art = Some(a);
                }
            }
            Err(e) if matches!(e.code, ErrorCode::Network | ErrorCode::RateLimited) => {
                return Err(e)
            }
            Err(_) => {}
        }
    }
    Ok(d)
}

/// Hace los borradores en orden y avisa de cada carta.
pub async fn borradores(
    app: &AppHandle,
    job_id: u32,
    cartas: Vec<DeckCard>,
    mut usados: Vec<String>,
    coleccion: Option<Collection>,
    lang: DesignLang,
    cancelado: &AtomicBool,
) -> AppResult<()> {
    let estado = app.state::<AppState>();
    let total = cartas.len() as u32;
    let mut hechos: HashMap<String, CardDesign> = HashMap::new();
    for (i, card) in cartas.into_iter().enumerate() {
        if cancelado.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::Cancelled));
        }
        let id = borrador::id_scryfall(&card).map(String::from);
        let resultado: AppResult<Option<DeckCard>> = match (&card.design, id) {
            (None, Some(id)) => {
                let d = match hechos.get(&id) {
                    Some(d) => Ok(d.clone()),
                    None => borrador_de(&estado, &card, &id, coleccion.as_ref(), &usados).await,
                };
                match d {
                    Ok(d) => {
                        if let std::collections::hash_map::Entry::Vacant(v) = hechos.entry(id) {
                            usados.push(d.footer.number.clone());
                            v.insert(d.clone());
                        }
                        let app2 = app.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            vista::aplicar(&app2.state::<AppState>(), &card, &d, lang)
                        })
                        .await
                        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?
                        .map(Some)
                    }
                    Err(e) => Err(e),
                }
            }
            // Ya diseñada, o propia sin vincular: no se toca.
            _ => Ok(None),
        };
        let (card, failed) = match resultado {
            Ok(c) => (c, false),
            // Sin red, o con Scryfall pidiendo calma, no tiene sentido seguir
            // carta a carta.
            Err(e) if matches!(e.code, ErrorCode::Network | ErrorCode::RateLimited) => {
                return Err(e)
            }
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "no se pudo hacer el borrador");
                (None, true)
            }
        };
        let _ = app.emit(
            "design://progress",
            DesignProgressEvent {
                job_id,
                done: i as u32 + 1,
                total,
                card,
                failed,
            },
        );
    }
    Ok(())
}
