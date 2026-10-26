//! **La mesa de juego en red** (Fase E): conexiones directas entre jugadores
//! con iroh (QUIC por clave pública, camino directo y relé de respaldo).
//!
//! - `perfil.rs`: `PB_PERFIL`, varias instancias en el mismo equipo (debug).
//! - `identidad.rs`: la clave de la instalación, el apodo y el avatar.
//! - `nodo.rs`: el `Endpoint` y sus protocolos.
//!
//! Toda entrada que llega de otro equipo es **no confiable**: se valida
//! antes de tocar nada (tamaños, tipos, permisos en `mesa::resolver`).

pub mod identidad;
pub mod nodo;
pub mod perfil;

use iroh::{Endpoint, SecretKey};

use crate::error::AppResult;

/// La red de la app: la clave y, mientras hace falta, el nodo levantado.
pub struct Red {
    clave: SecretKey,
    nodo: tokio::sync::Mutex<Option<Endpoint>>,
}

impl Red {
    pub fn nueva(clave: SecretKey) -> Self {
        Red {
            clave,
            nodo: tokio::sync::Mutex::new(None),
        }
    }

    pub fn clave(&self) -> &SecretKey {
        &self.clave
    }

    /// El nodo, levantándolo si aún no lo está.
    pub async fn nodo(&self) -> AppResult<Endpoint> {
        let mut n = self.nodo.lock().await;
        if let Some(ep) = n.as_ref().filter(|e| !e.is_closed()) {
            return Ok(ep.clone());
        }
        let ep = nodo::crear(self.clave.clone(), nodo::relay_de_entorno()).await?;
        *n = Some(ep.clone());
        Ok(ep)
    }

    /// Cierra el nodo (al salir de la partida en red o de la app).
    pub async fn parar(&self) {
        if let Some(ep) = self.nodo.lock().await.take() {
            ep.close().await;
        }
    }
}
