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
        // La medida de la casilla: la de la carta, o la de una carta grande.
        card: CardSize {
            width_mm: slot.trim.w,
            height_mm: slot.trim.h,
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;
    use crate::store::blobs::tests::png;
    use proxy_core::layout::RectMm;
    use proxy_core::model::{ImageOrigin, Quality, Rotation};

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    fn imagen(origin: ImageOrigin) -> FaceImage {
        FaceImage {
            blob: "a".repeat(64),
            width: 10,
            height: 14,
            origin,
        }
    }

    fn casilla(face: SlotFace, apply_edits: bool) -> Slot {
        Slot {
            index: 0,
            row: 0,
            col: 0,
            card_id: "c".into(),
            face,
            image: None,
            apply_edits,
            trim: RectMm {
                x: 10.0,
                y: 10.0,
                w: 88.9,
                h: 127.0,
            },
            bleed: RectMm {
                x: 9.0,
                y: 9.0,
                w: 90.9,
                h: 129.0,
            },
            bleed_full_mm: 1.5,
        }
    }

    fn girada() -> CardEdits {
        CardEdits {
            rotation: Rotation::R90,
            ..CardEdits::default()
        }
    }

    fn oscurecer() -> EdgeDarkening {
        EdgeDarkening {
            enabled: true,
            apply_scryfall: true,
            apply_uploads: false,
            apply_backs: false,
            ..EdgeDarkening::default()
        }
    }

    // ── Errores ──

    #[test]
    fn los_errores_de_imagen_llegan_con_su_codigo() {
        assert_eq!(
            AppError::from(ImagingError::Unsupported).code,
            ErrorCode::Unsupported
        );
        let e = AppError::from(ImagingError::TooLarge);
        assert_eq!(e.code, ErrorCode::Image);
        assert_eq!(
            e.params.get("motivo").map(String::as_str),
            Some("too_large")
        );
        let e = AppError::from(ImagingError::Corrupt("x".into()));
        assert_eq!(e.code, ErrorCode::Image);
        assert!(e.params.contains_key("detalle"));
    }

    // ── Especificaciones ──

    #[test]
    fn la_casilla_manda_la_medida_el_sangrado_y_los_ppp() {
        let s = PrintSettings {
            quality: Quality::Dpi300,
            fill_corners: true,
            ..PrintSettings::default()
        };
        let img = imagen(ImageOrigin::ScryfallPng);
        let spec = spec_de_casilla(&casilla(SlotFace::Front, true), &img, &girada(), &s);
        assert_eq!(spec.blob, img.blob);
        assert_eq!(spec.origin, ImageOrigin::ScryfallPng);
        assert_eq!(spec.edits, girada());
        // Una carta grande: la medida de su casilla, no la del deck.
        assert_eq!((spec.card.width_mm, spec.card.height_mm), (88.9, 127.0));
        assert_eq!(spec.bleed_mm, 1.5);
        assert_eq!(spec.target, SpecTarget::Dpi(300.0));
        assert_eq!(spec.format, s.encoding.format);
        assert_eq!(spec.jpeg_quality, s.encoding.jpeg_quality);
        assert!(spec.fill_corners);
        assert!(spec.diseno.is_none());
    }

    #[test]
    fn sin_retoques_en_la_casilla_van_los_de_fabrica() {
        let s = PrintSettings::default();
        let img = imagen(ImageOrigin::Upload);
        let spec = spec_de_casilla(&casilla(SlotFace::Back, false), &img, &girada(), &s);
        assert_eq!(spec.edits, CardEdits::default());
    }

    #[test]
    fn el_oscurecido_depende_del_origen_y_de_si_es_dorso() {
        let s = PrintSettings {
            edge_darkening: oscurecer(),
            ..PrintSettings::default()
        };
        let png = imagen(ImageOrigin::ScryfallPng);
        let subida = imagen(ImageOrigin::Upload);
        let e = CardEdits::default();
        let frente = casilla(SlotFace::Front, true);
        let dorso = casilla(SlotFace::Back, false);
        assert!(spec_de_casilla(&frente, &png, &e, &s).edge.is_some());
        assert!(spec_de_casilla(&frente, &subida, &e, &s).edge.is_none());
        // De dorso solo si se pide para los dorsos.
        assert!(spec_de_casilla(&dorso, &png, &e, &s).edge.is_none());
        // La otra cara de una DFC como carta aparte no es un dorso.
        let cara = casilla(SlotFace::DfcFace, false);
        assert!(spec_de_casilla(&cara, &png, &e, &s).edge.is_some());
    }

    fn params() -> PreviewParams {
        PreviewParams {
            edits: girada(),
            edge: oscurecer(),
            back: false,
            bleed_mm: 2.0,
            bleed_mode: BleedMode::default(),
            fill_corners: false,
            card: CardSize::MTG,
        }
    }

    #[test]
    fn la_miniatura_va_en_jpeg_al_ancho_pedido() {
        let img = imagen(ImageOrigin::ScryfallLarge);
        let spec = spec_de_miniatura("b", &img, &params(), 240);
        assert_eq!(spec.blob, "b");
        assert_eq!(spec.target, SpecTarget::Width(240));
        assert_eq!(spec.format, ImageFormat::Jpeg);
        assert_eq!(spec.jpeg_quality, 85);
        assert_eq!(spec.edits, girada());
        assert_eq!(spec.bleed_mm, 2.0);
        assert!(spec.edge.is_some());
        // El dorso no se oscurece si no se pide.
        let p = PreviewParams {
            back: true,
            ..params()
        };
        assert!(spec_de_miniatura("b", &img, &p, 240).edge.is_none());
        // Otro ancho es otra clave de caché.
        assert_ne!(
            spec_de_miniatura("b", &img, &params(), 240).key(),
            spec_de_miniatura("b", &img, &params(), 241).key()
        );
    }

    #[test]
    fn los_parametros_de_la_url_aceptan_que_falte_rellenar_esquinas() {
        let mut v = serde_json::to_value(params()).unwrap();
        v.as_object_mut().unwrap().remove("fillCorners");
        let p: PreviewParams = serde_json::from_value(v).unwrap();
        assert!(!p.fill_corners);
    }

    // ── Almacén y caché ──

    #[test]
    fn leer_un_blob_da_sus_bytes_medidas_y_origen() {
        let (_dir, s) = estado();
        let bytes = png(5, 7, 3);
        let img = s
            .blobs
            .put_bytes(&s.db, &bytes, ImageOrigin::ScryfallPng)
            .unwrap();
        let (leidos, info) = s.leer_blob(&img.blob).unwrap();
        assert_eq!(leidos, bytes);
        assert_eq!(info, img);
        assert_eq!((info.width, info.height), (5, 7));
        assert_eq!(info.origin, ImageOrigin::ScryfallPng);
    }

    #[test]
    fn un_blob_que_no_esta_es_no_encontrado() {
        let (_dir, s) = estado();
        let e = s.leer_blob(&"f".repeat(64)).unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
        // Algo que ni siquiera es una huella tampoco.
        assert_eq!(s.leer_blob("../etc").unwrap_err().code, ErrorCode::NotFound);
    }

    #[test]
    fn la_miniatura_se_guarda_en_la_cache_y_se_sirve_de_ella() {
        let (_dir, s) = estado();
        let img = s
            .blobs
            .put_bytes(&s.db, &png(63, 88, 5), ImageOrigin::Upload)
            .unwrap();
        let p = PreviewParams {
            edits: CardEdits::default(),
            edge: EdgeDarkening::default(),
            ..params()
        };
        let a = s.miniatura(&img.blob, &p, 50).unwrap();
        assert_eq!(&a[..2], &[0xFF, 0xD8], "es un JPEG");
        let spec = spec_de_miniatura(&img.blob, &img, &p, 50);
        let ruta = s.carpeta_renders().join(format!("{}.jpg", spec.key()));
        assert_eq!(std::fs::read(&ruta).unwrap(), a);
        // La segunda vez sale del disco: si se cambia el fichero, se sirve el cambiado.
        std::fs::write(&ruta, b"de la cache").unwrap();
        assert_eq!(s.miniatura(&img.blob, &p, 50).unwrap(), b"de la cache");
        // Borrar la caché no rompe nada: se vuelve a hacer igual.
        std::fs::remove_dir_all(s.carpeta_renders()).unwrap();
        assert_eq!(s.miniatura(&img.blob, &p, 50).unwrap(), a);
    }

    #[test]
    fn el_render_para_imprimir_no_deja_nada_en_la_cache() {
        let (_dir, s) = estado();
        let img = s
            .blobs
            .put_bytes(&s.db, &png(63, 88, 5), ImageOrigin::Upload)
            .unwrap();
        let ajustes = PrintSettings {
            quality: Quality::Dpi300,
            ..PrintSettings::default()
        };
        let spec = spec_de_casilla(
            &casilla(SlotFace::Front, true),
            &img,
            &CardEdits::default(),
            &ajustes,
        );
        let r = render_para_pdf(&s, &spec).unwrap();
        assert!(!r.bytes().is_empty());
        assert!(!s.carpeta_renders().exists());
        // Un blob que no existe es un error, no un pánico.
        let otra = RenderSpec {
            blob: "e".repeat(64),
            ..spec
        };
        assert_eq!(
            render_para_pdf(&s, &otra).unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}
