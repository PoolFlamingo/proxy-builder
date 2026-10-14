//! **Renders**: de un blob y unos ajustes a la imagen procesada, con caché en
//! disco por clave. Lo usan el protocolo de miniaturas y la exportación.

use std::path::PathBuf;

use proxy_core::imaging::pipeline::{edge_for, render, RenderSpec, Rendered, SpecTarget};
use proxy_core::imaging::ImagingError;
use proxy_core::layout::{Slot, SlotFace};
use proxy_core::model::{
    BleedMode, CardEdits, CardSize, EdgeDarkening, FaceImage, ImageFormat, PrintSettings,
};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

impl From<ImagingError> for AppError {
    fn from(e: ImagingError) -> Self {
        match e {
            ImagingError::Unsupported => AppError::new(ErrorCode::Unsupported),
            ImagingError::TooLarge => AppError::new(ErrorCode::Image).with("motivo", "too_large"),
            otro => AppError::new(ErrorCode::Image).with("detalle", otro),
        }
    }
}

/// Lo que la pantalla manda en la URL de una miniatura (base64url de este JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreviewParams {
    pub edits: CardEdits,
    /// Los ajustes de oscurecido del deck; Rust decide si aplican a esta
    /// imagen (por su origen y por ser dorso), igual que al exportar.
    pub edge: EdgeDarkening,
    pub back: bool,
    pub bleed_mm: f64,
    pub bleed_mode: BleedMode,
    #[serde(default)]
    pub fill_corners: bool,
    pub card: CardSize,
}

/// La especificación de render de una casilla del plan, para imprimir.
pub fn spec_de_casilla(
    slot: &Slot,
    img: &FaceImage,
    edits: &CardEdits,
    s: &PrintSettings,
) -> RenderSpec {
    let es_dorso = slot.face == SlotFace::Back;
    RenderSpec {
        blob: img.blob.clone(),
        origin: img.origin,
        edits: if slot.apply_edits {
            *edits
        } else {
            CardEdits::default()
        },
        edge: edge_for(&s.edge_darkening, img.origin, es_dorso),
        card: s.card_size,
        bleed_mm: slot.bleed_full_mm,
        bleed_mode: s.bleed_mode,
        fill_corners: s.fill_corners,
        target: SpecTarget::Dpi(s.quality.dpi()),
        format: s.encoding.format,
        jpeg_quality: s.encoding.jpeg_quality,
        diseno: None,
    }
}

pub fn spec_de_miniatura(blob: &str, img: &FaceImage, p: &PreviewParams, ancho: u32) -> RenderSpec {
    RenderSpec {
        blob: blob.to_string(),
        origin: img.origin,
        edits: p.edits,
        edge: edge_for(&p.edge, img.origin, p.back),
        card: p.card,
        bleed_mm: p.bleed_mm,
        bleed_mode: p.bleed_mode,
        fill_corners: p.fill_corners,
        target: SpecTarget::Width(ancho),
        format: ImageFormat::Jpeg,
        jpeg_quality: 85,
        diseno: None,
    }
}

impl AppState {
    pub fn carpeta_renders(&self) -> PathBuf {
        self.paths.cache.join("renders")
    }

    /// Bytes de un blob del almacén.
    pub fn leer_blob(&self, hash: &str) -> AppResult<(Vec<u8>, FaceImage)> {
        let (ruta, _) = self
            .blobs
            .locate(&self.db, hash)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("blob", hash))?;
        let bytes = std::fs::read(&ruta)?;
        let probe = proxy_core::imaging::probe::probe(&bytes)?;
        let origen: String =
            self.db
                .conn()?
                .query_row("SELECT origin FROM blobs WHERE hash = ?1", [hash], |r| {
                    r.get(0)
                })?;
        let origin = serde_json::from_value(serde_json::Value::String(origen))
            .unwrap_or(proxy_core::model::ImageOrigin::Upload);
        Ok((
            bytes,
            FaceImage {
                blob: hash.to_string(),
                width: probe.width,
                height: probe.height,
                origin,
            },
        ))
    }

    /// Render de una miniatura, de la caché si ya existe.
    pub fn miniatura(&self, blob: &str, p: &PreviewParams, ancho: u32) -> AppResult<Vec<u8>> {
        let (bytes, img) = self.leer_blob(blob)?;
        let spec = spec_de_miniatura(blob, &img, p, ancho);
        let ruta = self.carpeta_renders().join(format!("{}.jpg", spec.key()));
        if let Ok(b) = std::fs::read(&ruta) {
            return Ok(b);
        }
        let r = render(&bytes, &spec)?;
        let out = r.bytes().to_vec();
        let _ = crate::store::blobs::escribir_atomico(&ruta, &out);
        Ok(out)
    }
}

/// Render para imprimir (sin caché en disco: pesa demasiado).
pub fn render_para_pdf(state: &AppState, spec: &RenderSpec) -> AppResult<Rendered> {
    let (bytes, _) = state.leer_blob(&spec.blob)?;
    Ok(render(&bytes, spec)?)
}
