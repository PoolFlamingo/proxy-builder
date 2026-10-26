//! **La mesa vista desde la red**: lo que necesitan las tareas del anfitrión y
//! del invitado de la app (la sesión abierta y avisar a la pantalla), detrás
//! de un trait para poder probarlas sin Tauri.

use std::sync::{Arc, Mutex};

use mesa::{IdCarta, IdJugador};
use serde::Serialize;
use tauri::{Emitter, Manager};
use ts_rs::TS;

use super::mirada::Mirada;
use super::protocolo::{EnSala, Enlace};
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
    /// Cómo va el enlace de cada jugador con el anfitrión y cuánto va el reloj
    /// de aquí por detrás del suyo (`desfase_ms`: se suma a `Date.now()` para
    /// el temporizador del turno).
    #[serde(rename_all = "camelCase")]
    Enlaces {
        desfase_ms: i32,
        filas: Vec<Enlace>,
    },
    /// Alguien arrastra una carta (o mueve su cursor, sin carta) por un
    /// tablero, en coordenadas 0–10.000. Nada de esto se guarda.
    #[serde(rename_all = "camelCase")]
    Fantasma {
        jugador: IdJugador,
        tablero: IdJugador,
        carta: Option<IdCarta>,
        x: u16,
        y: u16,
    },
    /// Alguien señala una carta o a un jugador (un aro de 2 s).
    Senal {
        jugador: IdJugador,
        carta: Option<IdCarta>,
        a: Option<IdJugador>,
    },
}

pub trait Mesa: Clone + Send + Sync + 'static {
    /// Hace algo con la sesión abierta (si la hay).
    fn con<R>(&self, f: impl FnOnce(&mut Option<Sesion>) -> R) -> R;
    /// Lo que ve un espectador (no tiene sesión).
    fn mirada<R>(&self, f: impl FnOnce(&mut Option<Mirada>) -> R) -> R;
    fn avisar(&self, aviso: AvisoRed);
    /// La carpeta de las imágenes de la mesa (`app_cache_dir/mesa/`).
    fn carpeta(&self) -> std::path::PathBuf;
}

/// La de la app: la sesión de `AppState.mesa` y los avisos por evento.
pub struct MesaDeApp<Rt: tauri::Runtime = tauri::Wry>(pub tauri::AppHandle<Rt>);

impl<Rt: tauri::Runtime> Clone for MesaDeApp<Rt> {
    fn clone(&self) -> Self {
        MesaDeApp(self.0.clone())
    }
}

impl<Rt: tauri::Runtime> Mesa for MesaDeApp<Rt> {
    fn con<R>(&self, f: impl FnOnce(&mut Option<Sesion>) -> R) -> R {
        let estado = self.0.state::<AppState>();
        let mut g = match estado.mesa.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    fn mirada<R>(&self, f: impl FnOnce(&mut Option<Mirada>) -> R) -> R {
        let estado = self.0.state::<AppState>();
        let mut g = match estado.mirada.lock() {
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

    fn carpeta(&self) -> std::path::PathBuf {
        crate::mesa::imagenes::carpeta(&self.0.state::<AppState>())
    }
}

/// Para los tests: una sesión en memoria y los avisos apuntados.
#[derive(Clone)]
pub struct MesaEnMemoria {
    pub sesion: Arc<Mutex<Option<Sesion>>>,
    pub mirada: Arc<Mutex<Option<Mirada>>>,
    pub avisos: Arc<Mutex<Vec<AvisoRed>>>,
    /// Una carpeta temporal propia para las imágenes.
    pub carpeta: std::path::PathBuf,
}

impl Default for MesaEnMemoria {
    fn default() -> Self {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        MesaEnMemoria {
            sesion: Arc::default(),
            mirada: Arc::default(),
            avisos: Arc::default(),
            carpeta: std::env::temp_dir().join(format!(
                "pb-mesa-{}-{n}-{}",
                std::process::id(),
                crate::mesa::sesion::semilla()
            )),
        }
    }
}

impl Mesa for MesaEnMemoria {
    fn con<R>(&self, f: impl FnOnce(&mut Option<Sesion>) -> R) -> R {
        let mut g = match self.sesion.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        f(&mut g)
    }

    fn mirada<R>(&self, f: impl FnOnce(&mut Option<Mirada>) -> R) -> R {
        let mut g = match self.mirada.lock() {
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

    fn carpeta(&self) -> std::path::PathBuf {
        self.carpeta.clone()
    }
}
