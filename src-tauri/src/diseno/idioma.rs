//! **Completar un idioma de diseño** en las cartas ya diseñadas de un deck:
//! al elegir, por ejemplo, el francés, cada diseño al que le falta se baja
//! su impresión francesa (nombre, tipo y reglas oficiales) y se le ponen los
//! textos en francés; luego se rehace su miniatura. En japonés, coreano y
//! chino, antes se bajan sus letras (`letras_cjk`). Sin red, las cartas se
//! quedan con el inglés de respaldo y el aviso; se puede volver a lanzar.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::diseno::modelo::{CardDesign, DesignLang};
use proxy_core::model::DeckCard;
use tauri::{AppHandle, Emitter, Manager};

use super::lote::DesignProgressEvent;
use super::{borrador, letras_cjk, vista};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

/// Si al diseño le falta algo de `lang` (los datos de Scryfall o sus textos).
pub fn le_falta(d: &CardDesign, lang: DesignLang) -> bool {
    !d.card.tiene_idioma(lang) || d.faces.iter().any(|f| !f.texts.tiene(lang))
}

/// El diseño con `lang` completo.
async fn completar(estado: &AppState, d: &CardDesign, lang: DesignLang) -> AppResult<CardDesign> {
    let mut d = d.clone();
    if !d.card.tiene_idioma(lang) {
        let api = estado.scryfall.clone();
        let en = borrador::carta_completa(&estado.db, api.as_ref(), &d.card.scryfall_id).await?;
        let p = borrador::impresion(&estado.db, api.as_ref(), &en, lang).await?;
        borrador::anadir_idioma(&mut d.card, &en, lang, p.as_ref());
    }
    borrador::poner_textos(&mut d, lang);
    Ok(d)
}

/// Completa `lang` carta a carta y avisa de cada una por
/// `designlang://progress` (la carta ya con su miniatura en ese idioma).
pub async fn completar_deck(
    app: &AppHandle,
    job_id: u32,
    cartas: Vec<DeckCard>,
    lang: DesignLang,
    cancelado: &AtomicBool,
) -> AppResult<()> {
    let estado = app.state::<AppState>();
    if letras_cjk::necesita(lang) {
        letras_cjk::descargar(&estado, lang).await?;
    }
    if matches!(lang, DesignLang::Es | DesignLang::En) {
        return Ok(());
    }
    let pendientes: Vec<DeckCard> = cartas
        .into_iter()
        .filter(|c| c.design.as_ref().is_some_and(|d| le_falta(d, lang)))
        .collect();
    let total = pendientes.len() as u32;
    // Las copias de una carta comparten diseño: se completa una vez.
    let mut hechos: HashMap<String, CardDesign> = HashMap::new();
    for (i, card) in pendientes.into_iter().enumerate() {
        if cancelado.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::Cancelled));
        }
        let Some(d) = card.design.clone() else {
            continue;
        };
        let clave = serde_json::to_string(&d).unwrap_or_default();
        let nuevo = match hechos.get(&clave) {
            Some(n) => Ok(n.clone()),
            None => completar(&estado, &d, lang).await,
        };
        let resultado = match nuevo {
            Ok(n) => {
                hechos.insert(clave, n.clone());
                let app2 = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    vista::aplicar(&app2.state::<AppState>(), &card, &n, lang)
                })
                .await
                .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?
                .map(Some)
            }
            Err(e) => Err(e),
        };
        let (card, failed) = match resultado {
            Ok(c) => (c, false),
            // Sin red, o con Scryfall pidiendo calma, no se sigue.
            Err(e) if matches!(e.code, ErrorCode::Network | ErrorCode::RateLimited) => {
                return Err(e)
            }
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "no se pudo completar el idioma");
                (None, true)
            }
        };
        let _ = app.emit(
            "designlang://progress",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn falta_mientras_no_tenga_datos_ni_textos_del_idioma() {
        let mut d: CardDesign = serde_json::from_str(include_str!(
            "../../crates/proxy-core/tests/compatibilidad/diseno-2.5.9.json"
        ))
        .unwrap();
        assert!(!le_falta(&d, DesignLang::Es));
        assert!(le_falta(&d, DesignLang::Fr));
        // Sin impresión en rumano: con saberlo basta para los datos.
        d.card.poner_sin_texto(DesignLang::Ro, true);
        assert!(le_falta(&d, DesignLang::Ro));
        borrador::poner_textos(&mut d, DesignLang::Ro);
        assert!(!le_falta(&d, DesignLang::Ro));
        // El nombre temático español pasa al rumano.
        assert_eq!(
            d.faces[0].texts.get(DesignLang::Ro).name,
            "Crepúsculo de Veldora"
        );
    }
}
