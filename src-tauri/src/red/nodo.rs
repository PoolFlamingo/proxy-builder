//! **El nodo de iroh**: el `Endpoint` con el que la app se conecta a los
//! demás. Se levanta solo cuando hace falta (al crear o unirse a una
//! partida en red) y se cierra al salir de ella o de la app.
//!
//! Dos protocolos (ALPN): `proxy-builder/mesa/1` (la partida) e
//! `proxy-builder/img/1` (las imágenes propias de cada jugador).
//!
//! Relés: con `PB_RELAY` (solo en debug), el que diga (`none` = sin relé,
//! para LAN y pruebas); si no, los de n0 hasta que exista el propio de la
//! Fase F (pregunta Q-E1 del plan: si se quedan de último recurso).

use iroh::endpoint::presets;
use iroh::{Endpoint, RelayMode, RelayUrl, SecretKey};

use crate::error::{AppError, AppResult, ErrorCode};

pub const ALPN_MESA: &[u8] = b"proxy-builder/mesa/1";
pub const ALPN_IMG: &[u8] = b"proxy-builder/img/1";

/// Por dónde pasa la conexión cuando no hay camino directo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relay {
    /// Sin relé: solo direcciones directas (LAN, pruebas).
    Ninguno,
    /// Los relés públicos de n0 (y su búsqueda de direcciones por clave).
    N0,
    /// Uno propio.
    Propio(RelayUrl),
}

/// El relé que toca: `PB_RELAY` en debug (`none` o una URL), si no n0.
pub fn relay_de_entorno() -> Relay {
    if cfg!(debug_assertions) {
        if let Ok(v) = std::env::var("PB_RELAY") {
            let v = v.trim();
            if v.eq_ignore_ascii_case("none") {
                return Relay::Ninguno;
            }
            if let Ok(url) = v.parse::<RelayUrl>() {
                return Relay::Propio(url);
            }
            tracing::warn!(valor = v, "PB_RELAY no es una URL: se usan los relés de n0");
        }
    }
    Relay::N0
}

pub(crate) fn de_red(e: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Network).with("detalle", e.to_string())
}

/// Levanta un nodo con esta clave y estos relés.
pub async fn crear(clave: SecretKey, relay: Relay) -> AppResult<Endpoint> {
    let alpns = vec![ALPN_MESA.to_vec(), ALPN_IMG.to_vec()];
    let b = match &relay {
        Relay::Ninguno => Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled),
        Relay::N0 => Endpoint::builder(presets::N0),
        Relay::Propio(url) => {
            Endpoint::builder(presets::N0).relay_mode(RelayMode::custom([url.clone()]))
        }
    };
    let ep = b
        .secret_key(clave)
        .alpns(alpns)
        .bind()
        .await
        .map_err(de_red)?;
    tracing::info!(id = %ep.id().fmt_short(), relay = ?relay, "nodo de red levantado");
    Ok(ep)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Dos nodos en el mismo proceso, sin relé (direcciones locales).
    pub async fn dos() -> (Endpoint, Endpoint) {
        let a = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
        let b = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
        (a, b)
    }

    #[tokio::test]
    async fn dos_nodos_se_conectan_por_cada_protocolo() {
        let (a, b) = dos().await;
        let dir = a.addr();
        let servidor = tokio::spawn(async move {
            let mut vistos = Vec::new();
            for _ in 0..2 {
                let conn = a.accept().await.unwrap().await.unwrap();
                vistos.push(conn.alpn().to_vec());
                let (mut tx, mut rx) = conn.accept_bi().await.unwrap();
                let mut buf = [0u8; 4];
                rx.read_exact(&mut buf).await.unwrap();
                tx.write_all(&buf).await.unwrap();
                tx.finish().unwrap();
                conn.closed().await;
            }
            vistos
        });
        for alpn in [ALPN_MESA, ALPN_IMG] {
            let conn = b.connect(dir.clone(), alpn).await.unwrap();
            let (mut tx, mut rx) = conn.open_bi().await.unwrap();
            tx.write_all(b"hola").await.unwrap();
            let mut buf = [0u8; 4];
            rx.read_exact(&mut buf).await.unwrap();
            assert_eq!(&buf, b"hola");
            conn.close(0u32.into(), b"fin");
        }
        let vistos = servidor.await.unwrap();
        assert_eq!(vistos, vec![ALPN_MESA.to_vec(), ALPN_IMG.to_vec()]);
    }
}
