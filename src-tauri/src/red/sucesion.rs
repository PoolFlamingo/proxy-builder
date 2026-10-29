//! **El traspaso del anfitrión sin servidor** (Fase K). Con código directo
//! no hay nadie que vigile: lo hacen los propios jugadores.
//!
//! - **Preparados:** cada jugador sabe quién es quién (`NodoJugador`: asiento,
//!   nodo y dirección), por la `Bienvenida`; los espectadores, por su primera
//!   vista. Y cada jugador tiene la partida entera, así que cualquiera puede
//!   seguirla con su copia y el mismo secreto (el código sigue valiendo).
//! - **Buscar** (`buscar`, en el invitado que pierde al anfitrión): tras
//!   `INTENTOS_ANTES` intentos con el de antes, se prueban los demás
//!   jugadores por orden de asiento. Quien llega a su propio turno pasa a
//!   anfitrión (`hacerse_anfitrion`).
//! - **Mayoría:** un anfitrión de código directo solo deja jugar con más de
//!   la mitad de los jugadores conectados (contándose él), y se para si se
//!   queda con menos de la mitad (`vigilar`). Dos grupos que no se ven no
//!   pueden tener los dos la mayoría, así que nunca hay dos partidas a la vez.
//!   Para que no se crucen, activarse tarda más (`ACTIVAR_TRAS`) que pararse
//!   (`PARAR_TRAS`).
//! - **Juntarse:** un anfitrión parado prueba cada `CEDER_CADA` a unirse a
//!   otro: a uno de asiento anterior aunque tampoco tenga la mayoría, a uno
//!   posterior solo si ya juega (`solo_si_activo`). Así nunca se unen dos el
//!   uno al otro. Al unirse, suelta a los suyos, que buscan de nuevo.
//! - Con dos jugadores, el que queda nunca tiene la mayoría (1 de 2): espera a
//!   que vuelva el otro.
//!
//! Con el servidor de salas no se usa nada de esto: elige él (`servidor/sala.rs`).

use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use iroh::{Endpoint, EndpointAddr, EndpointId};
use mesa::IdJugador;

use super::anfitrion::{atender, vigilar_ausentes, Anfitrion, ESPERA_MULLIGAN};
use super::codigo::Codigo;
use super::invitado::{conectar, seguir_con, Primera, Union};
use super::mesa::{AvisoRed, Mesa};
use super::protocolo::{NodoJugador, Rol};
use crate::error::ErrorCode;
use crate::mesa::sesion::RedSesion;

/// La unidad de los plazos (en los tests, mucho más corta).
pub const UNIDAD: Duration = if cfg!(test) {
    Duration::from_millis(40)
} else {
    Duration::from_secs(1)
};
/// Intentos con el anfitrión de antes (1 + 2 + 4 unidades) antes de buscar otro.
pub const INTENTOS_ANTES: u32 = 3;
/// Lo que se prueba con cada candidato.
const ESPERA_CANDIDATO: Duration = UNIDAD.saturating_mul(10);
/// Lo que espera cada intento de conexión.
pub const ESPERA_INTENTO: Duration = UNIDAD.saturating_mul(3);
/// Con menos de la mitad, el anfitrión se para tras esto…
const PARAR_TRAS: Duration = UNIDAD.saturating_mul(2);
/// … y con la mayoría se activa tras esto (más: que no se crucen).
const ACTIVAR_TRAS: Duration = UNIDAD.saturating_mul(4);
/// Cada cuánto prueba un anfitrión parado a unirse a otro.
const CEDER_CADA: Duration = UNIDAD.saturating_mul(5);
const CADA_VIGIA: Duration = Duration::from_millis(if cfg!(test) { 20 } else { 500 });

/// Si esta unión puede buscar otro anfitrión sola (código directo).
pub fn se_puede_traspasar(u: &Union) -> bool {
    !u.codigo.secreto.is_empty()
}

/// La dirección de un jugador: la que dio (si es de su nodo) o, si no, solo
/// su nodo (iroh lo busca).
pub fn direccion_de(n: &NodoJugador) -> Option<EndpointAddr> {
    let nodo = n.nodo.parse::<EndpointId>().ok()?;
    let dada = n
        .direccion
        .as_deref()
        .and_then(|d| serde_json::from_str::<EndpointAddr>(d).ok())
        .filter(|a| a.id == nodo);
    Some(dada.unwrap_or_else(|| EndpointAddr::from(nodo)))
}

/// Lo que dice un anfitrión que se va a propósito (`DelHost::Traspaso`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Traspaso {
    /// El asiento del que se va: ya no cuenta para la mayoría.
    pub ido: IdJugador,
    /// Quien sigue de anfitrión.
    pub sucesor: IdJugador,
}

/// Los candidatos a seguir de anfitrión, por orden de asiento, sin el que
/// se fue. Si el anfitrión dijo quién sigue, ese va el primero.
pub fn candidatos(
    nodos: &[NodoJugador],
    sin: &str,
    sucesor: Option<IdJugador>,
) -> Vec<NodoJugador> {
    let mut v: Vec<NodoJugador> = nodos.iter().filter(|n| n.nodo != sin).cloned().collect();
    v.sort_by_key(|n| (Some(n.jugador) != sucesor, n.jugador));
    v
}

/// Lo que encontró un invitado que perdió al anfitrión.
pub enum Busqueda {
    /// Conectado con el anfitrión que sigue.
    Conectado(Primera),
    /// Esta app sigue de anfitrión.
    Anfitrion,
}

/// Lo que hace falta saber de la partida para buscar: los nodos, la época,
/// el asiento de esta app (si juega) y el último evento que tiene.
fn datos<M: Mesa>(mesa: &M) -> Option<(Vec<NodoJugador>, u32, Option<IdJugador>, u64)> {
    mesa.con(|m| {
        let s = m.as_ref()?;
        match &s.red {
            Some(RedSesion::Invitado(i)) => {
                Some((i.nodos.clone(), i.epoch, Some(s.yo), s.partida().seq))
            }
            _ => None,
        }
    })
    .or_else(|| {
        mesa.mirada(|m| {
            m.as_ref()
                .map(|v| (v.nodos.clone(), v.epoch, None, v.vista.version))
        })
    })
}

/// El invitado no recupera al anfitrión: prueba con los demás jugadores por
/// orden de asiento; si le toca a él, sigue de anfitrión.
pub async fn buscar<M: Mesa>(
    ep: &Endpoint,
    u: &mut Union,
    mesa: &M,
    traspaso: Option<Traspaso>,
) -> Option<Busqueda> {
    if !se_puede_traspasar(u) {
        return None;
    }
    let (nodos, epoch, yo, _) = datos(mesa)?;
    let perdido = u.codigo.addr.id.to_string();
    let mio = ep.id().to_string();
    let sucesor = traspaso.map(|t| t.sucesor);
    for (k, c) in candidatos(&nodos, &perdido, sucesor).iter().enumerate() {
        let epoch_c = epoch.saturating_add(1 + k as u32);
        if c.nodo == mio {
            // Los espectadores no siguen de anfitrión: no tienen la partida.
            let Some(yo) = yo.filter(|_| u.rol == Rol::Jugador) else {
                continue;
            };
            hacerse_anfitrion(ep, u, mesa, epoch_c, yo, &nodos, traspaso.map(|t| t.ido));
            return Some(Busqueda::Anfitrion);
        }
        let Some(addr) = direccion_de(c) else {
            continue;
        };
        mesa.avisar(AvisoRed::BuscandoAnfitrion {
            candidato: c.jugador,
        });
        let mut v = u.clone();
        v.codigo.addr = addr;
        let hasta = Instant::now() + ESPERA_CANDIDATO;
        while Instant::now() < hasta {
            let ultimo = datos(mesa).map(|d| d.3);
            match tokio::time::timeout(ESPERA_INTENTO, conectar(ep, &v, ultimo, epoch_c, false))
                .await
            {
                Ok(Ok(primera)) => {
                    *u = v;
                    mesa.avisar(AvisoRed::NuevoAnfitrion {
                        jugador: c.jugador,
                        soy_yo: false,
                    });
                    return Some(Busqueda::Conectado(primera));
                }
                // No lo deja entrar: no es él.
                Ok(Err(e)) if e.code == ErrorCode::Rejected => break,
                _ => tokio::time::sleep(UNIDAD).await,
            }
        }
    }
    None
}

/// Esta app (un invitado) sigue la partida de anfitrión, con su copia y el
/// mismo secreto. Empieza parada hasta tener la mayoría.
fn hacerse_anfitrion<M: Mesa>(
    ep: &Endpoint,
    u: &Union,
    mesa: &M,
    epoch: u32,
    yo: IdJugador,
    nodos: &[NodoJugador],
    ido: Option<IdJugador>,
) {
    let anf = Anfitrion::desde(
        epoch,
        u.codigo.secreto.clone(),
        u.apodo.clone(),
        ep.id(),
        yo,
        nodos,
    );
    // Quien se fue a propósito no vuelve a votar: con él fuera, dos jugadores
    // (o uno solo) siguen la partida sin esperar a nadie.
    if let Some(ido) = ido {
        anf.marcar_ido(ido);
    }
    mesa.con(|m| {
        if let Some(s) = m.as_mut() {
            s.red = Some(RedSesion::Anfitrion(anf.clone()));
        }
    });
    vigilar_ausentes(&anf, mesa.clone(), ESPERA_MULLIGAN);
    // `atender` también lanza el vigía de la mayoría.
    let tarea = tokio::spawn(atender(ep.clone(), anf.clone(), mesa.clone())).abort_handle();
    let codigo = Codigo {
        addr: ep.addr(),
        secreto: u.codigo.secreto.clone(),
    }
    .a_texto()
    .unwrap_or_default();
    mesa.ser_anfitrion(anf, tarea, codigo);
    mesa.avisar(AvisoRed::NuevoAnfitrion {
        jugador: yo,
        soy_yo: true,
    });
    tracing::info!(epoch, jugador = yo.0, "esta app sigue de anfitrión");
}

/// El vigía de un anfitrión de código directo: se para sin la mayoría, se
/// activa con ella y, parado, prueba a unirse a otro.
pub async fn vigilar<M: Mesa>(anf: Weak<Anfitrion>, ep: Endpoint, mesa: M) {
    let mut minoria_desde: Option<Instant> = None;
    let mut mayoria_desde: Option<Instant> = None;
    let mut ultimo_intento: Option<Instant> = None;
    let mut contado: Option<(bool, usize, usize)> = None;
    loop {
        tokio::time::sleep(CADA_VIGIA).await;
        let Some(a) = anf.upgrade() else { return };
        if a.acabado() {
            return;
        }
        if !a.empezada() {
            continue;
        }
        if a.activo() {
            if a.minoria() {
                let desde = *minoria_desde.get_or_insert_with(Instant::now);
                if desde.elapsed() >= PARAR_TRAS {
                    tracing::warn!("el anfitrión se queda sin la mayoría: se para");
                    a.activar(false);
                    minoria_desde = None;
                }
            } else {
                minoria_desde = None;
            }
        } else if a.mayoria() {
            let desde = *mayoria_desde.get_or_insert_with(Instant::now);
            if desde.elapsed() >= ACTIVAR_TRAS {
                tracing::info!("el anfitrión tiene la mayoría: se juega");
                a.activar(true);
                mayoria_desde = None;
            }
        } else {
            mayoria_desde = None;
            let toca = ultimo_intento.is_none_or(|t| t.elapsed() >= CEDER_CADA);
            if toca {
                ultimo_intento = Some(Instant::now());
                if ceder(&a, &ep, &mesa).await {
                    return;
                }
            }
        }
        let (total, conectados) = a.recuento();
        let ahora = (a.activo(), conectados, total);
        if contado != Some(ahora) {
            contado = Some(ahora);
            mesa.avisar(AvisoRed::Mayoria {
                activa: ahora.0,
                conectados: u32::try_from(conectados).unwrap_or(u32::MAX),
                total: u32::try_from(total).unwrap_or(u32::MAX),
            });
        }
    }
}

/// Un anfitrión parado prueba a unirse a otro: a uno de asiento anterior
/// aunque esté parado, a uno posterior solo si ya juega. Si lo logra, suelta
/// a los suyos y sigue de invitado.
async fn ceder<M: Mesa>(anf: &Arc<Anfitrion>, ep: &Endpoint, mesa: &M) -> bool {
    let seq = mesa.con(|m| m.as_ref().map(|s| s.partida().seq));
    for n in anf.nodos(ep.id()) {
        if n.jugador == anf.mi_jugador {
            continue;
        }
        let Some(addr) = direccion_de(&n) else {
            continue;
        };
        let u = Union {
            codigo: Codigo {
                addr,
                secreto: anf.secreto().to_string(),
            },
            apodo: anf.apodo(),
            rol: Rol::Jugador,
            deck: None,
        };
        let solo_si_activo = n.jugador > anf.mi_jugador;
        let r = tokio::time::timeout(
            ESPERA_INTENTO,
            conectar(ep, &u, seq, anf.epoch, solo_si_activo),
        )
        .await;
        if let Ok(Ok(primera)) = r {
            tracing::info!(jugador = n.jugador.0, "el anfitrión parado se une a otro");
            anf.soltar();
            let conexion = seguir_con(ep.clone(), u, mesa.clone(), primera);
            mesa.ser_invitado(conexion, Rol::Jugador);
            mesa.avisar(AvisoRed::NuevoAnfitrion {
                jugador: n.jugador,
                soy_yo: false,
            });
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodo(j: u8, n: &str) -> NodoJugador {
        NodoJugador {
            jugador: IdJugador(j),
            nodo: n.into(),
            direccion: None,
        }
    }

    #[test]
    fn los_candidatos_van_por_asiento_y_sin_el_que_se_fue() {
        let v = vec![nodo(2, "c"), nodo(0, "a"), nodo(1, "b")];
        let c: Vec<String> = candidatos(&v, "a", None)
            .into_iter()
            .map(|n| n.nodo)
            .collect();
        assert_eq!(c, vec!["b", "c"]);
        let c: Vec<String> = candidatos(&v, "b", None)
            .into_iter()
            .map(|n| n.nodo)
            .collect();
        assert_eq!(c, vec!["a", "c"]);
        // Si el anfitrión que se va dice quién sigue, ese va el primero.
        let c: Vec<String> = candidatos(&v, "a", Some(IdJugador(2)))
            .into_iter()
            .map(|n| n.nodo)
            .collect();
        assert_eq!(c, vec!["c", "b"]);
    }

    #[test]
    fn una_direccion_de_otro_nodo_no_vale() {
        let a = iroh::SecretKey::generate().public();
        let b = iroh::SecretKey::generate().public();
        let mut n = nodo(1, &a.to_string());
        n.direccion = serde_json::to_string(&EndpointAddr::from(b)).ok();
        assert_eq!(direccion_de(&n).map(|d| d.id), Some(a));
        n.nodo = "no es un nodo".into();
        assert!(direccion_de(&n).is_none());
    }
}
