//! **El protocolo de imágenes `pbimg://`.** La vista previa pide aquí lo que
//! enseña y Rust lo procesa en ese momento (y lo guarda en la caché). Como la
//! pantalla desmonta las imágenes de las páginas que no se ven, solo se
//! procesa lo visible.
//!
//! Rutas (y ninguna más):
//! - `/r/<sha256>/<params base64url>/<ancho>` → miniatura procesada de un blob.
//! - `/t/<sha256 de URL>` → miniatura de un resultado de búsqueda de Scryfall.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::Manager;

use crate::render::PreviewParams;
use crate::scryfall::client::CardApi;
use crate::state::AppState;
use crate::store::blobs::es_huella;

pub const ESQUEMA: &str = "pbimg";
/// Anchos de miniatura que se aceptan (el del trim, sin sangrado).
pub const ANCHOS: [u32; 6] = [146, 244, 488, 672, 745, 1000];
const MAX_PARAMS: usize = 2048;

#[derive(Debug, PartialEq)]
pub enum Ruta {
    Render {
        blob: String,
        params: PreviewParams,
        ancho: u32,
    },
    Miniatura {
        huella: String,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum Fallo {
    Mala,
    NoEsta,
}

pub fn leer_ruta(ruta: &str) -> Result<Ruta, Fallo> {
    let partes: Vec<&str> = ruta.trim_start_matches('/').split('/').collect();
    match partes.as_slice() {
        ["r", blob, params, ancho] => {
            if !es_huella(blob) || params.len() > MAX_PARAMS {
                return Err(Fallo::Mala);
            }
            let ancho: u32 = ancho.parse().map_err(|_| Fallo::Mala)?;
            if !ANCHOS.contains(&ancho) {
                return Err(Fallo::Mala);
            }
            let json = URL_SAFE_NO_PAD.decode(params).map_err(|_| Fallo::Mala)?;
            let params: PreviewParams = serde_json::from_slice(&json).map_err(|_| Fallo::Mala)?;
            Ok(Ruta::Render {
                blob: (*blob).to_string(),
                params,
                ancho,
            })
        }
        ["t", huella] if es_huella(huella) => Ok(Ruta::Miniatura {
            huella: (*huella).to_string(),
        }),
        _ => Err(Fallo::NoEsta),
    }
}

fn respuesta(estado: StatusCode, tipo: &str, cuerpo: Vec<u8>) -> Response<Vec<u8>> {
    let mut r = Response::builder()
        .status(estado)
        .header(header::CONTENT_TYPE, tipo)
        .header("X-Content-Type-Options", "nosniff")
        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*");
    if estado == StatusCode::OK {
        // La URL lleva todo lo que cambia la imagen: puede quedarse en caché siempre.
        r = r.header(header::CACHE_CONTROL, "max-age=31536000, immutable");
    }
    r.body(cuerpo).unwrap_or_else(|_| Response::new(Vec::new()))
}

fn vacia(estado: StatusCode) -> Response<Vec<u8>> {
    respuesta(estado, "text/plain", Vec::new())
}

async fn atender<R: tauri::Runtime>(app: tauri::AppHandle<R>, ruta: String) -> Response<Vec<u8>> {
    let estado = app.state::<AppState>();
    match leer_ruta(&ruta) {
        Err(Fallo::Mala) => vacia(StatusCode::BAD_REQUEST),
        Err(Fallo::NoEsta) => vacia(StatusCode::NOT_FOUND),
        Ok(Ruta::Render {
            blob,
            params,
            ancho,
        }) => {
            let _permiso = estado.renders.acquire().await;
            let app2 = app.clone();
            let r = tauri::async_runtime::spawn_blocking(move || {
                app2.state::<AppState>().miniatura(&blob, &params, ancho)
            })
            .await;
            match r {
                Ok(Ok(bytes)) => respuesta(StatusCode::OK, "image/jpeg", bytes),
                Ok(Err(e)) if e.code == crate::error::ErrorCode::NotFound => {
                    vacia(StatusCode::NOT_FOUND)
                }
                _ => vacia(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Ok(Ruta::Miniatura { huella }) => {
            let ruta = estado
                .carpeta_renders()
                .join("t")
                .join(format!("{huella}.jpg"));
            if let Ok(b) = std::fs::read(&ruta) {
                return respuesta(StatusCode::OK, "image/jpeg", b);
            }
            let url = estado
                .miniaturas
                .lock()
                .ok()
                .and_then(|m| m.get(&huella).cloned());
            let Some(url) = url else {
                return vacia(StatusCode::NOT_FOUND);
            };
            match estado.scryfall.download(&url).await {
                Ok(b) => {
                    let _ = crate::store::blobs::escribir_atomico(&ruta, &b);
                    respuesta(StatusCode::OK, "image/jpeg", b)
                }
                Err(_) => vacia(StatusCode::BAD_GATEWAY),
            }
        }
    }
}

/// Lo que se registra en el `Builder`.
pub fn manejador<R: tauri::Runtime>(
    ctx: tauri::UriSchemeContext<'_, R>,
    peticion: Request<Vec<u8>>,
    respuesta: tauri::UriSchemeResponder,
) {
    let app = ctx.app_handle().clone();
    let ruta = peticion.uri().path().to_string();
    tauri::async_runtime::spawn(async move {
        respuesta.respond(atender(app, ruta).await);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use proxy_core::model::{BleedMode, CardEdits, CardSize, EdgeDarkening};

    fn params_b64() -> String {
        let p = PreviewParams {
            edits: CardEdits::default(),
            edge: EdgeDarkening::default(),
            back: false,
            bleed_mm: 1.0,
            bleed_mode: BleedMode::Clamp,
            fill_corners: false,
            card: CardSize::MTG,
        };
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&p).unwrap())
    }

    #[test]
    fn la_ruta_buena_se_entiende() {
        let h = "a".repeat(64);
        let r = leer_ruta(&format!("/r/{h}/{}/488", params_b64())).unwrap();
        assert!(matches!(r, Ruta::Render { ancho: 488, .. }));
        assert!(matches!(
            leer_ruta(&format!("/t/{h}")),
            Ok(Ruta::Miniatura { .. })
        ));
    }

    #[test]
    fn las_rutas_raras_se_rechazan() {
        let h = "a".repeat(64);
        let p = params_b64();
        for mala in [
            format!("/r/{h}/{p}/500"),
            format!("/r/{}/{p}/488", "a".repeat(63)),
            format!("/r/{}/{p}/488", "A".repeat(64)),
            format!("/r/{h}/no-es-base64!/488"),
            format!("/r/{h}/{p}/488/x"),
            format!("/r/../{p}/488"),
            format!("/t/{}", "g".repeat(64)),
            "/t/..".to_string(),
            "/home/estigio/secreto.png".to_string(),
            "/r/%2e%2e".to_string(),
        ] {
            assert!(leer_ruta(&mala).is_err(), "{mala}");
        }
        assert_eq!(
            leer_ruta(&format!("/r/{h}/{}/488", "A".repeat(3000))),
            Err(Fallo::Mala)
        );
    }
}
