//! **Cuándo y cómo reacciona el fantasma.** Sin IA y sin reglas: tablas,
//! como Playgroup (`inventario-playgroup.md` §10).
//!
//! Tres ocasiones, mirando la partida antes y después de cada evento del
//! jugador:
//!
//! - **lanza** algo (de la mano o el mando a la pila, o un permanente que
//!   no es tierra directo al campo) → puede contrarrestarlo;
//! - **llega** al campo un permanente que no es tierra (al resolverse o
//!   puesto a mano), o una tierra muy fuerte → puede eliminarlo, devolverlo
//!   a la mano o hacer sacrificar una criatura;
//! - **pasa el turno** con criaturas → puede barrer.
//!
//! La probabilidad de cada una es `base × ritmo de la ronda × intensidad ×
//! peso` (el de la amenaza o, en los barridos, el de cuántas criaturas hay),
//! con un tope. Las bases salen de que, en la mezcla de una partida, las
//! reacciones se repartan como en Playgroup: eliminar 58 %, contrarrestar
//! 19 %, barrer 13 %, devolver 7 %, sacrificio 3 % (lo comprueba
//! `tests/sparring.rs` con 500 partidas simuladas por intensidad). Son
//! números de partida para afinar jugando, no medidos.
//!
//! Límites del fantasma: en la ronda N tiene N manás (salvo las cartas
//! gratis); respeta antimaleficio y velo (busca otro objetivo o se espera;
//! los sacrificios y los barridos no apuntan); no destruye lo indestructible
//! (sí lo exilia); de las tierras solo apunta a las muy fuertes
//! (`amenaza::tierra_fuerte`); y como
//! mucho reacciona una vez por ronda (tres en «Despiadado»), nunca con una
//! reacción sin decidir y nunca durante el mulligan.

use std::collections::BTreeMap;

use super::amenaza::{indestructible, peso, protegida, tipos};
use super::cartas::{todas, CartaDelFantasma};
use super::{CartaFantasma, Explicacion, Intensidad, ReaccionSparring, TipoReaccion};
use crate::accion::{Accion, Actor, Cambio, Evento};
use crate::azar::Azar;
use crate::ids::{IdCarta, IdJugador};
use crate::modelo::{Partida, RefZona, Zona, ZonaComun};

/// El ritmo de la interacción por ronda (1 = el pico), del 0 al 12: tranquilo
/// al principio, el pico a mitad de partida.
const RITMO: [f64; 13] = [
    0.0, 0.15, 0.35, 0.6, 0.85, 1.0, 1.0, 0.95, 0.85, 0.75, 0.7, 0.65, 0.6,
];
/// El de los barridos: empieza hacia la ronda 4 y tiene el pico más tarde.
const RITMO_BARRIDO: [f64; 13] = [
    0.0, 0.0, 0.0, 0.1, 0.3, 0.6, 0.85, 1.0, 1.0, 0.9, 0.8, 0.75, 0.7,
];

/// La probabilidad de cada ocasión en el pico, con «Realista» y peso 1.
const BASE_LLEGA: f64 = 0.42;
const BASE_LANZA: f64 = 0.072;
const BASE_PASA: f64 = 0.052;
/// Ninguna ocasión salta con más de esto.
const TOPE: f64 = 0.9;

/// Cuánto pesa cada reacción al llegar algo (eliminar, devolver, sacrificio).
const REPARTO_LLEGA: [(TipoReaccion, u32); 3] = [
    (TipoReaccion::Eliminar, 58),
    (TipoReaccion::Devolver, 7),
    (TipoReaccion::Sacrificio, 3),
];

/// El reparto de Playgroup, para la explicación.
fn por_ciento(t: TipoReaccion) -> u32 {
    match t {
        TipoReaccion::Eliminar => 58,
        TipoReaccion::Contrarrestar => 19,
        TipoReaccion::Barrer => 13,
        TipoReaccion::Devolver => 7,
        TipoReaccion::Sacrificio => 3,
    }
}

pub fn clave_tipo(t: TipoReaccion) -> &'static str {
    match t {
        TipoReaccion::Eliminar => "eliminar",
        TipoReaccion::Contrarrestar => "contrarrestar",
        TipoReaccion::Barrer => "barrer",
        TipoReaccion::Devolver => "devolver",
        TipoReaccion::Sacrificio => "sacrificio",
    }
}

fn clave_intensidad(i: Intensidad) -> &'static str {
    match i {
        Intensidad::Apagado => "apagado",
        Intensidad::Suave => "suave",
        Intensidad::Realista => "realista",
        Intensidad::Despiadado => "despiadado",
    }
}

/// Cuánto multiplica cada intensidad.
pub fn factor(i: Intensidad) -> f64 {
    match i {
        Intensidad::Apagado => 0.0,
        Intensidad::Suave => 0.45,
        Intensidad::Realista => 1.0,
        Intensidad::Despiadado => 1.8,
    }
}

fn por_ronda(t: &[f64; 13], ronda: u32) -> f64 {
    t.get(ronda as usize)
        .or_else(|| t.last())
        .copied()
        .unwrap_or(0.0)
}

/// El multiplicador de los barridos por criaturas: 0–1 → 1; 2 → 1,3; 3+ → 2,5.
fn por_criaturas(n: usize) -> f64 {
    match n {
        0 | 1 => 1.0,
        2 => 1.3,
        _ => 2.5,
    }
}

fn pct(x: f64) -> String {
    format!("{}", (x * 100.0).round() as i64)
}

fn un_decimal(x: f64) -> String {
    format!("{:.1}", x)
}

fn explicacion(clave: &str, params: &[(&str, String)]) -> Explicacion {
    Explicacion {
        clave: clave.into(),
        params: params
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
    }
}

/// Tira con probabilidad `p` (a diezmilésimas).
fn salta(azar: &mut Azar, p: f64) -> bool {
    let umbral = (p.clamp(0.0, 1.0) * 10_000.0).round() as u64;
    azar.menor_que(10_000) < umbral
}

fn una<'a, T>(azar: &mut Azar, v: &'a [T]) -> Option<&'a T> {
    v.get(azar.menor_que(v.len() as u64) as usize)
}

/// Las criaturas del campo de `j`.
fn criaturas(p: &Partida, j: IdJugador) -> Vec<IdCarta> {
    p.zona(&RefZona::de(j, Zona::Campo))
        .iter()
        .copied()
        .filter(|id| {
            p.cartas
                .get(id)
                .is_some_and(|c| c.boca_abajo || tipos(&c.origen.info).criatura)
        })
        .collect()
}

/// Lo que se ha movido con el evento: lo lanzado y lo que ha llegado al campo.
fn movidas(antes: &Partida, despues: &Partida, j: IdJugador) -> (Vec<IdCarta>, Vec<IdCarta>) {
    let mut lanzadas = Vec::new();
    let mut llegadas = Vec::new();
    let campo = RefZona::de(j, Zona::Campo);
    let pila = RefZona::comun(ZonaComun::Pila);
    let de_la_mano = |z: &RefZona| z.es(Zona::Mano) || z.es(Zona::Mando);
    for (id, c) in &despues.cartas {
        let Some((ahora, _)) = despues.donde(*id) else {
            continue;
        };
        let antes_z = antes.donde(*id).map(|(z, _)| z);
        if antes_z == Some(ahora) || c.es_ficha || c.habilidad.is_some() {
            continue;
        }
        let info = &c.origen.info;
        let tierra = tipos(info).tierra;
        let desde_la_mano = antes_z.is_some_and(|z| z.jugador() == Some(j) && de_la_mano(&z));
        if ahora == pila && desde_la_mano {
            lanzadas.push(*id);
        }
        if ahora == campo {
            // Las tierras no se lanzan; llegar, sí (las fuertes se quitan).
            if desde_la_mano && !tierra {
                lanzadas.push(*id);
            }
            llegadas.push(*id);
        }
    }
    (lanzadas, llegadas)
}

/// La reacción del fantasma a un evento del jugador, si la hay. `antes` es la
/// partida antes de aplicar `ev` y `despues`, la de ahora; la reacción lleva
/// de id el `seq` del evento que la traerá (`despues.seq + 1`).
pub fn reaccionar(
    antes: &Partida,
    despues: &Partida,
    ev: &Evento,
    azar: &mut Azar,
) -> Option<ReaccionSparring> {
    let s = &despues.sparring;
    let intensidad = s.intensidad;
    let Actor::Jugador(j) = ev.actor else {
        return None;
    };
    let Cambio::Accion { accion, .. } = &ev.cambio else {
        return None;
    };
    if intensidad == Intensidad::Apagado
        || despues.jugadores.len() != 1
        || despues.mulligan.is_some()
        || despues.fin.is_some()
        || s.reacciones.iter().any(|r| r.decision.is_none())
        || matches!(
            accion,
            Accion::DecidirSparring { .. } | Accion::IntensidadSparring { .. }
        )
    {
        return None;
    }
    let ronda = despues.turno.ronda;
    let tope = if intensidad == Intensidad::Despiadado {
        3
    } else {
        1
    };
    if s.reacciones.iter().filter(|r| r.ronda == ronda).count() >= tope {
        return None;
    }
    let mut ctx = Contexto {
        p: despues,
        j,
        ronda,
        intensidad,
        id: despues.seq.saturating_add(1),
        azar,
    };
    if matches!(accion, Accion::PasarTurno) {
        return ctx.barrer();
    }
    let (lanzadas, llegadas) = movidas(antes, despues, j);
    if let Some(&id) = lanzadas.last() {
        if let Some(r) = ctx.contrarrestar(id) {
            return Some(r);
        }
    }
    // Lo último que llegó y pesa algo (una tierra básica no cuenta).
    let id = llegadas
        .iter()
        .rev()
        .copied()
        .find(|c| ctx.peso_de(*c) > 0.0)?;
    ctx.al_llegar(id)
}

struct Contexto<'a> {
    p: &'a Partida,
    j: IdJugador,
    ronda: u32,
    intensidad: Intensidad,
    id: u64,
    azar: &'a mut Azar,
}

impl Contexto<'_> {
    fn nombre(&self, id: IdCarta) -> String {
        self.p
            .cartas
            .get(&id)
            .map(|c| c.origen.nombre.clone())
            .unwrap_or_default()
    }

    fn peso_de(&self, id: IdCarta) -> f64 {
        self.p
            .cartas
            .get(&id)
            .map_or(0.0, |c| peso(&c.origen.info, &c.origen.nombre, c.es_ficha))
    }

    /// Las cartas del fantasma de este tipo que puede pagar en esta ronda.
    fn pagables(&self, tipo: TipoReaccion) -> Vec<&'static CartaDelFantasma> {
        todas()
            .iter()
            .filter(|c| c.tipo == tipo && c.se_puede_pagar(self.ronda))
            .collect()
    }

    /// Las que pueden con `id` (tipo, coste, antimaleficio, indestructible).
    fn validas_para(&self, tipo: TipoReaccion, id: IdCarta) -> Vec<&'static CartaDelFantasma> {
        let Some(c) = self.p.cartas.get(&id) else {
            return Vec::new();
        };
        let info = &c.origen.info;
        let prot = protegida(info, Some(c));
        let indes = indestructible(info, Some(c));
        self.pagables(tipo)
            .into_iter()
            .filter(|f| f.vale_para(info))
            .filter(|f| !prot || f.no_apunta)
            .filter(|f| !(indes && tipo == TipoReaccion::Eliminar && !f.exilia))
            .collect()
    }

    fn carta(f: &CartaDelFantasma) -> CartaFantasma {
        CartaFantasma {
            nombre: f.nombre.clone(),
            // Va por nombre: la app la busca en Scryfall al enseñarla.
            scryfall_id: String::new(),
            coste: f.coste.clone(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn reaccion(
        &self,
        tipo: TipoReaccion,
        objetivo: Vec<IdCarta>,
        f: &CartaDelFantasma,
        ritmo: f64,
        prob: f64,
        amenaza: Option<(IdCarta, f64)>,
        criaturas: Option<(usize, f64)>,
    ) -> ReaccionSparring {
        let ronda = self.ronda.to_string();
        let mut por_que = vec![("ronda", ronda.clone()), ("ritmo", pct(ritmo))];
        let mut base = vec![
            explicacion(
                "reparto",
                &[
                    ("tipo", clave_tipo(tipo).into()),
                    ("pct", por_ciento(tipo).to_string()),
                ],
            ),
            explicacion("ritmo", &[("ronda", ronda), ("pct", pct(ritmo))]),
        ];
        if let Some((id, w)) = amenaza {
            por_que.push(("carta", self.nombre(id)));
            por_que.push(("peso", un_decimal(w)));
            base.push(explicacion(
                "amenaza",
                &[("carta", self.nombre(id)), ("peso", un_decimal(w))],
            ));
        }
        if let Some((n, m)) = criaturas {
            por_que.push(("criaturas", n.to_string()));
            base.push(explicacion(
                "criaturas",
                &[("n", n.to_string()), ("mult", un_decimal(m))],
            ));
        }
        base.push(explicacion(
            "intensidad",
            &[("intensidad", clave_intensidad(self.intensidad).into())],
        ));
        base.push(explicacion("probabilidad", &[("pct", pct(prob))]));
        base.push(explicacion("modelo", &[]));
        ReaccionSparring {
            id: self.id,
            tipo,
            objetivo,
            carta: Self::carta(f),
            exilia: f.exilia,
            por_que: explicacion(clave_tipo(tipo), &por_que),
            base,
        }
    }

    fn contrarrestar(&mut self, id: IdCarta) -> Option<ReaccionSparring> {
        let w = self.peso_de(id);
        let ritmo = por_ronda(&RITMO, self.ronda);
        let prob = (BASE_LANZA * ritmo * factor(self.intensidad) * w.min(2.0)).min(TOPE);
        if w <= 0.0 || !salta(self.azar, prob) {
            return None;
        }
        // Un contrahechizo apunta a un hechizo: el antimaleficio no lo para.
        let info = &self.p.cartas.get(&id)?.origen.info;
        let validas: Vec<_> = self
            .pagables(TipoReaccion::Contrarrestar)
            .into_iter()
            .filter(|f| f.vale_para(info))
            .collect();
        let f = *una(self.azar, &validas)?;
        Some(self.reaccion(
            TipoReaccion::Contrarrestar,
            vec![id],
            f,
            ritmo,
            prob,
            Some((id, w)),
            None,
        ))
    }

    fn al_llegar(&mut self, id: IdCarta) -> Option<ReaccionSparring> {
        let w = self.peso_de(id);
        let ritmo = por_ronda(&RITMO, self.ronda);
        let prob = (BASE_LLEGA * ritmo * factor(self.intensidad) * w.min(2.0)).min(TOPE);
        if w <= 0.0 || !salta(self.azar, prob) {
            return None;
        }
        let hay_criaturas = !criaturas(self.p, self.j).is_empty();
        let opciones: Vec<(TipoReaccion, u32)> = REPARTO_LLEGA
            .iter()
            .copied()
            .filter(|(t, _)| *t != TipoReaccion::Sacrificio || hay_criaturas)
            .collect();
        let total: u32 = opciones.iter().map(|(_, k)| k).sum();
        let mut x = self.azar.menor_que(u64::from(total.max(1))) as u32;
        let tipo = opciones
            .iter()
            .find(|(_, k)| {
                if x < *k {
                    true
                } else {
                    x -= k;
                    false
                }
            })
            .map_or(TipoReaccion::Eliminar, |(t, _)| *t);

        if tipo == TipoReaccion::Sacrificio {
            let pagables = self.pagables(TipoReaccion::Sacrificio);
            let f = *una(self.azar, &pagables)?;
            return Some(self.reaccion(tipo, Vec::new(), f, ritmo, prob, Some((id, w)), None));
        }
        // Primero lo que acaba de llegar; si no se puede (antimaleficio,
        // indestructible, nada que responda), lo que más pese del campo.
        let campo = RefZona::de(self.j, Zona::Campo);
        let mut otras: Vec<(IdCarta, f64)> = self
            .p
            .zona(&campo)
            .iter()
            .filter(|c| **c != id)
            .map(|c| (*c, self.peso_de(*c)))
            .filter(|(_, w)| *w > 0.0)
            .collect();
        otras.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        for (objetivo, wo) in std::iter::once((id, w)).chain(otras) {
            let validas = self.validas_para(tipo, objetivo);
            if let Some(f) = una(self.azar, &validas).copied() {
                return Some(self.reaccion(
                    tipo,
                    vec![objetivo],
                    f,
                    ritmo,
                    prob,
                    Some((objetivo, wo)),
                    None,
                ));
            }
        }
        None
    }

    fn barrer(&mut self) -> Option<ReaccionSparring> {
        let todas = criaturas(self.p, self.j);
        if todas.is_empty() {
            return None;
        }
        let mult = por_criaturas(todas.len());
        let ritmo = por_ronda(&RITMO_BARRIDO, self.ronda);
        let prob = (BASE_PASA * ritmo * factor(self.intensidad) * mult).min(TOPE);
        if !salta(self.azar, prob) {
            return None;
        }
        let pagables = self.pagables(TipoReaccion::Barrer);
        let f = *una(self.azar, &pagables)?;
        // Lo indestructible se queda si el barrido destruye (no si exilia).
        let objetivo: Vec<IdCarta> = todas
            .iter()
            .copied()
            .filter(|id| {
                f.exilia
                    || self
                        .p
                        .cartas
                        .get(id)
                        .is_some_and(|c| !indestructible(&c.origen.info, Some(c)))
            })
            .collect();
        if objetivo.is_empty() {
            return None;
        }
        Some(self.reaccion(
            TipoReaccion::Barrer,
            objetivo,
            f,
            ritmo,
            prob,
            None,
            Some((todas.len(), mult)),
        ))
    }
}
