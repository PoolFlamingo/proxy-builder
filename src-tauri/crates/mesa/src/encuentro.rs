//! **El encuentro**: varias partidas seguidas con un marcador («al mejor de
//! 1, 3 o 5») y el cambio de cartas entre el mazo y el banquillo.
//!
//! No es parte de la `Partida`: la partida es una sola, y así `Accion` y
//! `Cambio` no cambian. Lo guarda quien lleva la sesión (la app) y esto son
//! solo las cuentas, puras. El marcador se calcula de los resultados ya
//! cerrados más el de la partida en curso, si ya terminó: así deshacer un
//! «Terminar partida» no deja puntos de más.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::IdJugador;
use crate::preparar::{DeckDeMesa, Seccion};

/// Las duraciones que se ofrecen.
pub const DURACIONES: [u8; 3] = [1, 3, 5];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Encuentro {
    /// 1, 3 o 5.
    pub al_mejor_de: u8,
    /// Quién ganó cada partida ya cerrada, en orden (vacío = tablas).
    pub cerradas: Vec<Vec<IdJugador>>,
}

/// Cómo va un encuentro.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoEncuentro {
    pub al_mejor_de: u8,
    /// Partidas ganadas por cada jugador, por asiento.
    pub marcador: Vec<u8>,
    /// Partidas terminadas, con la de ahora si ya acabó.
    pub jugadas: u8,
    /// Cuántas hacen falta para ganarlo.
    pub necesarias: u8,
    /// El encuentro está decidido (o no quedan partidas).
    pub terminado: bool,
    /// Quién lo ha ganado (varios en equipos; vacío = sin ganador).
    pub ganadores: Vec<IdJugador>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ErrorEncuentro {
    #[error("duración no válida")]
    Duracion,
    #[error("sale y entra deben tener las mismas cartas")]
    Tamano,
    #[error("la carta {0} no está en el mazo")]
    NoEstaEnElMazo(String),
    #[error("la carta {0} no está en el banquillo")]
    NoEstaEnElBanquillo(String),
    #[error("la carta {0} está repetida")]
    Repetida(String),
}

impl Encuentro {
    pub fn nuevo(al_mejor_de: u8) -> Result<Encuentro, ErrorEncuentro> {
        if !DURACIONES.contains(&al_mejor_de) {
            return Err(ErrorEncuentro::Duracion);
        }
        Ok(Encuentro {
            al_mejor_de,
            cerradas: Vec::new(),
        })
    }

    /// Las partidas que hacen falta para ganar: la mitad más una.
    pub fn necesarias(&self) -> u8 {
        self.al_mejor_de / 2 + 1
    }

    /// El marcador de `n` jugadores con las cerradas y, si hay, la de ahora.
    pub fn marcador(&self, n: usize, actual: Option<&[IdJugador]>) -> Vec<u8> {
        let mut m = vec![0u8; n];
        for g in self.cerradas.iter().map(Vec::as_slice).chain(actual) {
            for j in g {
                if let Some(v) = m.get_mut(usize::from(j.0)) {
                    *v = v.saturating_add(1);
                }
            }
        }
        m
    }

    /// Cómo va. `actual`: quién ganó la partida en curso, si ya terminó
    /// (vacío = tablas); `None`, sigue en juego.
    pub fn estado(&self, n: usize, actual: Option<&[IdJugador]>) -> EstadoEncuentro {
        let marcador = self.marcador(n, actual);
        let jugadas = (self.cerradas.len() + usize::from(actual.is_some())).min(255) as u8;
        let necesarias = self.necesarias();
        let ganadores: Vec<IdJugador> = marcador
            .iter()
            .enumerate()
            .filter(|(_, v)| **v >= necesarias)
            .map(|(i, _)| IdJugador(i as u8))
            .collect();
        let terminado = !ganadores.is_empty() || jugadas >= self.al_mejor_de;
        EstadoEncuentro {
            al_mejor_de: self.al_mejor_de,
            marcador,
            jugadas,
            necesarias,
            terminado,
            ganadores,
        }
    }

    /// Cierra la partida de ahora con su resultado: empieza a contar.
    pub fn cerrar(&mut self, ganadores: Vec<IdJugador>) {
        self.cerradas.push(ganadores);
    }

    /// Quién empieza la partida siguiente: el primero de los que perdieron la
    /// anterior (en torneo elige quien perdió). `None`: se decide al azar.
    pub fn primero_siguiente(n: usize, ultimos: &[IdJugador]) -> Option<IdJugador> {
        if ultimos.is_empty() {
            return None;
        }
        (0..n)
            .map(|i| IdJugador(i as u8))
            .find(|j| !ultimos.contains(j))
    }
}

/// Cambia cartas entre el mazo y el banquillo de un deck: salen `sale` (del
/// mazo) y entran `entra` (del banquillo), por el id de carta del deck. El
/// tamaño del mazo no cambia. No toca nada si algo no encaja.
pub fn cambiar(
    deck: &mut DeckDeMesa,
    sale: &[String],
    entra: &[String],
) -> Result<(), ErrorEncuentro> {
    if sale.len() != entra.len() {
        return Err(ErrorEncuentro::Tamano);
    }
    let mut vistas: Vec<&str> = Vec::with_capacity(sale.len() + entra.len());
    for id in sale.iter().chain(entra) {
        if vistas.contains(&id.as_str()) {
            return Err(ErrorEncuentro::Repetida(id.clone()));
        }
        vistas.push(id);
    }
    let esta = |id: &String, en: Seccion| {
        deck.cartas
            .iter()
            .any(|c| &c.carta_deck == id && c.seccion == en)
    };
    for id in sale {
        if !esta(id, Seccion::Mazo) {
            return Err(ErrorEncuentro::NoEstaEnElMazo(id.clone()));
        }
    }
    for id in entra {
        if !esta(id, Seccion::Banquillo) {
            return Err(ErrorEncuentro::NoEstaEnElBanquillo(id.clone()));
        }
    }
    for c in &mut deck.cartas {
        if sale.contains(&c.carta_deck) {
            c.seccion = Seccion::Banquillo;
        } else if entra.contains(&c.carta_deck) {
            c.seccion = Seccion::Mazo;
        }
    }
    Ok(())
}
