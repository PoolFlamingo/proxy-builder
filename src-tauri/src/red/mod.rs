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
//! - `latencia.rs`: RTT, camino y reloj con el anfitrión; el movimiento en vivo.
//!
//! Toda entrada que llega de otro equipo es **no confiable**: se valida
//! antes de tocar nada (tamaños, tipos, permisos en `mesa::resolver`).

pub mod anfitrion;
pub mod codigo;
pub mod colores;
pub mod identidad;
pub mod imagenes;
pub mod invitado;
pub mod latencia;
pub mod mesa;
pub mod mirada;
pub mod nodo;
pub mod perfil;
pub mod protocolo;
#[cfg(test)]
mod pruebas;
#[cfg(test)]
mod pruebas_procesos;
pub mod servidor;
pub mod sucesion;
pub mod vuelta;

use std::sync::{Arc, Mutex};

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
    /// Sale: el anfitrión cede la partida a otro jugador si puede y, si no,
    /// la cierra para todos (`Anfitrion::irse`); el invitado se despide.
    pub async fn cerrar(self) {
        match self {
            Sala::Anfitrion { anf, tarea, .. } => {
                anf.irse().await;
                tarea.abort();
            }
            Sala::Invitado { conexion, .. } => conexion.salir().await,
        }
    }
}

/// La red de la app: la clave y, mientras hace falta, el nodo levantado.
pub struct Red {
    clave: std::sync::RwLock<SecretKey>,
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
            clave: std::sync::RwLock::new(clave),
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

    /// La clave de la instalación (una copia: puede cambiar al importar otra identidad).
    pub fn clave(&self) -> SecretKey {
        match self.clave.read() {
            Ok(g) => g.clone(),
            Err(e) => e.into_inner().clone(),
        }
    }

    /// Cambia la identidad: cierra el nodo y el cliente del servidor (que se
    /// levantarán de nuevo con la clave nueva cuando haga falta).
    pub async fn cambiar_clave(&self, nueva: SecretKey) {
        self.parar().await;
        self.con_servidor(|c| *c = None);
        match self.clave.write() {
            Ok(mut g) => *g = nueva,
            Err(e) => *e.into_inner() = nueva,
        }
    }

    /// Si hay una partida en red o una sala del servidor abiertas.
    pub fn ocupada(&self) -> bool {
        self.con_sala(|s| s.is_some()) || self.con_salon(|s| s.is_some())
    }

    /// El nodo, levantándolo si aún no lo está.
    pub async fn nodo(&self) -> AppResult<Endpoint> {
        let mut n = self.nodo.lock().await;
        if let Some(ep) = n.as_ref().filter(|e| !e.is_closed()) {
            return Ok(ep.clone());
        }
        let ep = nodo::crear(self.clave(), nodo::relay_de_entorno()).await?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn al_cambiar_la_identidad_el_nodo_se_levanta_con_la_clave_nueva() {
        let a = SecretKey::generate();
        let b = SecretKey::generate();
        let red = Red::nueva(a.clone());
        assert!(!red.ocupada());
        // El nodo se levanta con la clave de ahora (sin relé: solo local).
        let ep = nodo::crear(red.clave(), nodo::Relay::Ninguno)
            .await
            .unwrap();
        assert_eq!(ep.id(), a.public());
        ep.close().await;
        red.cambiar_clave(b.clone()).await;
        assert_eq!(red.clave().public(), b.public());
        let ep = nodo::crear(red.clave(), nodo::Relay::Ninguno)
            .await
            .unwrap();
        assert_eq!(ep.id(), b.public());
        ep.close().await;
    }
}

/// El deck con el que juega esta app la partida en red (y su lista): el
/// anfitrión lo tiene en su configuración, pero un invitado solo recibe la
/// partida. Sirve para que su historial la guarde bajo ese deck.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiDeck {
    pub deck_id: String,
    pub lista: Option<String>,
    pub nombre: String,
}

static MI_DECK: Mutex<Option<MiDeck>> = Mutex::new(None);

/// Apunta el deck con el que se juega (`None`: se mira, sin deck).
pub fn recordar_mi_deck(d: Option<MiDeck>) {
    if let Ok(mut m) = MI_DECK.lock() {
        *m = d;
    }
}

pub fn mi_deck() -> Option<MiDeck> {
    MI_DECK.lock().ok().and_then(|m| m.clone())
}
