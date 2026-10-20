//! **Las tres fuentes de ilustración** del editor (Pablo, 14/10/2026):
//! subir/pegar/arrastrar una imagen, el `art_crop` de Scryfall de partida y
//! recortar la ilustración de una carta ya hecha del deck.

use image::RgbaImage;
use proxy_core::diseno::modelo::{ArtFraming, ArtRef, ArtSource};
use proxy_core::model::ImageOrigin;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::state::AppState;

fn arte(image: proxy_core::model::FaceImage, source: ArtSource) -> ArtRef {
    ArtRef {
        image,
        source,
        framing: ArtFraming::default(),
    }
}

/// Una imagen pegada del portapapeles (ya leída por `portapapeles`).
pub fn desde_imagen(state: &AppState, imagen: &RgbaImage) -> AppResult<ArtRef> {
    let bytes = proxy_core::imaging::encode::png(imagen)?;
    let img = state
        .blobs
        .put_bytes(&state.db, &bytes, ImageOrigin::Upload)?;
    Ok(arte(img, ArtSource::Upload))
}

/// Un fichero ya importado al almacén (subido o arrastrado).
pub fn desde_blob(state: &AppState, blob: &str) -> AppResult<ArtRef> {
    let (_, img) = state.leer_blob(blob)?;
    Ok(arte(img, ArtSource::Upload))
}

/// El `art_crop` de Scryfall de una carta (el de la cara pedida en las de
/// doble cara).
pub async fn desde_scryfall(state: &AppState, scryfall_id: &str, cara: usize) -> AppResult<ArtRef> {
    let c =
        crate::diseno::borrador::carta_completa(&state.db, state.scryfall.as_ref(), scryfall_id)
            .await?;
    let de_cara = |i: usize| {
        c.card_faces
            .get(i)
            .and_then(|f| f.image_uris.as_ref())
            .and_then(|u| u.art_crop.clone())
    };
    let url = de_cara(cara)
        .or_else(|| c.image_uris.as_ref().and_then(|u| u.art_crop.clone()))
        .or_else(|| de_cara(0))
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_arte"))?;
    let bytes = state.scryfall.download(&url).await?;
    let img = state
        .blobs
        .put_bytes(&state.db, &bytes, ImageOrigin::Upload)?;
    Ok(arte(img, ArtSource::ScryfallArtCrop { url }))
}

/// Recorta la zona `(x, y, w, h)` (fracciones de 0 a 1) de una imagen del
/// almacén, normalmente una carta ya hecha, y la guarda como ilustración.
pub fn desde_carta(
    state: &AppState,
    blob: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
) -> AppResult<ArtRef> {
    let (bytes, _) = state.leer_blob(blob)?;
    let img = proxy_core::imaging::decode::decode(&bytes)?;
    let (iw, ih) = (f64::from(img.width()), f64::from(img.height()));
    let c = |v: f64| v.clamp(0.0, 1.0);
    let (x0, y0) = ((c(x) * iw).round() as u32, (c(y) * ih).round() as u32);
    let (x1, y1) = (
        (c(x + w) * iw).round() as u32,
        (c(y + h) * ih).round() as u32,
    );
    if x1 <= x0 + 8 || y1 <= y0 + 8 {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "recorte_vacio"));
    }
    let recorte = image::imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image();
    let png = proxy_core::imaging::encode::png(&recorte)?;
    let nueva = state
        .blobs
        .put_bytes(&state.db, &png, ImageOrigin::Upload)?;
    Ok(arte(
        nueva,
        ArtSource::CardCrop {
            blob: blob.to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, Paths};

    #[test]
    fn pegar_y_recortar() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let a = desde_imagen(&s, &RgbaImage::new(100, 140)).unwrap();
        assert_eq!((a.image.width, a.image.height), (100, 140));
        let r = desde_carta(&s, &a.image.blob, 0.1, 0.1, 0.8, 0.5).unwrap();
        assert_eq!((r.image.width, r.image.height), (80, 70));
        assert!(matches!(r.source, ArtSource::CardCrop { .. }));
        assert!(desde_carta(&s, &a.image.blob, 0.5, 0.5, 0.0, 0.0).is_err());
    }
}
