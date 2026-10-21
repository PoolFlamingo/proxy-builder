//! **Guardar una carta suelta como imagen** (clic derecho en la casilla):
//! la misma imagen que iría al PDF —con los retoques, el oscurecido y la
//! carta diseñada pintada por el motor—, a los PPP de la calidad del PDF y
//! sin sangrado, en el formato que diga la extensión del fichero.

use std::path::PathBuf;

use proxy_core::diseno::componer::medida;
use proxy_core::diseno::render::Salida;
use proxy_core::imaging::encode::{en_formato, FormatoImagen};
use proxy_core::imaging::pipeline::{edge_for, render_rgba, RenderSpec, SpecTarget};
use proxy_core::model::{BackSpec, DeckCard, FaceImage, ImageFormat, PrintSettings};
use tauri::{AppHandle, Manager};

use crate::diseno::pintar::{colecciones_por_id, efectivo_de, pintar};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;

/// La ruta con su formato; sin extensión, PNG.
fn destino_y_formato(ruta: &str) -> AppResult<(PathBuf, FormatoImagen)> {
    let ruta = PathBuf::from(ruta);
    match ruta.extension().and_then(|e| e.to_str()) {
        None => Ok((ruta.with_extension("png"), FormatoImagen::Png)),
        Some(e) => {
            let f = FormatoImagen::de_extension(e)
                .ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "formato"))?;
            Ok((ruta, f))
        }
    }
}

/// Qué cara del diseño es esta cara de la casilla, si la carta está diseñada.
fn cara_diseno(card: &DeckCard, dorso: bool) -> Option<u32> {
    let d = card.design.as_ref()?;
    if !dorso {
        return Some(0);
    }
    match &card.back {
        BackSpec::Dfc { .. } if d.faces.len() >= 2 => Some(1),
        _ => None,
    }
}

/// La imagen del blob de esa cara: el frente, el dorso propio, el de la
/// otra cara o el del deck.
fn imagen_de_cara(card: &DeckCard, dorso: bool, del_deck: Option<&FaceImage>) -> Option<FaceImage> {
    if !dorso {
        return card.front.clone();
    }
    match &card.back {
        BackSpec::Dfc { image } | BackSpec::Custom { image } => Some(image.clone()),
        BackSpec::DeckDefault => del_deck.cloned(),
    }
}

pub fn guardar(
    state: &AppState,
    card: &DeckCard,
    dorso: bool,
    s: &PrintSettings,
    del_deck: Option<&FaceImage>,
    ruta: &str,
) -> AppResult<PathBuf> {
    let (destino, formato) = destino_y_formato(ruta)?;
    let ppp = s.quality.dpi();
    let imagen = match (&card.design, cara_diseno(card, dorso)) {
        (Some(d), Some(cara)) => {
            let e = efectivo_de(d, &colecciones_por_id(state)?);
            let (ancho_mm, _) = medida(&e, s.card_size);
            let ancho = (ancho_mm / 25.4 * ppp).round() as u32;
            pintar(
                state,
                &e,
                cara,
                s.design_lang,
                s.card_size,
                ancho,
                Salida::Impresion,
            )?
            .imagen
        }
        _ => {
            let img = imagen_de_cara(card, dorso, del_deck)
                .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_imagen"))?;
            let es_dorso = dorso && !matches!(card.back, BackSpec::Dfc { .. });
            let spec = RenderSpec {
                blob: img.blob.clone(),
                origin: img.origin,
                // Como en la hoja: los retoques son del frente.
                edits: if dorso {
                    Default::default()
                } else {
                    card.edits
                },
                edge: edge_for(&s.edge_darkening, img.origin, es_dorso),
                card: s.card_size,
                bleed_mm: 0.0,
                bleed_mode: s.bleed_mode,
                fill_corners: s.fill_corners,
                target: SpecTarget::Dpi(ppp),
                format: ImageFormat::Png,
                jpeg_quality: s.encoding.jpeg_quality,
                diseno: None,
            };
            let (bytes, _) = state.leer_blob(&img.blob)?;
            render_rgba(&bytes, &spec)?
        }
    };
    let bytes = en_formato(&imagen, formato, s.encoding.jpeg_quality)?;
    escribir_atomico(&destino, &bytes)?;
    Ok(destino)
}

/// Guarda una cara de una carta como imagen y devuelve la ruta escrita.
#[tauri::command]
pub async fn card_export_image(
    app: AppHandle,
    card: DeckCard,
    back: bool,
    settings: PrintSettings,
    deck_back: Option<FaceImage>,
    path: String,
) -> AppResult<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        guardar(&state, &card, back, &settings, deck_back.as_ref(), &path)
            .map(|p: PathBuf| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;
    use crate::store::blobs::tests::png;
    use proxy_core::model::{CardEdits, CardMeta, CardSource, ImageOrigin};

    #[test]
    fn la_extension_manda() {
        assert_eq!(
            destino_y_formato("/x/carta.WEBP").unwrap().1,
            FormatoImagen::Webp
        );
        let (p, f) = destino_y_formato("/x/carta").unwrap();
        assert_eq!((p, f), (PathBuf::from("/x/carta.png"), FormatoImagen::Png));
        assert!(destino_y_formato("/x/carta.gif").is_err());
    }

    #[test]
    fn guarda_una_carta_con_imagen_a_los_ppp_del_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let img = s
            .blobs
            .put_bytes(&s.db, &png(63, 88, 3), ImageOrigin::Upload)
            .unwrap();
        let card = DeckCard {
            id: "c".into(),
            source: CardSource::Blank,
            front: Some(img),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
            design: None,
            marca: None,
        };
        let ajustes = PrintSettings {
            quality: proxy_core::model::Quality::Dpi300,
            ..PrintSettings::default()
        };
        for ext in ["png", "jpg", "webp", "tiff", "bmp"] {
            let ruta = dir.path().join(format!("carta.{ext}"));
            let fuera = guardar(&s, &card, false, &ajustes, None, ruta.to_str().unwrap()).unwrap();
            let leida = image::open(&fuera).unwrap();
            // 63 × 88 mm a 300 PPP.
            assert_eq!((leida.width(), leida.height()), (744, 1039), "{ext}");
        }
        // El dorso sin imagen del deck no existe.
        let r = guardar(
            &s,
            &card,
            true,
            &ajustes,
            None,
            dir.path().join("d.png").to_str().unwrap(),
        );
        assert!(r.is_err());
    }
}
