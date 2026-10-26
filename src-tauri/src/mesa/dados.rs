//! **El aspecto de los dados y la moneda** de quien juega: color propio (o el
//! del jugador), color de los números, acabado, una imagen propia y el metal
//! de la moneda. Es solo pintura: el resultado de cada tirada lo sigue
//! decidiendo el motor. Va en `app_settings` (`mesa.dados`); la imagen se
//! prepara al importarla (un cuadrado de `LADO_IMAGEN` px, recortado del
//! centro) y se guarda en el almacén de blobs, protegida del borrado en
//! `blob_refs` (`owner_kind = 'mesa'`, `owner_id = 'dados'`, como el tapete).
//! A la pantalla llega como `data:` URL: así el lienzo de WebGL puede usarla
//! de textura sin que el esquema `pbimg:` lo «manche» (otro origen).

use base64::Engine;
use proxy_core::model::ImageOrigin;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::settings;

const CLAVE: &str = "mesa.dados";
const DUENO: &str = "dados";

/// El lado de la imagen preparada: sobra para una cara de dado.
pub const LADO_IMAGEN: u32 = 256;
/// Como el almacén de blobs: una imagen más grande ni se abre.
const LADO_MAXIMO: u32 = 12_000;

/// El acabado del dado (se pinta en pantalla; el translúcido solo en 3D).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Acabado {
    #[default]
    Liso,
    Nacarado,
    Marmol,
    Purpurina,
    Translucido,
}

/// El metal de la moneda.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Metal {
    #[default]
    Oro,
    Plata,
    Cobre,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct AspectoDados {
    /// `#rrggbb`; sin él, el color del jugador.
    pub color: Option<String>,
    /// `#rrggbb` de los números y símbolos.
    pub numeros: String,
    pub acabado: Acabado,
    /// La huella de la imagen propia ya preparada; tapa el color y el acabado.
    pub imagen: Option<String>,
    pub moneda: Metal,
}

impl Default for AspectoDados {
    fn default() -> Self {
        Self {
            color: None,
            numeros: "#ffffff".into(),
            acabado: Acabado::Liso,
            imagen: None,
            moneda: Metal::Oro,
        }
    }
}

/// El aspecto y su imagen lista para pintar.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaAspectoDados {
    pub aspecto: AspectoDados,
    /// `data:image/jpeg;base64,…`, si hay imagen propia y sigue en el almacén.
    pub imagen: Option<String>,
}

fn es_color(c: &str) -> bool {
    c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|x| x.is_ascii_hexdigit())
}

fn color_malo() -> AppError {
    AppError::new(ErrorCode::InvalidInput).with("motivo", "color")
}

/// El aspecto guardado (o el de fábrica).
pub fn leer(state: &AppState) -> AppResult<AspectoDados> {
    Ok(settings::get(&state.db, CLAVE)?
        .and_then(|j| serde_json::from_str(&j).ok())
        .unwrap_or_default())
}

/// El aspecto con su imagen para la pantalla. Si la imagen ya no está en el
/// almacén, sale sin ella (el aspecto guardado no cambia).
pub fn vista(state: &AppState) -> AppResult<VistaAspectoDados> {
    let aspecto = leer(state)?;
    let imagen = aspecto
        .imagen
        .as_deref()
        .and_then(|h| state.leer_blob(h).ok())
        .map(|(bytes, _)| {
            format!(
                "data:image/jpeg;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        });
    Ok(VistaAspectoDados { aspecto, imagen })
}

/// Guarda el aspecto (comprobando los colores y que la imagen exista) y
/// protege su imagen del borrado.
pub fn guardar(state: &AppState, a: &AspectoDados) -> AppResult<()> {
    let a = AspectoDados {
        color: a.color.as_ref().map(|c| c.to_ascii_lowercase()),
        numeros: a.numeros.to_ascii_lowercase(),
        ..a.clone()
    };
    if !es_color(&a.numeros) || a.color.as_deref().is_some_and(|c| !es_color(c)) {
        return Err(color_malo());
    }
    if let Some(h) = &a.imagen {
        state.leer_blob(h)?;
    }
    {
        // La conexión se suelta antes de `settings::set`, que la vuelve a pedir.
        let c = state.db.conn()?;
        c.execute(
            "DELETE FROM blob_refs WHERE owner_kind = 'mesa' AND owner_id = ?1",
            [DUENO],
        )?;
        if let Some(h) = &a.imagen {
            c.execute(
                "INSERT OR IGNORE INTO blob_refs (owner_kind, owner_id, blob) VALUES ('mesa', ?1, ?2)",
                (DUENO, h),
            )?;
        }
    }
    let j = serde_json::to_string(&a)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    settings::set(&state.db, CLAVE, &j)
}

/// Prepara una imagen del equipo para los dados (el cuadrado del centro a
/// `LADO_IMAGEN` px, en JPEG) y la mete en el almacén. Devuelve su huella.
pub fn importar_imagen(state: &AppState, ruta: &std::path::Path) -> AppResult<String> {
    let bytes = std::fs::read(ruta)?;
    let info = proxy_core::imaging::probe::probe(&bytes)?;
    if info.width > LADO_MAXIMO || info.height > LADO_MAXIMO {
        return Err(AppError::new(ErrorCode::Image).with("motivo", "too_large"));
    }
    let img = proxy_core::imaging::decode::decode(&bytes)?;
    let lado = img.width().min(img.height()).max(1);
    let cuadrado = image::imageops::crop_imm(
        &img,
        (img.width() - lado) / 2,
        (img.height() - lado) / 2,
        lado,
        lado,
    )
    .to_image();
    let lista = image::imageops::resize(
        &cuadrado,
        LADO_IMAGEN,
        LADO_IMAGEN,
        image::imageops::FilterType::Triangle,
    );
    let jpg = proxy_core::imaging::encode::jpeg(&lista, 90)?;
    Ok(state
        .blobs
        .put_bytes(&state.db, &jpg, ImageOrigin::Upload)?
        .blob)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    fn refs(s: &AppState) -> Vec<String> {
        let c = s.db.conn().unwrap();
        let mut q = c
            .prepare("SELECT blob FROM blob_refs WHERE owner_kind = 'mesa' AND owner_id = 'dados'")
            .unwrap();
        let v = q
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        v
    }

    #[test]
    fn sin_guardar_sale_el_de_fabrica() {
        let (_d, s) = estado();
        assert_eq!(leer(&s).unwrap(), AspectoDados::default());
        assert_eq!(vista(&s).unwrap().imagen, None);
    }

    #[test]
    fn los_colores_se_comprueban() {
        let (_d, s) = estado();
        let malo = AspectoDados {
            color: Some("rojo".into()),
            ..AspectoDados::default()
        };
        assert_eq!(
            guardar(&s, &malo).unwrap_err().code,
            ErrorCode::InvalidInput
        );
        let bueno = AspectoDados {
            color: Some("#AA00ff".into()),
            numeros: "#000000".into(),
            acabado: Acabado::Marmol,
            moneda: Metal::Cobre,
            imagen: None,
        };
        guardar(&s, &bueno).unwrap();
        let leido = leer(&s).unwrap();
        assert_eq!(leido.color.as_deref(), Some("#aa00ff"));
        assert_eq!(leido.acabado, Acabado::Marmol);
    }

    #[test]
    fn la_imagen_se_prepara_cuadrada_y_queda_protegida() {
        let (dir, s) = estado();
        let ruta = dir.path().join("foto.png");
        image::RgbaImage::from_pixel(600, 300, image::Rgba([200, 30, 30, 255]))
            .save(&ruta)
            .unwrap();
        let h = importar_imagen(&s, &ruta).unwrap();
        let (bytes, _) = s.leer_blob(&h).unwrap();
        let img = proxy_core::imaging::decode::decode(&bytes).unwrap();
        assert_eq!((img.width(), img.height()), (LADO_IMAGEN, LADO_IMAGEN));

        let a = AspectoDados {
            imagen: Some(h.clone()),
            ..AspectoDados::default()
        };
        guardar(&s, &a).unwrap();
        assert_eq!(refs(&s), vec![h]);
        let v = vista(&s).unwrap();
        assert!(v.imagen.unwrap().starts_with("data:image/jpeg;base64,"));

        // Sin imagen, deja de protegerla.
        guardar(&s, &AspectoDados::default()).unwrap();
        assert!(refs(&s).is_empty());
    }

    #[test]
    fn una_imagen_que_no_esta_no_se_guarda() {
        let (_d, s) = estado();
        let a = AspectoDados {
            imagen: Some("0".repeat(64)),
            ..AspectoDados::default()
        };
        assert!(guardar(&s, &a).is_err());
    }
}
