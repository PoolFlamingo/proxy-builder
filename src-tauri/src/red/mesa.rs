//! **La mesa vista desde la red**: lo que necesitan las tareas del anfitrión y
//! del invitado de la app (la sesión abierta y avisar a la pantalla), detrás
//! de un trait para poder probarlas sin Tauri.

use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager};
use ts_rs::TS;

use super::protocolo::EnSala;
use crate::mesa::sesion::Sesion;
use crate::state::AppState;

/// Lo que la red le cuenta a la pantalla (`red://aviso`).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum AvisoRed {
    /// Quién hay en la sala de espera.
    Sala {
        jugadores: Vec<EnSala>,
    },
    /// Empezó la partida (o volvió): la pantalla se suscribe a la mesa.
    Empezada,
    /// Se cortó la conexión con el anfitrión y se intenta volver.
    Reconectando {
        intento: u32,
    },
    Conectada,
    /// No se pudo volver.
    Perdida,
    /// El anfitrión cerró la partida.
    Cerrada,
    /// El anfitrión no dejó entrar (versión distinta, código malo, llena…).
    Rechazada {
        motivo: String,
    },
}

pub trait Mesa: Clone + Send + Sync + 'static {
    /// Hace algo con la sesión abierta (si la hay).
    fn con<R>(&self, f: impl FnOnce(&mut Option<Sesion>) -> R) -> R;
    fn avisar(&self, aviso: AvisoRed);
}

/// La de la app: la sesión de `AppState.mesa` y los avisos por evento.
#[derive(Clone)]
pub struct MesaDeApp(pub tauri::AppHandle);

impl Mesa for MesaDeApp {
    fn con<R>(&self, f: impl FnOnce(&mut Option<Sesion>) -> R) -> R {
        let estado = self.0.state::<AppState>();
        let mut g = match estado.mesa.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    fn avisar(&self, aviso: AvisoRed) {
        if let Err(e) = self.0.emit("red://aviso", aviso) {
            tracing::warn!(error = %e, "no se pudo avisar a la pantalla");
        }
    }
}

/// Para los tests: una sesión en memoria y los avisos apuntados.
#[derive(Clone, Default)]
pub struct MesaEnMemoria {
    pub sesion: Arc<Mutex<Option<Sesion>>>,
    pub avisos: Arc<Mutex<Vec<AvisoRed>>>,
}

impl Mesa for MesaEnMemoria {
    fn con<R>(&self, f: impl FnOnce(&mut Option<Sesion>) -> R) -> R {
        let mut g = match self.sesion.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    fn avisar(&self, aviso: AvisoRed) {
        if let Ok(mut v) = self.avisos.lock() {
            v.push(aviso);
        }
    }
}
