//! **La mesa vista desde la red**: lo que necesitan las tareas del anfitrión y
//! del invitado de la app (la sesión abierta y avisar a la pantalla), detrás
//! de un trait para poder probarlas sin Tauri.

use std::sync::{Arc, Mutex};

use mesa::{IdCarta, IdJugador};
use serde::Serialize;
use tauri::{Emitter, Manager};
use ts_rs::TS;

use super::anfitrion::Anfitrion;
use super::invitado::Conexion;
use super::mirada::Mirada;
use super::protocolo::{Ausente, EnSala, Enlace, Rol};
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
    /// Quién falta de la partida (los que no están conectados con el
    /// anfitrión) y desde hace cuánto: sustituye a la lista anterior.
    Ausentes {
        filas: Vec<Ausente>,
    },
    /// Alguien arrastra una carta (o mueve su cursor, sin carta) por un
    /// tablero, en coordenadas 0–10.000. Nada de esto se guarda.
    #[serde(rename_all = "camelCase")]
    Fantasma {
        jugador: IdJugador,
        tablero: IdJugador,
        carta: Option<IdCarta>,
        /// Las demás cartas que arrastra con ella.
        mas: Vec<IdCarta>,
        x: u16,
        y: u16,
    },
    /// Alguien señala una carta o a un jugador (un aro de 2 s).
    Senal {
        jugador: IdJugador,
        carta: Option<IdCarta>,
        a: Option<IdJugador>,
    },
    /// El anfitrión no vuelve: se prueba con el siguiente (Fase K).
    BuscandoAnfitrion {
        candidato: IdJugador,
    },
    /// Hay anfitrión nuevo (`soy_yo`: esta app).
    #[serde(rename_all = "camelCase")]
    NuevoAnfitrion {
        jugador: IdJugador,
        soy_yo: bool,
    },
    /// Esta app es el anfitrión: cuántos jugadores están conectados (con
    /// ella) de cuántos hay, y si deja jugar (con más de la mitad).
    Mayoria {
        activa: bool,
        conectados: u32,
        total: u32,
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
    /// Esta app pasa a ser el anfitrión (Fase K): `tarea` atiende a los que
    /// llegan y `codigo` es el código nuevo (su dirección, el mismo secreto).
    fn ser_anfitrion(&self, anf: Arc<Anfitrion>, tarea: tokio::task::AbortHandle, codigo: String);
    /// Esta app, que era el anfitrión, sigue como invitado de otro (Fase K).
    fn ser_invitado(&self, conexion: Conexion, rol: Rol);
    /// Apunta (o, con `None`, olvida) la partida a la que se puede volver
    /// (`vuelta.rs`). En los tests no se guarda nada.
    fn recordar_vuelta(&self, _u: Option<&super::invitado::Union>) {}
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

    fn ser_anfitrion(&self, anf: Arc<Anfitrion>, tarea: tokio::task::AbortHandle, codigo: String) {
        let viejo = self
            .0
            .state::<AppState>()
            .red
            .con_sala(|s| s.replace(super::Sala::Anfitrion { anf, tarea, codigo }));
        // El bucle del invitado ya terminó (o termina solo): nada que cerrar.
        drop(viejo);
    }

    fn ser_invitado(&self, conexion: Conexion, rol: Rol) {
        let viejo = self
            .0
            .state::<AppState>()
            .red
            .con_sala(|s| s.replace(super::Sala::Invitado { conexion, rol }));
        // Deja de atender a los que lleguen como anfitrión.
        if let Some(super::Sala::Anfitrion { tarea, .. }) = viejo {
            tarea.abort();
        }
    }

    fn recordar_vuelta(&self, u: Option<&super::invitado::Union>) {
        let db = &self.0.state::<AppState>().db;
        let r = match u {
            Some(u) => super::vuelta::recordar(db, u),
            None => super::vuelta::olvidar(db),
        };
        if let Err(e) = r {
            tracing::warn!(error = ?e, "no se pudo apuntar la partida a la que volver");
        }
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
    /// Lo que esta app es en la partida tras un traspaso (Fase K):
    /// `"anfitrion"` o `"invitado"`, con lo que haga falta para seguir.
    pub papel: Arc<Mutex<Option<PapelEnMemoria>>>,
}

/// Lo que queda en los tests de un traspaso.
pub enum PapelEnMemoria {
    Anfitrion {
        anf: Arc<Anfitrion>,
        tarea: tokio::task::AbortHandle,
    },
    Invitado(Conexion),
}

impl Default for MesaEnMemoria {
    fn default() -> Self {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        MesaEnMemoria {
            sesion: Arc::default(),
            mirada: Arc::default(),
            avisos: Arc::default(),
            papel: Arc::default(),
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

    fn ser_anfitrion(&self, anf: Arc<Anfitrion>, tarea: tokio::task::AbortHandle, _codigo: String) {
        if let Ok(mut p) = self.papel.lock() {
            *p = Some(PapelEnMemoria::Anfitrion { anf, tarea });
        }
    }

    fn ser_invitado(&self, conexion: Conexion, _rol: Rol) {
        if let Ok(mut p) = self.papel.lock() {
            if let Some(PapelEnMemoria::Anfitrion { tarea, .. }) = p.take() {
                tarea.abort();
            }
            *p = Some(PapelEnMemoria::Invitado(conexion));
        }
    }
}
