//! Formatos, variantes y opciones de una partida, con sus valores de fábrica.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::IdJugador;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Formato {
    Commander,
    /// 60 cartas.
    Constructed,
    Limited,
    Libre,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Variantes {
    pub equipos: ModoEquipos,
    pub archenemy: Option<IdJugador>,
    pub planechase: bool,
}

impl Variantes {
    /// «A todos los oponentes» cuenta a los compañeros.
    pub fn fuego_amigo(&self) -> bool {
        match self.equipos {
            ModoEquipos::Ninguno => true,
            ModoEquipos::Equipos { fuego_amigo } => fuego_amigo,
            ModoEquipos::DosCabezas => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum ModoEquipos {
    #[default]
    Ninguno,
    #[serde(rename_all = "camelCase")]
    Equipos { fuego_amigo: bool },
    /// Two-Headed Giant: equipos de dos con la vida compartida.
    DosCabezas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModoMulligan {
    Londres,
    LondresPrimeroGratis,
    /// Quedarse sin poner nada al fondo, sin límite de mulligans.
    Amistoso,
    /// Al fondo las que uno quiera.
    Libre,
}

/// Las ayudas que hace la mesa sola (se pueden apagar: no son reglas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Ayudas {
    pub robar_al_empezar: bool,
    pub enderezar_al_empezar: bool,
    pub impuesto_automatico: bool,
    pub contadores_al_entrar: bool,
    pub avisar_vida_cero: bool,
}

impl Default for Ayudas {
    fn default() -> Self {
        Ayudas {
            robar_al_empezar: true,
            enderezar_al_empezar: true,
            impuesto_automatico: true,
            contadores_al_entrar: true,
            avisar_vida_cero: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Opciones {
    pub vida_inicial: i32,
    pub mano_inicial: u8,
    pub mulligan: ModoMulligan,
    /// El primero en jugar no roba en su primer turno.
    pub primero_no_roba: bool,
    pub ayudas: Ayudas,
    pub limite_veneno: i32,
    pub limite_comandante: i32,
    /// Cualquiera mueve las cartas de cualquiera (como en una mesa de
    /// verdad). De fábrica, cada uno lo suyo (Q-A1).
    pub todos_tocan_todo: bool,
}

impl Opciones {
    /// Los valores de fábrica de un formato para `jugadores` jugadores.
    pub fn de_fabrica(formato: Formato, variantes: &Variantes, jugadores: usize) -> Self {
        let vida_inicial = match (formato, variantes.equipos) {
            (_, ModoEquipos::DosCabezas) => 30,
            (Formato::Commander, _) => 40,
            _ => 20,
        };
        Opciones {
            vida_inicial,
            mano_inicial: 7,
            mulligan: if formato == Formato::Commander {
                ModoMulligan::LondresPrimeroGratis
            } else {
                ModoMulligan::Londres
            },
            // Como Playgroup: sí en duelos y solitario, no con 3 o más.
            primero_no_roba: jugadores <= 2,
            ayudas: Ayudas::default(),
            limite_veneno: 10,
            limite_comandante: 21,
            todos_tocan_todo: false,
        }
    }
}

/// La vida del archienemigo (Archenemy).
pub const VIDA_ARCHIENEMIGO: i32 = 40;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valores_de_fabrica() {
        let v = Variantes::default();
        let c = Opciones::de_fabrica(Formato::Commander, &v, 4);
        assert_eq!(c.vida_inicial, 40);
        assert_eq!(c.mulligan, ModoMulligan::LondresPrimeroGratis);
        assert!(!c.primero_no_roba);
        let d = Opciones::de_fabrica(Formato::Constructed, &v, 2);
        assert_eq!(d.vida_inicial, 20);
        assert!(d.primero_no_roba);
        assert_eq!(d.mano_inicial, 7);
        let s = Opciones::de_fabrica(Formato::Limited, &v, 1);
        assert!(s.primero_no_roba);
        let dos = Variantes {
            equipos: ModoEquipos::DosCabezas,
            ..Default::default()
        };
        assert_eq!(
            Opciones::de_fabrica(Formato::Constructed, &dos, 4).vida_inicial,
            30
        );
        assert!(!dos.fuego_amigo());
        assert!(Variantes::default().fuego_amigo());
    }
}
