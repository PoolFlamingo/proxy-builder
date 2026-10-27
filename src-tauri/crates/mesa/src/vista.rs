//! **Lo que ve cada uno**: la vista de la mesa para la pantalla y sus
//! parches.
//!
//! 🛑 Es lo que guarda lo oculto. De una zona que el mirador no ve solo va
//! **el número** de cartas (ni ids ni orden: no se puede seguir una carta
//! conocida por su id después de barajar), más las que conozca (la de
//! arriba revelada, las que está mirando), con su sitio. Una carta boca
//! abajo que no conoce va sin `origen`, solo con su dorso.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::aplicar::Tocado;
use crate::formato::{Formato, Opciones, Variantes};
use crate::ids::{IdCarta, IdEquipo, IdJugador};
use crate::modelo::{
    Bolita, Carta, Designaciones, Eliminacion, Equipo, EstadoMulligan, Etiqueta, Flecha, Jugador,
    OrigenCarta, Partida, Pos, Pt, RefImagen, RefZona, Resultado, Senal, Turno, VidaFantasma, Zona,
};
use crate::muerte::{fuera_de_juego, Causa};
use crate::sparring::EstadoSparring;
use crate::visibilidad::{ve, zona_abierta, Mirador};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaMesa {
    /// El `seq` del último evento.
    #[ts(type = "number")]
    pub version: u64,
    /// Quién mira (`None` = espectador o repetición).
    pub yo: Option<IdJugador>,
    pub formato: Formato,
    pub variantes: Variantes,
    pub opciones: Opciones,
    pub jugadores: Vec<VistaJugador>,
    /// Ordenadas por id.
    pub cartas: Vec<VistaCarta>,
    /// Ordenadas por zona.
    pub zonas: Vec<VistaZona>,
    pub general: VistaGeneral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaGeneral {
    pub turno: Turno,
    pub equipos: Vec<Equipo>,
    pub flechas: Vec<Flecha>,
    pub bolitas: Vec<Bolita>,
    pub designaciones: Designaciones,
    pub pausada: bool,
    pub fantasma: Option<VidaFantasma>,
    pub sparring: EstadoSparring,
    pub mulligan: Option<EstadoMulligan>,
    pub fin: Option<Resultado>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaJugador {
    pub id: IdJugador,
    pub nombre: String,
    pub color: String,
    pub equipo: Option<IdEquipo>,
    pub vida: i32,
    pub veneno: i32,
    pub contadores: BTreeMap<String, i32>,
    pub dano_comandante: BTreeMap<IdCarta, i32>,
    pub lanzamientos: BTreeMap<IdCarta, u32>,
    pub sin_coste_extra: Vec<IdCarta>,
    pub tapete: Option<RefImagen>,
    pub mulligans: u8,
    pub eliminado: Option<Eliminacion>,
    /// Ha llegado a un límite (vida, veneno, comandante) y no ha sido
    /// eliminado: la mesa lo avisa y la persona lo confirma (G8).
    pub fuera_de_juego: Option<Causa>,
    pub mano_revelada: bool,
    pub de_arriba_revelada: bool,
    pub senal: Option<Senal>,
    /// Las fichas de su deck (solo las del que mira).
    pub fichas: Vec<OrigenCarta>,
    pub dorso: RefImagen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaCarta {
    pub id: IdCarta,
    pub zona: RefZona,
    pub dueno: IdJugador,
    pub controlador: IdJugador,
    /// `None`: boca abajo y no la conoce.
    pub origen: Option<OrigenCarta>,
    pub dorso: RefImagen,
    pub cara: u8,
    pub girada: bool,
    pub boca_abajo: bool,
    pub fase_fuera: bool,
    pub no_endereza: bool,
    pub contadores: BTreeMap<String, i32>,
    pub palabras_clave: Vec<String>,
    pub etiquetas: Vec<Etiqueta>,
    pub pt: Pt,
    pub adjunta_a: Option<IdCarta>,
    pub pos: Option<Pos>,
    pub z: u32,
    pub resaltada: Option<u8>,
    pub es_ficha: bool,
    pub copia_de: Option<IdCarta>,
    pub cantidad: u32,
    pub agrupada_bajo: Option<IdCarta>,
    pub llego_en_ronda: u32,
    pub habilidad: Option<String>,
    /// La está mirando el que mira (escrutar, buscar): para que la
    /// pantalla sepa qué enseñar en el diálogo.
    pub mirada: bool,
    /// Revelada a todos (en una zona oculta).
    pub revelada: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaZona {
    pub zona: RefZona,
    pub cuantas: u32,
    /// Las cartas en orden, si el que mira ve la zona.
    pub cartas: Option<Vec<IdCarta>>,
    /// En una zona oculta, las que conoce y su sitio.
    pub visibles: Vec<(u32, IdCarta)>,
}

/// Solo lo que cambió.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParcheVista {
    #[ts(type = "number")]
    pub version: u64,
    pub cartas: Vec<VistaCarta>,
    pub quitadas: Vec<IdCarta>,
    pub zonas: Vec<VistaZona>,
    pub jugadores: Vec<VistaJugador>,
    pub general: Option<VistaGeneral>,
}

fn vista_jugador(p: &Partida, j: &Jugador, m: Mirador) -> VistaJugador {
    VistaJugador {
        id: j.id,
        nombre: j.nombre.clone(),
        color: j.color.clone(),
        equipo: j.equipo,
        vida: j.vida,
        veneno: j.veneno,
        contadores: j.contadores.clone(),
        dano_comandante: j.dano_comandante.clone(),
        lanzamientos: j.lanzamientos.clone(),
        sin_coste_extra: j.sin_coste_extra.iter().copied().collect(),
        tapete: j.tapete.clone(),
        mulligans: j.mulligans,
        eliminado: j.eliminado.clone(),
        fuera_de_juego: fuera_de_juego(p, j.id),
        mano_revelada: j.mano_revelada,
        de_arriba_revelada: j.de_arriba_revelada,
        senal: j.senal,
        fichas: if m.jugador() == Some(j.id) || m == Mirador::Todo {
            j.fichas.clone()
        } else {
            Vec::new()
        },
        dorso: j.dorso.clone(),
    }
}

fn vista_general(p: &Partida) -> VistaGeneral {
    VistaGeneral {
        turno: p.turno.clone(),
        equipos: p.equipos.clone(),
        flechas: p.flechas.clone(),
        bolitas: p.bolitas.clone(),
        designaciones: p.designaciones.clone(),
        pausada: p.pausada,
        fantasma: p.fantasma.clone(),
        sparring: p.sparring.clone(),
        mulligan: p.mulligan.clone(),
        fin: p.fin.clone(),
    }
}

fn vista_carta(c: &Carta, zona: RefZona, conocida: bool, m: Mirador) -> VistaCarta {
    let yo = m.jugador();
    VistaCarta {
        id: c.id,
        zona,
        dueno: c.dueno,
        controlador: c.controlador,
        origen: conocida.then(|| c.origen.clone()),
        dorso: c.origen.dorso.clone(),
        cara: if conocida { c.cara } else { 0 },
        girada: c.girada,
        boca_abajo: c.boca_abajo,
        fase_fuera: c.fase_fuera,
        no_endereza: c.no_endereza,
        contadores: c.contadores.clone(),
        palabras_clave: c.palabras_clave.iter().cloned().collect(),
        etiquetas: c.etiquetas.clone(),
        pt: c.pt.clone(),
        adjunta_a: c.adjunta_a,
        pos: c.pos,
        z: c.z,
        resaltada: c.resaltada,
        es_ficha: c.es_ficha,
        copia_de: c.copia_de,
        cantidad: c.cantidad,
        agrupada_bajo: c.agrupada_bajo,
        llego_en_ronda: c.llego_en_ronda,
        habilidad: c.habilidad.clone(),
        mirada: yo.is_some()
            && !zona.es_publica()
            && match &c.vista_por {
                crate::modelo::Vista::Jugadores(v) => yo.is_some_and(|y| v.contains(&y)),
                _ => false,
            },
        revelada: c.vista_por == crate::modelo::Vista::Todos,
    }
}

fn vista_zona(p: &Partida, z: &RefZona, m: Mirador) -> VistaZona {
    let ids = p.zona(z);
    if zona_abierta(p, z, m) {
        return VistaZona {
            zona: *z,
            cuantas: ids.len() as u32,
            cartas: Some(ids.to_vec()),
            visibles: Vec::new(),
        };
    }
    let visibles = ids
        .iter()
        .enumerate()
        .filter(|(i, id)| p.cartas.get(id).is_some_and(|c| ve(p, c, z, *i, m)))
        .map(|(i, id)| (i as u32, *id))
        .collect();
    VistaZona {
        zona: *z,
        cuantas: ids.len() as u32,
        cartas: None,
        visibles,
    }
}

/// La carta tal como la ve el mirador, si la ve en algo (si está en una
/// zona oculta y no la conoce, `None`: para él no existe).
fn carta_para(p: &Partida, id: IdCarta, m: Mirador) -> Option<VistaCarta> {
    let c = p.cartas.get(&id)?;
    let (z, i) = p.donde(id)?;
    let conocida = ve(p, c, &z, i, m);
    if !zona_abierta(p, &z, m) && !conocida {
        return None;
    }
    Some(vista_carta(c, z, conocida, m))
}

pub fn vista(p: &Partida, m: Mirador) -> VistaMesa {
    let mut cartas = Vec::new();
    let mut zonas = Vec::new();
    for (z, ids) in &p.zonas {
        let vz = vista_zona(p, z, m);
        let abierta = vz.cartas.is_some();
        for (i, id) in ids.iter().enumerate() {
            let Some(c) = p.cartas.get(id) else { continue };
            let conocida = ve(p, c, z, i, m);
            if abierta || conocida {
                cartas.push(vista_carta(c, *z, conocida, m));
            }
        }
        zonas.push(vz);
    }
    cartas.sort_by_key(|c| c.id);
    VistaMesa {
        version: p.seq,
        yo: m.jugador(),
        formato: p.formato,
        variantes: p.variantes.clone(),
        opciones: p.opciones.clone(),
        jugadores: p.jugadores.iter().map(|j| vista_jugador(p, j, m)).collect(),
        cartas,
        zonas,
        general: vista_general(p),
    }
}

/// El parche de lo tocado por un evento, para un mirador.
pub fn parche(p: &Partida, t: &Tocado, m: Mirador) -> ParcheVista {
    let mut zonas: BTreeSet<RefZona> = t.zonas.clone();
    // Lo que ve uno de las zonas de un jugador depende de él (mano
    // revelada, la de arriba revelada).
    for j in &t.jugadores {
        for z in [Zona::Mano, Zona::Biblioteca, Zona::Banquillo] {
            zonas.insert(RefZona::de(*j, z));
        }
    }
    // Lo que se sabe de una zona oculta (`visibles`) cambia con sus cartas.
    for id in &t.cartas {
        if let Some((z, _)) = p.donde(*id) {
            zonas.insert(z);
        }
    }
    let mut ids: BTreeSet<IdCarta> = t.cartas.clone();
    for z in &zonas {
        ids.extend(p.zona(z).iter().copied());
    }
    let mut cartas = Vec::new();
    let mut quitadas = Vec::new();
    for id in ids {
        match carta_para(p, id, m) {
            Some(c) => cartas.push(c),
            None => quitadas.push(id),
        }
    }
    ParcheVista {
        version: p.seq,
        cartas,
        quitadas,
        zonas: zonas
            .iter()
            .filter(|z| p.zonas.contains_key(z))
            .map(|z| vista_zona(p, z, m))
            .collect(),
        jugadores: p
            .jugadores
            .iter()
            // La vida compartida de un equipo está en `general`, no en el jugador.
            .filter(|j| t.jugadores.contains(&j.id) || (t.general && j.equipo.is_some()))
            .map(|j| vista_jugador(p, j, m))
            .collect(),
        general: t.general.then(|| vista_general(p)),
    }
}

/// Aplica un parche sobre una vista (lo mismo que hace `mesa-store` en la
/// pantalla; aquí, para los tests).
pub fn aplicar_parche(v: &mut VistaMesa, pa: &ParcheVista) {
    let mut cartas: BTreeMap<IdCarta, VistaCarta> = v.cartas.drain(..).map(|c| (c.id, c)).collect();
    for id in &pa.quitadas {
        cartas.remove(id);
    }
    for c in &pa.cartas {
        cartas.insert(c.id, c.clone());
    }
    v.cartas = cartas.into_values().collect();
    let mut zonas: BTreeMap<RefZona, VistaZona> = v.zonas.drain(..).map(|z| (z.zona, z)).collect();
    for z in &pa.zonas {
        zonas.insert(z.zona, z.clone());
    }
    v.zonas = zonas.into_values().collect();
    for j in &pa.jugadores {
        if let Some(x) = v.jugadores.iter_mut().find(|x| x.id == j.id) {
            *x = j.clone();
        }
    }
    if let Some(g) = &pa.general {
        v.general = g.clone();
    }
    v.version = pa.version;
}
