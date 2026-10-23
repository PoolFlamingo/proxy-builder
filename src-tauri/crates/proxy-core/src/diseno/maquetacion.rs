//! **Qué maquetación lleva cada carta** (Fase 5 del plan de la 2.0).
//!
//! Scryfall da un `layout` por carta, pero no basta: Secuela, Fusionar y
//! Habitación vienen como `split`, las batallas como `transform`, y los
//! planeswalkers, mazmorras y cartas con Estación como `normal`. Aquí se pasa
//! de `(layout, keywords, tipo, texto)` a una [`Maquetacion`] de la carta y a
//! un [`Cuerpo`] por cara (lo que va en su caja de reglas).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::modelo::{CardData, FaceData};

/// Cómo se compone la carta entera.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Maquetacion {
    Normal,
    Ficha,
    Planeswalker,
    Saga,
    Clase,
    Caso,
    Partida,
    Fusionar,
    Secuela,
    Habitacion,
    Aventura,
    /// Preparar: como la aventura, con el hechizo a la derecha.
    Preparar,
    Flip,
    Transformable,
    Mdfc,
    Batalla,
    Meld,
    Reversible,
    FichaDosCaras,
    Leveler,
    Prototipo,
    Mutar,
    Estacion,
    Emblema,
    Mazmorra,
    Plano,
    Plan,
    Vanguard,
}

/// Lo que va en la caja de reglas de una cara.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cuerpo {
    /// Reglas y sabor, sin más.
    Texto,
    /// Habilidades de lealtad con su escudo.
    Lealtad,
    /// Capítulos en vertical, con el arte a la derecha.
    Capitulos,
    /// Niveles de una Clase.
    Niveles,
    /// Resolver / Resuelto de un Caso.
    Caso,
    /// Franjas de nivel con su fuerza/resistencia.
    Leveler,
    /// Umbrales de Estación.
    Estacion,
    /// El recuadro del Prototipo.
    Prototipo,
    /// Salas de una mazmorra.
    Mazmorra,
}

/// Cuántas caras impresas tiene (las de doble cara van con dorso propio).
impl Maquetacion {
    pub fn caras_impresas(self) -> usize {
        match self {
            Maquetacion::Transformable
            | Maquetacion::Mdfc
            | Maquetacion::Batalla
            | Maquetacion::Meld
            | Maquetacion::Reversible
            | Maquetacion::FichaDosCaras => 2,
            _ => 1,
        }
    }

    /// Carta de tamaño grande (planos, planes y Vanguard impresos grandes).
    pub fn es_grande(self) -> bool {
        matches!(
            self,
            Maquetacion::Plano | Maquetacion::Plan | Maquetacion::Vanguard
        )
    }

    /// Se compone de dos mitades en una sola cara.
    pub fn es_partida(self) -> bool {
        matches!(
            self,
            Maquetacion::Partida | Maquetacion::Fusionar | Maquetacion::Habitacion
        )
    }
}

fn tipo_en(f: Option<&FaceData>) -> &str {
    f.map_or("", |f| f.type_line.en.as_str())
}

fn tiene_palabra(tipo: &str, palabra: &str) -> bool {
    tipo.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .any(|p| p == palabra)
}

fn con_estacion(c: &CardData) -> bool {
    c.keywords.iter().any(|k| k == "Station")
        || c.faces.iter().any(|f| {
            f.rules
                .en
                .lines()
                .any(|l| l.starts_with("Station (") || l.starts_with("Station."))
        })
}

/// Las caras impresas de una carta concreta: las de su maquetación, pero no
/// más de las que tiene (la carta fundida de un meld, diseñada sola, es una).
pub fn caras_impresas(c: &CardData) -> usize {
    maquetacion(c).0.caras_impresas().min(c.faces.len()).max(1)
}

/// La maquetación de una carta. El `bool` dice si el layout no se conoce
/// (cae a `Normal` y se avisa).
pub fn maquetacion(c: &CardData) -> (Maquetacion, bool) {
    let primera = c.faces.first();
    let tipo = tipo_en(primera);
    let kw = |k: &str| c.keywords.iter().any(|x| x.eq_ignore_ascii_case(k));
    let m = match c.layout.as_str() {
        "split" => {
            if kw("Fuse") {
                Maquetacion::Fusionar
            } else if kw("Aftermath") {
                Maquetacion::Secuela
            } else if c
                .faces
                .iter()
                .any(|f| tiene_palabra(&f.type_line.en, "Room"))
            {
                Maquetacion::Habitacion
            } else {
                Maquetacion::Partida
            }
        }
        "flip" => Maquetacion::Flip,
        "adventure" => Maquetacion::Aventura,
        "prepare" => Maquetacion::Preparar,
        "transform" | "battle" if tiene_palabra(tipo, "Battle") => Maquetacion::Batalla,
        "transform" => Maquetacion::Transformable,
        "modal_dfc" => Maquetacion::Mdfc,
        "meld" => Maquetacion::Meld,
        "reversible_card" => Maquetacion::Reversible,
        "double_faced_token" if tiene_palabra(tipo, "Dungeon") => Maquetacion::Mazmorra,
        "double_faced_token" => Maquetacion::FichaDosCaras,
        "leveler" => Maquetacion::Leveler,
        "class" => Maquetacion::Clase,
        "case" => Maquetacion::Caso,
        "saga" => Maquetacion::Saga,
        "prototype" => Maquetacion::Prototipo,
        "mutate" => Maquetacion::Mutar,
        "emblem" => Maquetacion::Emblema,
        "planar" => Maquetacion::Plano,
        "scheme" => Maquetacion::Plan,
        "vanguard" => Maquetacion::Vanguard,
        "token" => Maquetacion::Ficha,
        "normal" => {
            if tiene_palabra(tipo, "Dungeon") {
                Maquetacion::Mazmorra
            } else if tiene_palabra(tipo, "Planeswalker") {
                Maquetacion::Planeswalker
            } else if con_estacion(c) {
                Maquetacion::Estacion
            } else if c.es_ficha() {
                Maquetacion::Ficha
            } else {
                Maquetacion::Normal
            }
        }
        _ => return (Maquetacion::Normal, true),
    };
    (m, false)
}

/// El cuerpo de una cara: sale del tipo de esa cara (una transformable puede
/// tener una saga delante y un planeswalker detrás) y, si no, de la carta.
pub fn cuerpo(c: &CardData, cara: usize) -> Cuerpo {
    let f = c.faces.get(cara);
    let tipo = tipo_en(f);
    if tiene_palabra(tipo, "Planeswalker") {
        return Cuerpo::Lealtad;
    }
    if tiene_palabra(tipo, "Saga") {
        return Cuerpo::Capitulos;
    }
    if tiene_palabra(tipo, "Class") {
        return Cuerpo::Niveles;
    }
    if tiene_palabra(tipo, "Case") {
        return Cuerpo::Caso;
    }
    if tiene_palabra(tipo, "Dungeon") {
        return Cuerpo::Mazmorra;
    }
    match c.layout.as_str() {
        "leveler" => Cuerpo::Leveler,
        "prototype" => Cuerpo::Prototipo,
        _ if cara == 0 && con_estacion(c) => Cuerpo::Estacion,
        _ => Cuerpo::Texto,
    }
}

/// Una parte del texto estructurado, para enseñarla en el editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Parte {
    /// La marca de la izquierda (`+1`, `I, II`, `2-6`, `8+`…).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub marca: Option<String>,
    /// La cabecera (`{1}{U}: Nivel 2`, `Resolver`…).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub cabecera: Option<String>,
    pub texto: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fr: Option<String>,
}

/// Cómo se va a maquetar una cara, para el editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Resumen {
    pub maquetacion: Maquetacion,
    pub caras_impresas: u32,
    /// El texto partido en habilidades, capítulos, niveles… (vacío si es
    /// texto normal o no sigue el patrón).
    pub partes: Vec<Parte>,
}

/// El resumen de una cara de un diseño.
pub fn resumen(
    d: &super::modelo::CardDesign,
    cara: usize,
    lang: super::modelo::DesignLang,
) -> Resumen {
    use super::estructura as es;
    let (m, _) = maquetacion(&d.card);
    let reglas = super::borrador::reglas_efectivas(d, cara, lang);
    let tramos = |t: Vec<es::Tramo>| -> Vec<Parte> {
        if t.len() < 2 {
            return Vec::new();
        }
        t.into_iter()
            .map(|x| Parte {
                marca: None,
                cabecera: x.cabecera,
                texto: x.texto.join("\n"),
                fr: x.fr,
            })
            .collect()
    };
    let partes = match cuerpo(&d.card, cara) {
        Cuerpo::Lealtad => {
            let h = es::planeswalker(&reglas);
            if h.iter().all(|x| x.coste.is_none()) {
                Vec::new()
            } else {
                h.into_iter()
                    .map(|x| Parte {
                        marca: x.coste,
                        cabecera: None,
                        texto: x.texto,
                        fr: None,
                    })
                    .collect()
            }
        }
        Cuerpo::Capitulos => es::saga(&reglas)
            .capitulos
            .into_iter()
            .map(|c| Parte {
                marca: Some(
                    c.numeros
                        .iter()
                        .map(|n| es::a_romano(*n))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                cabecera: None,
                texto: c.texto,
                fr: None,
            })
            .collect(),
        Cuerpo::Niveles => tramos(es::clase(&reglas)),
        Cuerpo::Caso => tramos(es::caso(&reglas)),
        Cuerpo::Leveler => tramos(es::leveler(&reglas))
            .into_iter()
            .map(|p| Parte {
                marca: p.cabecera,
                cabecera: None,
                ..p
            })
            .collect(),
        Cuerpo::Estacion => tramos(es::estacion(&reglas))
            .into_iter()
            .map(|p| Parte {
                marca: p.cabecera,
                cabecera: None,
                ..p
            })
            .collect(),
        Cuerpo::Prototipo => es::prototipo(&reglas)
            .map(|p| {
                vec![Parte {
                    marca: Some(p.coste),
                    cabecera: None,
                    texto: p.recordatorio.unwrap_or_default(),
                    fr: Some(p.fr),
                }]
            })
            .unwrap_or_default(),
        Cuerpo::Mazmorra => es::mazmorra(&reglas)
            .into_iter()
            .map(|s| Parte {
                marca: None,
                cabecera: Some(s.nombre),
                texto: s.texto,
                fr: None,
            })
            .collect(),
        Cuerpo::Texto => Vec::new(),
    };
    Resumen {
        maquetacion: m,
        caras_impresas: caras_impresas(&d.card) as u32,
        partes,
    }
}

/// Las medidas de la carta impresa en mm (ancho × alto, en vertical).
pub fn medida_grande() -> (f64, f64) {
    // Planos, planes y Vanguard: 3,5 × 5 pulgadas (88,9 × 127 mm).
    (88.9, 127.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::modelo::ByLang;

    fn cara(tipo: &str, reglas: &str) -> FaceData {
        FaceData {
            name: "X".into(),
            type_line: ByLang {
                es: tipo.into(),
                en: tipo.into(),
                otros: Default::default(),
            },
            rules: ByLang {
                es: reglas.into(),
                en: reglas.into(),
                otros: Default::default(),
            },
            ..FaceData::default()
        }
    }

    fn carta(layout: &str, kw: &[&str], caras: Vec<FaceData>) -> CardData {
        CardData {
            layout: layout.into(),
            keywords: kw.iter().map(|s| s.to_string()).collect(),
            faces: caras,
            ..CardData::default()
        }
    }

    /// La tabla de §2 del sub-plan de la Fase 5, fila a fila.
    #[test]
    fn la_tabla_de_layouts() {
        use Maquetacion as M;
        let casos: Vec<(CardData, M)> = vec![
            (
                carta(
                    "normal",
                    &[],
                    vec![cara("Legendary Planeswalker — Liliana", "+1: X")],
                ),
                M::Planeswalker,
            ),
            (
                carta(
                    "saga",
                    &[],
                    vec![cara("Enchantment Land — Urza's Saga", "")],
                ),
                M::Saga,
            ),
            (
                carta("class", &[], vec![cara("Enchantment — Class", "")]),
                M::Clase,
            ),
            (
                carta("case", &["Solved"], vec![cara("Enchantment — Case", "")]),
                M::Caso,
            ),
            (
                carta("split", &[], vec![cara("Instant", ""), cara("Instant", "")]),
                M::Partida,
            ),
            (
                carta(
                    "split",
                    &["Fuse"],
                    vec![cara("Instant", ""), cara("Instant", "")],
                ),
                M::Fusionar,
            ),
            (
                carta(
                    "split",
                    &["Aftermath"],
                    vec![cara("Sorcery", ""), cara("Sorcery", "")],
                ),
                M::Secuela,
            ),
            (
                carta(
                    "split",
                    &[],
                    vec![
                        cara("Enchantment — Room", ""),
                        cara("Enchantment — Room", ""),
                    ],
                ),
                M::Habitacion,
            ),
            (
                carta(
                    "adventure",
                    &[],
                    vec![
                        cara("Creature — Giant", ""),
                        cara("Instant — Adventure", ""),
                    ],
                ),
                M::Aventura,
            ),
            (
                carta(
                    "prepare",
                    &[],
                    vec![cara("Creature — Human Wizard", ""), cara("Instant", "")],
                ),
                M::Preparar,
            ),
            (
                carta(
                    "flip",
                    &[],
                    vec![cara("Creature", ""), cara("Legendary Creature", "")],
                ),
                M::Flip,
            ),
            (
                carta(
                    "transform",
                    &[],
                    vec![cara("Creature — Human Wizard", ""), cara("Creature", "")],
                ),
                M::Transformable,
            ),
            (
                carta(
                    "modal_dfc",
                    &[],
                    vec![cara("Instant", ""), cara("Land", "")],
                ),
                M::Mdfc,
            ),
            (
                carta(
                    "transform",
                    &[],
                    vec![cara("Battle — Siege", ""), cara("Creature", "")],
                ),
                M::Batalla,
            ),
            (
                carta("meld", &[], vec![cara("Legendary Creature — Angel", "")]),
                M::Meld,
            ),
            (
                carta(
                    "reversible_card",
                    &[],
                    vec![cara("Creature", ""), cara("Creature", "")],
                ),
                M::Reversible,
            ),
            (
                carta(
                    "double_faced_token",
                    &[],
                    vec![cara("Token Creature — Angel", ""), cara("Token", "")],
                ),
                M::FichaDosCaras,
            ),
            (
                carta("leveler", &["Level Up"], vec![cara("Creature", "")]),
                M::Leveler,
            ),
            (
                carta(
                    "prototype",
                    &["Prototype"],
                    vec![cara("Artifact Creature", "")],
                ),
                M::Prototipo,
            ),
            (
                carta("mutate", &["Mutate"], vec![cara("Creature — Beast", "")]),
                M::Mutar,
            ),
            (
                carta(
                    "normal",
                    &[],
                    vec![cara(
                        "Artifact — Spacecraft",
                        "Station (Tap another creature…)\n8+ | Flying",
                    )],
                ),
                M::Estacion,
            ),
            (
                carta("emblem", &[], vec![cara("Emblem — Ajani", "")]),
                M::Emblema,
            ),
            (carta("normal", &[], vec![cara("Dungeon", "")]), M::Mazmorra),
            (
                carta("planar", &[], vec![cara("Plane — Zendikar", "")]),
                M::Plano,
            ),
            (carta("scheme", &[], vec![cara("Scheme", "")]), M::Plan),
            (
                carta("vanguard", &[], vec![cara("Vanguard", "")]),
                M::Vanguard,
            ),
            (
                carta("token", &[], vec![cara("Token Creature — Goblin", "")]),
                M::Ficha,
            ),
            (
                carta("normal", &[], vec![cara("Creature — Bear", "")]),
                M::Normal,
            ),
        ];
        for (c, esperada) in casos {
            assert_eq!(maquetacion(&c), (esperada, false), "{}", c.layout);
        }
    }

    #[test]
    fn un_layout_inventado_cae_a_normal_y_avisa() {
        for l in ["augment", "host", "inventado_2031"] {
            let c = carta(l, &[], vec![cara("Creature", "")]);
            assert_eq!(maquetacion(&c), (Maquetacion::Normal, true));
        }
    }

    #[test]
    fn cada_cara_tiene_su_cuerpo() {
        let c = carta(
            "transform",
            &[],
            vec![
                cara("Enchantment — Saga", "I — X"),
                cara("Legendary Planeswalker — Ajani", "+1: X"),
            ],
        );
        assert_eq!(cuerpo(&c, 0), Cuerpo::Capitulos);
        assert_eq!(cuerpo(&c, 1), Cuerpo::Lealtad);
        assert_eq!(Maquetacion::Transformable.caras_impresas(), 2);
        assert_eq!(Maquetacion::Partida.caras_impresas(), 1);
    }
}
