//! **La lista del deck en texto**, para llevarla a otra web o al cliente:
//! los formatos de «Export» de Moxfield (para Moxfield, Arena, MTGO y texto
//! plano).
//!
//! Sale de las cartas del deck (lo que se imprime), juntando las copias de la
//! misma impresión. Las secciones (comandante, compañero, banquillo) no las
//! guarda cada carta: se sacan de la lista que escribió el usuario, por el
//! nombre. Las fichas y los huecos en blanco no van.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{parse, LineKind, Section};
use crate::model::{CardSource, Deck, DeckCard};

/// A qué se exporta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FormatoLista {
    /// `1 Sol Ring (CMM) 410`; el banquillo tras `SIDEBOARD:`.
    Moxfield,
    /// Con las cabeceras de Arena (`Commander`, `Deck`, `Sideboard`…).
    Arena,
    /// `1 Sol Ring`; el banquillo (y el comandante) tras una línea en blanco.
    Mtgo,
    /// `1 Sol Ring`, todo seguido salvo el banquillo.
    Texto,
}

/// Una línea de la lista: copias de la misma impresión en la misma sección.
#[derive(Debug, Clone, PartialEq)]
struct Linea {
    seccion: Section,
    nombre: String,
    /// La cara de delante, para las de dos caras (Arena y MTGO la piden así).
    delante: String,
    set: Option<String>,
    numero: Option<String>,
    copias: u32,
}

/// La lista del deck en ese formato (con salto de línea al final).
pub fn exportar(deck: &Deck, formato: FormatoLista) -> String {
    let lineas = juntar(deck);
    let de = |s: &[Section]| -> Vec<&Linea> {
        lineas.iter().filter(|l| s.contains(&l.seccion)).collect()
    };
    let arriba = de(&[Section::Commander, Section::Companion]);
    let mazo = de(&[Section::Main]);
    let banquillo = de(&[Section::Sideboard]);
    let mut out = String::new();
    let mut bloque = |cabecera: Option<&str>, ls: &[&Linea], f: &dyn Fn(&Linea) -> String| {
        if ls.is_empty() {
            return;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        if let Some(c) = cabecera {
            out.push_str(c);
            out.push('\n');
        }
        for l in ls {
            out.push_str(&f(l));
            out.push('\n');
        }
    };
    match formato {
        FormatoLista::Moxfield => {
            let mut principal = arriba.clone();
            principal.extend(&mazo);
            bloque(None, &principal, &|l| con_impresion(l, &l.nombre));
            bloque(Some("SIDEBOARD:"), &banquillo, &|l| {
                con_impresion(l, &l.nombre)
            });
        }
        FormatoLista::Arena => {
            let comandante = de(&[Section::Commander]);
            let companero = de(&[Section::Companion]);
            bloque(Some("Commander"), &comandante, &|l| {
                con_impresion(l, &l.delante)
            });
            bloque(Some("Companion"), &companero, &|l| {
                con_impresion(l, &l.delante)
            });
            bloque(Some("Deck"), &mazo, &|l| con_impresion(l, &l.delante));
            bloque(Some("Sideboard"), &banquillo, &|l| {
                con_impresion(l, &l.delante)
            });
        }
        FormatoLista::Mtgo => {
            // En MTGO el comandante va en el banquillo.
            let mut fuera = arriba.clone();
            fuera.extend(&banquillo);
            bloque(None, &mazo, &|l| format!("{} {}", l.copias, l.delante));
            bloque(None, &fuera, &|l| format!("{} {}", l.copias, l.delante));
        }
        FormatoLista::Texto => {
            let mut principal = arriba.clone();
            principal.extend(&mazo);
            bloque(None, &principal, &|l| format!("{} {}", l.copias, l.nombre));
            bloque(None, &banquillo, &|l| format!("{} {}", l.copias, l.nombre));
        }
    }
    out
}

fn con_impresion(l: &Linea, nombre: &str) -> String {
    match (&l.set, &l.numero) {
        (Some(s), Some(n)) => format!("{} {nombre} ({}) {n}", l.copias, s.to_uppercase()),
        (Some(s), None) => format!("{} {nombre} ({})", l.copias, s.to_uppercase()),
        _ => format!("{} {nombre}", l.copias),
    }
}

/// Las cartas del deck en líneas, en el orden en que aparecen.
fn juntar(deck: &Deck) -> Vec<Linea> {
    let secciones = secciones_de(&deck.decklist_text);
    let mut out: Vec<Linea> = Vec::new();
    for c in &deck.cards {
        if !va_en_la_lista(c) {
            continue;
        }
        let nombre = c.meta.name.trim();
        if nombre.is_empty() {
            continue;
        }
        let seccion = seccion_de(&secciones, nombre);
        if matches!(seccion, Section::Tokens | Section::Maybeboard) {
            continue;
        }
        let set = c.meta.set.clone().filter(|s| !s.is_empty());
        let numero = c.meta.collector_number.clone().filter(|n| !n.is_empty());
        match out.iter_mut().find(|l| {
            l.seccion == seccion && l.nombre == nombre && l.set == set && l.numero == numero
        }) {
            Some(l) => l.copias += 1,
            None => out.push(Linea {
                seccion,
                nombre: nombre.to_string(),
                delante: if c.meta.is_dfc {
                    nombre.split(" // ").next().unwrap_or(nombre).to_string()
                } else {
                    nombre.to_string()
                },
                set,
                numero,
                copias: 1,
            }),
        }
    }
    out
}

/// Las fichas y los huecos no son cartas del mazo.
fn va_en_la_lista(c: &DeckCard) -> bool {
    match &c.source {
        CardSource::Token { .. } | CardSource::Blank => false,
        // Una propia vinculada a una ficha de otra fuente (Cockatrice) es una
        // ficha; las demás propias, por su tipo si lo tienen.
        CardSource::Custom { externa, .. } => {
            externa.is_none() && !es_ficha(c.meta.type_line.as_deref())
        }
        CardSource::Scryfall { .. } => !es_ficha(c.meta.type_line.as_deref()),
    }
}

fn es_ficha(tipo: Option<&str>) -> bool {
    tipo.is_some_and(|t| {
        let t = t.to_lowercase();
        t.starts_with("token") || t.contains(" token") || t.starts_with("emblem")
    })
}

/// La sección de cada nombre según la lista escrita (la primera vez que
/// sale). Con las dos caras, también por la de delante.
fn secciones_de(texto: &str) -> Vec<(String, Section)> {
    let mut v: Vec<(String, Section)> = Vec::new();
    for l in parse(texto).lines {
        if l.kind != LineKind::Card {
            continue;
        }
        let n = l.name.to_lowercase();
        if !v.iter().any(|(k, _)| *k == n) {
            v.push((n, l.section));
        }
    }
    v
}

fn seccion_de(secciones: &[(String, Section)], nombre: &str) -> Section {
    let n = nombre.to_lowercase();
    let delante = n.split(" // ").next().unwrap_or(&n).to_string();
    secciones
        .iter()
        .find(|(k, _)| *k == n || *k == delante || k.split(" // ").next() == Some(&delante))
        .map_or(Section::Main, |(_, s)| *s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{BackSpec, CardEdits, CardMeta, PrintSettings};

    fn carta(nombre: &str, set: &str, num: &str) -> DeckCard {
        DeckCard {
            id: format!("{nombre}-{set}-{num}"),
            source: CardSource::Scryfall {
                scryfall_id: "x".into(),
            },
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: nombre.into(),
                set: Some(set.into()),
                collector_number: Some(num.into()),
                type_line: Some("Creature".into()),
                ..CardMeta::default()
            },
            design: None,
        }
    }

    fn deck() -> Deck {
        let mut ficha = carta("Treasure", "tmh2", "21");
        ficha.source = CardSource::Token {
            scryfall_id: "t".into(),
        };
        let mut delver = carta("Delver of Secrets // Insectile Aberration", "isd", "51");
        delver.meta.is_dfc = true;
        let mut propia = carta("Kelp", "", "");
        propia.source = CardSource::Custom {
            scryfall_id: None,
            externa: Some(crate::model::RefExterna {
                fuente: crate::model::FuenteCarta::Cockatrice,
                id: "kelp".into(),
            }),
        };
        Deck {
            id: "d".into(),
            name: "Yusri".into(),
            decklist_text: "Commander\n1 Yusri, Fortune's Flame\n\nDeck\n2 Sol Ring\n1 Delver of Secrets\n\nSideboard\n1 Pyroblast\n".into(),
            cards: vec![
                carta("Yusri, Fortune's Flame", "mh2", "218"),
                carta("Sol Ring", "cmm", "410"),
                carta("Sol Ring", "cmm", "410"),
                delver,
                carta("Pyroblast", "ice", "213"),
                ficha,
                propia,
                DeckCard {
                    source: CardSource::Blank,
                    ..carta("", "", "")
                },
            ],
            settings: PrintSettings::default(),
            default_back: None,
            schema_version: 1,
        }
    }

    #[test]
    fn para_moxfield() {
        assert_eq!(
            exportar(&deck(), FormatoLista::Moxfield),
            "1 Yusri, Fortune's Flame (MH2) 218\n2 Sol Ring (CMM) 410\n1 Delver of Secrets // Insectile Aberration (ISD) 51\n\nSIDEBOARD:\n1 Pyroblast (ICE) 213\n"
        );
    }

    #[test]
    fn para_arena() {
        assert_eq!(
            exportar(&deck(), FormatoLista::Arena),
            "Commander\n1 Yusri, Fortune's Flame (MH2) 218\n\nDeck\n2 Sol Ring (CMM) 410\n1 Delver of Secrets (ISD) 51\n\nSideboard\n1 Pyroblast (ICE) 213\n"
        );
    }

    #[test]
    fn para_mtgo_el_comandante_va_en_el_banquillo() {
        assert_eq!(
            exportar(&deck(), FormatoLista::Mtgo),
            "2 Sol Ring\n1 Delver of Secrets\n\n1 Yusri, Fortune's Flame\n1 Pyroblast\n"
        );
    }

    #[test]
    fn en_texto_plano() {
        assert_eq!(
            exportar(&deck(), FormatoLista::Texto),
            "1 Yusri, Fortune's Flame\n2 Sol Ring\n1 Delver of Secrets // Insectile Aberration\n\n1 Pyroblast\n"
        );
    }

    #[test]
    fn una_carta_que_no_esta_en_la_lista_va_al_mazo() {
        let mut d = deck();
        d.decklist_text.clear();
        let t = exportar(&d, FormatoLista::Arena);
        assert!(
            t.starts_with("Deck\n1 Yusri, Fortune's Flame (MH2) 218\n"),
            "{t}"
        );
        assert!(!t.contains("Sideboard"));
    }

    #[test]
    fn impresiones_distintas_van_en_lineas_distintas() {
        let mut d = deck();
        d.cards[2].meta.collector_number = Some("411".into());
        let t = exportar(&d, FormatoLista::Moxfield);
        assert!(
            t.contains("1 Sol Ring (CMM) 410\n1 Sol Ring (CMM) 411\n"),
            "{t}"
        );
    }
}
