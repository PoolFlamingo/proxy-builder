//! **Listas para jugar.** Un deck puede tener más cartas de las que se juegan
//! (variantes de diseño de una misma carta, alternativas para probar otras
//! sinergias…): una lista elige, de entre ellas, las que van a la partida. En
//! Commander, el comandante (una copia concreta: la variante de diseño que se
//! quiera) y las 99.
//!
//! Aquí solo el dato y su comprobación, puros: el tamaño del formato, las
//! copias de más (salvo las tierras básicas y las cartas que lo permiten) y la
//! identidad de color del comandante. La app junta los datos de cada carta
//! (de Scryfall o del diseño) y guarda las listas.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::formato::Formato;

/// Una lista para jugar de un deck. Los ids son los de las copias del deck
/// (`DeckCard.id`): así se elige qué variante de diseño se juega.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListaDeJuego {
    pub id: String,
    pub nombre: String,
    /// Uno, o dos con compañero o trasfondo. Vacío fuera de Commander.
    pub comandantes: Vec<String>,
    /// Las demás cartas que se juegan (sin los comandantes).
    pub cartas: Vec<String>,
}

/// Lo que hace falta saber de una copia del deck para elegirla y comprobar
/// la lista.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartaDeLista {
    /// El id de la copia en el deck.
    pub id: String,
    /// El nombre de la carta (en inglés: las copias se cuentan por él).
    pub nombre: String,
    pub tipo: Option<String>,
    /// La identidad de color (`W`, `U`, `B`, `R`, `G`).
    pub identidad: Vec<char>,
    /// Criatura legendaria, trasfondo o «can be your commander».
    pub puede_ser_comandante: bool,
    /// Compañero, «Friends forever», «Choose a Background», trasfondo o
    /// «Doctor's companion»: puede ir de comandante con otro.
    pub con_pareja: bool,
    /// Tierra básica o «A deck can have any number of cards named…».
    pub sin_limite: bool,
    /// La huella de la miniatura de la copia (la pone la app; para pintarla).
    pub imagen: Option<String>,
    /// Si la copia está marcada en el deck (`variante`, `opcional`, `banquillo`).
    pub marca: Option<String>,
}

/// Lo que puede estar mal en una lista. La pantalla lo escribe con `t()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum AvisoLista {
    /// No son las cartas que pide el formato (`hasta`: `None` = sin máximo).
    Tamano {
        hay: u32,
        desde: u32,
        hasta: Option<u32>,
    },
    SinComandante,
    DemasiadosComandantes,
    NoPuedeSerComandante {
        nombre: String,
    },
    /// Dos comandantes y alguno sin compañero ni trasfondo.
    SinPareja,
    /// Más copias de las que deja el formato.
    Repetida {
        nombre: String,
        veces: u32,
    },
    /// Fuera de la identidad de color del comandante.
    FueraDeIdentidad {
        nombre: String,
    },
    /// Copias de la lista que ya no están en el deck (se ignoran).
    YaNoEstan {
        n: u32,
    },
}

impl AvisoLista {
    /// Si impide empezar la partida. La identidad de color y la pareja de
    /// comandantes solo avisan: los datos pueden faltar (una carta propia
    /// sin vincular) y en casa se juega como se quiera.
    pub fn impide(&self) -> bool {
        matches!(
            self,
            AvisoLista::Tamano { .. }
                | AvisoLista::SinComandante
                | AvisoLista::DemasiadosComandantes
                | AvisoLista::Repetida { .. }
        )
    }
}

/// Las reglas de construcción de un formato que se comprueban.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Construccion {
    pub desde: u32,
    pub hasta: Option<u32>,
    /// Copias de una misma carta (`None` = las que se quiera).
    pub copias: Option<u32>,
    pub comandante: bool,
}

impl Construccion {
    pub fn de(formato: Formato) -> Option<Construccion> {
        match formato {
            Formato::Commander => Some(Construccion {
                desde: 100,
                hasta: Some(100),
                copias: Some(1),
                comandante: true,
            }),
            Formato::Constructed => Some(Construccion {
                desde: 60,
                hasta: None,
                copias: Some(4),
                comandante: false,
            }),
            Formato::Limited => Some(Construccion {
                desde: 40,
                hasta: None,
                copias: None,
                comandante: false,
            }),
            Formato::Libre => None,
        }
    }

    /// Si un deck con `n` cartas que cuentan necesita una lista: tiene más
    /// de las que caben.
    pub fn pide_lista(&self, n: u32) -> bool {
        self.hasta.is_some_and(|h| n > h)
    }
}

/// Comprueba una lista contra las cartas del deck y el formato.
pub fn comprobar(
    cartas: &[CartaDeLista],
    lista: &ListaDeJuego,
    formato: Formato,
) -> Vec<AvisoLista> {
    let mut avisos = Vec::new();
    let por_id: BTreeMap<&str, &CartaDeLista> = cartas.iter().map(|c| (c.id.as_str(), c)).collect();
    let mut faltan = 0u32;
    let mut coger = |ids: &[String]| -> Vec<&CartaDeLista> {
        let mut vistas = BTreeSet::new();
        ids.iter()
            .filter(|id| vistas.insert(id.as_str()))
            .filter_map(|id| {
                let c = por_id.get(id.as_str()).copied();
                if c.is_none() {
                    faltan += 1;
                }
                c
            })
            .collect()
    };
    let comandantes = coger(&lista.comandantes);
    let resto = coger(&lista.cartas);
    if faltan > 0 {
        avisos.push(AvisoLista::YaNoEstan { n: faltan });
    }
    let Some(reglas) = Construccion::de(formato) else {
        return avisos;
    };

    // El tamaño.
    let hay = (comandantes.len() + resto.len()) as u32;
    if hay < reglas.desde || reglas.hasta.is_some_and(|h| hay > h) {
        avisos.push(AvisoLista::Tamano {
            hay,
            desde: reglas.desde,
            hasta: reglas.hasta,
        });
    }

    // Los comandantes.
    if reglas.comandante {
        match comandantes.len() {
            0 => avisos.push(AvisoLista::SinComandante),
            1 | 2 => {}
            _ => avisos.push(AvisoLista::DemasiadosComandantes),
        }
        for c in &comandantes {
            if !c.puede_ser_comandante {
                avisos.push(AvisoLista::NoPuedeSerComandante {
                    nombre: c.nombre.clone(),
                });
            }
        }
        if comandantes.len() == 2 && comandantes.iter().any(|c| !c.con_pareja) {
            avisos.push(AvisoLista::SinPareja);
        }
    }

    // Las copias de más, por nombre (las variantes de diseño son la misma carta).
    if let Some(max) = reglas.copias {
        let mut veces: BTreeMap<&str, u32> = BTreeMap::new();
        for c in comandantes.iter().chain(resto.iter()) {
            if !c.sin_limite {
                *veces.entry(c.nombre.as_str()).or_default() += 1;
            }
        }
        for (nombre, n) in veces {
            if n > max {
                avisos.push(AvisoLista::Repetida {
                    nombre: nombre.to_string(),
                    veces: n,
                });
            }
        }
    }

    // La identidad de color.
    if reglas.comandante && !comandantes.is_empty() {
        let identidad: BTreeSet<char> = comandantes
            .iter()
            .flat_map(|c| c.identidad.iter().copied())
            .collect();
        let mut fuera = BTreeSet::new();
        for c in &resto {
            if c.identidad.iter().any(|x| !identidad.contains(x)) {
                fuera.insert(c.nombre.clone());
            }
        }
        avisos.extend(
            fuera
                .into_iter()
                .map(|nombre| AvisoLista::FueraDeIdentidad { nombre }),
        );
    }
    avisos
}

/// Si el texto de reglas permite cualquier número de copias.
pub fn sin_limite_por_texto(oracle: &str) -> bool {
    let t = oracle.to_lowercase();
    t.contains("a deck can have any number of cards named") || t.contains("a deck can have up to")
}

/// Si una carta puede ir de comandante, y si puede ir con otro.
pub fn comandante_por_texto(tipo: &str, oracle: &str, keywords: &[String]) -> (bool, bool) {
    let tipo = tipo.to_lowercase();
    let delante = tipo.split(" // ").next().unwrap_or(&tipo);
    let t = oracle.to_lowercase();
    let trasfondo = delante.contains("background");
    let puede = (delante.contains("legendary") && delante.contains("creature"))
        || trasfondo
        || t.contains("can be your commander");
    let pareja = trasfondo
        || keywords.iter().any(|k| {
            let k = k.to_lowercase();
            k.starts_with("partner") || k == "friends forever" || k == "doctor's companion"
        })
        || t.contains("partner")
        || t.contains("choose a background")
        || t.contains("friends forever")
        || t.contains("doctor's companion");
    (puede, pareja)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carta(id: &str, nombre: &str, identidad: &str) -> CartaDeLista {
        CartaDeLista {
            id: id.into(),
            nombre: nombre.into(),
            tipo: Some("Creature".into()),
            identidad: identidad.chars().collect(),
            puede_ser_comandante: false,
            con_pareja: false,
            sin_limite: false,
            imagen: None,
            marca: None,
        }
    }

    /// Un deck de 110: un comandante en dos variantes de diseño, 105 cartas
    /// distintas y 3 bosques.
    fn deck() -> Vec<CartaDeLista> {
        let mut v = Vec::new();
        for (id, _) in [("cmd-a", 0), ("cmd-b", 1)] {
            let mut c = carta(id, "Kruphix", "UG");
            c.puede_ser_comandante = true;
            v.push(c);
        }
        for i in 0..105 {
            v.push(carta(&format!("c{i}"), &format!("Carta {i}"), "G"));
        }
        for i in 0..3 {
            let mut c = carta(&format!("b{i}"), "Forest", "");
            c.sin_limite = true;
            v.push(c);
        }
        v
    }

    fn lista(comandantes: &[&str], cartas: Vec<String>) -> ListaDeJuego {
        ListaDeJuego {
            id: "l".into(),
            nombre: "Prueba".into(),
            comandantes: comandantes.iter().map(|s| s.to_string()).collect(),
            cartas,
        }
    }

    #[test]
    fn una_variante_de_comandante_y_99_vale() {
        let mut cartas: Vec<String> = (0..96).map(|i| format!("c{i}")).collect();
        cartas.extend((0..3).map(|i| format!("b{i}")));
        let avisos = comprobar(&deck(), &lista(&["cmd-b"], cartas), Formato::Commander);
        assert!(avisos.is_empty(), "{avisos:?}");
    }

    #[test]
    fn de_mas_sin_comandante_y_repetidas() {
        let cartas: Vec<String> = (0..105).map(|i| format!("c{i}")).collect();
        let avisos = comprobar(&deck(), &lista(&[], cartas), Formato::Commander);
        assert!(avisos.contains(&AvisoLista::SinComandante));
        assert!(avisos.contains(&AvisoLista::Tamano {
            hay: 105,
            desde: 100,
            hasta: Some(100)
        }));
        // Las dos variantes del comandante son la misma carta.
        let mut cartas: Vec<String> = (0..98).map(|i| format!("c{i}")).collect();
        cartas.push("cmd-a".into());
        let avisos = comprobar(&deck(), &lista(&["cmd-b"], cartas), Formato::Commander);
        assert_eq!(
            avisos,
            vec![AvisoLista::Repetida {
                nombre: "Kruphix".into(),
                veces: 2
            }]
        );
        assert!(avisos.iter().all(AvisoLista::impide));
    }

    #[test]
    fn la_identidad_solo_avisa() {
        let mut d = deck();
        d.push(carta("roja", "Shock", "R"));
        let mut cartas: Vec<String> = (0..98).map(|i| format!("c{i}")).collect();
        cartas.push("roja".into());
        let avisos = comprobar(&d, &lista(&["cmd-a"], cartas), Formato::Commander);
        assert_eq!(
            avisos,
            vec![AvisoLista::FueraDeIdentidad {
                nombre: "Shock".into()
            }]
        );
        assert!(!avisos[0].impide());
    }

    #[test]
    fn lo_que_ya_no_esta_se_cuenta_aparte() {
        let mut cartas: Vec<String> = (0..99).map(|i| format!("c{i}")).collect();
        cartas.push("borrada".into());
        let avisos = comprobar(&deck(), &lista(&["cmd-a"], cartas), Formato::Commander);
        assert_eq!(avisos, vec![AvisoLista::YaNoEstan { n: 1 }]);
    }

    #[test]
    fn en_60_cartas_hasta_cuatro_copias() {
        let mut d = deck();
        for i in 0..5 {
            d.push(carta(&format!("bolt{i}"), "Lightning Bolt", "R"));
        }
        let mut cartas: Vec<String> = (0..55).map(|i| format!("c{i}")).collect();
        cartas.extend((0..5).map(|i| format!("bolt{i}")));
        let avisos = comprobar(&d, &lista(&[], cartas), Formato::Constructed);
        assert_eq!(
            avisos,
            vec![AvisoLista::Repetida {
                nombre: "Lightning Bolt".into(),
                veces: 5
            }]
        );
    }

    #[test]
    fn quien_puede_ser_comandante() {
        assert_eq!(
            comandante_por_texto("Legendary Creature — God", "", &[]),
            (true, false)
        );
        assert_eq!(
            comandante_por_texto("Legendary Creature — Human", "Partner", &["Partner".into()]),
            (true, true)
        );
        assert_eq!(
            comandante_por_texto("Legendary Enchantment — Background", "", &[]),
            (true, true)
        );
        assert_eq!(
            comandante_por_texto(
                "Legendary Planeswalker — Teferi",
                "Teferi can be your commander.",
                &[]
            ),
            (true, false)
        );
        assert_eq!(
            comandante_por_texto("Creature — Elf", "", &[]),
            (false, false)
        );
        assert!(sin_limite_por_texto(
            "A deck can have any number of cards named Relentless Rats."
        ));
        assert!(!Construccion::de(Formato::Commander).is_some_and(|c| c.pide_lista(100)));
        assert!(Construccion::de(Formato::Commander).is_some_and(|c| c.pide_lista(110)));
    }
}
