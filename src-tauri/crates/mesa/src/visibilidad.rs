//! **Quién ve qué.** Las reglas de visibilidad, en un solo sitio.
//!
//! Una carta se ve si lo dice su zona o si alguien se la ha enseñado
//! (`Carta::vista_por`):
//! - biblioteca, planes y planos: nadie (salvo «de arriba revelada»: la de
//!   arriba, todos);
//! - mano y banquillo: el dueño (la mano, todos si la revela);
//! - campo, cementerio, exilio, mando, pila, plano activo y planes en
//!   marcha: todos, salvo lo que esté boca abajo (eso, solo quien diga
//!   `vista_por`: el controlador al jugarla boca abajo, nadie si se exilió
//!   «oculta»).
//!
//! Al cambiar de zona, `vista_por` vuelve a `Nadie` (`aplicar.rs`).

use crate::ids::IdJugador;
use crate::modelo::{Carta, Partida, RefZona, Zona};

/// Quién mira la mesa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirador {
    Jugador(IdJugador),
    Espectador,
    /// Todo a la vista: solo la repetición de una partida terminada.
    Todo,
}

impl Mirador {
    pub fn jugador(&self) -> Option<IdJugador> {
        match self {
            Mirador::Jugador(j) => Some(*j),
            _ => None,
        }
    }
}

/// Si el mirador ve qué cartas hay en la zona (aunque alguna esté boca
/// abajo). Si no, de esa zona solo sabe cuántas hay.
pub fn zona_abierta(p: &Partida, zona: &RefZona, m: Mirador) -> bool {
    if m == Mirador::Todo {
        return true;
    }
    if zona.es_publica() {
        return true;
    }
    let yo = m.jugador();
    match zona {
        RefZona::De {
            jugador,
            zona: Zona::Mano,
        } => yo == Some(*jugador) || p.jugador(*jugador).is_some_and(|j| j.mano_revelada),
        RefZona::De {
            jugador,
            zona: Zona::Banquillo,
        } => yo == Some(*jugador),
        _ => false,
    }
}

/// Si el mirador conoce la cara de esta carta, que está en `zona`, sitio
/// `indice`.
pub fn ve(p: &Partida, c: &Carta, zona: &RefZona, indice: usize, m: Mirador) -> bool {
    if m == Mirador::Todo {
        return true;
    }
    let yo = m.jugador();
    let enseñada = c.vista_por.incluye(yo, c.dueno);
    if zona.es_publica() {
        return !c.boca_abajo || enseñada;
    }
    if enseñada {
        return true;
    }
    match zona {
        RefZona::De {
            zona: Zona::Mano | Zona::Banquillo,
            ..
        } => zona_abierta(p, zona, m),
        RefZona::De {
            jugador,
            zona: Zona::Biblioteca,
        } => indice == 0 && p.jugador(*jugador).is_some_and(|j| j.de_arriba_revelada),
        _ => false,
    }
}
