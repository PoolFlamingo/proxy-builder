//! **El protocolo de imágenes `pbimg://`.** La vista previa pide aquí lo que
//! enseña y Rust lo procesa en ese momento (y lo guarda en la caché). Como la
//! pantalla desmonta las imágenes de las páginas que no se ven, solo se
//! procesa lo visible.
//!
//! Rutas (y ninguna más):
//! - `/r/<sha256>/<params base64url>/<ancho>` → miniatura procesada de un blob.
//! - `/t/<sha256 de URL>` → miniatura de un resultado de búsqueda de Scryfall.
//! - `/d/<huella>/<ancho>` → vista previa de un diseño registrado
//!   (`design_register`).
//! - `/c/<huella>/<ancho>` → esa carta sin la ilustración (`design_overlay`),
//!   para mover la ilustración por debajo mientras se arrastra.
//! - `/b/<sha256>/<lado>` → una ilustración del almacén reducida.
//! - `/m/<sha256>/<ancho>` y `/m/s/<id de Scryfall>/<cara>/<ancho>` → una
//!   imagen de la mesa de juego (`mesa/imagenes.rs`).

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
/// Anchos de las imágenes de la mesa: los de siempre y el del tapete.
pub const ANCHOS_MESA: [u32; 7] = [146, 244, 488, 672, 745, 1000, 1920];

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
    Diseno {
        huella: String,
        ancho: u32,
    },
    Capa {
        huella: String,
        ancho: u32,
    },
    Arte {
        blob: String,
        lado: u32,
    },
    /// Una imagen extra de un pack descargable (`/x/<pack>/<n>`).
    Extra {
        pack: String,
        indice: usize,
    },
    /// Una letra de un tema de tweakcn instalado (`/f/<huella>.woff2`).
    Letra {
        huella: String,
    },
    /// Una imagen de la mesa de juego.
    Mesa {
        imagen: crate::mesa::imagenes::ImagenMesa,
        ancho: u32,
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
        ["d", huella, ancho] if es_huella(huella) => {
            let ancho: u32 = ancho.parse().map_err(|_| Fallo::Mala)?;
            if !ANCHOS.contains(&ancho) {
                return Err(Fallo::Mala);
            }
            Ok(Ruta::Diseno {
                huella: (*huella).to_string(),
                ancho,
            })
        }
        ["c", huella, ancho] if es_huella(huella) => {
            let ancho: u32 = ancho.parse().map_err(|_| Fallo::Mala)?;
            if !ANCHOS.contains(&ancho) {
                return Err(Fallo::Mala);
            }
            Ok(Ruta::Capa {
                huella: (*huella).to_string(),
                ancho,
            })
        }
        ["b", blob, lado] if es_huella(blob) => {
            let lado: u32 = lado.parse().map_err(|_| Fallo::Mala)?;
            if !crate::diseno::vista::LADOS_ARTE.contains(&lado) {
                return Err(Fallo::Mala);
            }
            Ok(Ruta::Arte {
                blob: (*blob).to_string(),
                lado,
            })
        }
        ["x", pack, n]
            if !pack.is_empty()
                && pack.len() <= 40
                && pack
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
        {
            Ok(Ruta::Extra {
                pack: (*pack).to_string(),
                indice: n.parse().map_err(|_| Fallo::Mala)?,
            })
        }
        ["m", "s", id, cara, ancho] if crate::mesa::imagenes::es_id_scryfall(id) => {
            let ancho: u32 = ancho.parse().map_err(|_| Fallo::Mala)?;
            let cara: u8 = cara.parse().map_err(|_| Fallo::Mala)?;
            if !ANCHOS_MESA.contains(&ancho) || cara > 1 {
                return Err(Fallo::Mala);
            }
            Ok(Ruta::Mesa {
                imagen: crate::mesa::imagenes::ImagenMesa::Scryfall((*id).to_string(), cara),
                ancho,
            })
        }
        ["m", sha, ancho] if es_huella(sha) => {
            let ancho: u32 = ancho.parse().map_err(|_| Fallo::Mala)?;
            if !ANCHOS_MESA.contains(&ancho) {
                return Err(Fallo::Mala);
            }
            Ok(Ruta::Mesa {
                imagen: crate::mesa::imagenes::ImagenMesa::Propia((*sha).to_string()),
                ancho,
            })
        }
        ["f", fichero] => match fichero.strip_suffix(".woff2") {
            Some(h) if es_huella(h) => Ok(Ruta::Letra {
                huella: h.to_string(),
            }),
            _ => Err(Fallo::Mala),
        },
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
        Ok(Ruta::Diseno { huella, ancho }) => {
            let _permiso = estado.renders.acquire().await;
            let app2 = app.clone();
            let r = tauri::async_runtime::spawn_blocking(move || {
                crate::diseno::vista::imagen(&app2.state::<AppState>(), &huella, ancho)
            })
            .await;
            match r {
                Ok(Ok(bytes)) => respuesta(StatusCode::OK, "image/png", bytes),
                Ok(Err(e)) if e.code == crate::error::ErrorCode::NotFound => {
                    vacia(StatusCode::NOT_FOUND)
                }
                _ => vacia(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Ok(Ruta::Capa { huella, ancho }) => {
            let _permiso = estado.renders.acquire().await;
            let app2 = app.clone();
            let r = tauri::async_runtime::spawn_blocking(move || {
                crate::diseno::vista::imagen_capa(&app2.state::<AppState>(), &huella, ancho)
            })
            .await;
            match r {
                Ok(Ok(bytes)) => respuesta(StatusCode::OK, "image/png", bytes),
                Ok(Err(e)) if e.code == crate::error::ErrorCode::NotFound => {
                    vacia(StatusCode::NOT_FOUND)
                }
                _ => vacia(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Ok(Ruta::Extra { pack, indice }) => {
            match crate::diseno::descargables::extra_por_indice(&estado, &pack, indice) {
                Ok(b) => respuesta(StatusCode::OK, "image/png", b),
                Err(_) => vacia(StatusCode::NOT_FOUND),
            }
        }
        Ok(Ruta::Mesa { imagen, ancho }) => {
            let servir = |imagen: crate::mesa::imagenes::ImagenMesa| {
                let app2 = app.clone();
                tauri::async_runtime::spawn_blocking(move || {
                    crate::mesa::imagenes::servir(&app2.state::<AppState>(), &imagen, ancho)
                })
            };
            let mut r = servir(imagen.clone()).await;
            // En red, la de otro jugador se trae ahora (primero lo que se ve).
            if matches!(&r, Ok(Err(e)) if e.code == crate::error::ErrorCode::NotFound)
                && crate::red::imagenes::asegurar(&app, &imagen).await.is_ok()
            {
                r = servir(imagen).await;
            }
            match r {
                Ok(Ok(bytes)) => respuesta(StatusCode::OK, "image/jpeg", bytes),
                Ok(Err(e)) if e.code == crate::error::ErrorCode::NotFound => {
                    vacia(StatusCode::NOT_FOUND)
                }
                _ => vacia(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Ok(Ruta::Letra { huella }) => match crate::temas::letra(&estado.paths.data, &huella) {
            Ok(b) => respuesta(StatusCode::OK, "font/woff2", b),
            Err(_) => vacia(StatusCode::NOT_FOUND),
        },
        Ok(Ruta::Arte { blob, lado }) => {
            let _permiso = estado.renders.acquire().await;
            let app2 = app.clone();
            let r = tauri::async_runtime::spawn_blocking(move || {
                crate::diseno::vista::arte_reducido(&app2.state::<AppState>(), &blob, lado)
            })
            .await;
            match r {
                Ok(Ok((bytes, tipo))) => respuesta(StatusCode::OK, tipo, bytes),
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
                let tipo = if b.starts_with(&[0x89, b'P', b'N', b'G']) {
                    "image/png"
                } else {
                    "image/jpeg"
                };
                return respuesta(StatusCode::OK, tipo, b);
            }
            let url = estado
                .miniaturas
                .lock()
                .ok()
                .and_then(|m| m.get(&huella).cloned());
            let Some(url) = url else {
                return vacia(StatusCode::NOT_FOUND);
            };
            // Las de Scryfall, por su cliente; las de otras fuentes, solo por
            // el de la lista cerrada, y vueltas a codificar.
            let bajada = if url.starts_with("https://cards.scryfall.io/") {
                estado.scryfall.download(&url).await
            } else {
                match estado.externo.bajar(&url, crate::fuentes::MAX_IMAGEN).await {
                    Ok(b) => crate::fuentes::limpiar_imagen(&b),
                    Err(e) => Err(e),
                }
            };
            match bajada {
                Ok(b) => {
                    let _ = crate::store::blobs::escribir_atomico(&ruta, &b);
                    let tipo = if b.starts_with(&[0x89, b'P', b'N', b'G']) {
                        "image/png"
                    } else {
                        "image/jpeg"
                    };
                    respuesta(StatusCode::OK, tipo, b)
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
        assert_eq!(
            leer_ruta(&format!("/d/{h}/745")),
            Ok(Ruta::Diseno {
                huella: h.clone(),
                ancho: 745
            })
        );
        assert_eq!(
            leer_ruta(&format!("/c/{h}/488")),
            Ok(Ruta::Capa {
                huella: h.clone(),
                ancho: 488
            })
        );
        assert_eq!(
            leer_ruta(&format!("/b/{h}/2000")),
            Ok(Ruta::Arte {
                blob: h.clone(),
                lado: 2000
            })
        );
        assert_eq!(leer_ruta(&format!("/b/{h}/1234")), Err(Fallo::Mala));
        assert_eq!(leer_ruta(&format!("/c/{h}/7")), Err(Fallo::Mala));
    }

    #[test]
    fn la_ruta_de_un_extra() {
        assert_eq!(
            leer_ruta("/x/balatro/12"),
            Ok(Ruta::Extra {
                pack: "balatro".into(),
                indice: 12
            })
        );
    }

    #[test]
    fn las_rutas_de_la_mesa() {
        let h = "a".repeat(64);
        let id = "0000579f-7b35-4ed3-b44c-db2a538066fe";
        assert!(matches!(
            leer_ruta(&format!("/m/{h}/488")),
            Ok(Ruta::Mesa { ancho: 488, .. })
        ));
        assert!(matches!(
            leer_ruta(&format!("/m/{h}/1920")),
            Ok(Ruta::Mesa { ancho: 1920, .. })
        ));
        assert!(matches!(
            leer_ruta(&format!("/m/s/{id}/1/244")),
            Ok(Ruta::Mesa { ancho: 244, .. })
        ));
        for mala in [
            format!("/m/{h}/500"),
            format!("/m/{h}"),
            format!("/m/s/{id}/2/244"),
            "/m/s/../0/244".to_string(),
            format!("/m/s/{}/0/244", "z".repeat(36)),
            "/m/../../x/488".to_string(),
        ] {
            assert!(leer_ruta(&mala).is_err(), "{mala}");
        }
    }

    #[test]
    fn la_ruta_de_una_letra() {
        let h = "a".repeat(64);
        assert_eq!(
            leer_ruta(&format!("/f/{h}.woff2")),
            Ok(Ruta::Letra { huella: h.clone() })
        );
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
            format!("/d/{h}/500"),
            format!("/d/{}/745", "z".repeat(64)),
            format!("/d/{h}"),
            "/home/estigio/secreto.png".to_string(),
            "/r/%2e%2e".to_string(),
            "/x/../0".to_string(),
            "/x/balatro/-1".to_string(),
            "/x/balatro/uno".to_string(),
            "/x/bala%2Ftro/1".to_string(),
            format!("/f/{h}"),
            format!("/f/{h}.ttf"),
            "/f/../fuentes.woff2".to_string(),
        ] {
            assert!(leer_ruta(&mala).is_err(), "{mala}");
        }
        assert_eq!(
            leer_ruta(&format!("/r/{h}/{}/488", "A".repeat(3000))),
            Err(Fallo::Mala)
        );
    }
}
