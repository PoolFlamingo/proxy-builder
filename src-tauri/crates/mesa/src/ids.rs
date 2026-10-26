//! Ids pequeños y estables de la mesa.
//!
//! Son números y no uuid: viajan en cada evento y en cada vista, y la huella
//! de la partida (`huella.rs`) depende de que se ordenen siempre igual.

use std::fmt;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Un asiento de la mesa (0–9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IdJugador(pub u8);

/// Una carta de la partida. Las fichas y las copias reciben uno nuevo al
/// crearse (`Partida::siguiente_id`); nunca se reutiliza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IdCarta(pub u32);

/// Un equipo (Fase G).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct IdEquipo(pub u8);

impl fmt::Display for IdJugador {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "j{}", self.0)
    }
}

impl fmt::Display for IdCarta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}

impl fmt::Display for IdEquipo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}
