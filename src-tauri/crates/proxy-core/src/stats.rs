//! **Estadísticas del deck**: curva de maná, colores, tipos, rareza, precio
//! estimado y legalidad por formato. La app le pasa una entrada por copia con
//! los datos de Scryfall que tenga en caché; aquí solo se cuenta.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Una copia del deck con lo que hace falta para contarla.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatInput {
    pub name: String,
    pub cmc: Option<f64>,
    pub colors: Vec<String>,
    pub color_identity: Vec<String>,
    pub type_line: Option<String>,
    pub rarity: Option<String>,
    /// Precio en euros (texto de Scryfall, `"1.23"`).
    pub price_eur: Option<String>,
    pub legalities: BTreeMap<String, String>,
    pub is_token: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Contador {
    pub key: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Legalidad {
    pub format: String,
    pub legal: bool,
    /// Las cartas que lo impiden (sin repetir).
    pub offenders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckStats {
    /// Cartas contadas (sin tierras en la curva; con tokens fuera de todo).
    pub cards: u32,
    pub lands: u32,
    pub tokens: u32,
    /// Copias sin datos de Scryfall (propias, huecos o sin caché).
    pub unknown: u32,
    /// Curva de 0 a 7+ (índice 7 = 7 o más), sin tierras.
    pub curve: Vec<u32>,
    pub average_cmc: f64,
    /// W, U, B, R, G y C (incoloras), por copia.
    pub colors: Vec<Contador>,
    pub color_identity: Vec<String>,
    pub types: Vec<Contador>,
    pub rarity: Vec<Contador>,
    pub total_eur: f64,
    /// Copias sin precio en euros.
    pub without_price: u32,
    pub legality: Vec<Legalidad>,
}

/// Formatos que se enseñan, en este orden.
pub const FORMATOS: [&str; 9] = [
    "standard",
    "pioneer",
    "modern",
    "legacy",
    "vintage",
    "commander",
    "pauper",
    "historic",
    "brawl",
];

const TIPOS: [&str; 8] = [
    "Creature",
    "Planeswalker",
    "Battle",
    "Instant",
    "Sorcery",
    "Artifact",
    "Enchantment",
    "Land",
];

/// El tipo principal de una línea de tipo (`Legendary Artifact Creature — …` → `Creature`).
fn tipo_principal(linea: &str) -> String {
    let antes = linea.split('—').next().unwrap_or(linea);
    // De la cara frontal en las DFC (`A // B`).
    let frente = antes.split("//").next().unwrap_or(antes);
    TIPOS
        .iter()
        .find(|t| frente.split_whitespace().any(|w| w == **t))
        .map(|t| (*t).to_string())
        .unwrap_or_else(|| "Other".into())
}

fn es_tierra(c: &StatInput) -> bool {
    c.type_line
        .as_deref()
        .map(|l| tipo_principal(l) == "Land")
        .unwrap_or(false)
}

pub fn stats(entrada: &[StatInput]) -> DeckStats {
    let mut curva = vec![0u32; 8];
    let mut colores: BTreeMap<&str, u32> = BTreeMap::new();
    let mut identidad: Vec<String> = Vec::new();
    let mut tipos: BTreeMap<String, u32> = BTreeMap::new();
    let mut rareza: BTreeMap<String, u32> = BTreeMap::new();
    let (mut cartas, mut tierras, mut tokens, mut desconocidas, mut sin_precio) = (0, 0, 0, 0, 0);
    let (mut suma_cmc, mut con_cmc) = (0.0, 0u32);
    let mut total = 0.0;
    let mut legalidad: BTreeMap<&str, Vec<String>> =
        FORMATOS.iter().map(|f| (*f, Vec::new())).collect();

    for c in entrada {
        if c.is_token {
            tokens += 1;
            continue;
        }
        if c.type_line.is_none() && c.cmc.is_none() {
            desconocidas += 1;
            continue;
        }
        cartas += 1;
        if let Some(l) = &c.type_line {
            *tipos.entry(tipo_principal(l)).or_default() += 1;
        }
        if let Some(r) = &c.rarity {
            *rareza.entry(r.clone()).or_default() += 1;
        }
        if es_tierra(c) {
            tierras += 1;
        } else if let Some(cmc) = c.cmc {
            let i = (cmc.max(0.0).floor() as usize).min(7);
            curva[i] += 1;
            suma_cmc += cmc;
            con_cmc += 1;
        }
        if c.colors.is_empty() {
            if !es_tierra(c) {
                *colores.entry("C").or_default() += 1;
            }
        } else {
            for col in &c.colors {
                if let Some(k) = ["W", "U", "B", "R", "G"].iter().find(|k| **k == col) {
                    *colores.entry(k).or_default() += 1;
                }
            }
        }
        for col in &c.color_identity {
            if !identidad.contains(col) {
                identidad.push(col.clone());
            }
        }
        match c.price_eur.as_deref().and_then(|p| p.parse::<f64>().ok()) {
            Some(p) => total += p,
            None => sin_precio += 1,
        }
        for (formato, malas) in legalidad.iter_mut() {
            let estado = c.legalities.get(*formato).map(String::as_str);
            if !matches!(estado, Some("legal" | "restricted")) && !malas.contains(&c.name) {
                malas.push(c.name.clone());
            }
        }
    }

    let orden = ["W", "U", "B", "R", "G"];
    identidad.sort_by_key(|c| orden.iter().position(|o| o == c).unwrap_or(9));
    let contador = |m: BTreeMap<String, u32>| -> Vec<Contador> {
        let mut v: Vec<Contador> = m
            .into_iter()
            .map(|(key, count)| Contador { key, count })
            .collect();
        v.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
        v
    };
    DeckStats {
        cards: cartas,
        lands: tierras,
        tokens,
        unknown: desconocidas,
        curve: curva,
        average_cmc: if con_cmc > 0 {
            ((suma_cmc / f64::from(con_cmc)) * 100.0).round() / 100.0
        } else {
            0.0
        },
        colors: ["W", "U", "B", "R", "G", "C"]
            .iter()
            .map(|k| Contador {
                key: (*k).to_string(),
                count: colores.get(k).copied().unwrap_or(0),
            })
            .collect(),
        color_identity: identidad,
        types: contador(tipos),
        rarity: contador(rareza),
        total_eur: (total * 100.0).round() / 100.0,
        without_price: sin_precio,
        legality: FORMATOS
            .iter()
            .map(|f| {
                let malas = legalidad.remove(f).unwrap_or_default();
                Legalidad {
                    format: (*f).to_string(),
                    legal: cartas > 0 && malas.is_empty(),
                    offenders: malas,
                }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carta(nombre: &str, cmc: f64, tipo: &str, colores: &[&str], eur: Option<&str>) -> StatInput {
        StatInput {
            name: nombre.into(),
            cmc: Some(cmc),
            colors: colores.iter().map(|c| (*c).to_string()).collect(),
            color_identity: colores.iter().map(|c| (*c).to_string()).collect(),
            type_line: Some(tipo.into()),
            rarity: Some("common".into()),
            price_eur: eur.map(Into::into),
            legalities: FORMATOS
                .iter()
                .map(|f| ((*f).to_string(), "legal".to_string()))
                .collect(),
            is_token: false,
        }
    }

    #[test]
    fn cuenta_curva_colores_tipos_y_precio() {
        let mut caro = carta("Sol Ring", 1.0, "Artifact", &[], Some("2.50"));
        caro.legalities.insert("modern".into(), "not_legal".into());
        let entrada = vec![
            carta("Lightning Bolt", 1.0, "Instant", &["R"], Some("0.80")),
            carta("Lightning Bolt", 1.0, "Instant", &["R"], Some("0.80")),
            carta("Island", 0.0, "Basic Land — Island", &[], None),
            carta("Grave Titan", 6.0, "Creature — Giant", &["B"], Some("3")),
            carta(
                "Emrakul",
                15.0,
                "Legendary Creature — Eldrazi",
                &[],
                Some("10"),
            ),
            caro,
            StatInput {
                is_token: true,
                ..StatInput::default()
            },
            StatInput::default(),
        ];
        let s = stats(&entrada);
        assert_eq!(s.cards, 6);
        assert_eq!(s.lands, 1);
        assert_eq!(s.tokens, 1);
        assert_eq!(s.unknown, 1);
        assert_eq!(s.curve, vec![0, 3, 0, 0, 0, 0, 1, 1]);
        assert_eq!(s.total_eur, 17.1);
        assert_eq!(s.without_price, 1);
        assert_eq!(s.color_identity, vec!["B", "R"]);
        let r = s.colors.iter().find(|c| c.key == "R").unwrap();
        assert_eq!(r.count, 2);
        let c = s.colors.iter().find(|c| c.key == "C").unwrap();
        assert_eq!(c.count, 2, "Emrakul y Sol Ring; la tierra no cuenta");
        assert_eq!(s.types[0].key, "Creature");
        let modern = s.legality.iter().find(|l| l.format == "modern").unwrap();
        assert!(!modern.legal);
        assert_eq!(modern.offenders, vec!["Sol Ring"]);
        assert!(
            s.legality
                .iter()
                .find(|l| l.format == "legacy")
                .unwrap()
                .legal
        );
    }

    #[test]
    fn el_tipo_principal() {
        assert_eq!(
            tipo_principal("Legendary Artifact Creature — Golem"),
            "Creature"
        );
        assert_eq!(tipo_principal("Artifact — Equipment"), "Artifact");
        assert_eq!(tipo_principal("Instant // Sorcery"), "Instant");
        assert_eq!(tipo_principal("Tribal Kindred"), "Other");
    }

    #[test]
    fn un_deck_vacio_no_es_legal_en_nada() {
        let s = stats(&[]);
        assert!(s.legality.iter().all(|l| !l.legal));
        assert_eq!(s.average_cmc, 0.0);
    }
}
