//! **Resolver una intención**: de lo que pide un jugador a un evento.
//!
//! Aquí se mira si el actor puede tocar lo que nombra y se decide el azar
//! (una semilla nueva para el evento, o las tiradas de los dados). Que todo
//! lo nombrado exista y encaje lo comprueba `aplicar` al aplicarlo (el
//! registro lo aplica sobre una copia antes de darlo por bueno).
//!
//! Permisos de fábrica (como en una mesa de verdad, pero cada uno lo suyo):
//! las cartas que controla, su mano, su biblioteca y lo suyo de las demás
//! zonas; la vida de cualquiera, las flechas, las señales y el chat, todos.
//! «Todos tocan todo» (`Opciones::todos_tocan_todo`) lo abre. Con
//! `Opciones::campo_ajeno_protegido` (red) el campo de los demás se cierra: solo
//! se toca lo que uno controla.

use crate::accion::{Accion, Actor, Cambio, Evento, Intencion, Tirada};
use crate::aplicar::cara_planar;
use crate::azar::Azar;
use crate::error::Rechazo;
use crate::ids::{IdCarta, IdJugador};
use crate::modelo::{Extremo, Partida, RefZona, Zona};

/// Caras de dado que se pueden tirar.
pub const CARAS: [u8; 7] = [4, 6, 8, 10, 12, 20, 100];

pub fn resolver(
    p: &Partida,
    i: &Intencion,
    azar: &mut Azar,
    ahora_ms: u64,
) -> Result<Evento, Rechazo> {
    let yo = i.actor;
    if p.jugador(yo).is_none() {
        return Err(Rechazo::NoExiste);
    }
    if p.fin.is_some() && !matches!(i.accion, Accion::Chat { .. } | Accion::Aviso { .. }) {
        return Err(Rechazo::NoSePuede);
    }
    permisos(p, yo, &i.accion)?;
    let cambio = match &i.accion {
        Accion::TirarDados { dados } => {
            let total: u32 = dados.iter().map(|(_, n)| u32::from(*n)).sum();
            if dados.is_empty() || total == 0 || total > 100 {
                return Err(Rechazo::Limite);
            }
            let mut tiradas = Vec::new();
            for (caras, n) in dados {
                if !CARAS.contains(caras) {
                    return Err(Rechazo::Limite);
                }
                for _ in 0..*n {
                    let valor = azar.entre(u32::from(*caras)) as u8;
                    tiradas.push(Tirada {
                        caras: *caras,
                        valor,
                    });
                }
            }
            Cambio::Dados {
                tiradas,
                semilla_animacion: azar.next_u64() as u32,
            }
        }
        Accion::Moneda {
            cuantas,
            hasta_perder,
            pulgares,
        } => {
            if *cuantas == 0 || *cuantas > 50 || *pulgares > 4 {
                return Err(Rechazo::Limite);
            }
            // Con N pulgares de Krark se lanzan 2^N y se elige.
            let a_la_vez = 1usize << pulgares;
            let mut lanzamientos = Vec::new();
            let veces = if *hasta_perder {
                100
            } else {
                *cuantas as usize
            };
            for _ in 0..veces {
                let l: Vec<bool> = (0..a_la_vez).map(|_| azar.moneda()).collect();
                let gana = l.iter().any(|x| *x);
                lanzamientos.push(l);
                if *hasta_perder && !gana {
                    break;
                }
            }
            Cambio::Monedas {
                lanzamientos,
                hasta_perder: *hasta_perder,
                semilla_animacion: azar.next_u64() as u32,
            }
        }
        Accion::DadoPlanar => Cambio::DadoPlanar {
            cara: cara_planar(azar.entre(6)),
            coste: p.turno.tiradas_planar,
            semilla_animacion: azar.next_u64() as u32,
        },
        Accion::Reiniciar | Accion::Deshacer { .. } | Accion::Rehacer => {
            return Err(Rechazo::NoSePuede)
        }
        otra => Cambio::Accion {
            accion: otra.clone(),
            azar: necesita_azar(otra).then(|| azar.next_u64()),
        },
    };
    Ok(Evento {
        seq: p.seq.saturating_add(1),
        actor: Actor::Jugador(yo),
        t_ms: ahora_ms,
        cambio,
    })
}

/// Las acciones que barajan o eligen al azar.
pub fn necesita_azar(a: &Accion) -> bool {
    use crate::accion::{ModoDescarte, RestoExcavar, Sitio};
    match a {
        Accion::PorJugador { accion, .. } => necesita_azar(accion),
        Accion::Mover { a, .. } => matches!(a.sitio, Sitio::AlAzar | Sitio::BarajarDentro),
        Accion::ResolverExcavacion { a, .. } => *a == RestoExcavar::AbajoAlAzar,
        Accion::Descartar { modo } => matches!(modo, ModoDescarte::AlAzar(_)),
        Accion::Barajar
        | Accion::Rueda { .. }
        | Accion::Mulligan
        | Accion::Resolver { .. }
        | Accion::AlAzarDe { .. }
        | Accion::TodoAlFondoAlAzar { .. }
        | Accion::BarajarTodoDentro { .. } => true,
        _ => false,
    }
}

/// Si el actor puede tocar esta carta. **Como en una mesa de verdad, cada uno
/// puede hacer lo que quiera** (girar, poner contadores, destruir, exiliar o
/// devolver lo de otro, contrarrestar su hechizo, reanimar de su cementerio…):
/// lo que está a la vista (campo, cementerio, exilio, mando, pila) es de
/// todos. Lo oculto (mano, biblioteca, banquillo) solo lo de uno, o lo de otro
/// que ya se ve (mirado, revelado): la información oculta es lo único que se
/// guarda. Lo que se mande a una mano, biblioteca o cementerio va siempre a la
/// de su dueño.
pub fn puede_tocar(p: &Partida, yo: IdJugador, id: IdCarta) -> Result<(), Rechazo> {
    let c = p.cartas.get(&id).ok_or(Rechazo::NoExiste)?;
    if p.opciones.todos_tocan_todo {
        return Ok(());
    }
    let (zona, indice) = p.donde(id).ok_or(Rechazo::NoExiste)?;
    // En red, lo que está en el campo es de quien lo controla (se coge con
    // `TomarControl`, que todos pueden hacer).
    if p.opciones.campo_ajeno_protegido
        && matches!(
            zona,
            RefZona::De {
                zona: Zona::Campo,
                ..
            }
        )
        && c.controlador != yo
    {
        return Err(Rechazo::SinPermiso);
    }
    if zona.es_publica() || zona.jugador() == Some(yo) {
        return Ok(());
    }
    if crate::visibilidad::ve(
        p,
        c,
        &zona,
        indice,
        crate::visibilidad::Mirador::Jugador(yo),
    ) {
        Ok(())
    } else {
        Err(Rechazo::SinPermiso)
    }
}

fn todas(p: &Partida, yo: IdJugador, ids: &[IdCarta]) -> Result<(), Rechazo> {
    for id in ids {
        puede_tocar(p, yo, *id)?;
    }
    Ok(())
}

/// Las zonas de cualquier jugador se pueden mirar o tocar (lo que se ve
/// después lo decide `visibilidad`): basta con que exista.
fn suya(p: &Partida, _yo: IdJugador, de: IdJugador) -> Result<(), Rechazo> {
    p.jugador(de).map(|_| ()).ok_or(Rechazo::NoExiste)
}

fn permisos(p: &Partida, yo: IdJugador, a: &Accion) -> Result<(), Rechazo> {
    match a {
        // Lo que está en el campo es público: quien quiera lo toma (se ve en el
        // registro y, si hace falta, el dueño lo recupera igual).
        Accion::TomarControl { cartas, .. } => {
            for id in cartas {
                p.cartas.get(id).ok_or(Rechazo::NoExiste)?;
            }
            Ok(())
        }
        Accion::PorJugador { jugador, accion } => {
            if p.jugador(*jugador).is_none() {
                return Err(Rechazo::NoExiste);
            }
            if !accion.se_hace_por_otro() {
                return Err(Rechazo::NoSePuede);
            }
            permisos(p, *jugador, accion)
        }
        Accion::Colocar { cartas, .. } => {
            let ids: Vec<IdCarta> = cartas.iter().map(|(c, _)| *c).collect();
            todas(p, yo, &ids)
        }
        Accion::Mover { cartas, .. }
        | Accion::Girar { cartas, .. }
        | Accion::BocaAbajo { cartas, .. }
        | Accion::Contador { cartas, .. }
        | Accion::ContadorClave { cartas, .. }
        | Accion::MasXMasY { cartas, .. }
        | Accion::FaseFuera { cartas, .. }
        | Accion::NoEndereza { cartas, .. }
        | Accion::Resaltar { cartas, .. }
        | Accion::CopiaFicha { cartas }
        | Accion::DarControl { cartas, .. }
        | Accion::RevelarA { cartas, .. }
        | Accion::MostrarATodos { cartas }
        | Accion::DejarDeMostrar { cartas }
        | Accion::Amontonar { cartas }
        | Accion::Agrupar { cartas, .. } => todas(p, yo, cartas),
        Accion::CambiarCara { carta }
        | Accion::FijarContador { carta, .. }
        | Accion::MultiplicarContadores { carta, .. }
        | Accion::Pt { carta, .. }
        | Accion::Etiqueta { carta, .. }
        | Accion::Adjuntar { carta, .. }
        | Accion::HabilidadEnPila { carta, .. }
        | Accion::JugarBocaAbajo { carta, .. }
        | Accion::Resolver { carta, .. }
        | Accion::ApilarDespues { carta }
        | Accion::AbandonarPlan { carta } => puede_tocar(p, yo, *carta),
        Accion::SacarDelMonton { monton, .. }
        | Accion::CambiarMonton { monton, .. }
        | Accion::Separar { monton } => puede_tocar(p, yo, *monton),
        Accion::Ordenar {
            de,
            arriba,
            abajo,
            cementerio,
            exilio,
        } => {
            suya(p, yo, *de)?;
            let _ = (arriba, abajo, cementerio, exilio);
            Ok(())
        }
        Accion::Mirar { de, .. } | Accion::Buscar { de } | Accion::MirarMano { de, .. } => {
            suya(p, yo, *de)
        }
        Accion::ResolverExcavacion {
            elegidas, resto, ..
        } => {
            for (c, _) in elegidas {
                puede_tocar(p, yo, *c)?;
            }
            todas(p, yo, resto)
        }
        Accion::Flecha { de, .. } => match de {
            // Una flecha sale de algo mío (o de mí).
            Extremo::Carta(c) => {
                p.cartas.get(c).ok_or(Rechazo::NoExiste)?;
                Ok(())
            }
            Extremo::Jugador(_) | Extremo::Fantasma => Ok(()),
        },
        // Cada uno toca solo sus notas.
        Accion::EditarNota { id, .. } | Accion::QuitarNota { id } | Accion::AtarNota { id, .. } => {
            let n = p
                .notas
                .iter()
                .find(|n| n.id == *id)
                .ok_or(Rechazo::NoExiste)?;
            if n.autor == yo || p.opciones.todos_tocan_todo {
                Ok(())
            } else {
                Err(Rechazo::SinPermiso)
            }
        }
        Accion::PasarTurno => {
            if p.turno.activo == yo || p.opciones.todos_tocan_todo {
                Ok(())
            } else {
                Err(Rechazo::FueraDeTurno)
            }
        }
        Accion::Lanzamientos { comandante, .. } | Accion::SinCosteExtra { comandante, .. } => {
            let c = p.cartas.get(comandante).ok_or(Rechazo::NoExiste)?;
            suya(p, yo, c.dueno)
        }
        Accion::DecidirSparring { .. } | Accion::IntensidadSparring { .. } => {
            // El sparring es del solitario: solo con un jugador.
            if p.jugadores.len() == 1 {
                Ok(())
            } else {
                Err(Rechazo::NoSePuede)
            }
        }
        _ => Ok(()),
    }
}
