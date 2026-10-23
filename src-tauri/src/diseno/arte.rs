//! **Las tres fuentes de ilustración** del editor (Pablo, 14/10/2026):
//! subir/pegar/arrastrar una imagen, el `art_crop` de Scryfall de partida y
//! recortar la ilustración de una carta ya hecha del deck.

use image::RgbaImage;
use proxy_core::diseno::modelo::{ArtFraming, ArtRef, ArtSource};
use proxy_core::model::ImageOrigin;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;
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

/// Dónde se juntan las dos ilustraciones en el `art_crop` de una carta de
/// dos mitades (`split` en Scryfall: partidas, Fusionar, Habitaciones y
/// Secuelas), que las trae una al lado de la otra; `None` en las demás.
fn costura_de(c: &ScryCard) -> Option<f32> {
    use proxy_core::imaging::mitades::{COSTURA_PARTIDA, COSTURA_SECUELA};
    if c.layout != "split" {
        return None;
    }
    let secuela = c
        .keywords
        .as_ref()
        .is_some_and(|k| k.iter().any(|k| k == "Aftermath"))
        || c.card_faces.iter().any(|f| {
            f.oracle_text
                .as_deref()
                .is_some_and(|t| t.starts_with("Aftermath"))
        });
    Some(if secuela {
        COSTURA_SECUELA
    } else {
        COSTURA_PARTIDA
    })
}

/// El `art_crop` de Scryfall de una carta: el de la cara pedida en las de
/// doble cara y, en las de dos mitades, solo el trozo de la mitad pedida.
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
    let propia = de_cara(cara);
    let mitad = costura_de(&c).filter(|_| propia.is_none() && cara < 2);
    let url = propia
        .or_else(|| c.image_uris.as_ref().and_then(|u| u.art_crop.clone()))
        .or_else(|| de_cara(0))
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_arte"))?;
    let mut bytes = state.scryfall.download(&url).await?;
    if let Some(costura) = mitad {
        let img = proxy_core::imaging::decode::decode(&bytes)?;
        let trozo = proxy_core::imaging::mitades::mitad(&img, costura, cara);
        bytes = proxy_core::imaging::encode::png(&trozo)?;
    }
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

    /// Contra Scryfall: cada mitad de una partida y de una Secuela se queda
    /// con su ilustración, y una flip, con la única que tiene.
    /// `cargo test -p proxy-builder mitades_en_vivo -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn mitades_en_vivo() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let ancho = |id: &'static str, cara: usize| {
            let s = &s;
            async move { desde_scryfall(s, id, cara).await.unwrap().image.width }
        };
        // Fire // Ice: 808 px, mitad y mitad.
        let fuego = "18303862-4726-4136-814f-157aa7006579";
        for cara in [0, 1] {
            let w = ancho(fuego, cara).await;
            assert!((390..404).contains(&w), "{w}");
        }
        // Dusk // Dawn: 1024 px, la costura en la columna 628.
        let dusk = "5f2facfb-dfde-4e5b-8f6d-4fabb4c85519";
        assert!((615..628).contains(&ancho(dusk, 0).await));
        assert!((385..396).contains(&ancho(dusk, 1).await));
        // Akki Lavarunner (flip): la imagen entera en las dos.
        let akki = "6ee6cd34-c117-4d7e-97d1-8f8464bfaac8";
        assert_eq!(ancho(akki, 1).await, ancho(akki, 0).await);
    }
}
