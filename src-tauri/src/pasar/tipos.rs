//! **Lo que la pantalla ve de pasar decks** y lo que viaja en la oferta.
//! Todo con `ts-rs` (`pnpm bindings`): la pantalla solo pinta estos estados,
//! nunca decide nada del intercambio.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppError;

/// Un deck de la oferta (lo que quien envía cuenta de él, y lo que quien
/// recibe enseña). Entrada de fuera: se valida y se limpia al recibirla.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckOfrecido {
    pub nombre: String,
    pub cartas: u32,
    /// Lleva imágenes propias, diseños o reversos propios.
    pub proxies: bool,
    /// Lo que pesa el paquete tal como se enviaría (bytes).
    #[ts(type = "number")]
    pub tamano: u64,
    /// Lo que pesaría solo con las cartas originales (bytes).
    #[ts(type = "number")]
    pub tamano_originales: u64,
    pub linaje: Option<String>,
    /// La hora (ms) de esta versión del deck, en el equipo donde se cambió.
    #[ts(type = "number | null")]
    pub actualizado: Option<i64>,
    /// La portada: un JPEG pequeño en base64 (sin prefijo `data:`).
    pub portada: Option<String>,
}

/// Cuál de las dos copias es más reciente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MasReciente {
    /// El deck de aquí.
    Aqui,
    /// El que llega.
    Llega,
    /// La misma versión.
    Igual,
    /// No se sabe (el que llega no dice cuándo se cambió).
    Desconocido,
}

/// El deck de aquí que ya es el mismo que el que llega (mismo linaje).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ConflictoPasar")]
pub struct Conflicto {
    pub deck_id: String,
    pub nombre: String,
    /// La hora (ms) de la versión de aquí.
    #[ts(type = "number")]
    pub actualizado: i64,
    pub mas_reciente: MasReciente,
}

/// Un deck de la oferta con lo que ya hay aquí del mismo linaje.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckRevisado {
    pub deck: DeckOfrecido,
    pub conflicto: Option<Conflicto>,
}

/// Qué quiere quien recibe de cada deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "EleccionPasar")]
pub enum Eleccion {
    No,
    ConProxies,
    SoloOriginales,
}

/// Qué hacer con un deck que ya existe aquí.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ResolucionPasar")]
pub enum Resolucion {
    /// «Reemplazar el de aquí».
    Reemplazar,
    /// «Guardar los dos».
    GuardarLosDos,
}

/// La decisión de quien recibe sobre un deck de la oferta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "DecisionPasar")]
pub struct Decision {
    pub indice: u32,
    pub eleccion: Eleccion,
    /// Obligatoria si el deck tiene `conflicto` y no es `No`.
    #[serde(default)]
    pub resolucion: Option<Resolucion>,
}

/// El otro equipo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "RemotoPasar")]
pub struct Remoto {
    pub apodo: String,
    /// Los primeros caracteres de su clave, para distinguirlo.
    pub huella: String,
}

/// Cuánto va un deck.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ProgresoPasar")]
pub struct Progreso {
    pub indice: u32,
    pub nombre: String,
    #[ts(type = "number")]
    pub hechos: u64,
    #[ts(type = "number")]
    pub total: u64,
    /// Decks ya terminados y cuántos son en total.
    pub decks_hechos: u32,
    pub decks_total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EstadoResultado {
    Hecho,
    /// Quien recibe dijo que no a este deck.
    Rechazado,
    Fallo,
}

/// Qué pasó con cada deck.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "ResultadoPasar")]
pub struct Resultado {
    pub nombre: String,
    /// El deck recién guardado (quien recibe).
    pub deck_id: Option<String>,
    pub estado: EstadoResultado,
    pub error: Option<AppError>,
    /// Cartas a las que aún les falta bajar la imagen (solo originales).
    pub faltan_imagenes: u32,
    /// Se reemplazó uno que ya estaba.
    pub reemplazado: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FaseReceptor {
    /// Enseñando el QR y el código, esperando a que llegue alguien.
    Esperando,
    /// Alguien de la red local quiere enviar y hay que aceptarlo (por el
    /// código de 6 cifras).
    Peticion,
    /// Ya hay alguien conectado; falta que llegue la oferta.
    Conectado,
    /// Llegó la oferta: elegir qué se quiere.
    Oferta,
    Recibiendo,
    Terminado,
    Cancelado,
    /// Caducó el código sin que nadie llegara.
    Caducado,
    Error,
}

/// Todo lo que la pantalla de «Recibir» necesita saber.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoReceptor {
    pub fase: FaseReceptor,
    /// El código en texto (`PBD1-…`) y su QR (un SVG).
    pub codigo: String,
    pub qr: String,
    /// Cuándo caduca el código (ms desde 1970).
    #[ts(type = "number")]
    pub caduca: i64,
    /// Se anuncia en la red local (los que están cerca lo ven).
    pub cerca: bool,
    pub remoto: Option<Remoto>,
    /// El código de 6 cifras de este encuentro (los dos lo ven igual).
    pub codigo6: Option<String>,
    pub oferta: Vec<DeckRevisado>,
    pub progreso: Option<Progreso>,
    pub resultados: Vec<Resultado>,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FaseEmisor {
    Conectando,
    /// Esperando a que el otro equipo acepte (por el código de 6 cifras).
    EsperandoAceptacion,
    /// Esperando a que el otro elija qué quiere.
    EsperandoRespuesta,
    Enviando,
    Terminado,
    Cancelado,
    Error,
}

/// Lo que la pantalla de «Enviar» sabe del envío en marcha.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoEmisor {
    pub fase: FaseEmisor,
    pub remoto: Option<Remoto>,
    pub codigo6: Option<String>,
    pub decks: Vec<DeckOfrecido>,
    pub progreso: Option<Progreso>,
    pub resultados: Vec<Resultado>,
    pub error: Option<AppError>,
}

/// Un equipo de la red local que está esperando un deck.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "EquipoCerca")]
pub struct Cerca {
    /// Su clave completa.
    pub id: String,
    pub apodo: String,
    pub huella: String,
}

/// A quién se envía.
#[derive(Debug, Clone, PartialEq, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export, rename = "DestinoPasar")]
pub enum Destino {
    /// Uno de la lista de equipos cerca.
    Cerca { id: String },
    /// El código en texto o leído de un QR.
    Codigo { texto: String },
}

/// Un paquete leído de un fichero, antes de importarlo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaqueteLeido {
    pub deck: DeckOfrecido,
    pub conflicto: Option<Conflicto>,
}

/// Lo que se hizo al importar un paquete.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaqueteImportado {
    pub deck_id: String,
    pub nombre: String,
    pub faltan_imagenes: u32,
    pub reemplazado: bool,
}

/// Lo que pesa el paquete de un deck y si lleva proxies (para elegir al
/// compartirlo).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct InfoPaquete {
    pub proxies: bool,
    #[ts(type = "number")]
    pub tamano: u64,
    #[ts(type = "number")]
    pub tamano_originales: u64,
}

/// El avance de bajar las imágenes que faltan tras recibir «solo originales».
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImagenesProgreso {
    pub job_id: u32,
    pub hechas: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ImagenesFin {
    pub job_id: u32,
    /// El deck con sus imágenes ya puestas.
    pub bajadas: u32,
    /// Las que no se pudieron bajar.
    pub fallidas: u32,
    pub cancelado: bool,
    pub error: Option<AppError>,
}
