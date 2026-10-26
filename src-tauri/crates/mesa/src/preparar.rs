//! **Del deck de la app a la mesa.**
//!
//! La app construye el `DeckDeMesa` (sabe pintar las imágenes: Fase B); el
//! motor solo decide en qué sección va cada carta (`secciones_de_mesa`, por
//! la lista escrita y las marcas) y reparte la partida (`iniciar`).

use std::collections::BTreeMap;

use proxy_core::decklist::{seccion_de, secciones_de, Section};
use proxy_core::model::{cuentan, CardSource, Deck, DeckCard, MarcaCarta};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::azar::Azar;
use crate::error::Rechazo;
use crate::formato::{Formato, ModoEquipos, Opciones, Variantes, VIDA_ARCHIENEMIGO};
use crate::ids::{IdCarta, IdEquipo, IdJugador};
use crate::lista::ListaDeJuego;
use crate::modelo::{
    Carta, Designaciones, Equipo, EstadoMulligan, Jugador, OrigenCarta, Partida, RefImagen,
    RefZona, Turno, VidaFantasma, Zona, ZonaComun,
};
use crate::sparring::EstadoSparring;

pub const MAX_JUGADORES: usize = 10;

/// En qué parte de la partida empieza una carta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Seccion {
    Mazo,
    Comandante,
    Companero,
    Banquillo,
    /// Fichas a mano para crearlas (no son cartas de la partida).
    Ficha,
    Plano,
    Plan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartaDeMesa {
    pub carta_deck: String,
    pub seccion: Seccion,
    pub origen: OrigenCarta,
}

/// Un deck listo para la mesa (con sus imágenes ya preparadas por la app).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckDeMesa {
    pub nombre: String,
    pub cartas: Vec<CartaDeMesa>,
    pub tapete: Option<RefImagen>,
    pub dorso: RefImagen,
}

fn es_ficha(c: &DeckCard) -> bool {
    let tipo_ficha = c.meta.type_line.as_deref().is_some_and(|t| {
        let t = t.to_lowercase();
        t.starts_with("token") || t.contains(" token") || t.starts_with("emblem")
    });
    match &c.source {
        CardSource::Token { .. } => true,
        CardSource::Custom { externa, .. } => externa.is_some() || tipo_ficha,
        _ => tipo_ficha,
    }
}

fn layout_de(c: &DeckCard) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if let Some(l) = &c.meta.layout {
        v.push(l.to_lowercase());
    }
    if let Some(d) = &c.design {
        v.push(d.card.layout.to_lowercase());
    }
    v
}

/// La sección de cada carta del deck en la mesa. Las opcionales y la
/// maybeboard se quedan fuera; de las variantes, una (como `cuentan`); el
/// banquillo marcado va al banquillo; los huecos en blanco, fuera.
pub fn secciones_de_mesa<'a>(
    deck: &'a Deck,
    lista: Option<&ListaDeJuego>,
) -> Vec<(&'a DeckCard, Seccion)> {
    let secciones = secciones_de(&deck.decklist_text);
    let (validas, _) = cuentan(&deck.cards);
    let cuenta: std::collections::BTreeSet<&str> = validas.iter().map(|c| c.id.as_str()).collect();
    // Con una lista para jugar, solo lo elegido (y lo que no es del mazo:
    // fichas, planos, planes y el banquillo marcado).
    let elegidas = lista.map(|l| {
        let comandantes: std::collections::BTreeSet<&str> =
            l.comandantes.iter().map(String::as_str).collect();
        let cartas: std::collections::BTreeSet<&str> =
            l.cartas.iter().map(String::as_str).collect();
        (comandantes, cartas)
    });
    let mut out = Vec::new();
    for c in &deck.cards {
        if matches!(c.source, CardSource::Blank) {
            continue;
        }
        let layouts = layout_de(c);
        let seccion_lista = seccion_de(&secciones, &c.meta.name);
        let s = if es_ficha(c) || seccion_lista == Section::Tokens {
            Seccion::Ficha
        } else if layouts.iter().any(|l| l == "planar" || l == "phenomenon") {
            Seccion::Plano
        } else if layouts.iter().any(|l| l == "scheme") {
            Seccion::Plan
        } else if let Some((comandantes, cartas)) = &elegidas {
            if comandantes.contains(c.id.as_str()) {
                Seccion::Comandante
            } else if cartas.contains(c.id.as_str()) {
                Seccion::Mazo
            } else if c.marca == Some(MarcaCarta::Banquillo) {
                Seccion::Banquillo
            } else {
                continue;
            }
        } else {
            match c.marca {
                Some(MarcaCarta::Opcional) => continue,
                Some(MarcaCarta::Banquillo) => Seccion::Banquillo,
                Some(MarcaCarta::Variante) if !cuenta.contains(c.id.as_str()) => continue,
                _ => match seccion_lista {
                    Section::Main | Section::Tokens => Seccion::Mazo,
                    Section::Commander => Seccion::Comandante,
                    Section::Companion => Seccion::Companero,
                    Section::Sideboard => Seccion::Banquillo,
                    Section::Maybeboard => continue,
                },
            }
        };
        out.push((c, s));
    }
    out
}

/// Las copias de un deck que se pueden elegir para una lista para jugar:
/// todas menos los huecos en blanco, las fichas, los planos y los planes
/// (esos van a la partida por su cuenta), marcadas o no.
pub fn candidatas_de_lista(deck: &Deck) -> Vec<&DeckCard> {
    let secciones = secciones_de(&deck.decklist_text);
    deck.cards
        .iter()
        .filter(|c| !matches!(c.source, CardSource::Blank))
        .filter(|c| !es_ficha(c) && seccion_de(&secciones, &c.meta.name) != Section::Tokens)
        .filter(|c| {
            !layout_de(c)
                .iter()
                .any(|l| l == "planar" || l == "phenomenon" || l == "scheme")
        })
        .collect()
}

/// Un jugador que se sienta a la mesa.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JugadorNuevo {
    pub nombre: String,
    pub color: String,
    pub equipo: Option<IdEquipo>,
    pub mazo: DeckDeMesa,
}

/// Todo lo que hace falta para empezar una partida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub formato: Formato,
    pub variantes: Variantes,
    pub opciones: Opciones,
    pub jugadores: Vec<JugadorNuevo>,
    pub equipos: Vec<Equipo>,
    /// Quién empieza; `None` = al azar.
    pub primero: Option<IdJugador>,
}

/// Reparte la partida: ids, comandantes al mando, banquillo aparte, planos
/// al mazo común y planes al de su jugador, bibliotecas barajadas y la mano
/// inicial de cada uno (todos quedan «decidiendo» el mulligan).
pub fn iniciar(
    id: String,
    config: Config,
    semilla: u64,
    ahora_ms: u64,
) -> Result<Partida, Rechazo> {
    let n = config.jugadores.len();
    if n == 0 || n > MAX_JUGADORES {
        return Err(Rechazo::Limite);
    }
    let mut azar = Azar::new(semilla);
    let opciones = config.opciones.clone();
    let mut equipos = config.equipos.clone();
    let mut equipo_de: Vec<Option<IdEquipo>> = config.jugadores.iter().map(|j| j.equipo).collect();
    if config.variantes.equipos == ModoEquipos::DosCabezas && equipos.is_empty() {
        // Por parejas de asientos, con la vida compartida.
        for k in 0..n.div_ceil(2) {
            let e = IdEquipo(k as u8);
            equipos.push(Equipo {
                id: e,
                nombre: String::new(),
                color: String::new(),
                vida_compartida: Some(opciones.vida_inicial),
            });
            for (i, slot) in equipo_de.iter_mut().enumerate() {
                if i / 2 == k {
                    *slot = Some(e);
                }
            }
        }
    }
    for e in equipo_de.iter().flatten() {
        if !equipos.iter().any(|x| x.id == *e) {
            return Err(Rechazo::NoExiste);
        }
    }
    let mut p = Partida {
        id,
        formato: config.formato,
        variantes: config.variantes.clone(),
        opciones: opciones.clone(),
        jugadores: Vec::new(),
        equipos,
        cartas: BTreeMap::new(),
        zonas: BTreeMap::new(),
        turno: Turno {
            numero: 0,
            ronda: 0,
            activo: IdJugador(0),
            primero: IdJugador(0),
            desde_ms: ahora_ms,
            extra: Vec::new(),
            tiradas_planar: 0,
        },
        flechas: Vec::new(),
        bolitas: Vec::new(),
        designaciones: Designaciones::default(),
        pausada: false,
        fantasma: (n == 1).then(|| VidaFantasma {
            vida: opciones.vida_inicial,
            dano_comandante: BTreeMap::new(),
            visible: true,
        }),
        sparring: EstadoSparring::default(),
        mulligan: Some(EstadoMulligan::default()),
        seq: 0,
        siguiente_id: 1,
        fin: None,
    };
    let mut planos: Vec<IdCarta> = Vec::new();
    for (i, j) in config.jugadores.iter().enumerate() {
        let yo = IdJugador(i as u8);
        let vida = if config.variantes.archenemy == Some(yo) {
            VIDA_ARCHIENEMIGO
        } else {
            opciones.vida_inicial
        };
        for z in [
            Zona::Biblioteca,
            Zona::Mano,
            Zona::Campo,
            Zona::Cementerio,
            Zona::Exilio,
            Zona::Mando,
        ] {
            p.zonas.insert(RefZona::de(yo, z), Vec::new());
        }
        let mut fichas: Vec<OrigenCarta> = Vec::new();
        for c in &j.mazo.cartas {
            if c.seccion == Seccion::Ficha {
                if !fichas.iter().any(|f| f.nombre == c.origen.nombre) {
                    fichas.push(c.origen.clone());
                }
                continue;
            }
            let cid = IdCarta(p.siguiente_id);
            p.siguiente_id += 1;
            let mut carta = Carta::nueva(cid, yo, c.origen.clone());
            let zona = match c.seccion {
                Seccion::Mazo => Zona::Biblioteca,
                Seccion::Comandante => {
                    carta.origen.info.es_comandante = true;
                    Zona::Mando
                }
                Seccion::Companero => Zona::Mando,
                Seccion::Banquillo => Zona::Banquillo,
                Seccion::Plan => Zona::Planes,
                Seccion::Plano => {
                    p.cartas.insert(cid, carta);
                    planos.push(cid);
                    continue;
                }
                Seccion::Ficha => continue,
            };
            p.cartas.insert(cid, carta);
            p.zonas.entry(RefZona::de(yo, zona)).or_default().push(cid);
        }
        if let Some(lib) = p.zonas.get_mut(&RefZona::de(yo, Zona::Biblioteca)) {
            azar.barajar(lib);
        }
        if let Some(planes) = p.zonas.get_mut(&RefZona::de(yo, Zona::Planes)) {
            azar.barajar(planes);
        }
        p.jugadores.push(Jugador {
            id: yo,
            nombre: j.nombre.clone(),
            color: j.color.clone(),
            equipo: equipo_de.get(i).copied().flatten(),
            vida,
            veneno: 0,
            contadores: BTreeMap::new(),
            dano_comandante: BTreeMap::new(),
            lanzamientos: BTreeMap::new(),
            sin_coste_extra: Default::default(),
            tapete: j.mazo.tapete.clone(),
            mulligans: 0,
            eliminado: None,
            mano_revelada: false,
            de_arriba_revelada: false,
            senal: None,
            fichas,
            dorso: j.mazo.dorso.clone(),
        });
    }
    // Planechase: los planos de todos en un mazo común y el primero, boca
    // arriba.
    if !planos.is_empty() {
        azar.barajar(&mut planos);
        if config.variantes.planechase {
            let primero = planos.remove(0);
            p.zonas
                .insert(RefZona::comun(ZonaComun::PlanoActivo), vec![primero]);
        }
        p.zonas.insert(RefZona::comun(ZonaComun::Planos), planos);
    }
    p.zonas.entry(RefZona::comun(ZonaComun::Pila)).or_default();
    let primero = match config.primero {
        Some(j) if (j.0 as usize) < n => j,
        Some(_) => return Err(Rechazo::NoExiste),
        None => IdJugador(azar.menor_que(n as u64) as u8),
    };
    p.turno.activo = primero;
    p.turno.primero = primero;
    // La mano inicial de cada uno.
    let mano = usize::from(opciones.mano_inicial);
    for i in 0..n {
        let yo = IdJugador(i as u8);
        let lib = p
            .zonas
            .entry(RefZona::de(yo, Zona::Biblioteca))
            .or_default();
        let k = mano.min(lib.len());
        let robadas: Vec<IdCarta> = lib.drain(..k).collect();
        p.zonas
            .entry(RefZona::de(yo, Zona::Mano))
            .or_default()
            .extend(robadas);
    }
    Ok(p)
}
