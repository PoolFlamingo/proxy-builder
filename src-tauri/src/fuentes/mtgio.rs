//! **magicthegathering.io**: los datos de Gatherer, con el nombre impreso de
//! cada carta en otros idiomas (`foreignNames`). Solo para **nombres**: de un
//! nombre en español (u otro idioma) saca el inglés, que luego se pide a
//! Scryfall. No tiene fichas y sus imágenes son de baja resolución, así que no
//! se usa para nada más. Es lento y a veces no responde: se usa como último
//! recurso y, si falla, se sigue sin él.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Deserialize;

use super::Externo;
use crate::commands::identify::normalizar;
use crate::error::AppResult;

const API: &str = "https://api.magicthegathering.io/v1/cards";
/// Tope de una respuesta.
const MAX_RESPUESTA: usize = 2 * 1024 * 1024;

/// El idioma como lo llama la API, del código de Scryfall.
pub fn idioma_api(codigo: &str) -> Option<&'static str> {
    Some(match codigo {
        "es" => "spanish",
        "fr" => "french",
        "de" => "german",
        "it" => "italian",
        "pt" => "portuguese (brazil)",
        "ja" => "japanese",
        "ko" => "korean",
        "ru" => "russian",
        "zhs" => "chinese simplified",
        "zht" => "chinese traditional",
        _ => return None,
    })
}

#[derive(Debug, Deserialize)]
struct Respuesta {
    #[serde(default)]
    cards: Vec<Carta>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Carta {
    name: String,
    #[serde(default)]
    foreign_names: Vec<Extranjero>,
}

#[derive(Debug, Deserialize)]
struct Extranjero {
    name: String,
}

/// Los nombres en inglés de la respuesta: primero los de las cartas cuyo
/// nombre en ese idioma es exactamente el buscado.
fn nombres_de(json: &[u8], buscado: &str) -> Vec<String> {
    let Ok(r) = serde_json::from_slice::<Respuesta>(json) else {
        return Vec::new();
    };
    let b = normalizar(buscado);
    let mut exactas: Vec<String> = Vec::new();
    let mut resto: Vec<String> = Vec::new();
    for c in r.cards {
        let exacta = c.foreign_names.iter().any(|f| normalizar(&f.name) == b);
        let lista = if exacta { &mut exactas } else { &mut resto };
        if !lista.contains(&c.name) {
            lista.push(c.name);
        }
    }
    resto.retain(|n| !exactas.contains(n));
    exactas.extend(resto);
    exactas
}

static RECIENTES: Mutex<Option<HashMap<String, Vec<String>>>> = Mutex::new(None);

/// Los nombres en inglés de una carta escrita en `idioma` (código de
/// Scryfall). Recuerda las de la sesión. Sin red o sin respuesta, vacío.
pub async fn nombres_ingleses(externo: &Externo, nombre: &str, idioma: &str) -> Vec<String> {
    let Some(lengua) = idioma_api(idioma) else {
        return Vec::new();
    };
    let clave = format!("{idioma}\n{}", normalizar(nombre));
    if let Some(v) = RECIENTES
        .lock()
        .ok()
        .and_then(|m| m.as_ref().and_then(|m| m.get(&clave).cloned()))
    {
        return v;
    }
    let v = match pedir(externo, nombre, lengua).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(codigo = ?e.code, "magicthegathering.io no responde");
            return Vec::new();
        }
    };
    if let Ok(mut m) = RECIENTES.lock() {
        let m = m.get_or_insert_with(HashMap::new);
        if m.len() > 2_000 {
            m.clear();
        }
        m.insert(clave, v.clone());
    }
    v
}

async fn pedir(externo: &Externo, nombre: &str, lengua: &str) -> AppResult<Vec<String>> {
    let mut url = reqwest::Url::parse(API).map_err(|e| {
        crate::error::AppError::new(crate::error::ErrorCode::Internal).with("detalle", e)
    })?;
    url.query_pairs_mut()
        .append_pair("name", nombre)
        .append_pair("language", lengua)
        .append_pair("pageSize", "10");
    let json = externo.bajar(url.as_str(), MAX_RESPUESTA).await?;
    Ok(nombres_de(&json, nombre))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_nombre_ingles_de_la_exacta_primero() {
        let json = include_bytes!("../fixtures/mtgio-muro-de-algas.json");
        assert_eq!(
            nombres_de(json, "Muro de algas"),
            vec!["Wall of Kelp", "Kelpie Guide"]
        );
        assert!(nombres_de(b"no es json", "x").is_empty());
        assert_eq!(idioma_api("es"), Some("spanish"));
        assert_eq!(idioma_api("en"), None);
    }
}

/// Prueba en vivo: `cargo test --lib mtgio::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn muro_de_algas() {
        let e = Externo::nuevo().unwrap();
        let v = nombres_ingleses(&e, "Muro de algas", "es").await;
        println!("{v:?}");
        assert_eq!(v.first().map(String::as_str), Some("Wall of Kelp"));
    }
}
