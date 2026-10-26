//! **El modelo de la partida**: todo lo que hay en la mesa.
//!
//! Todo es `BTreeMap` y `Vec` (nunca `HashMap`): el JSON de una partida sale
//! siempre igual y su huella (`huella.rs`) se puede comparar entre equipos.
//! Las posiciones son enteros (`Pos`, 0–10.000) por la misma razón.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::formato::{Formato, Opciones, Variantes};
use crate::ids::{IdCarta, IdEquipo, IdJugador};
use crate::sparring::EstadoSparring;

/// Lo más lejos que llega una posición del campo en cada eje.
pub const POS_MAX: u16 = 10_000;

/// La partida entera. La lleva el host y, en red, cada jugador una copia
/// (Fase E): así cualquiera puede pasar a ser el host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Partida {
    pub id: String,
    pub formato: Formato,
    pub variantes: Variantes,
    pub opciones: Opciones,
    /// En el orden de los asientos (el de los turnos).
    pub jugadores: Vec<Jugador>,
    pub equipos: Vec<Equipo>,
    pub cartas: BTreeMap<IdCarta, Carta>,
    /// El contenido de cada zona, en orden (en la biblioteca, el 0 es la de
    /// arriba; en la pila, el 0 es la de abajo y la última, la de arriba).
    #[serde(with = "zonas_como_lista")]
    pub zonas: BTreeMap<RefZona, Vec<IdCarta>>,
    pub turno: Turno,
    pub flechas: Vec<Flecha>,
    pub bolitas: Vec<Bolita>,
    pub designaciones: Designaciones,
    pub pausada: bool,
    /// La vida del rival imaginado del solitario («Phantom life»).
    pub fantasma: Option<VidaFantasma>,
    #[serde(default)]
    pub sparring: EstadoSparring,
    /// Mientras alguien no se ha quedado con su mano inicial.
    pub mulligan: Option<EstadoMulligan>,
    /// El `seq` del último evento aplicado.
    pub seq: u64,
    pub siguiente_id: u32,
    pub fin: Option<Resultado>,
}

impl Partida {
    pub fn jugador(&self, id: IdJugador) -> Option<&Jugador> {
        self.jugadores.iter().find(|j| j.id == id)
    }

    pub fn jugador_mut(&mut self, id: IdJugador) -> Option<&mut Jugador> {
        self.jugadores.iter_mut().find(|j| j.id == id)
    }

    pub fn zona(&self, z: &RefZona) -> &[IdCarta] {
        self.zonas.get(z).map_or(&[], |v| v.as_slice())
    }

    /// Dónde está una carta: su zona y su sitio en ella.
    pub fn donde(&self, id: IdCarta) -> Option<(RefZona, usize)> {
        self.zonas
            .iter()
            .find_map(|(z, v)| v.iter().position(|c| *c == id).map(|i| (*z, i)))
    }

    pub fn equipo_de(&self, j: IdJugador) -> Option<IdEquipo> {
        self.jugador(j).and_then(|x| x.equipo)
    }

    /// Los rivales de `j` que siguen en pie. Con equipos y sin fuego amigo,
    /// los compañeros no cuentan.
    pub fn oponentes(&self, j: IdJugador) -> Vec<IdJugador> {
        let mio = self.equipo_de(j);
        let fuego_amigo = self.variantes.fuego_amigo();
        self.jugadores
            .iter()
            .filter(|x| x.id != j && x.eliminado.is_none())
            .filter(|x| fuego_amigo || mio.is_none() || x.equipo != mio)
            .map(|x| x.id)
            .collect()
    }
}

/// Una zona de un jugador o una de la mesa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum RefZona {
    De { jugador: IdJugador, zona: Zona },
    Comun { zona: ZonaComun },
}

impl RefZona {
    pub fn de(jugador: IdJugador, zona: Zona) -> Self {
        RefZona::De { jugador, zona }
    }

    pub fn comun(zona: ZonaComun) -> Self {
        RefZona::Comun { zona }
    }

    pub fn jugador(&self) -> Option<IdJugador> {
        match self {
            RefZona::De { jugador, .. } => Some(*jugador),
            RefZona::Comun { .. } => None,
        }
    }

    pub fn es(&self, z: Zona) -> bool {
        matches!(self, RefZona::De { zona, .. } if *zona == z)
    }

    pub fn es_campo(&self) -> bool {
        self.es(Zona::Campo)
    }

    /// Zonas a la vista de todos (salvo lo que esté boca abajo).
    pub fn es_publica(&self) -> bool {
        match self {
            RefZona::De { zona, .. } => matches!(
                zona,
                Zona::Campo | Zona::Cementerio | Zona::Exilio | Zona::Mando | Zona::PlanesEnMarcha
            ),
            RefZona::Comun { zona } => matches!(zona, ZonaComun::Pila | ZonaComun::PlanoActivo),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Zona {
    Biblioteca,
    Mano,
    Campo,
    Cementerio,
    Exilio,
    Mando,
    Banquillo,
    /// El mazo de planes del archienemigo.
    Planes,
    PlanesEnMarcha,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ZonaComun {
    Pila,
    /// El mazo común de planos (Planechase).
    Planos,
    PlanoActivo,
}

/// Una carta sobre la mesa (o una habilidad en la pila).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Carta {
    pub id: IdCarta,
    pub dueno: IdJugador,
    pub controlador: IdJugador,
    pub origen: OrigenCarta,
    /// La cara que se ve (0 = la de delante).
    pub cara: u8,
    pub girada: bool,
    pub boca_abajo: bool,
    pub fase_fuera: bool,
    pub no_endereza: bool,
    pub contadores: BTreeMap<String, i32>,
    /// Los «Keyword counters» (Flying, Haste…).
    pub palabras_clave: BTreeSet<String>,
    pub etiquetas: Vec<Etiqueta>,
    pub pt: Pt,
    pub adjunta_a: Option<IdCarta>,
    /// Solo en el campo.
    pub pos: Option<Pos>,
    /// Orden de pintado en el campo (la última movida, encima).
    pub z: u32,
    /// Quién conoce la cara además de lo que diga su zona (`visibilidad.rs`).
    pub vista_por: Vista,
    pub resaltada: Option<u8>,
    pub es_ficha: bool,
    pub copia_de: Option<IdCarta>,
    /// Un montón de fichas iguales es una sola carta con `cantidad > 1`.
    pub cantidad: u32,
    /// Bajo qué carta se pinta: un montón de cartas de verdad, o lo exiliado
    /// por una carta (impulsos).
    pub agrupada_bajo: Option<IdCarta>,
    /// La ronda en que llegó a su zona (el separador «Antes»).
    pub llego_en_ronda: u32,
    /// Una habilidad en la pila: el texto, y `origen` es el de la carta que
    /// la generó. Desaparece al salir de la pila.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub habilidad: Option<String>,
}

impl Carta {
    pub fn nueva(id: IdCarta, dueno: IdJugador, origen: OrigenCarta) -> Self {
        Carta {
            id,
            dueno,
            controlador: dueno,
            origen,
            cara: 0,
            girada: false,
            boca_abajo: false,
            fase_fuera: false,
            no_endereza: false,
            contadores: BTreeMap::new(),
            palabras_clave: BTreeSet::new(),
            etiquetas: Vec::new(),
            pt: Pt::default(),
            adjunta_a: None,
            pos: None,
            z: 0,
            vista_por: Vista::Nadie,
            resaltada: None,
            es_ficha: false,
            copia_de: None,
            cantidad: 1,
            agrupada_bajo: None,
            llego_en_ronda: 0,
            habilidad: None,
        }
    }

    /// Lo que pierde al salir del campo.
    pub fn limpiar_de_campo(&mut self) {
        self.girada = false;
        self.fase_fuera = false;
        self.no_endereza = false;
        self.contadores.clear();
        self.palabras_clave.clear();
        self.etiquetas.clear();
        self.pt = Pt::default();
        self.adjunta_a = None;
        self.pos = None;
        self.resaltada = None;
        self.agrupada_bajo = None;
        self.controlador = self.dueno;
    }
}

/// De dónde sale una carta: lo que la app sabe de ella al empezar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OrigenCarta {
    /// El id de la `DeckCard` del deck (vacío en las fichas libres).
    pub carta_deck: String,
    pub nombre: String,
    /// Una imagen por cara.
    pub caras: Vec<RefImagen>,
    pub dorso: RefImagen,
    pub scryfall_id: Option<String>,
    pub info: InfoCarta,
}

/// Lo mínimo de una carta para buscar, organizar, las ayudas y el sparring.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InfoCarta {
    pub tipo: Option<String>,
    /// El cmc × 10, para no usar coma flotante.
    pub cmc_decimas: Option<u16>,
    pub coste: Option<String>,
    pub colores: Vec<char>,
    pub es_tierra: bool,
    pub es_criatura: bool,
    pub es_legendaria: bool,
    pub fuerza: Option<String>,
    pub resistencia: Option<String>,
    pub lealtad: Option<String>,
    pub palabras_clave: Vec<String>,
    /// `ramp`, `draw`, `removal`… (`proxy_core::stats::funciones`).
    pub funciones: Vec<String>,
    /// El «+N/+N» fijo de un aura o un equipo (nunca «por cada»).
    pub bono_fijo: Option<(i32, i32)>,
    pub es_comandante: bool,
}

impl InfoCarta {
    fn tipo_tiene(&self, palabra: &str) -> bool {
        self.tipo
            .as_deref()
            .is_some_and(|t| t.to_lowercase().contains(&palabra.to_lowercase()))
    }

    /// Lo que se queda en el campo al resolverse.
    pub fn es_permanente(&self) -> bool {
        self.es_tierra
            || self.es_criatura
            || [
                "artifact",
                "enchantment",
                "planeswalker",
                "battle",
                "creature",
                "land",
            ]
            .iter()
            .any(|t| self.tipo_tiene(t))
    }

    pub fn es_saga(&self) -> bool {
        self.tipo_tiene("saga")
    }

    pub fn tiene_tipo(&self, palabra: &str) -> bool {
        self.tipo_tiene(palabra)
    }

    pub fn cmc(&self) -> u16 {
        self.cmc_decimas.unwrap_or(0) / 10
    }
}

/// Una imagen de una cara.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum RefImagen {
    /// Sin retoques: los demás la bajan de `cards.scryfall.io` (`url`).
    Scryfall { id: String, cara: u8, url: String },
    /// La imagen final preparada por la app (propia, retocada o diseñada).
    Propia {
        sha256: String,
        ancho: u32,
        alto: u32,
    },
    /// El reverso de la app.
    Generica,
}

/// Quién conoce la cara de una carta además de lo que diga su zona.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "jugadores", rename_all = "camelCase")]
#[ts(export)]
pub enum Vista {
    Nadie,
    Dueno,
    Todos,
    Jugadores(Vec<IdJugador>),
}

impl Vista {
    pub fn incluye(&self, m: Option<IdJugador>, dueno: IdJugador) -> bool {
        match self {
            Vista::Nadie => false,
            Vista::Todos => true,
            Vista::Dueno => m == Some(dueno),
            Vista::Jugadores(v) => m.is_some_and(|m| v.contains(&m)),
        }
    }

    /// Añade un jugador a los que la conocen.
    pub fn con(&self, j: IdJugador, dueno: IdJugador) -> Vista {
        match self {
            Vista::Todos => Vista::Todos,
            Vista::Nadie => Vista::Jugadores(vec![j]),
            Vista::Dueno if j == dueno => Vista::Dueno,
            Vista::Dueno => Vista::Jugadores(vec![dueno, j]),
            Vista::Jugadores(v) => {
                let mut v = v.clone();
                if !v.contains(&j) {
                    v.push(j);
                    v.sort();
                }
                Vista::Jugadores(v)
            }
        }
    }

    /// Quita un jugador de los que la conocen.
    pub fn sin(&self, j: IdJugador, dueno: IdJugador) -> Vista {
        match self {
            Vista::Dueno if j == dueno => Vista::Nadie,
            Vista::Jugadores(v) => {
                let v: Vec<IdJugador> = v.iter().copied().filter(|x| *x != j).collect();
                if v.is_empty() {
                    Vista::Nadie
                } else {
                    Vista::Jugadores(v)
                }
            }
            otra => otra.clone(),
        }
    }
}

/// Un punto del campo de un jugador, de 0 a `POS_MAX` en cada eje.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Pos {
    pub x: u16,
    pub y: u16,
}

impl Pos {
    pub fn nueva(x: u16, y: u16) -> Self {
        Pos {
            x: x.min(POS_MAX),
            y: y.min(POS_MAX),
        }
    }
}

/// Una etiqueta de texto sobre una carta (≤ 20 caracteres, 7 colores).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Etiqueta {
    pub texto: String,
    pub color: u8,
}

/// Las filas del diálogo de fuerza/resistencia. La base sale de
/// `InfoCarta`; los +1/+1, de `contadores`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Pt {
    pub otros: (i32, i32),
    /// Se borra al pasar el turno.
    pub hasta_fin_de_turno: (i32, i32),
    /// Tapa todo lo demás.
    pub fijo: Option<(i32, i32)>,
}

/// Un «Place marker»: una bolita de color en un punto del campo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Bolita {
    pub id: u32,
    pub campo_de: IdJugador,
    pub pos: Pos,
    /// Uno de 16 colores.
    pub color: u8,
    pub tamano: Tamano,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Tamano {
    S,
    M,
    L,
}

/// La vida del rival imaginado del solitario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VidaFantasma {
    pub vida: i32,
    pub dano_comandante: BTreeMap<IdCarta, i32>,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jugador {
    pub id: IdJugador,
    pub nombre: String,
    /// Color CSS (`#e11d48`).
    pub color: String,
    pub equipo: Option<IdEquipo>,
    pub vida: i32,
    pub veneno: i32,
    /// Los «trackers»: energía, experiencia, tormenta, maná…
    pub contadores: BTreeMap<String, i32>,
    pub dano_comandante: BTreeMap<IdCarta, i32>,
    /// Veces lanzado cada comandante (el impuesto es 2 × eso).
    pub lanzamientos: BTreeMap<IdCarta, u32>,
    /// Comandantes «sin coste extra».
    pub sin_coste_extra: BTreeSet<IdCarta>,
    pub tapete: Option<RefImagen>,
    pub mulligans: u8,
    pub eliminado: Option<Eliminacion>,
    pub mano_revelada: bool,
    pub de_arriba_revelada: bool,
    pub senal: Option<Senal>,
    /// Las fichas de la sección de fichas del deck.
    pub fichas: Vec<OrigenCarta>,
    /// El reverso de su deck.
    pub dorso: RefImagen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Eliminacion {
    pub ronda: u32,
    pub por: Option<IdJugador>,
}

/// Las señales de prioridad.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Senal {
    SinRespuesta,
    Espera,
    Pensando,
    Ausente,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Equipo {
    pub id: IdEquipo,
    pub nombre: String,
    pub color: String,
    pub vida_compartida: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Turno {
    /// 0 mientras dura el mulligan.
    pub numero: u32,
    pub ronda: u32,
    pub activo: IdJugador,
    /// El que empezó (cuando le vuelve a tocar, sube la ronda).
    pub primero: IdJugador,
    /// Hora del host al empezar el turno.
    #[ts(type = "number")]
    pub desde_ms: u64,
    /// Turnos extra pendientes, en orden.
    pub extra: Vec<IdJugador>,
    /// Tiradas del dado planar en este turno (la siguiente cuesta eso).
    pub tiradas_planar: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Designaciones {
    pub monarca: Option<IdJugador>,
    pub iniciativa: Option<IdJugador>,
    pub bendicion: Vec<IdJugador>,
    pub dia_noche: Option<DiaNoche>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DiaNoche {
    Dia,
    Noche,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Flecha {
    pub de: Extremo,
    pub a: Extremo,
    pub tipo: TipoFlecha,
    /// Del color del que la puso.
    pub de_jugador: IdJugador,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "id", rename_all = "camelCase")]
#[ts(export)]
pub enum Extremo {
    Carta(IdCarta),
    Jugador(IdJugador),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TipoFlecha {
    Ataque,
    Objetivo,
}

/// Quién se ha quedado ya con su mano.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoMulligan {
    pub decididos: Vec<IdJugador>,
}

/// Cómo acabó la partida.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Resultado {
    /// Varios = empate o un equipo.
    pub ganadores: Vec<IdJugador>,
    pub como: ComoGano,
    pub nota: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ComoGano {
    Combate,
    Combo,
    Comandante,
    Veneno,
    Moler,
    Alternativa,
    NoCombate,
    Concesion,
    Empate,
}

/// `zonas` como lista de pares: un `RefZona` no puede ser clave de un
/// objeto JSON.
mod zonas_como_lista {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::RefZona;
    use crate::ids::IdCarta;

    pub fn serialize<S: Serializer>(
        m: &BTreeMap<RefZona, Vec<IdCarta>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        let v: Vec<(&RefZona, &Vec<IdCarta>)> = m.iter().collect();
        v.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<RefZona, Vec<IdCarta>>, D::Error> {
        let v: Vec<(RefZona, Vec<IdCarta>)> = Vec::deserialize(d)?;
        Ok(v.into_iter().collect())
    }
}
