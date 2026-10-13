//! **La tubería completa** y la clave que identifica cada render.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::frame::{fit, px_por_mm, trim_px, Target};
use super::probe::{probe, Kind};
use super::{bleed, corners, decode::decode, edges, encode, ImagingError};
use crate::model::{BleedMode, CardEdits, CardSize, EdgeDarkening, ImageFormat, ImageOrigin};

/// Subirla invalida todas las cachés de renders (cambió algún resultado).
pub const PIPELINE_VERSION: u32 = 2;

/// Todo lo que cambia el resultado de un render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSpec {
    pub blob: String,
    pub origin: ImageOrigin,
    pub edits: CardEdits,
    /// El oscurecido, solo si aplica a esta imagen (origen y cara).
    pub edge: Option<EdgeDarkening>,
    pub card: CardSize,
    pub bleed_mm: f64,
    pub bleed_mode: BleedMode,
    pub target: SpecTarget,
    pub format: ImageFormat,
    pub jpeg_quality: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpecTarget {
    Dpi(f64),
    Width(u32),
}

impl From<SpecTarget> for Target {
    fn from(t: SpecTarget) -> Target {
        match t {
            SpecTarget::Dpi(d) => Target::Dpi(d),
            SpecTarget::Width(w) => Target::Width(w),
        }
    }
}

impl RenderSpec {
    /// Clave estable (sha256 en hex) del render.
    pub fn key(&self) -> String {
        let json = serde_json::to_string(&(PIPELINE_VERSION, self)).unwrap_or_default();
        Sha256::digest(json.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// Resultado de un render.
#[derive(Debug, Clone, PartialEq)]
pub enum Rendered {
    /// Los bytes originales sirven tal cual (JPEG sin procesado ni sangrado).
    Passthrough { bytes: Vec<u8> },
    Encoded {
        bytes: Vec<u8>,
        format: ImageFormat,
        width: u32,
        height: u32,
    },
}

impl Rendered {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Rendered::Passthrough { bytes } | Rendered::Encoded { bytes, .. } => bytes,
        }
    }
    pub fn is_jpeg(&self) -> bool {
        matches!(self, Rendered::Passthrough { .. })
            || matches!(
                self,
                Rendered::Encoded {
                    format: ImageFormat::Jpeg,
                    ..
                }
            )
    }
}

/// ¿Se puede incrustar el original sin tocarlo?
fn passthrough(bytes: &[u8], s: &RenderSpec) -> bool {
    if s.bleed_mm > 0.0 || !s.edits.is_identity() || s.edge.is_some() {
        return false;
    }
    if !matches!(s.target, SpecTarget::Dpi(_)) {
        return false;
    }
    match probe(bytes) {
        Ok(p) if p.kind == Kind::Jpeg => {
            let aspecto = f64::from(p.width) / f64::from(p.height);
            let carta = s.card.width_mm / s.card.height_mm;
            (aspecto / carta - 1.0).abs() <= 0.005
        }
        _ => false,
    }
}

/// Procesa una imagen según la especificación.
pub fn render(bytes: &[u8], s: &RenderSpec) -> Result<Rendered, ImagingError> {
    if passthrough(bytes, s) {
        return Ok(Rendered::Passthrough {
            bytes: bytes.to_vec(),
        });
    }
    let mut img = decode(bytes)?;
    corners::fill(&mut img);
    let (tw, th) = trim_px(s.card, s.target.into());
    let mut img = fit(img, s.edits.rotation, s.edits.crop, tw, th)?;
    let px_mm = px_por_mm(s.card, tw);
    super::adjust::apply(&mut img, &s.edits.adjust, px_mm);
    if let Some(e) = &s.edge {
        edges::apply(&mut img, e);
    }
    let b = (s.bleed_mm * px_mm).round().max(0.0) as u32;
    let img = bleed::extend(&img, b, s.bleed_mode);
    let (width, height) = img.dimensions();
    let bytes = match s.format {
        ImageFormat::Jpeg => encode::jpeg(&img, s.jpeg_quality)?,
        ImageFormat::Png => encode::png(&img)?,
    };
    Ok(Rendered::Encoded {
        bytes,
        format: s.format,
        width,
        height,
    })
}

/// ¿Se oscurece esta imagen? Según el origen y si es un dorso.
pub fn edge_for(e: &EdgeDarkening, origin: ImageOrigin, es_dorso: bool) -> Option<EdgeDarkening> {
    if !e.enabled {
        return None;
    }
    let por_origen = match origin {
        ImageOrigin::ScryfallPng | ImageOrigin::ScryfallLarge => e.apply_scryfall,
        ImageOrigin::Upload => e.apply_uploads,
        ImageOrigin::Generated => false,
    };
    let ok = if es_dorso { e.apply_backs } else { por_origen };
    ok.then_some(*e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::imaging::corners::tests::carta;
    use crate::model::Adjustments;

    fn spec(bleed: f64) -> RenderSpec {
        RenderSpec {
            blob: "a".repeat(64),
            origin: ImageOrigin::Upload,
            edits: CardEdits::default(),
            edge: None,
            card: CardSize::MTG,
            bleed_mm: bleed,
            bleed_mode: BleedMode::Clamp,
            target: SpecTarget::Dpi(300.0),
            format: ImageFormat::Jpeg,
            jpeg_quality: 95,
        }
    }

    #[test]
    fn misma_especificacion_misma_clave_y_cualquier_cambio_otra() {
        let a = spec(1.0);
        assert_eq!(a.key(), spec(1.0).key());
        assert_eq!(a.key().len(), 64);
        let mut b = spec(1.0);
        b.edits.adjust = Adjustments {
            brightness: 1.0,
            ..Default::default()
        };
        assert_ne!(a.key(), b.key());
        let mut c = spec(1.0);
        c.target = SpecTarget::Dpi(600.0);
        assert_ne!(a.key(), c.key());
    }

    #[test]
    fn un_jpeg_63_88_sin_tocar_y_sin_sangrado_va_tal_cual() {
        let img = carta(630, 880, [20, 20, 20, 255], [20, 20, 20, 255]);
        let jpg = encode::jpeg(&img, 90).unwrap();
        let r = render(&jpg, &spec(0.0)).unwrap();
        assert_eq!(r, Rendered::Passthrough { bytes: jpg.clone() });
        // Con sangrado ya no.
        assert!(matches!(
            render(&jpg, &spec(1.0)).unwrap(),
            Rendered::Encoded { .. }
        ));
    }

    #[test]
    fn un_png_de_scryfall_sale_con_su_sangrado_a_la_resolucion_pedida() {
        let img = carta(745, 1040, [10, 10, 10, 255], [0, 0, 0, 0]);
        let png = encode::png(&img).unwrap();
        let r = render(&png, &spec(1.0)).unwrap();
        let Rendered::Encoded { width, height, .. } = r else {
            panic!()
        };
        // 300 PPP: 744×1039 + 12 px por lado.
        assert_eq!((width, height), (744 + 24, 1039 + 24));
    }

    #[test]
    fn el_oscurecido_depende_del_origen_y_la_cara() {
        let mut e = EdgeDarkening {
            enabled: true,
            ..Default::default()
        };
        assert!(edge_for(&e, ImageOrigin::ScryfallPng, false).is_some());
        assert!(edge_for(&e, ImageOrigin::Upload, false).is_none());
        assert!(edge_for(&e, ImageOrigin::ScryfallPng, true).is_none());
        e.apply_backs = true;
        assert!(edge_for(&e, ImageOrigin::Upload, true).is_some());
        e.enabled = false;
        assert!(edge_for(&e, ImageOrigin::ScryfallPng, false).is_none());
    }
}
