//! **El registro de la partida**: deshacer, rehacer y repetición.
//!
//! - `Registro { inicial, eventos, fotos }` con una foto cada 50 eventos. Lo
//!   lleva el host; un invitado que entra a media partida empieza el suyo en
//!   la instantánea que recibe (su `inicial`).
//! - **Deshacer** lo calcula el host: la partida tal como estaba antes del
//!   evento deshecho (`en(seq - 1)`) viaja entera en un
//!   `Cambio::Restaurar`. Quien lo recibe reemplaza su copia, sin necesitar
//!   fotos de antes ni «el contrario» de cada acción.
//! - **Rehacer** (solo en solitario) vuelve a resolver las intenciones
//!   deshechas: un barajado rehecho da otro orden, como en la mesa.
//! - Permisos: en solitario sin límite; en multijugador, solo la última
//!   acción propia y si nadie ha hecho nada después.

use std::collections::BTreeMap;

use crate::accion::{Accion, Actor, Cambio, Evento, Intencion};
use crate::aplicar::{aplicar, Tocado};
use crate::azar::Azar;
use crate::error::Rechazo;
use crate::ids::IdJugador;
use crate::modelo::Partida;
use crate::resolver::resolver;

/// Cada cuántos eventos se guarda una foto de la partida.
pub const CADA_FOTO: u64 = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoDeshacer {
    /// Solitario (o partida local): todo lo que se quiera.
    SinLimite,
    /// En red: solo la última acción propia.
    UltimaPropia,
}

/// Un evento metido en el registro y lo que tocó.
#[derive(Debug, Clone)]
pub struct Hecho {
    pub evento: Evento,
    pub tocado: Tocado,
}

#[derive(Debug, Clone)]
pub struct Registro {
    pub inicial: Partida,
    pub eventos: Vec<Evento>,
    pub fotos: Vec<(u64, Partida)>,
    pub modo: ModoDeshacer,
    actual: Partida,
    /// Los `seq` que se pueden deshacer, en orden.
    vivos: Vec<u64>,
    /// Lo que pidió cada evento (solo el host lo sabe).
    intenciones: BTreeMap<u64, Intencion>,
    /// Lo deshecho, para rehacer (la última, la primera en salir).
    rehacer: Vec<Intencion>,
}

fn deshacible(ev: &Evento) -> bool {
    match &ev.cambio {
        Cambio::Accion { accion, .. } => !accion.es_comunicacion() && !accion.es_nota(),
        Cambio::Restaurar { .. } => false,
        _ => true,
    }
}

impl Registro {
    pub fn nuevo(inicial: Partida, modo: ModoDeshacer) -> Self {
        Registro {
            actual: inicial.clone(),
            inicial,
            eventos: Vec::new(),
            fotos: Vec::new(),
            modo,
            vivos: Vec::new(),
            intenciones: BTreeMap::new(),
            rehacer: Vec::new(),
        }
    }

    /// Un registro guardado (la partida inicial y sus eventos), vuelto a
    /// aplicar. Lo que se pierde es solo lo que no viaja en los eventos: las
    /// intenciones para rehacer.
    pub fn reconstruir(
        inicial: Partida,
        eventos: Vec<Evento>,
        modo: ModoDeshacer,
    ) -> Result<Self, Rechazo> {
        let mut r = Registro::nuevo(inicial, modo);
        for ev in eventos {
            r.meter(ev)?;
        }
        Ok(r)
    }

    pub fn partida(&self) -> &Partida {
        &self.actual
    }

    /// Los eventos que siguen contando, en orden: sin lo deshecho, sin lo
    /// de antes de reiniciar y sin los propios `Restaurar` (el registro que
    /// se enseña).
    pub fn vigentes(&self) -> Vec<&Evento> {
        let mut v: Vec<&Evento> = Vec::new();
        for ev in &self.eventos {
            match &ev.cambio {
                Cambio::Restaurar {
                    deshecho: Some(k), ..
                } => v.retain(|e| e.seq < *k),
                Cambio::Restaurar { deshecho: None, .. } => v.clear(),
                _ => v.push(ev),
            }
        }
        v
    }

    pub fn se_puede_rehacer(&self) -> bool {
        !self.rehacer.is_empty()
    }

    pub fn se_puede_deshacer(&self, actor: IdJugador) -> bool {
        self.a_deshacer(actor, None).is_ok()
    }

    /// Lo que hace el host con una intención: la resuelve, la aplica y la
    /// apunta (o deshace, o rehace).
    pub fn procesar(
        &mut self,
        i: &Intencion,
        azar: &mut Azar,
        ahora_ms: u64,
    ) -> Result<Hecho, Rechazo> {
        match &i.accion {
            Accion::Deshacer { hasta } => self.deshacer(i.actor, *hasta, ahora_ms),
            Accion::Rehacer => self.rehacer_una(azar, ahora_ms),
            _ => {
                let ev = resolver(&self.actual, i, azar, ahora_ms)?;
                let seq = ev.seq;
                let hecho = self.meter(ev)?;
                if deshacible(&hecho.evento) {
                    self.intenciones.insert(seq, i.clone());
                    self.rehacer.clear();
                }
                Ok(hecho)
            }
        }
    }

    /// Un evento que genera la propia mesa (la reacción del sparring).
    pub fn de_la_mesa(&mut self, cambio: Cambio, ahora_ms: u64) -> Result<Hecho, Rechazo> {
        let ev = Evento {
            seq: self.actual.seq.saturating_add(1),
            actor: Actor::Mesa,
            t_ms: ahora_ms,
            cambio,
        };
        self.meter(ev)
    }

    /// Cambia la partida entera (reiniciar): no se puede deshacer más allá.
    pub fn reemplazar(
        &mut self,
        actor: IdJugador,
        partida: Partida,
        ahora_ms: u64,
    ) -> Result<Hecho, Rechazo> {
        let ev = Evento {
            seq: self.actual.seq.saturating_add(1),
            actor: Actor::Jugador(actor),
            t_ms: ahora_ms,
            cambio: Cambio::Restaurar {
                deshecho: None,
                partida: Box::new(partida),
            },
        };
        self.rehacer.clear();
        self.meter(ev)
    }

    /// Lo que hace un invitado con lo que le manda el host.
    pub fn recibir(&mut self, ev: Evento) -> Result<Tocado, Rechazo> {
        Ok(self.meter(ev)?.tocado)
    }

    fn meter(&mut self, ev: Evento) -> Result<Hecho, Rechazo> {
        let tocado = aplicar(&mut self.actual, &ev)?;
        match &ev.cambio {
            Cambio::Restaurar {
                deshecho: Some(k), ..
            } => self.vivos.retain(|s| s < k),
            Cambio::Restaurar { deshecho: None, .. } => self.vivos.clear(),
            _ if deshacible(&ev) => self.vivos.push(ev.seq),
            _ => {}
        }
        if ev.seq.is_multiple_of(CADA_FOTO) {
            self.fotos.push((ev.seq, self.actual.clone()));
        }
        self.eventos.push(ev.clone());
        Ok(Hecho { evento: ev, tocado })
    }

    /// Desde qué `seq` se deshace (y si se puede).
    fn a_deshacer(&self, actor: IdJugador, hasta: Option<u64>) -> Result<u64, Rechazo> {
        let ultimo = *self.vivos.last().ok_or(Rechazo::NoSePuede)?;
        match self.modo {
            ModoDeshacer::SinLimite => {
                let k = hasta.unwrap_or(ultimo);
                if self.vivos.contains(&k) {
                    Ok(k)
                } else {
                    Err(Rechazo::NoSePuede)
                }
            }
            ModoDeshacer::UltimaPropia => {
                if hasta.is_some_and(|h| h != ultimo) {
                    return Err(Rechazo::NoSePuede);
                }
                // El último evento que cuenta (sin el chat) tiene que ser
                // ese y del actor.
                let ultimo_ev = self
                    .eventos
                    .iter()
                    .rev()
                    .find(|e| match &e.cambio {
                        Cambio::Accion { accion, .. } => {
                            !accion.es_comunicacion() && !accion.es_nota()
                        }
                        _ => true,
                    })
                    .ok_or(Rechazo::NoSePuede)?;
                if ultimo_ev.seq == ultimo && ultimo_ev.actor == Actor::Jugador(actor) {
                    Ok(ultimo)
                } else {
                    Err(Rechazo::NoSePuede)
                }
            }
        }
    }

    fn deshacer(
        &mut self,
        actor: IdJugador,
        hasta: Option<u64>,
        ahora_ms: u64,
    ) -> Result<Hecho, Rechazo> {
        let k = self.a_deshacer(actor, hasta)?;
        let mut antes = self.en(k.saturating_sub(1))?;
        // Las notas no son del juego: deshacer no se las lleva.
        antes.notas = self.actual.notas.clone();
        let deshechas: Vec<Intencion> = self
            .vivos
            .iter()
            .filter(|s| **s >= k)
            .filter_map(|s| self.intenciones.get(s).cloned())
            .collect();
        let ev = Evento {
            seq: self.actual.seq.saturating_add(1),
            actor: Actor::Jugador(actor),
            t_ms: ahora_ms,
            cambio: Cambio::Restaurar {
                deshecho: Some(k),
                partida: Box::new(antes),
            },
        };
        let hecho = self.meter(ev)?;
        // Se rehacen en el orden en que se hicieron.
        for i in deshechas.into_iter().rev() {
            self.rehacer.push(i);
        }
        Ok(hecho)
    }

    fn rehacer_una(&mut self, azar: &mut Azar, ahora_ms: u64) -> Result<Hecho, Rechazo> {
        if self.modo != ModoDeshacer::SinLimite {
            return Err(Rechazo::NoSePuede);
        }
        let i = self.rehacer.last().cloned().ok_or(Rechazo::NoSePuede)?;
        let ev = resolver(&self.actual, &i, azar, ahora_ms)?;
        let seq = ev.seq;
        let hecho = self.meter(ev)?;
        self.rehacer.pop();
        self.intenciones.insert(seq, i);
        Ok(hecho)
    }

    /// La partida justo después del evento `seq` (la repetición). Con las
    /// fotos, en menos de `CADA_FOTO` aplicaciones.
    pub fn en(&self, seq: u64) -> Result<Partida, Rechazo> {
        let base = self.inicial.seq;
        if seq < base || seq > self.actual.seq {
            return Err(Rechazo::NoExiste);
        }
        let (desde, mut p) = self
            .fotos
            .iter()
            .rev()
            .find(|(s, _)| *s <= seq)
            .map(|(s, p)| (*s, p.clone()))
            .unwrap_or((base, self.inicial.clone()));
        for ev in self
            .eventos
            .iter()
            .filter(|e| e.seq > desde && e.seq <= seq)
        {
            aplicar(&mut p, ev)?;
        }
        Ok(p)
    }
}
