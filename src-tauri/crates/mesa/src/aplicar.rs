//! **Aplicar un evento**: de un evento resuelto a la partida nueva.
//!
//! 🛑 Es total: en red, un invitado aplica lo que le manda el host, y eso
//! también es entrada de fuera. Nada se indexa con `[]` y todo lo nombrado
//! se comprueba. Se trabaja sobre una copia y solo se cambia la partida si
//! todo ha ido bien: un `Err` la deja como estaba (el invitado pide entonces
//! una instantánea nueva).
//!
//! Sin reloj ni azar de fuera: la hora llega en el evento y el azar sale de
//! su semilla (`Cambio::Accion::azar`).

use std::collections::BTreeSet;

use crate::accion::{
    Accion, Cambio, CaraPlanar, ComoMover, ComoResolver, Designacion, Destino, Evento, FilaPt,
    FormaExcavar, Lado, ModoDescarte, ModoExilio, OrigenFicha, RestoExcavar, Sitio, ZonaDestino,
};
use crate::azar::Azar;
use crate::error::Rechazo;
use crate::formato::ModoMulligan;
use crate::ids::{IdCarta, IdJugador};
use crate::modelo::{
    Bolita, Carta, Eliminacion, Extremo, Flecha, InfoCarta, OrigenCarta, Partida, Pos, RefImagen,
    RefZona, Senal, Vista, Zona, ZonaComun, POS_MAX,
};
use crate::sparring::{Decision, ReaccionApuntada, TipoReaccion};

/// Topes: lo que pase de aquí no es una partida, es un error o un abuso.
pub const MAX_CANTIDAD: u32 = 1_000;
pub const MAX_FICHAS_SUELTAS: u32 = 100;
pub const MAX_CHAT: usize = 500;
pub const MAX_ETIQUETA: usize = 20;
pub const MAX_ETIQUETAS: usize = 4;
pub const MAX_FLECHAS: usize = 200;
pub const MAX_BOLITAS: usize = 200;
const MAX_DELTA: i32 = 1_000_000;
const MAX_TEXTO: usize = 200;

/// Lo que cambió con un evento: la pantalla solo recibe eso (`vista.rs`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tocado {
    pub cartas: BTreeSet<IdCarta>,
    pub zonas: BTreeSet<RefZona>,
    pub jugadores: BTreeSet<IdJugador>,
    /// Turno, flechas, bolitas, designaciones, fantasma, sparring, fin…
    pub general: bool,
}

impl Tocado {
    /// Todo cambió (una partida restaurada).
    pub fn todo(p: &Partida) -> Tocado {
        Tocado {
            cartas: p.cartas.keys().copied().collect(),
            zonas: p.zonas.keys().copied().collect(),
            jugadores: p.jugadores.iter().map(|j| j.id).collect(),
            general: true,
        }
    }
}

/// Aplica el evento. Si no encaja, `Err` y la partida no cambia.
pub fn aplicar(p: &mut Partida, ev: &Evento) -> Result<Tocado, Rechazo> {
    if ev.seq != p.seq.saturating_add(1) {
        return Err(Rechazo::NoSePuede);
    }
    let mut q = p.clone();
    ejecutar(&mut q, ev)?;
    q.seq = ev.seq;
    let t = diferencias(p, &q);
    *p = q;
    Ok(t)
}

fn diferencias(a: &Partida, b: &Partida) -> Tocado {
    let mut t = Tocado::default();
    for (id, c) in &b.cartas {
        if a.cartas.get(id) != Some(c) {
            t.cartas.insert(*id);
        }
    }
    for id in a.cartas.keys() {
        if !b.cartas.contains_key(id) {
            t.cartas.insert(*id);
        }
    }
    for (z, v) in &b.zonas {
        if a.zonas.get(z) != Some(v) {
            t.zonas.insert(*z);
        }
    }
    for z in a.zonas.keys() {
        if !b.zonas.contains_key(z) {
            t.zonas.insert(*z);
        }
    }
    for j in &b.jugadores {
        if a.jugador(j.id) != Some(j) {
            t.jugadores.insert(j.id);
        }
    }
    t.general = a.turno != b.turno
        || a.flechas != b.flechas
        || a.bolitas != b.bolitas
        || a.designaciones != b.designaciones
        || a.pausada != b.pausada
        || a.fantasma != b.fantasma
        || a.sparring != b.sparring
        || a.mulligan != b.mulligan
        || a.fin != b.fin
        || a.equipos != b.equipos
        || a.jugadores.len() != b.jugadores.len();
    t
}

struct Ctx {
    actor: Option<IdJugador>,
    t_ms: u64,
    azar: Option<Azar>,
}

impl Ctx {
    fn actor(&self) -> Result<IdJugador, Rechazo> {
        self.actor.ok_or(Rechazo::NoSePuede)
    }

    fn azar(&mut self) -> Result<&mut Azar, Rechazo> {
        self.azar.as_mut().ok_or(Rechazo::NoSePuede)
    }
}

fn ejecutar(q: &mut Partida, ev: &Evento) -> Result<(), Rechazo> {
    match &ev.cambio {
        Cambio::Accion { accion, azar } => {
            let mut ctx = Ctx {
                actor: ev.actor.jugador(),
                t_ms: ev.t_ms,
                azar: azar.map(Azar::new),
            };
            accion_(q, accion, &mut ctx, ev.seq)
        }
        Cambio::Dados { tiradas, .. } => {
            if tiradas.is_empty()
                || tiradas.len() > 200
                || tiradas
                    .iter()
                    .any(|t| t.valor == 0 || t.caras == 0 || t.valor > t.caras)
            {
                return Err(Rechazo::Limite);
            }
            Ok(())
        }
        Cambio::Monedas { lanzamientos, .. } => {
            if lanzamientos.is_empty() || lanzamientos.len() > 200 {
                return Err(Rechazo::Limite);
            }
            Ok(())
        }
        Cambio::DadoPlanar { coste, .. } => {
            if *coste != q.turno.tiradas_planar {
                return Err(Rechazo::NoSePuede);
            }
            q.turno.tiradas_planar = q.turno.tiradas_planar.saturating_add(1);
            Ok(())
        }
        Cambio::Restaurar { partida, .. } => {
            *q = (**partida).clone();
            Ok(())
        }
        Cambio::Sparring { reaccion } => {
            if reaccion.id != ev.seq || reaccion.objetivo.iter().any(|c| !q.cartas.contains_key(c))
            {
                return Err(Rechazo::NoExiste);
            }
            let ronda = q.turno.ronda;
            q.sparring.reacciones.push(ReaccionApuntada {
                ronda,
                reaccion: reaccion.clone(),
                decision: None,
            });
            Ok(())
        }
    }
}

// ── Ayudas ──────────────────────────────────────────────────────────────────

fn carta(q: &Partida, id: IdCarta) -> Result<&Carta, Rechazo> {
    q.cartas.get(&id).ok_or(Rechazo::NoExiste)
}

fn carta_mut(q: &mut Partida, id: IdCarta) -> Result<&mut Carta, Rechazo> {
    q.cartas.get_mut(&id).ok_or(Rechazo::NoExiste)
}

fn existe_jugador(q: &Partida, j: IdJugador) -> Result<(), Rechazo> {
    q.jugador(j).map(|_| ()).ok_or(Rechazo::NoExiste)
}

fn delta_ok(d: i32) -> Result<i32, Rechazo> {
    if d.abs() > MAX_DELTA {
        Err(Rechazo::Limite)
    } else {
        Ok(d)
    }
}

fn sin_repetir(cartas: &[IdCarta]) -> Result<(), Rechazo> {
    let s: BTreeSet<_> = cartas.iter().collect();
    if s.len() != cartas.len() || cartas.is_empty() || cartas.len() > 1_000 {
        return Err(Rechazo::NoSePuede);
    }
    Ok(())
}

fn biblioteca(j: IdJugador) -> RefZona {
    RefZona::de(j, Zona::Biblioteca)
}

/// En la biblioteca y los mazos, «arriba» es el principio de la lista; en
/// el resto (cementerio, pila…), el final.
fn arriba_es_inicio(z: &RefZona) -> bool {
    matches!(
        z,
        RefZona::De {
            zona: Zona::Biblioteca | Zona::Planes,
            ..
        } | RefZona::Comun {
            zona: ZonaComun::Planos
        }
    )
}

fn en_campo(q: &Partida, id: IdCarta) -> Result<IdJugador, Rechazo> {
    match q.donde(id) {
        Some((
            RefZona::De {
                jugador,
                zona: Zona::Campo,
            },
            _,
        )) => Ok(jugador),
        Some(_) => Err(Rechazo::NoSePuede),
        None => Err(Rechazo::NoExiste),
    }
}

fn en_zona(q: &Partida, id: IdCarta, z: &RefZona) -> Result<usize, Rechazo> {
    match q.donde(id) {
        Some((zz, i)) if zz == *z => Ok(i),
        Some(_) => Err(Rechazo::NoSePuede),
        None => Err(Rechazo::NoExiste),
    }
}

fn siguiente_z(q: &Partida) -> u32 {
    q.cartas
        .values()
        .map(|c| c.z)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn nuevo_id(q: &mut Partida) -> Result<IdCarta, Rechazo> {
    let id = IdCarta(q.siguiente_id);
    q.siguiente_id = q.siguiente_id.checked_add(1).ok_or(Rechazo::Limite)?;
    Ok(id)
}

/// Fila del campo para una carta (en el centro de la carta).
fn fila(info: &InfoCarta) -> u16 {
    if info.es_tierra {
        8_000
    } else if info.es_criatura || info.tiene_tipo("creature") {
        1_600
    } else {
        4_800
    }
}

/// Un hueco en el campo de `j` para una carta así, sin pisar a nadie.
fn pos_libre(q: &Partida, j: IdJugador, info: &InfoCarta) -> Pos {
    let y = fila(info);
    let ocupadas: Vec<Pos> = q
        .zona(&RefZona::de(j, Zona::Campo))
        .iter()
        .filter_map(|id| q.cartas.get(id).and_then(|c| c.pos))
        .collect();
    // Huecos de 9 % de ancho, el primero lejos del borde: en una pantalla
    // ancha una carta ocupa un 8 % y pegada al borde se cortaría.
    for k in 0..11u16 {
        let x = 600 + k * 900;
        let libre = ocupadas
            .iter()
            .all(|p| p.x.abs_diff(x) >= 450 || p.y.abs_diff(y) >= 1_200);
        if libre {
            return Pos::nueva(x, y);
        }
    }
    // Todo lleno: encima de lo último, un poco movida.
    let n = ocupadas.len() as u16;
    Pos::nueva(600 + (n % 40) * 220, y.saturating_add(300))
}

/// Saca una carta de su zona.
fn sacar(q: &mut Partida, id: IdCarta) -> Result<(RefZona, usize), Rechazo> {
    let (z, i) = q.donde(id).ok_or(Rechazo::NoExiste)?;
    if let Some(v) = q.zonas.get_mut(&z) {
        if i < v.len() {
            v.remove(i);
        }
    }
    Ok((z, i))
}

/// Dónde se mete una carta en su zona nueva.
#[derive(Debug, Clone, Copy)]
enum Hueco {
    Arriba,
    Abajo,
    Indice(usize),
    Campo(Option<Pos>),
}

/// Mueve una carta a `a`, con todo lo que pasa al cambiar de zona.
fn mover(q: &mut Partida, id: IdCarta, a: RefZona, hueco: Hueco) -> Result<(), Rechazo> {
    let c = carta(q, id)?.clone();
    if a.jugador().is_some_and(|j| q.jugador(j).is_none()) {
        return Err(Rechazo::NoExiste);
    }
    let (desde, _) = sacar(q, id)?;
    let misma = desde == a;
    let mut c = c;
    if desde.es_campo() && !a.es_campo() {
        c.limpiar_de_campo();
        soltar_de_campo(q, id);
    }
    if !misma {
        c.vista_por = Vista::Nadie;
        c.boca_abajo = false;
        c.agrupada_bajo = None;
        c.llego_en_ronda = q.turno.ronda;
        for o in q.cartas.values_mut() {
            if o.agrupada_bajo == Some(id) && !desde.es_campo() {
                o.agrupada_bajo = None;
            }
        }
        if desde.es_campo() && a.es_campo() {
            // De un campo a otro: cambia de controlador, sigue en juego.
            c.adjunta_a = None;
        }
    }
    // Las fichas y las habilidades solo existen en el campo y en la pila.
    let pila = RefZona::comun(ZonaComun::Pila);
    if (c.es_ficha && !a.es_campo() && a != pila) || (c.habilidad.is_some() && a != pila) {
        q.cartas.remove(&id);
        q.flechas.retain(|f| !toca_carta(f, id));
        return Ok(());
    }
    if a.es_campo() {
        let Some(nuevo) = a.jugador() else {
            return Err(Rechazo::NoExiste);
        };
        let antes = c.pos;
        let pos = match hueco {
            Hueco::Campo(Some(p)) => p,
            _ if misma => antes.unwrap_or_else(|| pos_libre(q, nuevo, &c.origen.info)),
            _ => pos_libre(q, nuevo, &c.origen.info),
        };
        c.controlador = nuevo;
        c.pos = Some(pos);
        c.z = siguiente_z(q);
        if !desde.es_campo() && q.opciones.ayudas.contadores_al_entrar && !c.boca_abajo {
            contadores_al_entrar(&mut c);
        }
        // Lo agrupado bajo ella (un montón de cartas) va con ella.
        if misma {
            for o in q.cartas.values_mut() {
                if o.agrupada_bajo == Some(id) {
                    o.pos = Some(pos);
                }
            }
        }
    } else {
        c.pos = None;
        c.controlador = c.dueno;
    }
    let v = q.zonas.entry(a).or_default();
    let i = match hueco {
        Hueco::Indice(i) => i.min(v.len()),
        Hueco::Arriba if arriba_es_inicio(&a) => 0,
        Hueco::Abajo if !arriba_es_inicio(&a) => 0,
        _ => v.len(),
    };
    v.insert(i, id);
    q.cartas.insert(id, c);
    Ok(())
}

/// Lo que pasa al resto de la mesa cuando una carta deja el campo.
fn soltar_de_campo(q: &mut Partida, id: IdCarta) {
    for o in q.cartas.values_mut() {
        if o.adjunta_a == Some(id) {
            o.adjunta_a = None;
        }
        if o.agrupada_bajo == Some(id) {
            o.agrupada_bajo = None;
        }
    }
    q.flechas.retain(|f| !toca_carta(f, id));
}

fn toca_carta(f: &Flecha, id: IdCarta) -> bool {
    f.de == Extremo::Carta(id) || f.a == Extremo::Carta(id)
}

fn contadores_al_entrar(c: &mut Carta) {
    if let Some(l) = c
        .origen
        .info
        .lealtad
        .as_deref()
        .and_then(|l| l.trim().parse::<i32>().ok())
    {
        c.contadores.insert("loyalty".into(), l);
    }
    if c.origen.info.es_saga() {
        c.contadores.insert("lore".into(), 1);
    }
}

/// La zona de un destino para esta carta: el campo puede ser de otro; el
/// resto, siempre del dueño.
fn zona_destino(c: &Carta, d: &Destino, actor: IdJugador) -> RefZona {
    match d.zona {
        ZonaDestino::De(Zona::Campo) => RefZona::de(d.jugador.unwrap_or(actor), Zona::Campo),
        ZonaDestino::De(z) => RefZona::de(c.dueno, z),
        ZonaDestino::Comun(z) => RefZona::comun(z),
    }
}

fn barajar_zona(q: &mut Partida, z: &RefZona, azar: &mut Azar) {
    if let Some(v) = q.zonas.get_mut(z) {
        azar.barajar(v);
        let ids = v.clone();
        for id in ids {
            if let Some(c) = q.cartas.get_mut(&id) {
                c.vista_por = Vista::Nadie;
            }
        }
    }
}

fn arriba_de(q: &Partida, z: &RefZona) -> Option<IdCarta> {
    let v = q.zona(z);
    if arriba_es_inicio(z) {
        v.first().copied()
    } else {
        v.last().copied()
    }
}

fn robar(q: &mut Partida, j: IdJugador, n: u32) -> Result<u32, Rechazo> {
    let mut robadas = 0;
    for _ in 0..n {
        let Some(id) = arriba_de(q, &biblioteca(j)) else {
            break;
        };
        // A la mano, al final (su «arriba»).
        mover(q, id, RefZona::de(j, Zona::Mano), Hueco::Arriba)?;
        robadas += 1;
    }
    Ok(robadas)
}

/// Lo que puede tocar una carta del actor (sus permisos los mira
/// `resolver.rs`; aquí solo que exista).
fn cada<F>(q: &mut Partida, cartas: &[IdCarta], mut f: F) -> Result<(), Rechazo>
where
    F: FnMut(&mut Carta) -> Result<(), Rechazo>,
{
    sin_repetir(cartas)?;
    for id in cartas {
        f(carta_mut(q, *id)?)?;
    }
    Ok(())
}

fn vida(q: &mut Partida, j: IdJugador, delta: i32) -> Result<(), Rechazo> {
    let delta = delta_ok(delta)?;
    let equipo = q.equipo_de(j);
    if let Some(e) = equipo.and_then(|e| q.equipos.iter_mut().find(|x| x.id == e)) {
        if let Some(v) = e.vida_compartida.as_mut() {
            *v = v.saturating_add(delta);
            return Ok(());
        }
    }
    let x = q.jugador_mut(j).ok_or(Rechazo::NoExiste)?;
    x.vida = x.vida.saturating_add(delta);
    Ok(())
}

/// El siguiente asiento vivo después de `desde` y si con eso empieza una
/// ronda nueva: la ronda cambia al pasar por el asiento del primer jugador
/// (esté vivo o no), no por el asiento 0, que es solo el primero de la lista.
fn siguiente_asiento(q: &Partida, desde: IdJugador) -> Option<(IdJugador, bool)> {
    let n = q.jugadores.len();
    let i = q.jugadores.iter().position(|j| j.id == desde)?;
    let primero = q.jugadores.iter().position(|j| j.id == q.turno.primero);
    let mut vuelta = false;
    for k in 1..=n {
        let pos = (i + k) % n;
        let jj = q.jugadores.get(pos)?;
        vuelta |= Some(pos) == primero;
        if jj.eliminado.is_none() {
            return Some((jj.id, vuelta));
        }
    }
    None
}

/// Las ayudas del principio del turno.
fn empezar_turno(q: &mut Partida, j: IdJugador) -> Result<(), Rechazo> {
    if q.opciones.ayudas.enderezar_al_empezar {
        let ids: Vec<IdCarta> = q.zona(&RefZona::de(j, Zona::Campo)).to_vec();
        for id in ids {
            if let Some(c) = q.cartas.get_mut(&id) {
                if !c.no_endereza && c.controlador == j {
                    c.girada = false;
                }
            }
        }
    }
    let primer_turno = q.turno.numero == 1;
    if q.opciones.ayudas.robar_al_empezar && !(primer_turno && q.opciones.primero_no_roba) {
        robar(q, j, 1)?;
    }
    Ok(())
}

fn pasar_turno(q: &mut Partida, t_ms: u64) -> Result<(), Rechazo> {
    if q.mulligan.is_some() {
        return Err(Rechazo::NoSePuede);
    }
    for c in q.cartas.values_mut() {
        c.pt.hasta_fin_de_turno = (0, 0);
    }
    q.flechas.clear();
    let (siguiente, vuelta) = if q.turno.extra.is_empty() {
        siguiente_asiento(q, q.turno.activo).ok_or(Rechazo::NoSePuede)?
    } else {
        (q.turno.extra.remove(0), false)
    };
    if vuelta {
        q.turno.ronda = q.turno.ronda.saturating_add(1);
    }
    q.turno.numero = q.turno.numero.saturating_add(1);
    q.turno.activo = siguiente;
    q.turno.desde_ms = t_ms;
    q.turno.tiradas_planar = 0;
    empezar_turno(q, siguiente)
}

fn origen_libre(
    nombre: &str,
    fuerza: &Option<String>,
    resistencia: &Option<String>,
    colores: &[char],
) -> OrigenCarta {
    let criatura = fuerza.is_some() || resistencia.is_some();
    OrigenCarta {
        carta_deck: String::new(),
        nombre: nombre.to_string(),
        caras: vec![RefImagen::Generica],
        dorso: RefImagen::Generica,
        scryfall_id: None,
        info: InfoCarta {
            tipo: Some(if criatura {
                "Token Creature".into()
            } else {
                "Token".into()
            }),
            colores: colores.to_vec(),
            es_criatura: criatura,
            fuerza: fuerza.clone(),
            resistencia: resistencia.clone(),
            ..InfoCarta::default()
        },
    }
}

/// Crea una ficha en el campo de `j`.
fn crear_ficha(
    q: &mut Partida,
    j: IdJugador,
    origen: OrigenCarta,
    cantidad: u32,
    pos: Option<Pos>,
    copia_de: Option<IdCarta>,
) -> Result<IdCarta, Rechazo> {
    let id = nuevo_id(q)?;
    let mut c = Carta::nueva(id, j, origen);
    c.es_ficha = true;
    c.cantidad = cantidad;
    c.copia_de = copia_de;
    q.cartas.insert(id, c);
    q.zonas.entry(RefZona::de(j, Zona::Campo)).or_default();
    // Se mete en el campo como si llegara de fuera (pos, z, contadores).
    q.zonas
        .entry(RefZona::comun(ZonaComun::Pila))
        .or_default()
        .push(id);
    mover(q, id, RefZona::de(j, Zona::Campo), Hueco::Campo(pos))?;
    Ok(id)
}

fn desplazada(p: Option<Pos>, k: u32) -> Option<Pos> {
    p.map(|p| {
        let d = (k.min(40) as u16) * 160;
        Pos::nueva(p.x.saturating_add(d).min(POS_MAX), p.y)
    })
}

/// Las cartas de una zona del actor (para «todo el cementerio…»).
fn zona_del(q: &Partida, j: IdJugador, z: Zona) -> Result<RefZona, Rechazo> {
    if matches!(z, Zona::Campo) {
        return Err(Rechazo::NoSePuede);
    }
    existe_jugador(q, j)?;
    Ok(RefZona::de(j, z))
}

// ── Cada acción ─────────────────────────────────────────────────────────────

fn accion_(q: &mut Partida, a: &Accion, ctx: &mut Ctx, seq: u64) -> Result<(), Rechazo> {
    let yo = ctx.actor()?;
    existe_jugador(q, yo)?;
    if q.mulligan.is_some()
        && !matches!(
            a,
            Accion::Mulligan
                | Accion::QuedarseCon { .. }
                | Accion::Chat { .. }
                | Accion::Aviso { .. }
                | Accion::Senal { .. }
                | Accion::Conceder
                | Accion::Eliminar { .. }
                | Accion::Terminar { .. }
                | Accion::IntensidadSparring { .. }
                | Accion::VerFantasma { .. }
                | Accion::Pausar { .. }
        )
    {
        return Err(Rechazo::NoSePuede);
    }
    match a {
        Accion::Mover { cartas, a, como } => {
            sin_repetir(cartas)?;
            if let (ZonaDestino::De(Zona::Campo), Some(j)) = (a.zona, a.jugador) {
                existe_jugador(q, j)?;
            }
            let mut huecos: Vec<(IdCarta, RefZona, Hueco)> = Vec::new();
            let mut barajar: BTreeSet<RefZona> = BTreeSet::new();
            for (k, id) in cartas.iter().enumerate() {
                let c = carta(q, *id)?;
                let z = zona_destino(c, a, yo);
                let hueco = match a.sitio {
                    Sitio::Arriba => Hueco::Arriba,
                    Sitio::Abajo => Hueco::Abajo,
                    Sitio::Indice(i) => Hueco::Indice(i as usize),
                    Sitio::XDesdeArriba(n) => {
                        let len = q.zona(&z).len();
                        let desde_arriba = (n.max(1) - 1) as usize;
                        if arriba_es_inicio(&z) {
                            Hueco::Indice(desde_arriba.min(len))
                        } else {
                            Hueco::Indice(len.saturating_sub(desde_arriba))
                        }
                    }
                    Sitio::EnCampo(p) => Hueco::Campo(desplazada(Some(p), k as u32)),
                    Sitio::AlAzar => {
                        let len = q.zona(&z).len() as u64;
                        Hueco::Indice(ctx.azar()?.menor_que(len + 1) as usize)
                    }
                    Sitio::BarajarDentro => {
                        barajar.insert(z);
                        Hueco::Arriba
                    }
                };
                if z.es_campo() && !matches!(hueco, Hueco::Campo(_)) {
                    huecos.push((*id, z, Hueco::Campo(None)));
                } else {
                    huecos.push((*id, z, hueco));
                }
            }
            for (id, z, hueco) in huecos {
                let del_mando = matches!(q.donde(id), Some((zz, _)) if zz.es(Zona::Mando));
                let dueno = carta(q, id)?.dueno;
                mover(q, id, z, hueco)?;
                if *como == Some(ComoMover::Lanzar)
                    && del_mando
                    && q.opciones.ayudas.impuesto_automatico
                {
                    let j = q.jugador_mut(dueno).ok_or(Rechazo::NoExiste)?;
                    let n = j.lanzamientos.entry(id).or_insert(0);
                    *n = n.saturating_add(1);
                }
            }
            for z in barajar {
                let azar = ctx.azar()?;
                barajar_zona(q, &z, azar);
            }
            Ok(())
        }
        Accion::Girar { cartas, girada } => cada(q, cartas, |c| {
            c.girada = *girada;
            Ok(())
        }),
        Accion::BocaAbajo { cartas, si } => cada(q, cartas, |c| {
            c.boca_abajo = *si;
            c.vista_por = if *si {
                Vista::Jugadores(vec![c.controlador])
            } else {
                Vista::Nadie
            };
            Ok(())
        }),
        Accion::CambiarCara { carta: id } => {
            let c = carta_mut(q, *id)?;
            let n = c.origen.caras.len().max(1) as u8;
            c.cara = (c.cara + 1) % n;
            Ok(())
        }
        Accion::Contador {
            cartas,
            tipo,
            delta,
        } => {
            let tipo = nombre_corto(tipo)?;
            let d = delta_ok(*delta)?;
            cada(q, cartas, |c| {
                let v = c.contadores.entry(tipo.clone()).or_insert(0);
                *v = v.saturating_add(d);
                if *v == 0 {
                    c.contadores.remove(&tipo);
                }
                Ok(())
            })
        }
        Accion::FijarContador {
            carta: id,
            tipo,
            valor,
        } => {
            let tipo = nombre_corto(tipo)?;
            let v = delta_ok(*valor)?;
            let c = carta_mut(q, *id)?;
            if v == 0 {
                c.contadores.remove(&tipo);
            } else {
                c.contadores.insert(tipo, v);
            }
            Ok(())
        }
        Accion::MultiplicarContadores { carta: id, por } => {
            if *por > 10 {
                return Err(Rechazo::Limite);
            }
            let c = carta_mut(q, *id)?;
            for v in c.contadores.values_mut() {
                *v = v
                    .saturating_mul(i32::from(*por))
                    .clamp(-MAX_DELTA, MAX_DELTA);
            }
            c.contadores.retain(|_, v| *v != 0);
            Ok(())
        }
        Accion::ContadorClave {
            cartas,
            palabra,
            si,
        } => {
            let p = nombre_corto(palabra)?;
            cada(q, cartas, |c| {
                if *si {
                    c.palabras_clave.insert(p.clone());
                } else {
                    c.palabras_clave.remove(&p);
                }
                Ok(())
            })
        }
        Accion::Pt {
            carta: id,
            fila,
            valor,
        } => {
            if let Some((x, y)) = valor {
                delta_ok(*x)?;
                delta_ok(*y)?;
            }
            let c = carta_mut(q, *id)?;
            match fila {
                FilaPt::Otros => c.pt.otros = valor.unwrap_or((0, 0)),
                FilaPt::HastaFinDeTurno => c.pt.hasta_fin_de_turno = valor.unwrap_or((0, 0)),
                FilaPt::Fijo => c.pt.fijo = *valor,
            }
            Ok(())
        }
        Accion::MasXMasY { cartas, x, y, fila } => {
            let (x, y) = (delta_ok(*x)?, delta_ok(*y)?);
            let suma = |a: (i32, i32)| {
                (
                    a.0.saturating_add(x).clamp(-MAX_DELTA, MAX_DELTA),
                    a.1.saturating_add(y).clamp(-MAX_DELTA, MAX_DELTA),
                )
            };
            cada(q, cartas, |c| {
                match fila {
                    FilaPt::Otros => c.pt.otros = suma(c.pt.otros),
                    FilaPt::HastaFinDeTurno => {
                        c.pt.hasta_fin_de_turno = suma(c.pt.hasta_fin_de_turno)
                    }
                    FilaPt::Fijo => c.pt.fijo = Some(suma(c.pt.fijo.unwrap_or((0, 0)))),
                }
                Ok(())
            })
        }
        Accion::Etiqueta {
            carta: id,
            etiqueta,
            poner,
        } => {
            let texto = etiqueta.texto.trim().to_string();
            if texto.is_empty() || texto.chars().count() > MAX_ETIQUETA || etiqueta.color >= 7 {
                return Err(Rechazo::Limite);
            }
            let c = carta_mut(q, *id)?;
            if *poner {
                if c.etiquetas.len() >= MAX_ETIQUETAS {
                    return Err(Rechazo::Limite);
                }
                c.etiquetas.push(crate::modelo::Etiqueta {
                    texto,
                    color: etiqueta.color,
                });
            } else {
                c.etiquetas.retain(|e| e.texto != texto);
            }
            Ok(())
        }
        Accion::PonerBolita {
            campo_de,
            pos,
            color,
            tamano,
        } => {
            existe_jugador(q, *campo_de)?;
            if *color >= 16 || q.bolitas.len() >= MAX_BOLITAS {
                return Err(Rechazo::Limite);
            }
            let id = q.bolitas.iter().map(|b| b.id).max().unwrap_or(0) + 1;
            q.bolitas.push(Bolita {
                id,
                campo_de: *campo_de,
                pos: Pos::nueva(pos.x, pos.y),
                color: *color,
                tamano: *tamano,
            });
            Ok(())
        }
        Accion::MoverBolita { id, pos } => {
            let b = q
                .bolitas
                .iter_mut()
                .find(|b| b.id == *id)
                .ok_or(Rechazo::NoExiste)?;
            b.pos = Pos::nueva(pos.x, pos.y);
            Ok(())
        }
        Accion::QuitarBolita { id } => {
            let antes = q.bolitas.len();
            q.bolitas.retain(|b| b.id != *id);
            if q.bolitas.len() == antes {
                return Err(Rechazo::NoExiste);
            }
            Ok(())
        }
        Accion::Adjuntar { carta: id, a } => {
            en_campo(q, *id)?;
            if let Some(a) = a {
                en_campo(q, *a)?;
                // Sin ciclos: lo que lleva `a` encima no puede ser `id`.
                let mut x = Some(*a);
                let mut pasos = 0;
                while let Some(y) = x {
                    if y == *id || pasos > 1_000 {
                        return Err(Rechazo::NoSePuede);
                    }
                    x = q.cartas.get(&y).and_then(|c| c.adjunta_a);
                    pasos += 1;
                }
            }
            carta_mut(q, *id)?.adjunta_a = *a;
            Ok(())
        }
        Accion::FaseFuera { cartas, si } => cada(q, cartas, |c| {
            c.fase_fuera = *si;
            Ok(())
        }),
        Accion::NoEndereza { cartas, si } => cada(q, cartas, |c| {
            c.no_endereza = *si;
            Ok(())
        }),
        Accion::Resaltar { cartas, color } => {
            if color.is_some_and(|c| c >= 16) {
                return Err(Rechazo::Limite);
            }
            cada(q, cartas, |c| {
                c.resaltada = *color;
                Ok(())
            })
        }
        Accion::CopiaFicha { cartas } => {
            sin_repetir(cartas)?;
            for (k, id) in cartas.iter().enumerate() {
                let c = carta(q, *id)?.clone();
                let pos = desplazada(c.pos, k as u32 + 1);
                let nueva = crear_ficha(q, c.controlador, c.origen, 1, pos, Some(*id))?;
                carta_mut(q, nueva)?.cara = c.cara;
            }
            Ok(())
        }
        Accion::DarControl { cartas, a } => {
            existe_jugador(q, *a)?;
            sin_repetir(cartas)?;
            for id in cartas {
                en_campo(q, *id)?;
                let pos = carta(q, *id)?.pos;
                mover(q, *id, RefZona::de(*a, Zona::Campo), Hueco::Campo(pos))?;
            }
            Ok(())
        }
        Accion::HabilidadEnPila { carta: id, texto } => {
            if texto.chars().count() > MAX_TEXTO {
                return Err(Rechazo::Limite);
            }
            let c = carta(q, *id)?.clone();
            let nueva = nuevo_id(q)?;
            let mut h = Carta::nueva(nueva, c.controlador, c.origen);
            h.es_ficha = true;
            h.cara = c.cara;
            h.habilidad = Some(texto.clone());
            h.llego_en_ronda = q.turno.ronda;
            q.cartas.insert(nueva, h);
            q.zonas
                .entry(RefZona::comun(ZonaComun::Pila))
                .or_default()
                .push(nueva);
            Ok(())
        }
        Accion::Flecha { de, a, tipo } => {
            for e in [de, a] {
                match e {
                    Extremo::Carta(c) => {
                        carta(q, *c)?;
                    }
                    Extremo::Jugador(j) => existe_jugador(q, *j)?,
                }
            }
            if q.flechas.len() >= MAX_FLECHAS {
                return Err(Rechazo::Limite);
            }
            q.flechas.push(Flecha {
                de: *de,
                a: *a,
                tipo: *tipo,
                de_jugador: yo,
            });
            Ok(())
        }
        Accion::QuitarFlechas { de } => {
            q.flechas
                .retain(|f| f.de_jugador != yo || de.is_some_and(|d| f.de != d));
            Ok(())
        }
        // ── Biblioteca ──
        Accion::Robar { n } => {
            limite(*n)?;
            if robar(q, yo, *n)? == 0 {
                return Err(Rechazo::ZonaVacia);
            }
            Ok(())
        }
        Accion::Moler { n } => {
            limite(*n)?;
            de_arriba(q, yo, *n, |q, id| {
                mover(q, id, RefZona::de(yo, Zona::Cementerio), Hueco::Arriba)
            })
        }
        Accion::Manifestar { n } => {
            limite(*n)?;
            de_arriba(q, yo, *n, |q, id| {
                mover(q, id, RefZona::de(yo, Zona::Campo), Hueco::Campo(None))?;
                let c = carta_mut(q, id)?;
                c.boca_abajo = true;
                c.vista_por = Vista::Jugadores(vec![yo]);
                c.contadores.clear();
                Ok(())
            })
        }
        Accion::ExiliarDeArriba { n, modo } => {
            limite(*n)?;
            de_arriba(q, yo, *n, |q, id| {
                mover(q, id, RefZona::de(yo, Zona::Exilio), Hueco::Arriba)?;
                let c = carta_mut(q, id)?;
                match modo {
                    ModoExilio::Normal => {}
                    ModoExilio::BocaAbajo => {
                        c.boca_abajo = true;
                        c.vista_por = Vista::Jugadores(vec![yo]);
                    }
                    ModoExilio::Oculta => {
                        c.boca_abajo = true;
                        c.vista_por = Vista::Nadie;
                    }
                }
                Ok(())
            })
        }
        Accion::Mirar { de, n, desde } => {
            limite(*n)?;
            existe_jugador(q, *de)?;
            let lib = q.zona(&biblioteca(*de)).to_vec();
            if lib.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            let n = (*n as usize).min(lib.len());
            let elegidas: Vec<IdCarta> = match desde {
                Lado::Arriba => lib.iter().take(n).copied().collect(),
                Lado::Abajo => lib.iter().rev().take(n).copied().collect(),
            };
            for id in elegidas {
                let c = carta_mut(q, id)?;
                c.vista_por = c.vista_por.con(yo, c.dueno);
            }
            Ok(())
        }
        Accion::Ordenar {
            de,
            arriba,
            abajo,
            cementerio,
            exilio,
        } => {
            let lib = biblioteca(*de);
            let todas: Vec<IdCarta> = arriba
                .iter()
                .chain(abajo)
                .chain(cementerio)
                .chain(exilio)
                .copied()
                .collect();
            sin_repetir(&todas)?;
            for id in &todas {
                en_zona(q, *id, &lib)?;
            }
            for id in arriba.iter().rev() {
                mover(q, *id, lib, Hueco::Arriba)?;
                carta_mut(q, *id)?.vista_por = Vista::Nadie;
            }
            for id in abajo {
                mover(q, *id, lib, Hueco::Abajo)?;
                carta_mut(q, *id)?.vista_por = Vista::Nadie;
            }
            for id in cementerio {
                let d = carta(q, *id)?.dueno;
                mover(q, *id, RefZona::de(d, Zona::Cementerio), Hueco::Arriba)?;
            }
            for id in exilio {
                let d = carta(q, *id)?.dueno;
                mover(q, *id, RefZona::de(d, Zona::Exilio), Hueco::Arriba)?;
            }
            Ok(())
        }
        Accion::RevelarArriba { n } => {
            limite(*n)?;
            let lib = q.zona(&biblioteca(yo)).to_vec();
            if lib.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            for id in lib.into_iter().take(*n as usize) {
                carta_mut(q, id)?.vista_por = Vista::Todos;
            }
            Ok(())
        }
        Accion::Excavar {
            forma,
            filtro,
            parar_tras,
        } => {
            let lib = q.zona(&biblioteca(yo)).to_vec();
            if lib.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            let objetivo = u32::from((*parar_tras).max(1));
            let mut aciertos = 0u32;
            for (k, id) in lib.into_iter().enumerate() {
                if let FormaExcavar::RevelarN(n) = forma {
                    if k >= usize::from(*n) {
                        break;
                    }
                }
                let c = carta_mut(q, id)?;
                c.vista_por = Vista::Todos;
                if filtro.cumple(&c.origen) {
                    aciertos += 1;
                    if matches!(forma, FormaExcavar::HastaQueCoincida) && aciertos >= objetivo {
                        break;
                    }
                }
            }
            Ok(())
        }
        Accion::ResolverExcavacion { elegidas, resto, a } => {
            let lib = biblioteca(yo);
            let ids: Vec<IdCarta> = elegidas
                .iter()
                .map(|(c, _)| *c)
                .chain(resto.iter().copied())
                .collect();
            if !ids.is_empty() {
                sin_repetir(&ids)?;
            }
            for id in &ids {
                en_zona(q, *id, &lib)?;
            }
            for (id, d) in elegidas {
                let z = zona_destino(carta(q, *id)?, d, yo);
                let hueco = match d.sitio {
                    Sitio::EnCampo(p) => Hueco::Campo(Some(p)),
                    Sitio::Abajo => Hueco::Abajo,
                    _ if z.es_campo() => Hueco::Campo(None),
                    _ => Hueco::Arriba,
                };
                mover(q, *id, z, hueco)?;
            }
            let mut resto = resto.clone();
            if *a == RestoExcavar::AbajoAlAzar {
                ctx.azar()?.barajar(&mut resto);
            }
            for id in resto {
                match a {
                    RestoExcavar::Abajo | RestoExcavar::AbajoAlAzar => {
                        mover(q, id, lib, Hueco::Abajo)?;
                    }
                    RestoExcavar::Cementerio => {
                        mover(q, id, RefZona::de(yo, Zona::Cementerio), Hueco::Arriba)?;
                    }
                    RestoExcavar::Exilio => {
                        mover(q, id, RefZona::de(yo, Zona::Exilio), Hueco::Arriba)?;
                    }
                    RestoExcavar::Arriba => {}
                }
                carta_mut(q, id)?.vista_por = Vista::Nadie;
            }
            Ok(())
        }
        Accion::Buscar { de } => {
            existe_jugador(q, *de)?;
            let lib = q.zona(&biblioteca(*de)).to_vec();
            for id in lib {
                let c = carta_mut(q, id)?;
                c.vista_por = c.vista_por.con(yo, c.dueno);
            }
            Ok(())
        }
        Accion::DejarDeMirar { de } => {
            existe_jugador(q, *de)?;
            let lib = q.zona(&biblioteca(*de)).to_vec();
            for id in lib {
                let c = carta_mut(q, id)?;
                c.vista_por = c.vista_por.sin(yo, c.dueno);
            }
            Ok(())
        }
        Accion::DeArribaRevelada { si } => {
            q.jugador_mut(yo)
                .ok_or(Rechazo::NoExiste)?
                .de_arriba_revelada = *si;
            Ok(())
        }
        Accion::Barajar => {
            let azar = ctx.azar()?;
            barajar_zona(q, &biblioteca(yo), azar);
            Ok(())
        }
        Accion::Rueda { robar: n } => {
            let mano = RefZona::de(yo, Zona::Mano);
            let ids = q.zona(&mano).to_vec();
            let n = n.unwrap_or(ids.len() as u32);
            limite(n)?;
            for id in ids {
                mover(q, id, biblioteca(yo), Hueco::Abajo)?;
            }
            let azar = ctx.azar()?;
            barajar_zona(q, &biblioteca(yo), azar);
            robar(q, yo, n)?;
            Ok(())
        }
        // ── Campo entero ──
        Accion::EnderezarTodo => {
            let ids = q.zona(&RefZona::de(yo, Zona::Campo)).to_vec();
            for id in ids {
                let c = carta_mut(q, id)?;
                if !c.no_endereza {
                    c.girada = false;
                }
            }
            Ok(())
        }
        Accion::Organizar => {
            organizar(q, yo);
            Ok(())
        }
        Accion::CrearFicha {
            de,
            cantidad,
            monton,
            pos,
        } => {
            let tope = if *monton {
                MAX_CANTIDAD
            } else {
                MAX_FICHAS_SUELTAS
            };
            if *cantidad == 0 || *cantidad > tope {
                return Err(Rechazo::Limite);
            }
            let origen = match de {
                OrigenFicha::DelDeck { indice } => q
                    .jugador(yo)
                    .and_then(|j| j.fichas.get(*indice as usize))
                    .cloned()
                    .ok_or(Rechazo::NoExiste)?,
                OrigenFicha::Carta { origen } => origen.clone(),
                OrigenFicha::Libre {
                    nombre,
                    fuerza,
                    resistencia,
                    colores,
                } => {
                    let n = nombre.trim();
                    if n.is_empty() || n.chars().count() > 60 || colores.len() > 6 {
                        return Err(Rechazo::Limite);
                    }
                    origen_libre(n, fuerza, resistencia, colores)
                }
            };
            if *monton {
                crear_ficha(q, yo, origen, *cantidad, *pos, None)?;
            } else {
                for k in 0..*cantidad {
                    let p = if pos.is_some() {
                        desplazada(*pos, k)
                    } else {
                        None
                    };
                    crear_ficha(q, yo, origen.clone(), 1, p, None)?;
                }
            }
            Ok(())
        }
        // ── Mulligan ──
        Accion::Mulligan => {
            let m = q.mulligan.as_ref().ok_or(Rechazo::NoSePuede)?;
            if m.decididos.contains(&yo) {
                return Err(Rechazo::NoSePuede);
            }
            let mano = RefZona::de(yo, Zona::Mano);
            let ids = q.zona(&mano).to_vec();
            for id in ids {
                mover(q, id, biblioteca(yo), Hueco::Abajo)?;
            }
            let azar = ctx.azar()?;
            barajar_zona(q, &biblioteca(yo), azar);
            let n = u32::from(q.opciones.mano_inicial);
            robar(q, yo, n)?;
            let j = q.jugador_mut(yo).ok_or(Rechazo::NoExiste)?;
            if j.mulligans >= 20 {
                return Err(Rechazo::Limite);
            }
            j.mulligans += 1;
            Ok(())
        }
        Accion::QuedarseCon { al_fondo } => {
            let decididos = q
                .mulligan
                .as_ref()
                .ok_or(Rechazo::NoSePuede)?
                .decididos
                .clone();
            if decididos.contains(&yo) {
                return Err(Rechazo::NoSePuede);
            }
            let mulligans = q.jugador(yo).ok_or(Rechazo::NoExiste)?.mulligans;
            let toca = match q.opciones.mulligan {
                ModoMulligan::Londres => Some(usize::from(mulligans)),
                ModoMulligan::LondresPrimeroGratis => {
                    Some(usize::from(mulligans.saturating_sub(1)))
                }
                ModoMulligan::Amistoso => Some(0),
                ModoMulligan::Libre => None,
            };
            let mano = RefZona::de(yo, Zona::Mano);
            let en_mano = q.zona(&mano).len();
            let toca = toca.map(|t| t.min(en_mano));
            if toca.is_some_and(|t| t != al_fondo.len()) {
                return Err(Rechazo::Limite);
            }
            if !al_fondo.is_empty() {
                sin_repetir(al_fondo)?;
            }
            for id in al_fondo {
                en_zona(q, *id, &mano)?;
                mover(q, *id, biblioteca(yo), Hueco::Abajo)?;
            }
            let mut decididos = decididos;
            decididos.push(yo);
            let faltan = q
                .jugadores
                .iter()
                .any(|j| j.eliminado.is_none() && !decididos.contains(&j.id));
            if faltan {
                if let Some(m) = q.mulligan.as_mut() {
                    m.decididos = decididos;
                }
            } else {
                q.mulligan = None;
                q.turno.numero = 1;
                q.turno.ronda = 1;
                q.turno.desde_ms = ctx.t_ms;
                let primero = q.turno.primero;
                q.turno.activo = primero;
                empezar_turno(q, primero)?;
            }
            Ok(())
        }
        // ── Jugadores ──
        Accion::Vida { jugador, delta } => vida(q, *jugador, *delta),
        Accion::VidaATodos {
            delta,
            solo_oponentes,
        } => {
            let d = delta_ok(*delta)?;
            let quienes: Vec<IdJugador> = if *solo_oponentes {
                q.oponentes(yo)
            } else {
                q.jugadores
                    .iter()
                    .filter(|j| j.eliminado.is_none())
                    .map(|j| j.id)
                    .collect()
            };
            for j in quienes {
                vida(q, j, d)?;
            }
            if let Some(f) = q.fantasma.as_mut() {
                f.vida = f.vida.saturating_add(d);
            }
            Ok(())
        }
        Accion::Drenar { n } | Accion::Extorsion { n } => {
            let n = delta_ok(*n)?;
            if n < 0 {
                return Err(Rechazo::Limite);
            }
            let rivales = q.oponentes(yo);
            let mut ganado = 0i32;
            for j in rivales {
                vida(q, j, -n)?;
                ganado = ganado.saturating_add(n);
            }
            if let Some(f) = q.fantasma.as_mut() {
                f.vida = f.vida.saturating_sub(n);
                ganado = ganado.saturating_add(n);
            }
            vida(q, yo, ganado.min(MAX_DELTA))
        }
        Accion::Veneno { jugador, delta } => {
            let d = delta_ok(*delta)?;
            let j = q.jugador_mut(*jugador).ok_or(Rechazo::NoExiste)?;
            j.veneno = j.veneno.saturating_add(d).max(0);
            Ok(())
        }
        Accion::DanoComandante {
            jugador,
            comandante,
            delta,
        } => {
            let d = delta_ok(*delta)?;
            carta(q, *comandante)?;
            let j = q.jugador_mut(*jugador).ok_or(Rechazo::NoExiste)?;
            let v = j.dano_comandante.entry(*comandante).or_insert(0);
            *v = v.saturating_add(d).max(0);
            if *v == 0 {
                j.dano_comandante.remove(comandante);
            }
            vida(q, *jugador, -d)
        }
        Accion::ContadorJugador {
            jugador,
            tipo,
            delta,
        } => {
            let tipo = nombre_corto(tipo)?;
            let d = delta_ok(*delta)?;
            let j = q.jugador_mut(*jugador).ok_or(Rechazo::NoExiste)?;
            let v = j.contadores.entry(tipo.clone()).or_insert(0);
            *v = v.saturating_add(d);
            if tipo == "speed" {
                *v = (*v).clamp(0, 4);
            }
            if *v == 0 {
                j.contadores.remove(&tipo);
            }
            Ok(())
        }
        Accion::Designar { tipo, a } => {
            if let Some(j) = a {
                existe_jugador(q, *j)?;
            }
            let d = &mut q.designaciones;
            match tipo {
                Designacion::Monarca => d.monarca = *a,
                Designacion::Iniciativa => d.iniciativa = *a,
                Designacion::Bendicion => match a {
                    Some(j) if d.bendicion.contains(j) => d.bendicion.retain(|x| x != j),
                    Some(j) => {
                        d.bendicion.push(*j);
                        d.bendicion.sort();
                    }
                    None => d.bendicion.clear(),
                },
            }
            Ok(())
        }
        Accion::DiaNoche { valor } => {
            q.designaciones.dia_noche = *valor;
            Ok(())
        }
        Accion::Lanzamientos { comandante, delta } => {
            let d = delta_ok(*delta)?;
            let dueno = carta(q, *comandante)?.dueno;
            let j = q.jugador_mut(dueno).ok_or(Rechazo::NoExiste)?;
            let v = j.lanzamientos.entry(*comandante).or_insert(0);
            *v = (i64::from(*v) + i64::from(d)).clamp(0, 1_000) as u32;
            if *v == 0 {
                j.lanzamientos.remove(comandante);
            }
            Ok(())
        }
        Accion::SinCosteExtra { comandante, si } => {
            let dueno = carta(q, *comandante)?.dueno;
            let j = q.jugador_mut(dueno).ok_or(Rechazo::NoExiste)?;
            if *si {
                j.sin_coste_extra.insert(*comandante);
            } else {
                j.sin_coste_extra.remove(comandante);
            }
            Ok(())
        }
        Accion::RevelarMano { si } => {
            q.jugador_mut(yo).ok_or(Rechazo::NoExiste)?.mano_revelada = *si;
            Ok(())
        }
        Accion::RevelarA { cartas, a } => {
            for j in a {
                existe_jugador(q, *j)?;
            }
            cada(q, cartas, |c| {
                for j in a {
                    c.vista_por = c.vista_por.con(*j, c.dueno);
                }
                Ok(())
            })
        }
        Accion::JugarBocaAbajo { carta: id, pos } => {
            en_zona(q, *id, &RefZona::de(yo, Zona::Mano))?;
            mover(q, *id, RefZona::de(yo, Zona::Campo), Hueco::Campo(*pos))?;
            let c = carta_mut(q, *id)?;
            c.boca_abajo = true;
            c.vista_por = Vista::Jugadores(vec![yo]);
            c.contadores.clear();
            Ok(())
        }
        Accion::Descartar { modo } => {
            let mano = RefZona::de(yo, Zona::Mano);
            let en_mano = q.zona(&mano).to_vec();
            let ids: Vec<IdCarta> = match modo {
                ModoDescarte::Todas => en_mano,
                ModoDescarte::AlAzar(n) => {
                    limite(*n)?;
                    let idx = ctx.azar()?.elegir(en_mano.len(), *n as usize);
                    idx.into_iter()
                        .filter_map(|i| en_mano.get(i).copied())
                        .collect()
                }
                ModoDescarte::Elegidas(v) => {
                    sin_repetir(v)?;
                    for id in v {
                        en_zona(q, *id, &mano)?;
                    }
                    v.clone()
                }
            };
            if ids.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            for id in ids {
                mover(q, id, RefZona::de(yo, Zona::Cementerio), Hueco::Arriba)?;
            }
            Ok(())
        }
        Accion::Eliminar { jugador, por } => eliminar(q, *jugador, *por, ctx.t_ms),
        Accion::Conceder => eliminar(q, yo, None, ctx.t_ms),
        Accion::VidaFantasma { delta } => {
            let d = delta_ok(*delta)?;
            let f = q.fantasma.as_mut().ok_or(Rechazo::NoExiste)?;
            f.vida = f.vida.saturating_add(d);
            Ok(())
        }
        Accion::DanoComandanteFantasma { comandante, delta } => {
            let d = delta_ok(*delta)?;
            carta(q, *comandante)?;
            let f = q.fantasma.as_mut().ok_or(Rechazo::NoExiste)?;
            let v = f.dano_comandante.entry(*comandante).or_insert(0);
            *v = v.saturating_add(d).max(0);
            if *v == 0 {
                f.dano_comandante.remove(comandante);
            }
            f.vida = f.vida.saturating_sub(d);
            Ok(())
        }
        Accion::VerFantasma { si } => {
            q.fantasma.as_mut().ok_or(Rechazo::NoExiste)?.visible = *si;
            Ok(())
        }
        Accion::Senal { senal } => {
            q.jugador_mut(yo).ok_or(Rechazo::NoExiste)?.senal = *senal;
            Ok(())
        }
        // ── Turno ──
        Accion::PasarTurno => pasar_turno(q, ctx.t_ms),
        Accion::SaltarTurno { jugador } => {
            if *jugador != q.turno.activo {
                return Err(Rechazo::NoSePuede);
            }
            pasar_turno(q, ctx.t_ms)
        }
        Accion::TurnoExtra { jugador } => {
            existe_jugador(q, *jugador)?;
            if q.turno.extra.len() >= 20 {
                return Err(Rechazo::Limite);
            }
            q.turno.extra.push(*jugador);
            Ok(())
        }
        Accion::Pausar { si } => {
            q.pausada = *si;
            Ok(())
        }
        // ── Planechase / Archenemy ──
        Accion::Planeswalk => {
            let activo = RefZona::comun(ZonaComun::PlanoActivo);
            let mazo = RefZona::comun(ZonaComun::Planos);
            let antes = q.zona(&activo).to_vec();
            let siguiente = q.zona(&mazo).first().copied();
            if antes.is_empty() && siguiente.is_none() {
                return Err(Rechazo::ZonaVacia);
            }
            for id in antes {
                mover(q, id, mazo, Hueco::Abajo)?;
            }
            if let Some(id) = q.zona(&mazo).first().copied() {
                mover(q, id, activo, Hueco::Arriba)?;
            }
            Ok(())
        }
        Accion::PonerEnMarcha => {
            let planes = RefZona::de(yo, Zona::Planes);
            let id = q.zona(&planes).first().copied().ok_or(Rechazo::ZonaVacia)?;
            mover(q, id, RefZona::de(yo, Zona::PlanesEnMarcha), Hueco::Arriba)
        }
        Accion::AbandonarPlan { carta: id } => {
            en_zona(q, *id, &RefZona::de(yo, Zona::PlanesEnMarcha))?;
            mover(q, *id, RefZona::de(yo, Zona::Planes), Hueco::Abajo)
        }
        // ── Pila ──
        Accion::Resolver { carta: id, como } => {
            let pila = RefZona::comun(ZonaComun::Pila);
            en_zona(q, *id, &pila)?;
            let c = carta(q, *id)?.clone();
            if c.habilidad.is_some() {
                q.cartas.remove(id);
                if let Some(v) = q.zonas.get_mut(&pila) {
                    v.retain(|x| x != id);
                }
                q.flechas.retain(|f| !toca_carta(f, *id));
            } else {
                let como = match como {
                    ComoResolver::Auto if c.origen.info.es_permanente() => ComoResolver::Permanente,
                    ComoResolver::Auto => ComoResolver::Hechizo,
                    otro => *otro,
                };
                match como {
                    ComoResolver::Permanente => mover(
                        q,
                        *id,
                        RefZona::de(c.controlador, Zona::Campo),
                        Hueco::Campo(None),
                    )?,
                    ComoResolver::MitadAventura => {
                        mover(q, *id, RefZona::de(c.dueno, Zona::Exilio), Hueco::Arriba)?
                    }
                    ComoResolver::MitadOmen => {
                        mover(q, *id, biblioteca(c.dueno), Hueco::Arriba)?;
                        let azar = ctx.azar()?;
                        barajar_zona(q, &biblioteca(c.dueno), azar);
                    }
                    _ => mover(
                        q,
                        *id,
                        RefZona::de(c.dueno, Zona::Cementerio),
                        Hueco::Arriba,
                    )?,
                }
            }
            // Al resolverse la pila se quitan las esperas.
            for j in &mut q.jugadores {
                if j.senal == Some(Senal::Espera) {
                    j.senal = None;
                }
            }
            Ok(())
        }
        Accion::ApilarDespues { carta: id } => {
            carta(q, *id)?;
            mover(q, *id, RefZona::comun(ZonaComun::Pila), Hueco::Arriba)
        }
        // ── Montones ──
        Accion::Amontonar { cartas } => {
            sin_repetir(cartas)?;
            if cartas.len() < 2 {
                return Err(Rechazo::NoSePuede);
            }
            let (base, resto) = cartas.split_first().ok_or(Rechazo::NoSePuede)?;
            let campo = en_campo(q, *base)?;
            let b = carta(q, *base)?.clone();
            for id in resto {
                if en_campo(q, *id)? != campo {
                    return Err(Rechazo::NoSePuede);
                }
                let c = carta(q, *id)?.clone();
                if b.es_ficha && c.es_ficha && c.origen.nombre == b.origen.nombre {
                    let total = b_cantidad(q, *base)?.saturating_add(c.cantidad);
                    if total > MAX_CANTIDAD {
                        return Err(Rechazo::Limite);
                    }
                    carta_mut(q, *base)?.cantidad = total;
                    q.cartas.remove(id);
                    if let Some(v) = q.zonas.get_mut(&RefZona::de(campo, Zona::Campo)) {
                        v.retain(|x| x != id);
                    }
                    q.flechas.retain(|f| !toca_carta(f, *id));
                    for o in q.cartas.values_mut() {
                        if o.adjunta_a == Some(*id) {
                            o.adjunta_a = None;
                        }
                    }
                } else {
                    let m = carta_mut(q, *id)?;
                    m.agrupada_bajo = Some(*base);
                    m.pos = b.pos;
                }
            }
            Ok(())
        }
        Accion::SacarDelMonton { monton, n } => {
            let campo = en_campo(q, *monton)?;
            let b = carta(q, *monton)?.clone();
            if *n == 0 {
                return Err(Rechazo::Limite);
            }
            let pos = desplazada(b.pos, 1);
            if b.es_ficha && b.cantidad > 1 {
                if *n >= b.cantidad {
                    return Err(Rechazo::Limite);
                }
                carta_mut(q, *monton)?.cantidad -= *n;
                let nueva = crear_ficha(q, campo, b.origen.clone(), *n, pos, b.copia_de)?;
                carta_mut(q, nueva)?.cara = b.cara;
            } else {
                let bajo: Vec<IdCarta> = q
                    .zona(&RefZona::de(campo, Zona::Campo))
                    .iter()
                    .copied()
                    .filter(|id| {
                        q.cartas
                            .get(id)
                            .is_some_and(|c| c.agrupada_bajo == Some(*monton))
                    })
                    .collect();
                if bajo.is_empty() {
                    return Err(Rechazo::ZonaVacia);
                }
                for (k, id) in bajo.into_iter().rev().take(*n as usize).enumerate() {
                    let c = carta_mut(q, id)?;
                    c.agrupada_bajo = None;
                    c.pos = desplazada(b.pos, k as u32 + 1);
                }
            }
            Ok(())
        }
        Accion::CambiarMonton { monton, delta } => {
            let campo = en_campo(q, *monton)?;
            let c = carta(q, *monton)?;
            if !c.es_ficha {
                return Err(Rechazo::NoSePuede);
            }
            let nueva = i64::from(c.cantidad) + i64::from(delta_ok(*delta)?);
            if nueva > i64::from(MAX_CANTIDAD) {
                return Err(Rechazo::Limite);
            }
            if nueva <= 0 {
                q.cartas.remove(monton);
                if let Some(v) = q.zonas.get_mut(&RefZona::de(campo, Zona::Campo)) {
                    v.retain(|x| x != monton);
                }
                soltar_de_campo(q, *monton);
            } else {
                carta_mut(q, *monton)?.cantidad = nueva as u32;
            }
            Ok(())
        }
        Accion::Separar { monton } => {
            let campo = en_campo(q, *monton)?;
            let b = carta(q, *monton)?.clone();
            if b.es_ficha && b.cantidad > 1 {
                if b.cantidad > MAX_FICHAS_SUELTAS {
                    return Err(Rechazo::Limite);
                }
                carta_mut(q, *monton)?.cantidad = 1;
                for k in 1..b.cantidad {
                    let nueva = crear_ficha(
                        q,
                        campo,
                        b.origen.clone(),
                        1,
                        desplazada(b.pos, k),
                        b.copia_de,
                    )?;
                    carta_mut(q, nueva)?.cara = b.cara;
                }
            } else {
                let bajo: Vec<IdCarta> = q
                    .cartas
                    .values()
                    .filter(|c| c.agrupada_bajo == Some(*monton))
                    .map(|c| c.id)
                    .collect();
                for (k, id) in bajo.into_iter().enumerate() {
                    let c = carta_mut(q, id)?;
                    c.agrupada_bajo = None;
                    if c.pos.is_some() {
                        c.pos = desplazada(b.pos, k as u32 + 1);
                    }
                }
            }
            Ok(())
        }
        // ── Exilio, cementerio ──
        Accion::Agrupar { cartas, bajo } => {
            if let Some(b) = bajo {
                carta(q, *b)?;
                if cartas.contains(b) {
                    return Err(Rechazo::NoSePuede);
                }
            }
            cada(q, cartas, |c| {
                c.agrupada_bajo = *bajo;
                Ok(())
            })
        }
        Accion::AlAzarDe { zona, filtro, a } => {
            let z = zona_del(q, yo, *zona)?;
            let candidatas: Vec<IdCarta> = q
                .zona(&z)
                .iter()
                .copied()
                .filter(|id| q.cartas.get(id).is_some_and(|c| filtro.cumple(&c.origen)))
                .collect();
            if candidatas.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            let i = ctx.azar()?.menor_que(candidatas.len() as u64) as usize;
            let id = *candidatas.get(i).ok_or(Rechazo::NoExiste)?;
            let destino = RefZona::de(yo, *a);
            let hueco = if destino.es_campo() {
                Hueco::Campo(None)
            } else {
                Hueco::Arriba
            };
            mover(q, id, destino, hueco)
        }
        Accion::TodoAlFondoAlAzar { zona } => {
            let z = zona_del(q, yo, *zona)?;
            let mut ids = q.zona(&z).to_vec();
            if ids.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            ctx.azar()?.barajar(&mut ids);
            for id in ids {
                mover(q, id, biblioteca(yo), Hueco::Abajo)?;
            }
            Ok(())
        }
        Accion::BarajarTodoDentro { zona } => {
            let z = zona_del(q, yo, *zona)?;
            let ids = q.zona(&z).to_vec();
            for id in ids {
                mover(q, id, biblioteca(yo), Hueco::Arriba)?;
            }
            let azar = ctx.azar()?;
            barajar_zona(q, &biblioteca(yo), azar);
            Ok(())
        }
        Accion::ExiliarTodo { zona } => {
            let z = zona_del(q, yo, *zona)?;
            let ids = q.zona(&z).to_vec();
            if ids.is_empty() {
                return Err(Rechazo::ZonaVacia);
            }
            for id in ids {
                mover(q, id, RefZona::de(yo, Zona::Exilio), Hueco::Arriba)?;
            }
            Ok(())
        }
        // ── Sparring ──
        Accion::IntensidadSparring { intensidad } => {
            q.sparring.intensidad = *intensidad;
            Ok(())
        }
        Accion::DecidirSparring {
            reaccion,
            decision,
            eleccion,
        } => decidir_sparring(q, *reaccion, *decision, *eleccion),
        // ── Comunicación ──
        Accion::Chat { texto } => {
            if texto.trim().is_empty() || texto.chars().count() > MAX_CHAT {
                return Err(Rechazo::Limite);
            }
            Ok(())
        }
        Accion::Aviso { .. } => Ok(()),
        // ── Fin ──
        Accion::Terminar { resultado } => {
            for j in &resultado.ganadores {
                existe_jugador(q, *j)?;
            }
            if resultado.nota.chars().count() > MAX_CHAT {
                return Err(Rechazo::Limite);
            }
            q.fin = Some(resultado.clone());
            Ok(())
        }
        // Los hace la sesión (`registro.rs`).
        Accion::Reiniciar | Accion::Deshacer { .. } | Accion::Rehacer => {
            let _ = seq;
            Err(Rechazo::NoSePuede)
        }
        Accion::TirarDados { .. } | Accion::Moneda { .. } | Accion::DadoPlanar => {
            // Llegan resueltos como `Cambio::Dados`, `Monedas`, `DadoPlanar`.
            Err(Rechazo::NoSePuede)
        }
    }
}

fn b_cantidad(q: &Partida, id: IdCarta) -> Result<u32, Rechazo> {
    Ok(carta(q, id)?.cantidad)
}

fn limite(n: u32) -> Result<(), Rechazo> {
    if n == 0 || n > 1_000 {
        Err(Rechazo::Limite)
    } else {
        Ok(())
    }
}

/// Un nombre de contador o palabra clave: sin espacios a los lados, de 1 a
/// 30 caracteres.
fn nombre_corto(s: &str) -> Result<String, Rechazo> {
    let t = s.trim();
    if t.is_empty() || t.chars().count() > 30 {
        return Err(Rechazo::Limite);
    }
    Ok(t.to_string())
}

/// Hace `f` con las `n` de arriba de la biblioteca del actor.
fn de_arriba<F>(q: &mut Partida, yo: IdJugador, n: u32, mut f: F) -> Result<(), Rechazo>
where
    F: FnMut(&mut Partida, IdCarta) -> Result<(), Rechazo>,
{
    let ids: Vec<IdCarta> = q
        .zona(&biblioteca(yo))
        .iter()
        .take(n as usize)
        .copied()
        .collect();
    if ids.is_empty() {
        return Err(Rechazo::ZonaVacia);
    }
    for id in ids {
        f(q, id)?;
    }
    Ok(())
}

fn eliminar(
    q: &mut Partida,
    j: IdJugador,
    por: Option<IdJugador>,
    t_ms: u64,
) -> Result<(), Rechazo> {
    if let Some(p) = por {
        existe_jugador(q, p)?;
    }
    let ronda = q.turno.ronda;
    let x = q.jugador_mut(j).ok_or(Rechazo::NoExiste)?;
    if x.eliminado.is_some() {
        return Err(Rechazo::NoSePuede);
    }
    x.eliminado = Some(Eliminacion { ronda, por });
    // En el mulligan, el que se va ya no hace falta para empezar.
    if let Some(m) = q.mulligan.clone() {
        let faltan = q
            .jugadores
            .iter()
            .any(|x| x.eliminado.is_none() && !m.decididos.contains(&x.id));
        if !faltan && q.jugadores.iter().any(|x| x.eliminado.is_none()) {
            q.mulligan = None;
            q.turno.numero = 1;
            q.turno.ronda = 1;
            q.turno.desde_ms = t_ms;
            let primero = if q
                .jugador(q.turno.primero)
                .is_some_and(|x| x.eliminado.is_none())
            {
                q.turno.primero
            } else {
                siguiente_asiento(q, q.turno.primero)
                    .map(|x| x.0)
                    .ok_or(Rechazo::NoSePuede)?
            };
            q.turno.primero = primero;
            q.turno.activo = primero;
            empezar_turno(q, primero)?;
        }
        return Ok(());
    }
    // Si el que muere es el activo, el turno pasa solo.
    if q.turno.activo == j && q.jugadores.iter().any(|x| x.eliminado.is_none()) {
        pasar_turno(q, t_ms)?;
    }
    Ok(())
}

fn organizar(q: &mut Partida, j: IdJugador) {
    let campo = RefZona::de(j, Zona::Campo);
    let ids: Vec<IdCarta> = q.zona(&campo).to_vec();
    let mut filas: [Vec<(String, IdCarta)>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for id in &ids {
        let Some(c) = q.cartas.get(id) else { continue };
        if c.adjunta_a.is_some() || c.agrupada_bajo.is_some() {
            continue;
        }
        let i = match fila(&c.origen.info) {
            1_600 => 0,
            8_000 => 2,
            _ => 1,
        };
        // Boca abajo no se sabe qué es: con las criaturas.
        let i = if c.boca_abajo { 0 } else { i };
        if let Some(f) = filas.get_mut(i) {
            f.push((c.origen.nombre.to_lowercase(), *id));
        }
    }
    let ys = [1_600u16, 4_800, 8_000];
    for (f, y) in filas.iter_mut().zip(ys) {
        f.sort();
        let n = f.len() as u32;
        if n == 0 {
            continue;
        }
        let paso = (9_200 / n).min(720);
        for (k, (_, id)) in f.iter().enumerate() {
            let x = (400 + k as u32 * paso).min(u32::from(POS_MAX)) as u16;
            if let Some(c) = q.cartas.get_mut(id) {
                c.pos = Some(Pos::nueva(x, y));
            }
        }
    }
    let ids_campo: Vec<IdCarta> = ids;
    for id in ids_campo {
        let base = q.cartas.get(&id).and_then(|c| c.agrupada_bajo);
        if let Some(b) = base {
            let pos = q.cartas.get(&b).and_then(|c| c.pos);
            if let Some(c) = q.cartas.get_mut(&id) {
                c.pos = pos;
            }
        }
    }
}

fn decidir_sparring(
    q: &mut Partida,
    reaccion: u64,
    decision: Decision,
    eleccion: Option<IdCarta>,
) -> Result<(), Rechazo> {
    let i = q
        .sparring
        .reacciones
        .iter()
        .position(|r| r.reaccion.id == reaccion)
        .ok_or(Rechazo::NoExiste)?;
    let r = q
        .sparring
        .reacciones
        .get(i)
        .cloned()
        .ok_or(Rechazo::NoExiste)?;
    if r.decision.is_some() {
        return Err(Rechazo::NoSePuede);
    }
    if decision == Decision::Aceptar {
        let objetivos: Vec<IdCarta> = match r.reaccion.tipo {
            TipoReaccion::Sacrificio => {
                let e = eleccion.ok_or(Rechazo::NoSePuede)?;
                en_campo(q, e)?;
                vec![e]
            }
            _ => r.reaccion.objetivo.clone(),
        };
        for id in objetivos {
            // Lo que ya no está donde estaba se salta (lo movió el jugador).
            let Some((z, _)) = q.donde(id) else { continue };
            let Some(c) = q.cartas.get(&id).cloned() else {
                continue;
            };
            let destino = match r.reaccion.tipo {
                TipoReaccion::Contrarrestar => {
                    // En la pila, o ya en el campo si se jugó sin pasar por ella.
                    if z != RefZona::comun(ZonaComun::Pila) && !z.es_campo() {
                        continue;
                    }
                    RefZona::de(c.dueno, Zona::Cementerio)
                }
                TipoReaccion::Devolver => {
                    if !z.es_campo() {
                        continue;
                    }
                    RefZona::de(c.dueno, Zona::Mano)
                }
                _ => {
                    if !z.es_campo() {
                        continue;
                    }
                    if r.reaccion.exilia {
                        RefZona::de(c.dueno, Zona::Exilio)
                    } else {
                        RefZona::de(c.dueno, Zona::Cementerio)
                    }
                }
            };
            mover(q, id, destino, Hueco::Arriba)?;
        }
    }
    if let Some(x) = q.sparring.reacciones.get_mut(i) {
        x.decision = Some(decision);
    }
    Ok(())
}

/// La cara del dado planar (1 = planeswalk, 2 = caos, el resto nada).
pub fn cara_planar(valor: u32) -> CaraPlanar {
    match valor {
        1 => CaraPlanar::Planeswalk,
        2 => CaraPlanar::Caos,
        _ => CaraPlanar::Nada,
    }
}
