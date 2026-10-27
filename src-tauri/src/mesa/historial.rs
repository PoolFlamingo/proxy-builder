//! **El historial al terminar una partida** (H2): cuando llega `Terminar`,
//! la sesión arma lo que hay que guardar y se lo da a quien lo guarde (la
//! base de datos, que la sesión no conoce). El gancho se instala una vez al
//! arrancar; sin él (en los tests) no pasa nada.

use std::sync::OnceLock;

use mesa::resumir;

use super::guardada;
use super::sesion::{ahora_ms, RedSesion, Sesion};
use crate::store::partidas::{JugadorHistorial, ModoPartida, NuevaPartida};

type Gancho = Box<dyn Fn(NuevaPartida) + Send + Sync>;

static GANCHO: OnceLock<Gancho> = OnceLock::new();

/// Quién recibe las partidas terminadas.
pub fn instalar(f: impl Fn(NuevaPartida) + Send + Sync + 'static) {
    let _ = GANCHO.set(Box::new(f));
}

/// Da una partida terminada a quien la guarda.
pub fn avisar(p: NuevaPartida) {
    if let Some(g) = GANCHO.get() {
        g(p);
    }
}

/// Lo que se guarda de la partida de esta sesión, si ya terminó. El invitado
/// no tiene la partida desde el principio: guarda el resultado, sin registro.
pub fn de_sesion(s: &Sesion) -> Option<NuevaPartida> {
    let p = s.partida();
    let fin = p.fin.as_ref()?;
    let r = resumir(&s.registro);
    let invitado = matches!(s.red, Some(RedSesion::Invitado(_)));
    let registro = if invitado {
        None
    } else {
        match guardada::codificar(&guardada::de_sesion(s)) {
            Ok(b) => Some(b),
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "no se pudo comprimir la partida para el historial");
                None
            }
        }
    };
    let modo = if s.red.is_some() {
        ModoPartida::EnLinea
    } else if p.jugadores.len() > 1 {
        ModoPartida::Local
    } else {
        ModoPartida::Solitario
    };
    let ahora = i64::try_from(ahora_ms()).unwrap_or(i64::MAX);
    let terminada = if r.terminada_ms == 0 {
        ahora
    } else {
        i64::try_from(r.terminada_ms).unwrap_or(ahora)
    };
    Some(NuevaPartida {
        id: p.id.clone(),
        empezada: i64::try_from(r.empezada_ms).unwrap_or(terminada),
        terminada,
        formato: p.formato,
        modo,
        jugadores: p
            .jugadores
            .iter()
            .enumerate()
            .map(|(i, j)| JugadorHistorial {
                jugador: j.id,
                nombre: j.nombre.clone(),
                color: j.color.clone(),
                deck_id: s
                    .config
                    .jugadores
                    .get(i)
                    .map(|c| c.deck_id.clone())
                    .unwrap_or_default(),
                deck: s.mazos.get(i).map(|m| m.nombre.clone()).unwrap_or_default(),
                puesto: r
                    .puestos
                    .iter()
                    .find(|x| x.jugador == j.id)
                    .map_or(0, |x| x.puesto),
            })
            .collect(),
        ganadores: fin.ganadores.clone(),
        como: fin.como,
        nota: fin.nota.clone(),
        rondas: r.rondas,
        duracion_ms: r.duracion_ms,
        registro,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesa::sesion::tests::{config, mazo};
    use mesa::modelo::{ComoGano, Resultado};
    use mesa::Accion;

    #[test]
    fn una_partida_sin_terminar_no_se_guarda_y_una_terminada_lleva_su_registro() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        assert!(de_sesion(&s).is_none());
        s.accion(
            Accion::Terminar {
                resultado: Resultado {
                    ganadores: vec![],
                    como: ComoGano::Combate,
                    nota: "uf".into(),
                },
            },
            None,
        )
        .unwrap();
        let n = de_sesion(&s).unwrap();
        assert_eq!(n.modo, ModoPartida::Solitario);
        assert_eq!(n.como, ComoGano::Combate);
        assert_eq!(n.nota, "uf");
        assert_eq!(n.jugadores.len(), 1);
        assert_eq!(n.jugadores[0].puesto, 2, "perdió contra el rival");
        // El registro se puede volver a abrir y llega al mismo punto.
        let g = guardada::decodificar(&n.registro.unwrap()).unwrap();
        let s2 = guardada::sesion(g).unwrap();
        assert_eq!(mesa::huella(s2.partida()), mesa::huella(s.partida()));
    }
}
