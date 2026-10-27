//! Ids pequeños y estables de la mesa.
//!
//! Son números y no uuid: viajan en cada evento y en cada vista, y la huella
//! de la partida (`huella.rs`) depende de que se ordenen siempre igual.

use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use ts_rs::TS;

/// Un asiento de la mesa (0–9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, TS)]
#[ts(export)]
pub struct IdJugador(pub u8);

/// Una carta de la partida. Las fichas y las copias reciben uno nuevo al
/// crearse (`Partida::siguiente_id`); nunca se reutiliza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, TS)]
#[ts(export)]
pub struct IdCarta(pub u32);

/// Un equipo (Fase G).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, TS)]
#[ts(export)]
pub struct IdEquipo(pub u8);

/// Se escriben como número, pero se leen también de un texto con el número:
/// de clave de un mapa (`Partida::cartas`) dentro de un enum etiquetado
/// (`Cambio::Restaurar`, los mensajes de la red), serde guarda el JSON en un
/// búfer y la clave llega como texto.
macro_rules! id_numerico {
    ($tipo:ident, $n:ty) => {
        impl<'de> Deserialize<'de> for $tipo {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                struct V;
                impl Visitor<'_> for V {
                    type Value = $tipo;
                    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str("un número")
                    }
                    fn visit_u64<E: de::Error>(self, v: u64) -> Result<$tipo, E> {
                        <$n>::try_from(v).map($tipo).map_err(E::custom)
                    }
                    fn visit_i64<E: de::Error>(self, v: i64) -> Result<$tipo, E> {
                        <$n>::try_from(v).map($tipo).map_err(E::custom)
                    }
                    fn visit_str<E: de::Error>(self, v: &str) -> Result<$tipo, E> {
                        v.parse::<$n>().map($tipo).map_err(E::custom)
                    }
                }
                d.deserialize_any(V)
            }
        }
    };
}

id_numerico!(IdJugador, u8);
id_numerico!(IdCarta, u32);
id_numerico!(IdEquipo, u8);

impl fmt::Display for IdJugador {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "j{}", self.0)
    }
}

impl fmt::Display for IdCarta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "c{}", self.0)
    }
}

impl fmt::Display for IdEquipo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "e{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "tipo")]
    enum Etiquetado {
        Con {
            cartas: BTreeMap<IdCarta, IdJugador>,
        },
    }

    #[test]
    fn de_clave_de_un_mapa_dentro_de_un_enum_etiquetado() {
        let e = Etiquetado::Con {
            cartas: BTreeMap::from([(IdCarta(7), IdJugador(1))]),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"tipo":"Con","cartas":{"7":1}}"#);
        assert_eq!(serde_json::from_str::<Etiquetado>(&json).unwrap(), e);
        assert!(serde_json::from_str::<IdJugador>("300").is_err());
        assert!(serde_json::from_str::<IdCarta>("-1").is_err());
    }
}
