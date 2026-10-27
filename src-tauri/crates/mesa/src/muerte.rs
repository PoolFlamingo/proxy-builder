//! **Quién está fuera de juego** (G8). Magic no tiene un juez aquí: la mesa
//! solo *avisa* de que alguien llegó a 0 vidas, al límite de veneno o al de
//! daño de un mismo comandante, y es la persona (o quien la sustituye) la que
//! lo confirma con `Accion::Eliminar`. Esto es la cuenta de ese aviso, pura;
//! la pantalla no recalcula nada.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{IdCarta, IdJugador};
use crate::modelo::Partida;

/// Por qué se ha quedado fuera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum Causa {
    /// 0 vidas o menos (la vida compartida del equipo, si la hay).
    Vida,
    /// El límite de contadores de veneno (10, de fábrica).
    Veneno,
    /// El límite de daño de un mismo comandante (21), que es de `de`.
    Comandante { de: IdJugador, carta: IdCarta },
}

/// Si `j` ha llegado a un límite y todavía no está eliminado. Varias
/// causas a la vez: la vida manda, luego el veneno y luego el comandante.
pub fn fuera_de_juego(p: &Partida, j: IdJugador) -> Option<Causa> {
    let x = p.jugador(j)?;
    if x.eliminado.is_some() {
        return None;
    }
    let vida = x
        .equipo
        .and_then(|e| p.equipos.iter().find(|q| q.id == e))
        .and_then(|e| e.vida_compartida)
        .unwrap_or(x.vida);
    if vida <= 0 {
        return Some(Causa::Vida);
    }
    if x.veneno >= p.opciones.limite_veneno {
        return Some(Causa::Veneno);
    }
    x.dano_comandante
        .iter()
        .filter(|(_, d)| **d >= p.opciones.limite_comandante)
        .max_by_key(|(_, d)| **d)
        .map(|(carta, _)| Causa::Comandante {
            de: p.cartas.get(carta).map_or(j, |c| c.dueno),
            carta: *carta,
        })
}
