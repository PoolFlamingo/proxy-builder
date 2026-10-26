//! **El nodo de iroh**: el `Endpoint` con el que la app se conecta a los
//! demás. Se levanta solo cuando hace falta (al crear o unirse a una
//! partida en red) y se cierra al salir de ella o de la app.
//!
//! Dos protocolos (ALPN): `proxy-builder/mesa/1` (la partida) e
//! `proxy-builder/img/1` (las imágenes propias de cada jugador).
//!
//! Relés: con `PB_RELAY` (solo en debug), el que diga (`none` = sin relé,
//! para LAN y pruebas; una URL = solo ese). Si no, con el relé propio de
//! producción (`RELE_PROPIO`), el propio **y** los de n0 detrás, de último
//! recurso (por si el propio cae); mientras no haya dominio, solo los de n0.

use std::sync::Arc;

use iroh::endpoint::presets;
use iroh::{Endpoint, RelayConfig, RelayMap, RelayMode, RelayUrl, SecretKey};

use crate::error::{AppError, AppResult, ErrorCode};

pub const ALPN_MESA: &[u8] = b"proxy-builder/mesa/1";
pub const ALPN_IMG: &[u8] = b"proxy-builder/img/1";
/// La malla de la sala (Fase F): una conexión ligera con cada jugador solo
/// para medir la latencia y el camino.
pub const ALPN_PING: &[u8] = b"proxy-builder/ping/1";

/// Por dónde pasa la conexión cuando no hay camino directo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relay {
    /// Sin relé: solo direcciones directas (LAN, pruebas).
    Ninguno,
    /// Los relés públicos de n0 (y su búsqueda de direcciones por clave).
    N0,
    /// Solo uno propio (en debug, `PB_RELAY=<url>`, para probar el servidor).
    Propio(RelayUrl),
    /// El de producción: el propio y, detrás, los de n0 (Q-E1).
    PropioYN0(RelayUrl),
}

/// El relé propio de producción (`server/docs/despliegue.md`); mientras no
/// haya dominio, ninguno y se usan los de n0.
const RELE_PROPIO: Option<&str> = None;

/// Los relés del propio y los de n0 en un mapa: iroh se queda con el que
/// mejor responda, y si el propio cae, los de n0 siguen valiendo.
pub fn mapa_con_n0(propio: &RelayUrl) -> RelayMap {
    let mut relays: Vec<Arc<RelayConfig>> = vec![Arc::new(RelayConfig::from(propio.clone()))];
    relays.extend(
        iroh::defaults::prod::default_relay_map()
            .relays::<Vec<_>>()
            .into_iter()
            .filter(|c| c.url != *propio),
    );
    RelayMap::from_iter(relays)
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
    de_produccion(RELE_PROPIO)
}

/// Los relés de producción con este relé propio (o sin él).
fn de_produccion(propio: Option<&str>) -> Relay {
    match propio.and_then(|u| u.parse::<RelayUrl>().ok()) {
        Some(url) => Relay::PropioYN0(url),
        None => Relay::N0,
    }
}

pub(crate) fn de_red(e: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Network).with("detalle", e.to_string())
}

/// Levanta un nodo con esta clave y estos relés.
pub async fn crear(clave: SecretKey, relay: Relay) -> AppResult<Endpoint> {
    let alpns = vec![ALPN_MESA.to_vec(), ALPN_IMG.to_vec(), ALPN_PING.to_vec()];
    let b = match &relay {
        Relay::Ninguno => Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled),
        Relay::N0 => Endpoint::builder(presets::N0),
        Relay::Propio(url) => {
            Endpoint::builder(presets::N0).relay_mode(RelayMode::custom([url.clone()]))
        }
        Relay::PropioYN0(url) => {
            Endpoint::builder(presets::N0).relay_mode(RelayMode::Custom(mapa_con_n0(url)))
        }
    };
    // Los certificados del relé, con los del sistema (como reqwest y el
    // cliente del servidor de salas), no solo los de Mozilla incrustados.
    let ep = b
        .ca_tls_config(iroh::tls::CaTlsConfig::system())
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

    #[test]
    fn con_el_rele_propio_salen_el_propio_y_los_de_n0() {
        let propio: RelayUrl = "https://relay.ejemplo.test".parse().unwrap();
        let mapa = mapa_con_n0(&propio);
        let n0 = iroh::defaults::prod::default_relay_map();
        assert!(mapa.contains(&propio));
        for u in n0.urls::<Vec<RelayUrl>>() {
            assert!(mapa.contains(&u), "falta {u}");
        }
        assert_eq!(mapa.len(), n0.len() + 1);
        // Si el propio fuera uno de n0, no se repite.
        let uno = n0.urls::<Vec<RelayUrl>>().remove(0);
        assert_eq!(mapa_con_n0(&uno).len(), n0.len());
    }

    #[test]
    fn la_produccion_usa_el_propio_con_los_de_n0_detras_y_sin_el_solo_n0() {
        assert_eq!(de_produccion(None), Relay::N0);
        // Una URL rota tampoco rompe nada.
        assert_eq!(de_produccion(Some("no es una url")), Relay::N0);
        let url: RelayUrl = "https://relay.ejemplo.test".parse().unwrap();
        assert_eq!(
            de_produccion(Some("https://relay.ejemplo.test")),
            Relay::PropioYN0(url)
        );
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
