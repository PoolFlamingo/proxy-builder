//! **El sparring**: el rival fantasma del solitario.
//!
//! Aquí solo están sus datos (lo que se guarda en la partida, se deshace y
//! se repite como todo lo demás). Cuándo y cómo reacciona es la Fase D.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::IdCarta;

/// Cuánto molesta el fantasma. De fábrica, apagado.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Intensidad {
    #[default]
    Apagado,
    Suave,
    Realista,
    Despiadado,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoSparring {
    pub intensidad: Intensidad,
    pub reacciones: Vec<ReaccionApuntada>,
}

/// Una reacción de esta partida y lo que decidió el jugador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReaccionApuntada {
    pub ronda: u32,
    pub reaccion: ReaccionSparring,
    pub decision: Option<Decision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TipoReaccion {
    Eliminar,
    Contrarrestar,
    Barrer,
    Devolver,
    Sacrificio,
}

/// Lo que hace el fantasma: con qué carta y a qué.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReaccionSparring {
    /// El `seq` del evento que la trajo.
    #[ts(type = "number")]
    pub id: u64,
    pub tipo: TipoReaccion,
    pub objetivo: Vec<IdCarta>,
    pub carta: CartaFantasma,
    /// Lo eliminado va al exilio y no al cementerio («Farewell»).
    pub exilia: bool,
    pub por_que: Explicacion,
    pub base: Vec<Explicacion>,
}

/// La carta con la que reacciona el fantasma.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartaFantasma {
    pub nombre: String,
    pub scryfall_id: String,
    pub coste: String,
}

/// Un porqué como clave + parámetros: el texto lo pone la pantalla.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Explicacion {
    pub clave: String,
    pub params: BTreeMap<String, String>,
}

/// Qué decide el jugador ante una reacción.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Decision {
    /// «Resolverla»: se aplica.
    Aceptar,
    /// «Tengo respuesta»: se queda sin efecto.
    TeniaRespuesta,
    /// «No aplica»: se quita.
    NoAplica,
}
