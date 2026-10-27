//! **Las imágenes de la mesa**: la imagen final de cada cara (la misma que
//! iría al PDF), preparada una vez a `ANCHO_MESA` y guardada en
//! `app_cache_dir/mesa/` (caché: si se borra, se vuelve a preparar).
//!
//! - `Propia { sha256 }` → `mesa/<2 hex>/<sha256>.jpg`, por la huella de los
//!   bytes del JPEG (lo que viaja a los demás en la Fase E).
//! - `Scryfall { id, cara }` → `mesa/s/<id>-<cara>.jpg`: sin retoques; los
//!   demás la bajan de `cards.scryfall.io` (la `url`).
//!
//! La pantalla las pide por `pbimg://…/m/<sha256>/<ancho>` y
//! `/m/s/<id>/<cara>/<ancho>`, reducidas a los `ANCHOS` de siempre.

use std::path::{Path, PathBuf};

use image::RgbaImage;
use mesa::modelo::RefImagen;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::{es_huella, escribir_atomico};

/// El ancho al que se prepara cada cara (como el PNG de Scryfall).
pub const ANCHO_MESA: u32 = 745;
const CALIDAD: u8 = 85;

pub fn carpeta(state: &AppState) -> PathBuf {
    state.paths.cache.join("mesa")
}

pub fn ruta_propia(state: &AppState, sha: &str) -> PathBuf {
    propia_en(&carpeta(state), sha)
}

pub fn ruta_scryfall(state: &AppState, id: &str, cara: u8) -> PathBuf {
    scryfall_en(&carpeta(state), id, cara)
}

/// `ruta_propia` dentro de la carpeta de la mesa `carpeta`.
pub fn propia_en(carpeta: &Path, sha: &str) -> PathBuf {
    let dos = sha.get(..2).unwrap_or("00");
    carpeta.join(dos).join(format!("{sha}.jpg"))
}

pub fn scryfall_en(carpeta: &Path, id: &str, cara: u8) -> PathBuf {
    carpeta.join("s").join(format!("{id}-{cara}.jpg"))
}

/// Un id de Scryfall (un uuid): solo hexadecimal y guiones.
pub fn es_id_scryfall(s: &str) -> bool {
    s.len() == 36 && s.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Guarda una imagen ya pintada y devuelve su referencia.
pub fn guardar_propia(state: &AppState, img: &RgbaImage) -> AppResult<RefImagen> {
    let jpg = proxy_core::imaging::encode::jpeg(img, CALIDAD)?;
    let sha: String = Sha256::digest(&jpg)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let ruta = ruta_propia(state, &sha);
    if !ruta.exists() {
        escribir_atomico(&ruta, &jpg)?;
    }
    Ok(RefImagen::Propia {
        sha256: sha,
        ancho: img.width(),
        alto: img.height(),
    })
}

/// Guarda la imagen de una carta de Scryfall sin retoques.
pub fn guardar_scryfall(state: &AppState, id: &str, cara: u8, img: &RgbaImage) -> AppResult<()> {
    guardar_scryfall_en(&carpeta(state), id, cara, img)
}

/// Lo mismo, dentro de la carpeta de la mesa `carpeta`.
pub fn guardar_scryfall_en(carpeta: &Path, id: &str, cara: u8, img: &RgbaImage) -> AppResult<()> {
    let jpg = proxy_core::imaging::encode::jpeg(img, CALIDAD)?;
    escribir_atomico(&scryfall_en(carpeta, id, cara), &jpg)
}

/// Lo ya preparado de una cara, por su clave (lo que la hace única: la
/// huella del diseño o la del render).
fn ruta_indice(state: &AppState, clave: &str) -> PathBuf {
    carpeta(state).join("i").join(format!("{clave}.json"))
}

pub fn de_indice(state: &AppState, clave: &str) -> Option<RefImagen> {
    let txt = std::fs::read_to_string(ruta_indice(state, clave)).ok()?;
    let r: RefImagen = serde_json::from_str(&txt).ok()?;
    let existe = match &r {
        RefImagen::Propia { sha256, .. } => ruta_propia(state, sha256).exists(),
        RefImagen::Scryfall { id, cara, .. } => ruta_scryfall(state, id, *cara).exists(),
        RefImagen::Generica => true,
    };
    existe.then_some(r)
}

pub fn al_indice(state: &AppState, clave: &str, r: &RefImagen) {
    if let Ok(j) = serde_json::to_vec(r) {
        let _ = escribir_atomico(&ruta_indice(state, clave), &j);
    }
}

/// Qué imagen pide la pantalla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImagenMesa {
    Propia(String),
    Scryfall(String, u8),
}

/// La imagen pedida a ese ancho (de la caché de renders si ya se hizo).
pub fn servir(state: &AppState, img: &ImagenMesa, ancho: u32) -> AppResult<Vec<u8>> {
    let (origen, nombre) = match img {
        ImagenMesa::Propia(sha) if es_huella(sha) => (ruta_propia(state, sha), sha.clone()),
        ImagenMesa::Scryfall(id, cara) if es_id_scryfall(id) => {
            (ruta_scryfall(state, id, *cara), format!("s-{id}-{cara}"))
        }
        _ => return Err(AppError::new(ErrorCode::InvalidInput)),
    };
    if ancho >= ANCHO_MESA {
        return std::fs::read(&origen)
            .map_err(|_| AppError::new(ErrorCode::NotFound).with("imagen", nombre));
    }
    let reducida = state
        .carpeta_renders()
        .join("m")
        .join(format!("{nombre}-{ancho}.jpg"));
    if let Ok(b) = std::fs::read(&reducida) {
        return Ok(b);
    }
    let bytes = std::fs::read(&origen)
        .map_err(|_| AppError::new(ErrorCode::NotFound).with("imagen", nombre))?;
    let img = proxy_core::imaging::decode::decode(&bytes)?;
    let alto =
        ((u64::from(img.height()) * u64::from(ancho)) / u64::from(img.width().max(1))) as u32;
    let r = image::imageops::resize(
        &img,
        ancho,
        alto.max(1),
        image::imageops::FilterType::Triangle,
    );
    let out = proxy_core::imaging::encode::jpeg(&r, CALIDAD)?;
    let _ = escribir_atomico(&reducida, &out);
    Ok(out)
}
