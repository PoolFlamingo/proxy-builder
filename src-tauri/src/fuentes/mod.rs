//! **Fuentes de cartas aparte de Scryfall.** Hoy, la base de fichas de
//! Cockatrice (`cockatrice`): fichas que Scryfall no tiene («Kelp»).
//!
//! Todo lo que se baja de estas fuentes pasa por `Externo`, un cliente HTTP
//! con lista cerrada: solo https, solo unos sitios y rutas concretos (también
//! en cada redirección), con tope de tamaño leído por partes y sin guardar
//! nunca los bytes tal cual (las imágenes se decodifican y se vuelven a
//! codificar).

pub mod cockatrice;
pub mod mtgio;
pub mod mtgjson;

use std::time::Duration;

use proxy_core::imaging::{decode, encode};
use reqwest::redirect::Policy;
use reqwest::Url;

use proxy_core::model::{
    BackSpec, CardEdits, CardMeta, CardSource, DeckCard, FuenteCarta, ImageOrigin, RefExterna,
};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

/// Lo único que se puede bajar de fuera: sitio y principio de la ruta.
const PERMITIDOS: &[(&str, &str)] = &[
    // El catálogo de fichas de Cockatrice.
    ("raw.githubusercontent.com", "/Cockatrice/Magic-Token/"),
    // Las imágenes de las fichas que Scryfall no tiene (las que enlaza el
    // catálogo, 19/10/2026).
    (
        "raw.githubusercontent.com",
        "/SlightlyCircuitous/image-storage/",
    ),
    ("cdn.staticneo.com", "/w/mtg/"),
    ("i.imgur.com", "/"),
    // Nombres en otros idiomas (`mtgio`, `mtgjson`).
    ("api.magicthegathering.io", "/v1/cards"),
    // Los packs de marcos descargables (`diseno/descargables.rs`).
    (
        "raw.githubusercontent.com",
        "/PoolFlamingo/proxy-builder-packs/",
    ),
    ("mtgjson.com", "/api/v5/"),
    // Los temas de tweakcn y sus letras de Google Fonts (`temas.rs`).
    ("tweakcn.com", "/r/themes/"),
    ("fonts.googleapis.com", "/css"),
    ("fonts.gstatic.com", "/s/"),
    // Las imágenes de Scryfall de las cartas de los demás en la mesa en red
    // (`red/imagenes.rs`): sin pasar por la API ni por su limitador.
    ("cards.scryfall.io", "/"),
    // Los iconos de Iconify del selector (`iconos.rs`).
    ("api.iconify.design", "/"),
];

const NAVEGADOR: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

/// Tope de una imagen.
pub const MAX_IMAGEN: usize = 15 * 1024 * 1024;
/// Lado máximo de una imagen antes de decodificarla (como el almacén de blobs).
const LADO_MAXIMO: usize = 12_000;
/// Desde cuánto se pasa a JPEG un PNG (los de fuera suelen ser fotos grandes).
const PNG_GRANDE: usize = 2 * 1024 * 1024;
/// Medida a la que se dejan las imágenes grandes (la de las de Scryfall).
const ANCHO: u32 = 745;
const ALTO: u32 = 1040;

/// ¿Se puede pedir esta URL?
pub fn permitida(url: &Url) -> bool {
    url.scheme() == "https"
        && url.port().is_none()
        && url.host_str().is_some_and(|h| {
            PERMITIDOS
                .iter()
                .any(|(sitio, ruta)| h == *sitio && url.path().starts_with(ruta))
        })
}

/// La URL de una fuente, completada (las hay sin esquema: «//i.imgur.com/…»)
/// y comprobada contra la lista.
pub fn url_permitida(texto: &str) -> AppResult<Url> {
    let texto = texto.trim();
    let completa = if texto.starts_with("//") {
        format!("https:{texto}")
    } else {
        texto.to_string()
    };
    let url = Url::parse(&completa).map_err(|_| no_permitida(texto))?;
    if !permitida(&url) {
        return Err(no_permitida(texto));
    }
    Ok(url)
}

fn no_permitida(url: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput)
        .with("motivo", "host")
        .with("url", url)
}

use crate::error::red;

/// El cliente HTTP para las fuentes de fuera.
pub struct Externo {
    http: reqwest::Client,
}

impl Externo {
    pub fn nuevo() -> AppResult<Externo> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            // Cada salto se vuelve a comprobar: una redirección no saca de la lista.
            .redirect(Policy::custom(|intento| {
                if intento.previous().len() >= 3 {
                    intento.error("demasiadas redirecciones")
                } else if permitida(intento.url()) {
                    intento.follow()
                } else {
                    intento.stop()
                }
            }))
            .build()
            .map_err(red)?;
        Ok(Externo { http })
    }

    /// Los bytes de una URL permitida, como mucho `tope` (leídos por partes:
    /// lo que pase del tope corta la descarga).
    pub async fn bajar(&self, url: &str, tope: usize) -> AppResult<Vec<u8>> {
        self.bajar_con(url, tope, None).await
    }

    /// Como `bajar`, presentándose como un navegador: Google Fonts solo da
    /// woff2 a los que conoce.
    pub async fn bajar_como_navegador(&self, url: &str, tope: usize) -> AppResult<Vec<u8>> {
        self.bajar_con(url, tope, Some(NAVEGADOR)).await
    }

    async fn bajar_con(&self, url: &str, tope: usize, ua: Option<&str>) -> AppResult<Vec<u8>> {
        let url = url_permitida(url)?;
        let mut pedido = self.http.get(url.clone());
        if let Some(ua) = ua {
            pedido = pedido.header(reqwest::header::USER_AGENT, ua);
        }
        let mut r = pedido.send().await.map_err(red)?;
        if !permitida(r.url()) {
            return Err(no_permitida(r.url().as_str()));
        }
        if !r.status().is_success() {
            return Err(if r.status() == reqwest::StatusCode::NOT_FOUND {
                AppError::new(ErrorCode::NotFound)
                    .with("url", url.as_str())
                    .with("estado", r.status().as_u16())
            } else {
                crate::error::estado_http(r.status()).with("url", url.as_str())
            });
        }
        if r.content_length().is_some_and(|n| n as usize > tope) {
            return Err(grande(url.as_str()));
        }
        let mut out = Vec::new();
        while let Some(trozo) = r.chunk().await.map_err(red)? {
            if out.len() + trozo.len() > tope {
                return Err(grande(url.as_str()));
            }
            out.extend_from_slice(&trozo);
        }
        Ok(out)
    }

    /// Baja una URL permitida a un fichero, por partes (sin tenerla entera en
    /// memoria), como mucho `tope`. Devuelve su SHA-256 en hexadecimal.
    /// `progreso(bajados, total)` según llega.
    pub async fn bajar_a_fichero(
        &self,
        url: &str,
        ruta: &std::path::Path,
        tope: u64,
        mut progreso: impl FnMut(u64, Option<u64>) + Send,
    ) -> AppResult<String> {
        use sha2::{Digest, Sha256};
        use std::io::Write;
        let url = url_permitida(url)?;
        let mut r = self
            .http
            .get(url.clone())
            .timeout(Duration::from_secs(600))
            .send()
            .await
            .map_err(red)?;
        if !permitida(r.url()) {
            return Err(no_permitida(r.url().as_str()));
        }
        if !r.status().is_success() {
            return Err(crate::error::estado_http(r.status()).with("url", url.as_str()));
        }
        let total = r.content_length();
        if total.is_some_and(|n| n > tope) {
            return Err(grande(url.as_str()));
        }
        if let Some(dir) = ruta.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut f = std::io::BufWriter::new(std::fs::File::create(ruta)?);
        let mut h = Sha256::new();
        let mut bajados = 0u64;
        while let Some(trozo) = r.chunk().await.map_err(red)? {
            bajados += trozo.len() as u64;
            if bajados > tope {
                drop(f);
                let _ = std::fs::remove_file(ruta);
                return Err(grande(url.as_str()));
            }
            h.update(&trozo);
            f.write_all(&trozo)?;
            progreso(bajados, total);
        }
        f.flush()?;
        Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
    }
}

fn grande(url: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput)
        .with("motivo", "grande")
        .with("url", url)
}

/// Una imagen de fuera lista para el almacén: decodificada y vuelta a
/// codificar (nunca los bytes tal cual). JPEG al 95 % si venía en JPEG o si es
/// un PNG grande (reducido a 745 × 1040 como mucho); si no, PNG.
pub fn limpiar_imagen(bytes: &[u8]) -> AppResult<Vec<u8>> {
    let medidas = imagesize::blob_size(bytes)
        .map_err(|_| AppError::new(ErrorCode::Image).with("motivo", "no_es_imagen"))?;
    if medidas.width > LADO_MAXIMO || medidas.height > LADO_MAXIMO {
        return Err(AppError::new(ErrorCode::Image).with("motivo", "demasiado_grande"));
    }
    let img = decode::decode(bytes)?;
    let es_jpeg = bytes.starts_with(&[0xFF, 0xD8]);
    let grande = bytes.len() > PNG_GRANDE || img.height() > 1500;
    if es_jpeg || grande {
        let img = if img.width() > ANCHO || img.height() > ALTO {
            image::imageops::resize(&img, ANCHO, ALTO, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };
        Ok(encode::jpeg(&img, 95)?)
    } else {
        Ok(encode::png(&img)?)
    }
}

/// Como `limpiar_imagen`, pero guardando la proporción (un tapete apaisado
/// no se deforma) y siempre en JPEG 85: se reduce, si hace falta, para caber
/// en `ancho_max` × `alto_max`.
pub fn limpiar_imagen_con(bytes: &[u8], ancho_max: u32, alto_max: u32) -> AppResult<Vec<u8>> {
    let medidas = imagesize::blob_size(bytes)
        .map_err(|_| AppError::new(ErrorCode::Image).with("motivo", "no_es_imagen"))?;
    if medidas.width > LADO_MAXIMO || medidas.height > LADO_MAXIMO {
        return Err(AppError::new(ErrorCode::Image).with("motivo", "demasiado_grande"));
    }
    let img = decode::decode(bytes)?;
    let (w, h) = (img.width().max(1), img.height().max(1));
    let escala = (f64::from(ancho_max) / f64::from(w))
        .min(f64::from(alto_max) / f64::from(h))
        .min(1.0);
    let img = if escala < 1.0 {
        let nw = ((f64::from(w) * escala).round() as u32).max(1);
        let nh = ((f64::from(h) * escala).round() as u32).max(1);
        image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    Ok(encode::jpeg(&img, 85)?)
}

/// Una ficha de Cockatrice para las rejillas de búsqueda (miniatura por el
/// protocolo, que la baja con `Externo`).
pub fn resumen_cockatrice(
    state: &AppState,
    f: &cockatrice::FichaCockatrice,
) -> crate::commands::cards::CardSummary {
    let (set, url) = f
        .imagenes
        .first()
        .cloned()
        .map_or((String::new(), None), |(s, u)| (s, Some(u)));
    crate::commands::cards::CardSummary {
        id: f.id.clone(),
        name: f.nombre.clone(),
        printed_name: None,
        lang: "en".into(),
        set: set.to_lowercase(),
        set_name: "Cockatrice".into(),
        collector_number: String::new(),
        thumb: url
            .and_then(|u| url_permitida(&u).ok())
            .map(|u| crate::commands::cards::registrar_miniatura(state, u.as_str())),
        highres: false,
        image_status: None,
        is_dfc: false,
        type_line: Some(f.linea_de_tipo()),
        fuente: Some(FuenteCarta::Cockatrice),
    }
}

/// Los nombres en inglés de una carta escrita en otro idioma (código de
/// Scryfall), por las fuentes que no son Scryfall: el índice de MTGJSON si se
/// descargó (en Ajustes) y, si no, magicthegathering.io (en línea, lento).
/// Vacío si no se encuentra o no hay red.
pub async fn nombres_ingleses(state: &AppState, nombre: &str, idioma: &str) -> Vec<String> {
    // Primero el índice de MTGJSON, si se descargó (local y rápido).
    if let Some(v) = mtgjson::nombres_ingleses(&state.paths.cache, nombre, idioma) {
        return v;
    }
    mtgio::nombres_ingleses(&state.externo, nombre, idioma).await
}

/// Las fichas de Cockatrice para estos textos, como resultados de búsqueda:
/// las que se llaman así (`exactas`) o las que lo llevan en el nombre. Sin
/// catálogo (sin red la primera vez) no hay ninguna: no es un error.
pub async fn buscar_cockatrice(
    state: &AppState,
    textos: &[String],
    exactas: bool,
) -> Vec<crate::commands::cards::CardSummary> {
    let catalogo = match cockatrice::catalogo(&state.externo, &state.paths.cache).await {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(codigo = ?e.code, "sin el catálogo de Cockatrice");
            return Vec::new();
        }
    };
    let mut vistas = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in textos {
        let halladas = if exactas {
            catalogo.exactas(t)
        } else {
            catalogo.buscar(t)
        };
        for f in halladas {
            if vistas.insert(f.id.clone()) {
                out.push(resumen_cockatrice(state, f));
            }
        }
    }
    out
}

/// Lo que sabe de sí una carta vinculada a una ficha de Cockatrice (o
/// añadida desde allí).
pub fn meta_cockatrice(f: &cockatrice::FichaCockatrice) -> CardMeta {
    CardMeta {
        name: f.nombre.clone(),
        set: f.imagenes.first().map(|(s, _)| s.to_lowercase()),
        lang: Some("en".into()),
        layout: Some("token".into()),
        type_line: Some(f.linea_de_tipo()),
        ..CardMeta::default()
    }
}

/// Una ficha de Cockatrice como carta del deck: una propia con la imagen
/// bajada de allí y el vínculo a la ficha. Prueba sus imágenes por orden.
pub async fn carta_cockatrice(state: &AppState, id: &str) -> AppResult<DeckCard> {
    let catalogo = cockatrice::catalogo(&state.externo, &state.paths.cache).await?;
    let f = catalogo
        .por_id(id)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("ficha", id))?
        .clone();
    let mut error = AppError::new(ErrorCode::NotFound)
        .with("motivo", "sin_imagen")
        .with("ficha", &f.nombre);
    for (_, url) in &f.imagenes {
        let limpia = match state.externo.bajar(url, MAX_IMAGEN).await {
            Ok(b) => limpiar_imagen(&b),
            Err(e) => Err(e),
        };
        match limpia {
            Ok(bytes) => {
                let front = state
                    .blobs
                    .put_bytes(&state.db, &bytes, ImageOrigin::Upload)?;
                return Ok(DeckCard {
                    id: uuid::Uuid::new_v4().to_string(),
                    source: CardSource::Custom {
                        scryfall_id: None,
                        externa: Some(RefExterna {
                            fuente: FuenteCarta::Cockatrice,
                            id: f.id.clone(),
                        }),
                    },
                    front: Some(front),
                    back: BackSpec::DeckDefault,
                    edits: CardEdits::default(),
                    locked: false,
                    meta: meta_cockatrice(&f),
                    design: None,
                    marca: None,
                });
            }
            Err(e) => error = e,
        }
    }
    Err(error)
}

#[cfg(test)]
mod tests {
    #[test]
    fn limpiar_con_medida_guarda_la_proporcion() {
        let img = image::RgbaImage::from_pixel(2400, 1000, image::Rgba([10, 200, 30, 255]));
        let png = proxy_core::imaging::encode::png(&img).unwrap();
        let jpg = super::limpiar_imagen_con(&png, 2560, 1600).unwrap();
        assert!(jpg.starts_with(&[0xFF, 0xD8]));
        let m = imagesize::blob_size(&jpg).unwrap();
        assert_eq!((m.width, m.height), (2400, 1000));
        let jpg = super::limpiar_imagen_con(&png, 745, 1040).unwrap();
        let m = imagesize::blob_size(&jpg).unwrap();
        assert_eq!((m.width, m.height), (745, 310));
        assert!(super::limpiar_imagen_con(b"no soy una imagen", 745, 1040).is_err());
    }

    use super::*;

    #[test]
    fn solo_lo_de_la_lista() {
        for si in [
            "https://raw.githubusercontent.com/SlightlyCircuitous/image-storage/refs/heads/main/Kelp.jpeg",
            "https://raw.githubusercontent.com/Cockatrice/Magic-Token/master/tokens.xml",
            "https://cdn.staticneo.com/w/mtg/a/b.jpg",
            "//i.imgur.com/zvrInRn.jpg",
        ] {
            assert!(url_permitida(si).is_ok(), "{si}");
        }
        for no in [
            "http://i.imgur.com/zvrInRn.jpg",
            "https://raw.githubusercontent.com/otro/repo/x.png",
            "https://i.imgur.com.malo.com/x.png",
            "https://i.imgur.com:8443/x.png",
            "https://cdn.staticneo.com/otra/x.png",
            "file:///etc/passwd",
            "nada",
        ] {
            let e = url_permitida(no).unwrap_err();
            assert_eq!(e.code, ErrorCode::InvalidInput, "{no}");
        }
    }

    #[test]
    fn las_imagenes_se_vuelven_a_codificar() {
        let png = crate::store::blobs::tests::png(30, 42, 7);
        let limpio = limpiar_imagen(&png).unwrap();
        assert!(limpio.starts_with(&[0x89, b'P', b'N', b'G']));
        let jpg = encode::jpeg(&decode::decode(&png).unwrap(), 90).unwrap();
        let limpio = limpiar_imagen(&jpg).unwrap();
        assert!(limpio.starts_with(&[0xFF, 0xD8]));
        let e = limpiar_imagen(b"<html>no soy una imagen</html>").unwrap_err();
        assert_eq!(e.code, ErrorCode::Image);
    }
}
