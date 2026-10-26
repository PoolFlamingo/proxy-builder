//! **La mesa de juego en red** (Fase E): conexiones directas entre jugadores
//! con iroh (QUIC por clave pública, camino directo y relé de respaldo).
//!
//! - `perfil.rs`: `PB_PERFIL`, varias instancias en el mismo equipo (debug).
//! - `identidad.rs`: la clave de la instalación, el apodo y el avatar.
//! - `nodo.rs`: el `Endpoint` y sus protocolos.
//! - `protocolo.rs`: los mensajes de la partida, sus tramas y sus topes.
//! - `anfitrion.rs` / `invitado.rs`: los dos lados de la partida.
//! - `codigo.rs`: el código directo (dirección + secreto) para unirse.
//! - `mesa.rs`: la sesión de la app y los avisos a la pantalla, tras un trait.
//! - `imagenes.rs`: las imágenes propias de cada jugador.
//!
//! Toda entrada que llega de otro equipo es **no confiable**: se valida
//! antes de tocar nada (tamaños, tipos, permisos en `mesa::resolver`).

pub mod anfitrion;
pub mod codigo;
pub mod colores;
pub mod identidad;
pub mod imagenes;
pub mod invitado;
pub mod mesa;
pub mod mirada;
pub mod nodo;
pub mod perfil;
pub mod protocolo;
#[cfg(test)]
mod pruebas;
pub mod servidor;

use std::sync::Arc;

use iroh::{Endpoint, SecretKey};

use crate::error::AppResult;

/// La partida en red de esta app (en la sala o ya empezada).
pub enum Sala {
    /// Esta app la abrió: atiende a los que llegan con `codigo`.
    Anfitrion {
        anf: Arc<anfitrion::Anfitrion>,
        tarea: tokio::task::AbortHandle,
        codigo: String,
    },
    /// Esta app se unió a la de otro.
    Invitado {
        conexion: invitado::Conexion,
        rol: protocolo::Rol,
    },
}

impl Sala {
    /// Sale: el anfitrión la cierra para todos; el invitado se despide.
    pub async fn cerrar(self) {
        match self {
            Sala::Anfitrion { anf, tarea, .. } => {
                anf.cerrar();
                tarea.abort();
            }
            Sala::Invitado { conexion, .. } => conexion.salir().await,
        }
    }
}

/// La red de la app: la clave y, mientras hace falta, el nodo levantado.
pub struct Red {
    clave: SecretKey,
    nodo: tokio::sync::Mutex<Option<Endpoint>>,
    sala: std::sync::Mutex<Option<Sala>>,
    /// El cliente del servidor de salas (Fase F), si se ha conectado.
    servidor: std::sync::Mutex<Option<Arc<servidor::cliente::Cliente>>>,
    /// La sala del servidor en la que está esta app.
    salon: std::sync::Mutex<Option<Arc<servidor::sala::Salon>>>,
}

impl Red {
    pub fn nueva(clave: SecretKey) -> Self {
        Red {
            clave,
            nodo: tokio::sync::Mutex::new(None),
            sala: std::sync::Mutex::new(None),
            servidor: std::sync::Mutex::new(None),
            salon: std::sync::Mutex::new(None),
        }
    }

    /// Con la sala del servidor (si la hay).
    pub fn con_salon<R>(&self, f: impl FnOnce(&mut Option<Arc<servidor::sala::Salon>>) -> R) -> R {
        let mut g = match self.salon.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    /// Con el cliente del servidor (si se ha arrancado).
    pub fn con_servidor<R>(
        &self,
        f: impl FnOnce(&mut Option<Arc<servidor::cliente::Cliente>>) -> R,
    ) -> R {
        let mut g = match self.servidor.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    /// Con la sala (si la hay).
    pub fn con_sala<R>(&self, f: impl FnOnce(&mut Option<Sala>) -> R) -> R {
        let mut g = match self.sala.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    /// Quita la sala (para cerrarla fuera del cerrojo).
    pub fn quitar_sala(&self) -> Option<Sala> {
        self.con_sala(Option::take)
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
