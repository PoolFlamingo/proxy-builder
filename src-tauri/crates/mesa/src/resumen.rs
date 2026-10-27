//! **El resumen de una partida** (H1): en qué puesto acabó cada uno, cuándo
//! y por quién cayó, cuántas rondas y cuánto duró. Se cuenta a partir del
//! registro (el orden exacto de las eliminaciones solo está ahí, no en
//! `Eliminacion`, que apunta la ronda) y lo usan el diálogo de «Terminar», la
//! pantalla del final y el historial. La pantalla no recalcula nada.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::accion::{Actor, Cambio};
use crate::ids::{IdEquipo, IdJugador};
use crate::modelo::{ComoGano, Partida};
use crate::registro::Registro;
use crate::Accion;

/// Dónde acabó un jugador.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Puesto {
    pub jugador: IdJugador,
    /// 1 = ganó (varios pueden compartirlo: empate o un mismo equipo).
    pub puesto: u32,
    /// La ronda en que cayó, si cayó.
    pub ronda: Option<u32>,
    /// Quién lo eliminó, si se sabe.
    pub por: Option<IdJugador>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResumenPartida {
    /// Del primero al último; a igualdad, por asiento.
    pub puestos: Vec<Puesto>,
    pub rondas: u32,
    pub turnos: u32,
    /// Del primer al último evento que sigue contando.
    #[ts(type = "number")]
    pub duracion_ms: u64,
    #[ts(type = "number")]
    pub empezada_ms: u64,
    #[ts(type = "number")]
    pub terminada_ms: u64,
}

/// Cuenta la partida tal como está en el registro (acabada o no: sin
/// resultado, los que siguen en pie comparten el primer puesto).
pub fn resumir(r: &Registro) -> ResumenPartida {
    let p = r.partida();
    let eventos = r.vigentes();
    let empezada_ms = eventos.first().map_or(0, |e| e.t_ms);
    let terminada_ms = eventos.last().map_or(0, |e| e.t_ms);

    // Quién cayó, en el orden en que cayó.
    let mut orden: Vec<IdJugador> = Vec::new();
    for ev in &eventos {
        let Cambio::Accion { accion, .. } = &ev.cambio else {
            continue;
        };
        let cae = match accion {
            Accion::Eliminar { jugador, .. } => Some(*jugador),
            Accion::Conceder => match ev.actor {
                Actor::Jugador(j) => Some(j),
                Actor::Mesa => None,
            },
            _ => None,
        };
        if let Some(j) = cae {
            if p.jugador(j).is_some_and(|x| x.eliminado.is_some()) && !orden.contains(&j) {
                orden.push(j);
            }
        }
    }
    // Lo que el estado dice eliminado y los eventos no (por si acaso): antes.
    let mut sueltos: Vec<(u32, IdJugador)> = p
        .jugadores
        .iter()
        .filter(|x| x.eliminado.is_some() && !orden.contains(&x.id))
        .map(|x| (x.eliminado.as_ref().map_or(0, |e| e.ronda), x.id))
        .collect();
    sueltos.sort();
    orden.splice(0..0, sueltos.into_iter().map(|(_, j)| j));

    let en_pie: Vec<IdJugador> = p
        .jugadores
        .iter()
        .filter(|x| x.eliminado.is_none())
        .map(|x| x.id)
        .collect();
    let (ganadores, gana_otro) = ganadores(p, &en_pie);

    // Los grupos, del mejor al peor: ganadores, el resto de los que siguen
    // en pie y luego los eliminados, el último en caer primero.
    let mut grupos: Vec<Vec<IdJugador>> = vec![ganadores.clone()];
    let resto: Vec<IdJugador> = en_pie
        .iter()
        .copied()
        .filter(|j| !ganadores.contains(j))
        .collect();
    if !resto.is_empty() {
        grupos.push(resto);
    }
    for j in orden.iter().rev() {
        if !ganadores.contains(j) {
            grupos.push(vec![*j]);
        }
    }

    let mut puestos: Vec<Puesto> = Vec::new();
    // Si ganó alguien de fuera (el rival del solitario), el primer puesto es suyo.
    let mut arriba = u32::from(gana_otro);
    for g in grupos.iter().filter(|g| !g.is_empty()) {
        for j in g {
            let fuera = p.jugador(*j).and_then(|x| x.eliminado.as_ref());
            puestos.push(Puesto {
                jugador: *j,
                puesto: arriba + 1,
                ronda: fuera.map(|e| e.ronda),
                por: fuera.and_then(|e| e.por),
            });
        }
        arriba += g.len() as u32;
    }
    // Los compañeros de equipo acaban en el mismo puesto: el mejor de ellos.
    let equipo_de = |j: IdJugador| p.jugador(j).and_then(|x| x.equipo);
    let mut mejor: BTreeMap<IdEquipo, u32> = BTreeMap::new();
    for x in &puestos {
        if let Some(e) = equipo_de(x.jugador) {
            let m = mejor.entry(e).or_insert(x.puesto);
            *m = (*m).min(x.puesto);
        }
    }
    for x in &mut puestos {
        if let Some(m) = equipo_de(x.jugador).and_then(|e| mejor.get(&e)) {
            x.puesto = *m;
        }
    }
    let asiento = |j: IdJugador| {
        p.jugadores
            .iter()
            .position(|x| x.id == j)
            .unwrap_or(usize::MAX)
    };
    puestos.sort_by_key(|x| (x.puesto, asiento(x.jugador)));

    ResumenPartida {
        puestos,
        rondas: p.turno.ronda,
        turnos: p.turno.numero,
        duracion_ms: terminada_ms.saturating_sub(empezada_ms),
        empezada_ms,
        terminada_ms,
    }
}

/// Los del primer puesto: los que dice el resultado (más sus compañeros de
/// equipo) o, sin resultado, los que siguen en pie. El segundo valor es
/// `true` si ganó alguien que no está en la mesa («El rival (perdí)»).
fn ganadores(p: &Partida, en_pie: &[IdJugador]) -> (Vec<IdJugador>, bool) {
    let Some(fin) = p.fin.as_ref() else {
        return (en_pie.to_vec(), false);
    };
    if fin.ganadores.is_empty() {
        return if fin.como == ComoGano::Empate {
            (en_pie.to_vec(), false)
        } else {
            (Vec::new(), true)
        };
    }
    let g = p
        .jugadores
        .iter()
        .filter(|x| {
            fin.ganadores.contains(&x.id)
                || x.equipo.is_some_and(|e| {
                    fin.ganadores
                        .iter()
                        .any(|g| p.jugador(*g).and_then(|y| y.equipo) == Some(e))
                })
        })
        .map(|x| x.id)
        .collect();
    (g, false)
}
