//! **Importar un deck desde una URL de Archidekt.** Su API pública devuelve
//! cada carta con el id exacto de Scryfall (`card.uid`), así que la impresión
//! no se adivina: se conserva la elegida en Archidekt.

use std::collections::HashMap;

use proxy_core::decklist::{self, LineKind, ParsedDecklist, ParsedLine, Section};
use serde::Deserialize;

use crate::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Deserialize)]
struct Mazo {
    #[serde(default)]
    categories: Vec<Categoria>,
    #[serde(default)]
    cards: Vec<Entrada>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Categoria {
    name: String,
    #[serde(default = "verdadero")]
    included_in_deck: bool,
}

fn verdadero() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Entrada {
    quantity: u32,
    #[serde(default)]
    categories: Option<Vec<String>>,
    card: Carta,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Carta {
    uid: String,
    #[serde(default)]
    collector_number: String,
    edition: Edicion,
    oracle_card: Oraculo,
}

#[derive(Debug, Deserialize)]
struct Edicion {
    editioncode: String,
}

#[derive(Debug, Deserialize)]
struct Oraculo {
    name: String,
}

/// El id numérico de una URL de Archidekt (`https://archidekt.com/decks/123/nombre`).
pub fn id_de_url(url: &str) -> Option<u64> {
    let u = url.trim();
    let resto = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .unwrap_or(u);
    let resto = resto.strip_prefix("www.").unwrap_or(resto);
    let resto = resto.strip_prefix("archidekt.com/")?;
    let mut partes = resto.split('/');
    match (partes.next(), partes.next()) {
        (Some("decks") | Some("api"), Some(n)) if n != "decks" => n.parse().ok(),
        (Some("api"), Some("decks")) => partes.next()?.parse().ok(),
        _ => None,
    }
}

/// Una lista importada: el texto para el cuadro y las líneas con su id de Scryfall.
#[derive(Debug, Clone, PartialEq)]
pub struct Importada {
    pub texto: String,
    pub lista: ParsedDecklist,
    /// Para cada línea (índice en `lista.lines`), el id exacto de Scryfall.
    pub ids: HashMap<u32, String>,
}

/// Convierte la respuesta de Archidekt en lista. Sin `con_sideboard`, la
/// sideboard queda fuera; lo que Archidekt marca fuera del deck (maybeboard)
/// no entra nunca.
pub fn desde_json(json: &str, con_sideboard: bool) -> AppResult<Importada> {
    let m: Mazo = serde_json::from_str(json)
        .map_err(|e| AppError::new(ErrorCode::InvalidInput).with("detalle", e))?;
    let incluidas: HashMap<&str, bool> = m
        .categories
        .iter()
        .map(|c| (c.name.as_str(), c.included_in_deck))
        .collect();
    let mut texto = String::new();
    let mut ids = HashMap::new();
    let mut lineas = Vec::new();
    for e in &m.cards {
        let cats = e.categories.clone().unwrap_or_default();
        let es_side = cats.iter().any(|c| c.eq_ignore_ascii_case("sideboard"));
        let es_cmdr = cats.iter().any(|c| c.eq_ignore_ascii_case("commander"));
        let dentro = cats.is_empty()
            || cats
                .iter()
                .any(|c| *incluidas.get(c.as_str()).unwrap_or(&true));
        if !dentro || (es_side && !con_sideboard) || e.quantity == 0 {
            continue;
        }
        let linea = format!(
            "{} {} ({}) {}",
            e.quantity,
            e.card.oracle_card.name,
            e.card.edition.editioncode,
            e.card.collector_number
        );
        texto.push_str(&linea);
        texto.push('\n');
        let line_no = lineas.len() as u32 + 1;
        ids.insert(line_no, e.card.uid.clone());
        lineas.push(ParsedLine {
            line_no,
            raw: linea,
            kind: LineKind::Card,
            qty: e.quantity,
            name: e.card.oracle_card.name.clone(),
            set: Some(e.card.edition.editioncode.to_lowercase()),
            collector: Some(e.card.collector_number.clone()).filter(|s| !s.is_empty()),
            foil: false,
            section: if es_cmdr {
                Section::Commander
            } else if es_side {
                Section::Sideboard
            } else {
                Section::Main
            },
            included: true,
            error: None,
        });
    }
    // Se vuelve a leer el texto para que lo que se ve y lo que se resuelve casen.
    let mut lista = decklist::parse(&texto);
    for (l, original) in lista.lines.iter_mut().zip(&lineas) {
        l.section = original.section;
    }
    Ok(Importada { texto, lista, ids })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_id_sale_de_varias_formas_de_url() {
        assert_eq!(
            id_de_url("https://archidekt.com/decks/123/mi-mazo"),
            Some(123)
        );
        assert_eq!(id_de_url("archidekt.com/decks/45"), Some(45));
        assert_eq!(id_de_url("https://www.archidekt.com/decks/9/x"), Some(9));
        assert_eq!(id_de_url("https://archidekt.com/api/decks/7/"), Some(7));
        assert_eq!(id_de_url("https://moxfield.com/decks/abc"), None);
        assert_eq!(id_de_url("https://archidekt.com/decks/abc"), None);
    }

    #[test]
    fn el_deck_real_de_archidekt_se_convierte_en_lista() {
        let json = include_str!("fixtures/archidekt-1.json");
        let i = desde_json(json, false).unwrap();
        assert!(i.lista.card_count > 0);
        assert_eq!(i.lista.invalid_count, 0);
        let primera = &i.lista.lines[0];
        assert_eq!(primera.name, "Thelon of Havenwood");
        assert_eq!(primera.section, Section::Commander);
        assert!(i.ids.get(&1).is_some_and(|id| id.len() == 36));
    }

    #[test]
    fn json_roto_es_entrada_no_valida() {
        assert_eq!(
            desde_json("{", false).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
}
