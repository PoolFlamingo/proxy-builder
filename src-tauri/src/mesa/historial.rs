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
/// recibe todos los eventos: si su copia empieza antes del primer turno (la
/// recibió en el mulligan, no al volver de un corte a media partida), guarda
/// la repetición entera como el anfitrión; si no, solo el resultado.
pub fn de_sesion(s: &Sesion) -> Option<NuevaPartida> {
    let p = s.partida();
    let fin = p.fin.as_ref()?;
    let r = resumir(&s.registro);
    let invitado = matches!(s.red, Some(RedSesion::Invitado(_)));
    let desde_el_principio = s.registro.inicial.turno.numero == 0;
    let registro = if invitado && !desde_el_principio {
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
                deck: s
                    .mazos
                    .get(i)
                    .map(|m| m.nombre.clone())
                    // Un invitado no tiene los mazos: el suyo, el que eligió.
                    .or_else(|| {
                        (j.id == s.yo && invitado)
                            .then(crate::red::mi_deck)
                            .flatten()
                            .map(|d| d.nombre)
                    })
                    .unwrap_or_default(),
                // Su nombre y su tamaño los apunta el historial al guardar.
                lista_id: s
                    .config
                    .jugadores
                    .get(i)
                    .and_then(|c| c.lista.clone())
                    .unwrap_or_default(),
                lista: String::new(),
                cartas_lista: 0,
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
        // Lo que pinta la repetición: lo del principio y lo de ahora (las
        // fichas que se crearon y se fueron no se pintan con imagen propia).
        imagenes: if registro.is_some() {
            let mut v = crate::red::imagenes::propias_de(&s.registro.inicial);
            v.extend(crate::red::imagenes::propias_de(p));
            let mut shas: Vec<String> = v.into_iter().map(|(sha, _)| sha).collect();
            shas.sort();
            shas.dedup();
            shas
        } else {
            Vec::new()
        },
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

    #[test]
    fn un_invitado_que_sigue_la_partida_desde_el_mulligan_guarda_la_repeticion_entera() {
        let mut cfg = config();
        let mut otro = cfg.jugadores[0].clone();
        otro.nombre = "Otro".into();
        cfg.jugadores.push(otro);
        let mut host = Sesion::nueva(cfg, vec![mazo(), mazo()]).unwrap();
        let inicial = host.registro.inicial.clone();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let inv = crate::red::invitado::Invitado::nuevo(1, 1, tx);
        let mut invitado = Sesion::invitada(inicial, mesa::IdJugador(1), inv);
        host.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        host.accion(
            Accion::QuedarseCon { al_fondo: vec![] },
            Some(mesa::IdJugador(1)),
        )
        .unwrap();
        host.accion(Accion::Robar { n: 1 }, None).unwrap();
        host.accion(
            Accion::Terminar {
                resultado: Resultado {
                    ganadores: vec![mesa::IdJugador(0)],
                    como: ComoGano::Combo,
                    nota: String::new(),
                },
            },
            None,
        )
        .unwrap();
        for ev in host.registro.eventos.clone() {
            invitado.recibir(ev, None);
        }
        let n = de_sesion(&invitado).unwrap();
        assert_eq!(n.modo, ModoPartida::EnLinea);
        let g = guardada::decodificar(&n.registro.expect("con registro")).unwrap();
        let s2 = guardada::sesion(g).unwrap();
        assert_eq!(mesa::huella(s2.partida()), mesa::huella(host.partida()));
    }
}
