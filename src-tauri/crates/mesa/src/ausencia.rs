//! **Lo que se hace por quien se ha ido** (G8): en el mulligan, quedarse con
//! la mano que tiene. Puro, como el resto: lo manda el anfitrión en nombre
//! del jugador ausente (`Sesion::suplir`).

use crate::accion::Accion;
use crate::formato::ModoMulligan;
use crate::ids::IdJugador;
use crate::modelo::{Partida, RefZona, Zona};

/// Cuántas cartas van al fondo al quedarse (`None`: las que se quieran).
pub fn toca_al_fondo(modo: ModoMulligan, mulligans: u8) -> Option<usize> {
    match modo {
        ModoMulligan::Londres => Some(usize::from(mulligans)),
        ModoMulligan::LondresPrimeroGratis => Some(usize::from(mulligans.saturating_sub(1))),
        ModoMulligan::Amistoso => Some(0),
        ModoMulligan::Libre => None,
    }
}

/// «Quedarse con la mano» por `j`, si la partida está en el mulligan y aún no
/// ha decidido: las últimas cartas de la mano van al fondo si le tocan.
pub fn quedarse_por_defecto(p: &Partida, j: IdJugador) -> Option<Accion> {
    let m = p.mulligan.as_ref()?;
    let x = p.jugador(j)?;
    if x.eliminado.is_some() || m.decididos.contains(&j) {
        return None;
    }
    let mano = p.zona(&RefZona::de(j, Zona::Mano));
    let toca = toca_al_fondo(p.opciones.mulligan, x.mulligans).map_or(0, |t| t.min(mano.len()));
    Some(Accion::QuedarseCon {
        al_fondo: mano[mano.len() - toca..].to_vec(),
    })
}
