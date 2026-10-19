//! **Nombres en otros idiomas, sin red** (descarga opcional en Ajustes): de
//! MTGJSON (`AtomicCards.json.zip`, ~52 MB) se guarda solo el nombre impreso
//! de cada carta en cada idioma y su nombre en inglés, en una tabla SQLite en
//! `app_cache_dir/mtgjson/` (se puede borrar sin perder nada). Sirve para lo
//! mismo que magicthegathering.io (de «Muro de algas» a Wall of Kelp), pero
//! al momento. MTGJSON no tiene fichas que Scryfall no tenga: solo nombres.
//!
//! El zip se baja por partes a un fichero, se comprueba con el SHA-256 que
//! publica MTGJSON, se lee con tope de tamaño y se borra.

use std::collections::BTreeSet;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Externo;
use crate::commands::identify::normalizar;
use crate::error::{AppError, AppResult, ErrorCode};

const URL: &str = "https://mtgjson.com/api/v5/AtomicCards.json.zip";
const URL_SHA: &str = "https://mtgjson.com/api/v5/AtomicCards.json.zip.sha256";
/// Tope del zip (hoy, 52 MB) y de lo que tiene dentro (hoy, unos 400 MB).
const MAX_ZIP: u64 = 120 * 1024 * 1024;
const MAX_JSON: u64 = 1024 * 1024 * 1024;
const CARPETA: &str = "mtgjson";
const BASE: &str = "nombres.sqlite";

fn carpeta(cache: &Path) -> PathBuf {
    cache.join(CARPETA)
}

/// El código de idioma de Scryfall de un idioma de MTGJSON.
fn codigo(idioma: &str) -> Option<&'static str> {
    Some(match idioma {
        "Spanish" => "es",
        "French" => "fr",
        "German" => "de",
        "Italian" => "it",
        "Portuguese (Brazil)" | "Portuguese" => "pt",
        "Japanese" => "ja",
        "Korean" => "ko",
        "Russian" => "ru",
        "Chinese Simplified" => "zhs",
        "Chinese Traditional" => "zht",
        "Phyrexian" => "ph",
        _ => return None,
    })
}

#[derive(Deserialize)]
struct Atomic {
    data: std::collections::HashMap<String, Vec<Cara>>,
}

#[derive(Deserialize)]
struct Cara {
    #[serde(default, rename = "foreignData")]
    foreign_data: Vec<Extranjero>,
}

#[derive(Deserialize)]
struct Extranjero {
    #[serde(default)]
    language: String,
    #[serde(default)]
    name: String,
}

/// Lee `AtomicCards.json` (sin cargar en memoria más que los nombres: lo demás
/// se salta al leer) y devuelve (nombre normalizado, idioma, inglés).
fn leer_nombres(json: impl Read) -> AppResult<BTreeSet<(String, String, String)>> {
    let a: Atomic = serde_json::from_reader(BufReader::new(json)).map_err(|e| {
        AppError::new(ErrorCode::InvalidInput)
            .with("motivo", "mtgjson")
            .with("detalle", e)
    })?;
    let mut out = BTreeSet::new();
    for (ingles, caras) in a.data {
        for cara in caras {
            for f in cara.foreign_data {
                if let (Some(l), false) = (codigo(&f.language), f.name.trim().is_empty()) {
                    out.insert((normalizar(&f.name), l.to_string(), ingles.clone()));
                }
            }
        }
    }
    Ok(out)
}

fn escribir_base(ruta: &Path, nombres: &BTreeSet<(String, String, String)>) -> AppResult<()> {
    let tmp = ruta.with_extension("tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        let mut c = Connection::open(&tmp)?;
        c.execute_batch(
            "CREATE TABLE nombres (extranjero TEXT NOT NULL, idioma TEXT NOT NULL, ingles TEXT NOT NULL);",
        )?;
        let tx = c.transaction()?;
        {
            let mut st = tx.prepare("INSERT INTO nombres VALUES (?1, ?2, ?3)")?;
            for (e, l, i) in nombres {
                st.execute(params![e, l, i])?;
            }
        }
        tx.commit()?;
        c.execute_batch("CREATE INDEX nombres_extranjero ON nombres(extranjero);")?;
    }
    std::fs::rename(&tmp, ruta)?;
    Ok(())
}

/// La base abierta de la sesión (se abre la primera vez que hace falta).
static BASE_ABIERTA: Mutex<Option<(PathBuf, Connection)>> = Mutex::new(None);

fn con_base<T>(cache: &Path, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Option<T> {
    let ruta = carpeta(cache).join(BASE);
    let mut g = BASE_ABIERTA.lock().ok()?;
    if g.as_ref().is_none_or(|(r, _)| *r != ruta) {
        if !ruta.exists() {
            return None;
        }
        let c =
            Connection::open_with_flags(&ruta, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;
        *g = Some((ruta, c));
    }
    g.as_ref().and_then(|(_, c)| f(c).ok())
}

fn cerrar_base() {
    if let Ok(mut g) = BASE_ABIERTA.lock() {
        *g = None;
    }
}

/// Los nombres en inglés de una carta escrita en `idioma` (código de
/// Scryfall); si en ese idioma no hay ninguna, en cualquiera. `None` si el
/// índice no se ha descargado.
pub fn nombres_ingleses(cache: &Path, nombre: &str, idioma: &str) -> Option<Vec<String>> {
    let n = normalizar(nombre);
    con_base(cache, |c| {
        let mut st = c.prepare_cached(
            "SELECT DISTINCT ingles FROM nombres WHERE extranjero = ?1 ORDER BY (idioma = ?2) DESC, ingles",
        )?;
        let v = st
            .query_map(params![n, idioma], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(v)
    })
}

/// Cómo está el índice.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct NombresIdiomasInfo {
    pub descargado: bool,
    /// Cuándo se descargó (ms desde 1970).
    #[ts(type = "number | null")]
    pub fecha: Option<i64>,
    pub nombres: u32,
    /// Lo que se baja (el zip), en bytes aproximados.
    #[ts(type = "number")]
    pub tamano: u64,
}

pub fn info(cache: &Path) -> NombresIdiomasInfo {
    let ruta = carpeta(cache).join(BASE);
    let fecha = std::fs::metadata(&ruta)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64);
    let nombres = con_base(cache, |c| {
        c.query_row("SELECT COUNT(*) FROM nombres", [], |r| r.get::<_, u32>(0))
            .optional()
            .map(|n| n.unwrap_or(0))
    })
    .unwrap_or(0);
    NombresIdiomasInfo {
        descargado: fecha.is_some(),
        fecha,
        nombres,
        tamano: 52 * 1024 * 1024,
    }
}

/// Lo que va haciendo la descarga.
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FaseNombres {
    Bajando,
    Leyendo,
}

/// Baja el índice: zip por partes → SHA-256 → nombres → base. `progreso(fase,
/// hecho, total)`.
pub async fn descargar(
    externo: &Externo,
    cache: &Path,
    progreso: impl Fn(FaseNombres, u64, u64) + Send + Sync + 'static,
) -> AppResult<NombresIdiomasInfo> {
    let dir = carpeta(cache);
    std::fs::create_dir_all(&dir)?;
    let esperado = String::from_utf8(externo.bajar(URL_SHA, 1024).await?)
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_lowercase();
    let zip = dir.join("AtomicCards.json.zip");
    let huella = externo
        .bajar_a_fichero(URL, &zip, MAX_ZIP, |hecho, total| {
            progreso(FaseNombres::Bajando, hecho, total.unwrap_or(0))
        })
        .await?;
    if esperado.len() != 64 || huella != esperado {
        let _ = std::fs::remove_file(&zip);
        return Err(AppError::new(ErrorCode::Network)
            .with("motivo", "huella")
            .with("fichero", "AtomicCards.json.zip"));
    }
    progreso(FaseNombres::Leyendo, 0, 1);
    let ruta = dir.join(BASE);
    let zip2 = zip.clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<()> {
        let f = std::fs::File::open(&zip2)?;
        let mut a = zip::ZipArchive::new(f).map_err(|e| {
            AppError::new(ErrorCode::InvalidInput)
                .with("motivo", "mtgjson")
                .with("detalle", e)
        })?;
        let mut entrada = a.by_name("AtomicCards.json").map_err(|e| {
            AppError::new(ErrorCode::InvalidInput)
                .with("motivo", "mtgjson")
                .with("detalle", e)
        })?;
        if entrada.size() > MAX_JSON {
            return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "grande"));
        }
        let nombres = leer_nombres((&mut entrada).take(MAX_JSON))?;
        cerrar_base();
        escribir_base(&ruta, &nombres)
    })
    .await
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))??;
    let _ = std::fs::remove_file(&zip);
    progreso(FaseNombres::Leyendo, 1, 1);
    Ok(info(cache))
}

/// Quita el índice.
pub fn quitar(cache: &Path) -> AppResult<()> {
    cerrar_base();
    let dir = carpeta(cache);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{
        "meta": {"version": "5"},
        "data": {
            "Lightning Bolt": [{"name": "Lightning Bolt", "text": "...", "foreignData": [
                {"language": "Spanish", "name": "Rayo", "text": "..."},
                {"language": "French", "name": "Foudre"}
            ]}],
            "Wall of Kelp": [{"foreignData": [{"language": "Spanish", "name": "Muro de algas"}]}],
            "Fire // Ice": [
                {"foreignData": [{"language": "Spanish", "name": "Fuego"}]},
                {"foreignData": [{"language": "Spanish", "name": "Hielo"}]}
            ],
            "Sin traducir": [{"name": "Sin traducir"}]
        }
    }"#;

    #[test]
    fn indice_de_nombres() {
        let dir = tempfile::tempdir().unwrap();
        let nombres = leer_nombres(JSON.as_bytes()).unwrap();
        assert_eq!(nombres.len(), 5);
        std::fs::create_dir_all(carpeta(dir.path())).unwrap();
        escribir_base(&carpeta(dir.path()).join(BASE), &nombres).unwrap();
        let rayo = nombres_ingleses(dir.path(), "Rayo", "es").unwrap();
        assert_eq!(rayo, vec!["Lightning Bolt"]);
        assert_eq!(
            nombres_ingleses(dir.path(), "muro de ALGAS", "es").unwrap(),
            vec!["Wall of Kelp"]
        );
        assert_eq!(
            nombres_ingleses(dir.path(), "Hielo", "es").unwrap(),
            vec!["Fire // Ice"]
        );
        // En otro idioma también, si no hay en el pedido.
        assert_eq!(
            nombres_ingleses(dir.path(), "Foudre", "es").unwrap(),
            vec!["Lightning Bolt"]
        );
        assert!(nombres_ingleses(dir.path(), "Nada", "es")
            .unwrap()
            .is_empty());
        let i = info(dir.path());
        assert!(i.descargado);
        assert_eq!(i.nombres, 5);
        quitar(dir.path()).unwrap();
        assert!(nombres_ingleses(dir.path(), "Rayo", "es").is_none());
        assert!(!info(dir.path()).descargado);
    }
}

/// Prueba en vivo (baja ~52 MB): `cargo test --release --lib mtgjson::en_vivo --
/// --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn baja_y_busca() {
        let dir = tempfile::tempdir().unwrap();
        let e = Externo::nuevo().unwrap();
        let t = std::time::Instant::now();
        let i = descargar(&e, dir.path(), |_, _, _| {}).await.unwrap();
        println!("{i:?} en {:?}", t.elapsed());
        for (n, l) in [
            ("Muro de algas", "es"),
            ("Rayo", "es"),
            ("Anillo solar", "es"),
        ] {
            println!("{n} → {:?}", nombres_ingleses(dir.path(), n, l));
        }
        let peso = std::fs::metadata(carpeta(dir.path()).join(BASE))
            .unwrap()
            .len();
        println!("base: {} MB", peso / 1024 / 1024);
    }
}
