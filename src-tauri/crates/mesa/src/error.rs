//! Por qué una intención no vale.
//!
//! Es un dato, no prosa: la pantalla lo traduce por su nombre (la app lo
//! convierte en `AppError { code: "Rejected", params: { motivo } }`).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS, thiserror::Error)]
#[ts(export)]
pub enum Rechazo {
    /// Algo de lo nombrado (carta, jugador, zona, reacción) no existe o no
    /// está donde la acción dice.
    #[error("no existe")]
    NoExiste,
    /// El actor no puede tocar eso (las cartas que controla otro, la
    /// biblioteca de otro…), salvo con «Todos tocan todo».
    #[error("sin permiso")]
    SinPermiso,
    /// La zona de la que se quiere sacar está vacía.
    #[error("zona vacía")]
    ZonaVacia,
    /// Solo el jugador activo puede hacerlo (pasar el turno).
    #[error("fuera de turno")]
    FueraDeTurno,
    /// Un número fuera de lo razonable (1.000 fichas, 500 caracteres…).
    #[error("límite")]
    Limite,
    /// La acción no tiene sentido ahora (mulligan con la partida empezada,
    /// deshacer lo de otro…).
    #[error("no se puede")]
    NoSePuede,
}
