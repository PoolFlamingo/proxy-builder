//! **La única puerta a Scryfall.**
//!
//! - Toda petición a `api.scryfall.com` pasa por el limitador (≥ 100 ms entre
//!   peticiones); las imágenes de `*.scryfall.io` no tienen ese límite, pero
//!   van como mucho de 6 en 6.
//! - 429 y 5xx se reintentan hasta 3 veces (0,5 s, 1 s, 2 s), respetando
//!   `Retry-After`.
//! - Cada petición real queda en el registro (`tracing`), para poder comprobar
//!   que un deck guardado no descarga nada.
//!
//! El trait `CardApi` permite probar la resolución con un Scryfall de mentira.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use tokio::sync::Semaphore;

use super::models::{Catalog, Collection, Identifier, ScryCard, SearchPage, SetInfo, SetList};
use super::rate::Limitador;
use crate::error::{AppError, AppResult, ErrorCode};

const API: &str = "https://api.scryfall.com";
const MAX_REINTENTOS: u32 = 3;
const DESCARGAS_A_LA_VEZ: usize = 6;
/// Tope de resultados al seguir páginas de una búsqueda.
const TOPE_BUSQUEDA: usize = 175;

/// Lo que la app necesita de Scryfall.
pub trait CardApi: Send + Sync {
    fn collection(&self, ids: &[Identifier]) -> impl Future<Output = AppResult<Collection>> + Send;
    fn card(&self, id: &str) -> impl Future<Output = AppResult<Option<ScryCard>>> + Send;
    /// `GET /cards/{set}/{número}/{idioma}`.
    fn card_lang(
        &self,
        set: &str,
        number: &str,
        lang: &str,
    ) -> impl Future<Output = AppResult<Option<ScryCard>>> + Send;
    /// Búsqueda con la sintaxis de Scryfall; `prints` = todas las impresiones
    /// (`unique=prints&include_multilingual=true`). Sigue las páginas hasta un tope.
    fn search(
        &self,
        q: &str,
        prints: bool,
        limit: usize,
    ) -> impl Future<Output = AppResult<Vec<ScryCard>>> + Send;
    fn download(&self, url: &str) -> impl Future<Output = AppResult<Vec<u8>>> + Send;
}

pub struct Scryfall {
    http: Client,
    limitador: Limitador,
    descargas: Arc<Semaphore>,
}

fn red(e: reqwest::Error) -> AppError {
    AppError::new(ErrorCode::Network).with("detalle", e)
}

impl Scryfall {
    pub fn new() -> AppResult<Scryfall> {
        let http = Client::builder()
            .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(red)?;
        Ok(Scryfall {
            http,
            limitador: Limitador::new(Duration::from_millis(100)),
            descargas: Arc::new(Semaphore::new(DESCARGAS_A_LA_VEZ)),
        })
    }

    /// Hace una petición con ritmo y reintentos. `None` si es 404.
    async fn enviar(
        &self,
        construir: impl Fn() -> reqwest::RequestBuilder,
        con_ritmo: bool,
    ) -> AppResult<Option<reqwest::Response>> {
        let mut espera = Duration::from_millis(500);
        for intento in 0..=MAX_REINTENTOS {
            if con_ritmo {
                self.limitador.turno().await;
            }
            let peticion = construir()
                .header(reqwest::header::ACCEPT, "application/json;q=0.9,*/*;q=0.8")
                .build()
                .map_err(red)?;
            let (metodo, url) = (peticion.method().clone(), peticion.url().clone());
            let r = self.http.execute(peticion).await.map_err(red)?;
            let estado = r.status();
            tracing::info!(%metodo, host = url.host_str().unwrap_or(""), ruta = url.path(), estado = estado.as_u16(), "red");
            if estado == StatusCode::NOT_FOUND {
                return Ok(None);
            }
            if estado.is_success() {
                return Ok(Some(r));
            }
            let reintentable = estado == StatusCode::TOO_MANY_REQUESTS || estado.is_server_error();
            if !reintentable || intento == MAX_REINTENTOS {
                return Err(if estado == StatusCode::TOO_MANY_REQUESTS {
                    AppError::new(ErrorCode::RateLimited)
                } else {
                    AppError::new(ErrorCode::Network).with("estado", estado.as_u16())
                });
            }
            let pedida = r
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
            tokio::time::sleep(pedida.unwrap_or(espera)).await;
            espera *= 2;
        }
        Err(AppError::new(ErrorCode::Network))
    }

    async fn json<T: DeserializeOwned>(
        &self,
        construir: impl Fn() -> reqwest::RequestBuilder,
    ) -> AppResult<Option<T>> {
        match self.enviar(construir, true).await? {
            None => Ok(None),
            Some(r) => Ok(Some(r.json::<T>().await.map_err(red)?)),
        }
    }

    pub async fn autocomplete(&self, q: &str) -> AppResult<Vec<String>> {
        let url = format!("{API}/cards/autocomplete");
        let c: Option<Catalog> = self
            .json(|| {
                self.http
                    .get(&url)
                    .query(&[("q", q), ("include_extras", "true")])
            })
            .await?;
        Ok(c.map(|c| c.data).unwrap_or_default())
    }

    pub async fn sets(&self) -> AppResult<Vec<SetInfo>> {
        let url = format!("{API}/sets");
        let l: Option<SetList> = self.json(|| self.http.get(&url)).await?;
        Ok(l.map(|l| l.data.into_iter().map(SetInfo::from).collect())
            .unwrap_or_default())
    }
}

impl CardApi for Scryfall {
    async fn collection(&self, ids: &[Identifier]) -> AppResult<Collection> {
        let url = format!("{API}/cards/collection");
        let cuerpo = serde_json::json!({ "identifiers": ids });
        Ok(self
            .json(|| self.http.post(&url).json(&cuerpo))
            .await?
            .unwrap_or_default())
    }

    async fn card(&self, id: &str) -> AppResult<Option<ScryCard>> {
        let url = format!("{API}/cards/{id}");
        self.json(|| self.http.get(&url)).await
    }

    async fn card_lang(&self, set: &str, number: &str, lang: &str) -> AppResult<Option<ScryCard>> {
        let url = format!(
            "{API}/cards/{}/{}/{}",
            urlencode(set),
            urlencode(number),
            urlencode(lang)
        );
        self.json(|| self.http.get(&url)).await
    }

    async fn search(&self, q: &str, prints: bool, limit: usize) -> AppResult<Vec<ScryCard>> {
        let limit = limit.min(TOPE_BUSQUEDA);
        let mut out = Vec::new();
        let primera = format!("{API}/cards/search");
        let mut pagina: Option<SearchPage> = self
            .json(|| {
                let r = self.http.get(&primera).query(&[("q", q)]);
                if prints {
                    r.query(&[("unique", "prints"), ("include_multilingual", "true")])
                } else {
                    r
                }
            })
            .await?;
        while let Some(p) = pagina {
            out.extend(p.data);
            if out.len() >= limit || !p.has_more {
                break;
            }
            let Some(siguiente) = p.next_page else { break };
            pagina = self.json(|| self.http.get(&siguiente)).await?;
        }
        out.truncate(limit);
        Ok(out)
    }

    async fn download(&self, url: &str) -> AppResult<Vec<u8>> {
        let _permiso = self
            .descargas
            .acquire()
            .await
            .map_err(|_| AppError::new(ErrorCode::Internal))?;
        let r = self
            .enviar(|| self.http.get(url), false)
            .await?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("url", url))?;
        Ok(r.bytes().await.map_err(red)?.to_vec())
    }
}

/// Codifica un trozo de ruta (sets y números pueden llevar `★` o `-`).
fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_ruta_se_codifica() {
        assert_eq!(urlencode("2063★"), "2063%E2%98%85");
        assert_eq!(urlencode("DOM-168"), "DOM-168");
    }

    /// Prueba en vivo contra Scryfall: `cargo test -- --ignored scryfall_en_vivo`.
    #[tokio::test]
    #[ignore]
    async fn scryfall_en_vivo() {
        let s = Scryfall::new().unwrap();
        let c = s
            .collection(&[Identifier::SetNumber {
                set: "cmm".into(),
                collector_number: "410".into(),
            }])
            .await
            .unwrap();
        assert_eq!(c.data[0].name, "Sol Ring");
        let es = s.card_lang("cmm", "410", "es").await.unwrap();
        println!("cmm 410 es: {:?}", es.map(|c| c.lang));
        let prints = s.search("!\"Sol Ring\" lang:es", true, 20).await.unwrap();
        assert!(prints.iter().all(|c| c.lang == "es"));
        assert!(!s.autocomplete("sol ri").await.unwrap().is_empty());
    }
}
