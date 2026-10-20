//! **Traducción sin IA** (Fase 7, experimental): cuando una carta no tiene
//! impresión española, se propone su texto a partir de frases **idénticas**
//! de otras cartas que sí la tienen. El texto que sale es oficial; lo que no
//! se encuentra se queda en inglés y marcado. Siempre es una sugerencia: nunca
//! se aplica sola.
//!
//! Cómo: el Oracle se parte en frases; cada una se busca en Scryfall
//! (`o:"frase" lang:es`, con `~` para el nombre de la carta); en la carta
//! española encontrada se parten igual su Oracle y su texto impreso, y se toma
//! la frase que ocupa el mismo sitio. La línea de tipo se busca entera.

use std::collections::HashMap;

use proxy_core::diseno::borrador::nombre_corto;
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;

/// Una frase y su traducción, si se encontró.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FraseSugerida {
    pub en: String,
    pub es: Option<String>,
}

/// La propuesta para una cara.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Sugerencia {
    /// La línea de tipo en español, si alguna carta la tiene igual.
    pub tipo: Option<String>,
    /// Las reglas con lo encontrado en español y lo demás en inglés.
    pub reglas: String,
    pub frases: Vec<FraseSugerida>,
    pub traducidas: u32,
    pub total: u32,
}

/// Parte una línea en frases: por el punto que va seguido de espacio,
/// fuera de paréntesis y comillas.
fn frases_de_linea(linea: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut actual = String::new();
    let mut nivel = 0i32;
    let mut comillas = false;
    let cs: Vec<char> = linea.chars().collect();
    for (i, &c) in cs.iter().enumerate() {
        actual.push(c);
        match c {
            '(' => nivel += 1,
            ')' => {
                nivel -= 1;
                // Un recordatorio entero es una frase.
                if nivel == 0 && cs.get(i + 1) == Some(&' ') {
                    out.push(actual.trim().to_string());
                    actual.clear();
                }
            }
            '"' | '“' | '”' | '«' | '»' => comillas = !comillas,
            '.' if nivel == 0 && !comillas && cs.get(i + 1) == Some(&' ') => {
                out.push(actual.trim().to_string());
                actual.clear();
            }
            _ => {}
        }
    }
    if !actual.trim().is_empty() {
        out.push(actual.trim().to_string());
    }
    out
}

/// Las frases de un texto, línea a línea.
pub fn frases(texto: &str) -> Vec<Vec<String>> {
    texto
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(frases_de_linea)
        .collect()
}

/// Cambia el nombre (y el corto de las legendarias) por `~`.
fn con_tilde(texto: &str, nombre: &str) -> String {
    let mut t = texto.to_string();
    if !nombre.is_empty() {
        t = t.replace(nombre, "~");
        if let Some(corto) = nombre_corto(nombre).filter(|c| *c != nombre && c.len() > 2) {
            t = t.replace(corto, "~");
        }
    }
    t
}

fn plano(v: Vec<Vec<String>>) -> Vec<String> {
    v.into_iter().flatten().collect()
}

/// En una carta española encontrada, la frase que ocupa el sitio de `frase`
/// en su Oracle. Sus textos se parten igual; si no cuadran, no vale.
fn alinear(c: &ScryCard, frase: &str) -> Option<String> {
    let oracle = c.oracle_text.as_deref()?;
    let impreso = c.printed_text.as_deref()?;
    let en = plano(frases(&con_tilde(oracle, &c.name)));
    let nombre_es = c.printed_name.clone().unwrap_or_default();
    let es = plano(frases(&con_tilde(impreso, &nombre_es)));
    if en.len() != es.len() {
        return None;
    }
    let i = en.iter().position(|f| f == frase)?;
    Some(es[i].clone())
}

/// Lo que se busca de una frase: sin el punto final y con las comillas
/// escapadas.
fn consulta(frase: &str) -> String {
    let f = frase.trim_end_matches('.').replace('"', "\\\"");
    format!("o:\"{f}\" lang:es order:released")
}

/// Scryfall no entendió la búsqueda (un 4xx): esa frase se queda en inglés.
fn rechazada(e: &crate::error::AppError) -> bool {
    e.code == ErrorCode::Network
        && e.params
            .get("estado")
            .and_then(|x| x.parse::<u16>().ok())
            .is_some_and(|x| (400..500).contains(&x))
}

/// Busca la traducción oficial de una frase en otras cartas.
async fn traducir_frase<A: CardApi>(api: &A, frase: &str) -> AppResult<Option<String>> {
    // Una frase muy corta («Vuela.») está en miles de cartas: igual vale.
    let r = match api.search(&consulta(frase), true, 30).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound || rechazada(&e) => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut candidatas: Vec<&ScryCard> = r
        .iter()
        .filter(|c| c.lang == "es" && c.printed_text.is_some())
        .collect();
    // Las más recientes primero: el texto vigente.
    candidatas.sort_by(|a, b| b.released_at.cmp(&a.released_at));
    Ok(candidatas.into_iter().find_map(|c| alinear(c, frase)))
}

/// La línea de tipo en español de una carta con la misma línea en inglés.
pub async fn traducir_tipo<A: CardApi>(api: &A, tipo: &str) -> AppResult<Option<String>> {
    let q = format!("t:\"{}\" lang:es order:released", tipo.replace('"', "\\\""));
    let r = match api.search(&q, true, 40).await {
        Ok(r) => r,
        Err(e) if e.code == ErrorCode::NotFound || rechazada(&e) => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(r.into_iter()
        .filter(|c| c.lang == "es" && c.type_line.as_deref() == Some(tipo))
        .find_map(|c| c.printed_type_line))
}

/// ¿Una línea de palabras clave sueltas («Vigilance, reach»)?
fn palabras_clave(linea: &str) -> Option<Vec<String>> {
    let t = linea.trim().trim_end_matches('.');
    if t.contains(['(', '{', ':', '—']) || !t.contains(", ") {
        return None;
    }
    let partes: Vec<String> = t.split(", ").map(|p| p.trim().to_string()).collect();
    let cortas = partes
        .iter()
        .all(|p| !p.is_empty() && p.split_whitespace().count() <= 3);
    cortas.then_some(partes)
}

/// Mayúscula la primera letra; las demás palabras clave van en minúscula.
fn capitalizar(s: &str, mayuscula: bool) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(p) if mayuscula => p.to_uppercase().chain(c).collect(),
        Some(p) => p.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// Traduce una frase con lo que ya se buscó.
async fn con_cache<A: CardApi>(
    api: &A,
    hechas: &mut HashMap<String, Option<String>>,
    f: &str,
) -> AppResult<Option<String>> {
    if let Some(x) = hechas.get(f) {
        return Ok(x.clone());
    }
    let mut x = traducir_frase(api, f).await?;
    // Una lista de palabras clave se traduce palabra a palabra.
    if x.is_none() {
        if let Some(ps) = palabras_clave(f) {
            let mut es = Vec::new();
            for (i, p) in ps.iter().enumerate() {
                let sola = capitalizar(p, true);
                match traducir_frase(api, &sola).await? {
                    Some(t) => es.push(capitalizar(t.trim_end_matches('.'), i == 0)),
                    None => {
                        es.clear();
                        break;
                    }
                }
            }
            if !es.is_empty() {
                x = Some(format!("{}.", es.join(", ")));
            }
        }
    }
    hechas.insert(f.to_string(), x.clone());
    Ok(x)
}

/// La sugerencia para una cara: su línea de tipo y sus reglas en inglés.
pub async fn sugerir<A: CardApi>(
    api: &A,
    nombre: &str,
    tipo: &str,
    reglas: &str,
) -> AppResult<Sugerencia> {
    let tipo_es = if tipo.trim().is_empty() {
        None
    } else {
        traducir_tipo(api, tipo).await?
    };
    let mut hechas: HashMap<String, Option<String>> = HashMap::new();
    let mut lineas = Vec::new();
    let mut lista = Vec::new();
    let (mut ok, mut total) = (0u32, 0u32);
    for linea in frases(&con_tilde(reglas, nombre)) {
        let mut partes = Vec::new();
        for f in linea {
            total += 1;
            let es = con_cache(api, &mut hechas, &f).await?;
            if es.is_some() {
                ok += 1;
            }
            // El `~` vuelve a ser el nombre original: `reglas_efectivas` lo
            // cambia por el propio también en las reglas a mano.
            let sale = es.clone().unwrap_or_else(|| f.clone()).replace('~', nombre);
            partes.push(sale);
            lista.push(FraseSugerida {
                en: f.replace('~', nombre),
                es: es.map(|e| e.replace('~', nombre)),
            });
        }
        lineas.push(partes.join(" "));
    }
    Ok(Sugerencia {
        tipo: tipo_es,
        reglas: lineas.join("\n"),
        frases: lista,
        traducidas: ok,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frases_fuera_de_parentesis_y_comillas() {
        let f = frases(
            "Flying. (This creature can't be blocked except by creatures with flying or reach.)\n~ gets +1/+1. Draw a card.\nIt has \"Hexproof. Indestructible.\" until end of turn.",
        );
        assert_eq!(
            f[0],
            [
                "Flying.",
                "(This creature can't be blocked except by creatures with flying or reach.)"
            ]
        );
        assert_eq!(f[1], ["~ gets +1/+1.", "Draw a card."]);
        assert_eq!(f[2].len(), 1, "lo de dentro de las comillas no se parte");
    }

    #[test]
    fn listas_de_palabras_clave() {
        assert_eq!(
            palabras_clave("Vigilance, reach"),
            Some(vec!["Vigilance".to_string(), "reach".to_string()])
        );
        assert_eq!(palabras_clave("Draw a card, then discard a card."), None);
        assert_eq!(capitalizar("Alcance", false), "alcance");
    }

    #[test]
    fn el_nombre_va_como_tilde() {
        assert_eq!(
            con_tilde(
                "When Arcades, the Strategist enters, Arcades draws.",
                "Arcades, the Strategist"
            ),
            "When ~ enters, ~ draws."
        );
    }

    #[test]
    fn alinear_por_posicion() {
        let c = ScryCard {
            name: "Grizzly Bears".into(),
            lang: "es".into(),
            oracle_text: Some("Flying.\nWhen ~ enters, draw a card.".replace('~', "Grizzly Bears")),
            printed_name: Some("Osos grizzly".into()),
            printed_text: Some("Vuela.\nCuando los Osos grizzly entren, roba una carta.".into()),
            ..ScryCard::default()
        };
        assert_eq!(alinear(&c, "Flying.").as_deref(), Some("Vuela."));
        // El nombre de la otra carta no casa con «~» en español si lleva
        // artículo: la frase sale igual, con el nombre a cambiar después.
        assert!(alinear(&c, "When ~ enters, draw a card.").is_some());
    }
}

/// Medida en vivo (red): `cargo test --lib traduccion::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;
    use crate::scryfall::client::Scryfall;

    #[tokio::test]
    #[ignore]
    async fn veinte_cartas_sin_espanol() {
        let api = Scryfall::new().unwrap();
        let cartas = api
            .search(
                "-in:es game:paper -is:funny -set:sld year>=2023 (t:creature or t:instant or t:sorcery or t:enchantment) order:random",
                false,
                20,
            )
            .await
            .unwrap();
        let (mut ok, mut total) = (0, 0);
        for c in cartas.iter().take(20) {
            let s = sugerir(
                &api,
                &c.name,
                c.type_line.as_deref().unwrap_or(""),
                c.oracle_text.as_deref().unwrap_or(""),
            )
            .await
            .unwrap();
            println!(
                "{} — {}/{} — tipo: {:?}\n{}\n",
                c.name, s.traducidas, s.total, s.tipo, s.reglas
            );
            ok += s.traducidas;
            total += s.total;
        }
        println!(
            "TOTAL {ok}/{total} = {:.0} %",
            ok as f64 * 100.0 / total.max(1) as f64
        );
    }
}
