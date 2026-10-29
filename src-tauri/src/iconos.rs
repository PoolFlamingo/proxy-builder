//! **Iconos de Iconify que elige la persona** (el selector de iconos). Los que
//! la app escribe tal cual van dentro de la pantalla (`scripts/iconos.mjs`);
//! estos se piden a la API de Iconify (`api.iconify.design`, por `Externo`:
//! la vista web no sale a internet).
//!
//! - El catálogo (colecciones y nombres de cada una) se guarda en
//!   `app_cache_dir/iconos/` y se renueva cada semana: se puede borrar.
//! - Los datos de los iconos vistos en el selector, en memoria.
//! - Los **elegidos** (los que se guardan en algo: una etiqueta…), en
//!   `app_data_dir/iconos/elegidos.json`, para que se vean sin red.
//!
//! El cuerpo de un icono es SVG que la pantalla mete tal cual: se rechaza el
//! que traiga scripts, manejadores, `foreignObject` o enlaces de fuera.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::fuentes::Externo;
use crate::store::blobs::escribir_atomico;

const API: &str = "https://api.iconify.design";
const MAX_JSON: usize = 8 * 1024 * 1024;
/// Iconos por petición (la URL lleva todos los nombres).
const POR_PETICION: usize = 120;
/// Lo que dura el catálogo guardado.
const RENOVAR: Duration = Duration::from_secs(7 * 24 * 3600);
/// Resultados de una búsqueda (el tope de la API).
const MAX_BUSQUEDA: usize = 999;

/// Lo que hace falta para pintar un icono: lo de dentro del `<svg>` y su
/// `viewBox`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct DatosIcono {
    pub body: String,
    pub vb: String,
}

/// Una colección de Iconify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ColeccionIconos {
    pub prefijo: String,
    pub nombre: String,
    pub total: u32,
    pub categoria: String,
}

fn red_mal(detalle: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Network).with("detalle", detalle)
}

fn mal(motivo: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput).with("motivo", motivo)
}

/// Un id `prefijo:nombre` con lo que admite Iconify (minúsculas, cifras y
/// guiones): nunca llega a la URL nada más.
fn partir(id: &str) -> Option<(&str, &str)> {
    let (p, n) = id.split_once(':')?;
    let ok = |s: &str| {
        !s.is_empty()
            && s.len() <= 64
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    };
    (ok(p) && ok(n)).then_some((p, n))
}

/// ¿Es un id de Iconify que se puede guardar?
pub fn id_valido(id: &str) -> bool {
    partir(id).is_some()
}

fn cache() -> &'static Mutex<HashMap<String, DatosIcono>> {
    static C: OnceLock<Mutex<HashMap<String, DatosIcono>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn busquedas() -> &'static Mutex<HashMap<String, Vec<String>>> {
    static C: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn dir_cache(cache: &Path) -> PathBuf {
    cache.join("iconos")
}

fn fichero_elegidos(data: &Path) -> PathBuf {
    data.join("iconos").join("elegidos.json")
}

fn leer_elegidos(data: &Path) -> BTreeMap<String, DatosIcono> {
    std::fs::read(fichero_elegidos(data))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Un JSON guardado en la caché, si no ha caducado.
fn de_cache<T: for<'de> Deserialize<'de>>(ruta: &Path) -> Option<T> {
    let m = std::fs::metadata(ruta).ok()?;
    let edad = SystemTime::now()
        .duration_since(m.modified().ok()?)
        .unwrap_or_default();
    if edad > RENOVAR {
        return None;
    }
    serde_json::from_slice(&std::fs::read(ruta).ok()?).ok()
}

fn a_cache<T: Serialize>(ruta: &Path, v: &T) {
    if let Ok(b) = serde_json::to_vec(v) {
        if let Some(d) = ruta.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        // Es caché: si no se puede escribir, se vuelve a pedir otro día.
        let _ = escribir_atomico(ruta, &b);
    }
}

async fn json(externo: &Externo, url: &str) -> AppResult<serde_json::Value> {
    let b = externo.bajar(url, MAX_JSON).await?;
    serde_json::from_slice(&b).map_err(red_mal)
}

/// Todas las colecciones, por categoría y nombre.
pub async fn colecciones(externo: &Externo, cache: &Path) -> AppResult<Vec<ColeccionIconos>> {
    let ruta = dir_cache(cache).join("colecciones.json");
    if let Some(v) = de_cache(&ruta) {
        return Ok(v);
    }
    let v = json(externo, &format!("{API}/collections")).await?;
    let mut out: Vec<ColeccionIconos> = v
        .as_object()
        .map(|o| {
            o.iter()
                .filter(|(p, _)| partir(&format!("{p}:x")).is_some())
                .filter(|(_, c)| !c.get("hidden").and_then(|h| h.as_bool()).unwrap_or(false))
                .map(|(p, c)| ColeccionIconos {
                    prefijo: p.clone(),
                    nombre: c
                        .get("name")
                        .and_then(|x| x.as_str())
                        .unwrap_or(p)
                        .to_string(),
                    total: c.get("total").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
                    categoria: c
                        .get("category")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| {
        a.categoria
            .cmp(&b.categoria)
            .then_with(|| a.nombre.to_lowercase().cmp(&b.nombre.to_lowercase()))
    });
    a_cache(&ruta, &out);
    Ok(out)
}

/// Los nombres de una colección (sin los ocultos), en orden.
pub async fn nombres(externo: &Externo, cache: &Path, prefijo: &str) -> AppResult<Vec<String>> {
    if partir(&format!("{prefijo}:x")).is_none() {
        return Err(mal("icono"));
    }
    let ruta = dir_cache(cache).join(format!("{prefijo}.json"));
    if let Some(v) = de_cache(&ruta) {
        return Ok(v);
    }
    let v = json(externo, &format!("{API}/collection?prefix={prefijo}")).await?;
    let mut set = BTreeSet::new();
    let mut meter = |x: &serde_json::Value| {
        for n in x.as_array().into_iter().flatten() {
            if let Some(s) = n.as_str() {
                set.insert(s.to_string());
            }
        }
    };
    if let Some(u) = v.get("uncategorized") {
        meter(u);
    }
    if let Some(c) = v.get("categories").and_then(|c| c.as_object()) {
        for x in c.values() {
            meter(x);
        }
    }
    let out: Vec<String> = set.into_iter().collect();
    a_cache(&ruta, &out);
    Ok(out)
}

/// Busca por nombre en todas las colecciones.
pub async fn buscar(externo: &Externo, texto: &str) -> AppResult<Vec<String>> {
    let q: String = texto
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == ' ')
        .take(40)
        .collect();
    if q.trim().is_empty() {
        return Ok(Vec::new());
    }
    if let Ok(c) = busquedas().lock() {
        if let Some(v) = c.get(&q) {
            return Ok(v.clone());
        }
    }
    let url = format!(
        "{API}/search?query={}&limit={MAX_BUSQUEDA}",
        q.replace(' ', "%20")
    );
    let v = json(externo, &url).await?;
    let out: Vec<String> = v
        .get("icons")
        .and_then(|x| x.as_array())
        .into_iter()
        .flatten()
        .filter_map(|x| x.as_str())
        .filter(|id| partir(id).is_some())
        .map(String::from)
        .collect();
    if let Ok(mut c) = busquedas().lock() {
        c.insert(q, out.clone());
    }
    Ok(out)
}

/// Lo que se ve de un icono: el suyo o el de su padre (un alias), con los
/// giros y volteos del alias.
#[derive(Default, Clone)]
struct Crudo {
    body: String,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    rotate: u8,
    h_flip: bool,
    v_flip: bool,
}

fn num(v: &serde_json::Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_f64())
}

fn resolver(coleccion: &serde_json::Value, nombre: &str) -> Option<Crudo> {
    let w = num(coleccion, "width").unwrap_or(16.0);
    let h = num(coleccion, "height").unwrap_or(16.0);
    let iconos = coleccion.get("icons")?;
    let alias = coleccion.get("aliases");
    let mut cadena = Vec::new();
    let mut actual = nombre.to_string();
    // Un alias apunta a otro (o a otro alias): como mucho unos saltos.
    for _ in 0..6 {
        if let Some(i) = iconos.get(&actual) {
            let mut c = Crudo {
                body: i.get("body")?.as_str()?.to_string(),
                left: num(i, "left").unwrap_or(0.0),
                top: num(i, "top").unwrap_or(0.0),
                width: num(i, "width").unwrap_or(w),
                height: num(i, "height").unwrap_or(h),
                rotate: num(i, "rotate").unwrap_or(0.0) as u8 % 4,
                h_flip: i.get("hFlip").and_then(|x| x.as_bool()).unwrap_or(false),
                v_flip: i.get("vFlip").and_then(|x| x.as_bool()).unwrap_or(false),
            };
            for a in cadena.iter().rev() {
                let a: &serde_json::Value = a;
                c.left = num(a, "left").unwrap_or(c.left);
                c.top = num(a, "top").unwrap_or(c.top);
                c.width = num(a, "width").unwrap_or(c.width);
                c.height = num(a, "height").unwrap_or(c.height);
                c.rotate = (c.rotate + num(a, "rotate").unwrap_or(0.0) as u8) % 4;
                c.h_flip ^= a.get("hFlip").and_then(|x| x.as_bool()).unwrap_or(false);
                c.v_flip ^= a.get("vFlip").and_then(|x| x.as_bool()).unwrap_or(false);
            }
            return Some(c);
        }
        let a = alias?.get(&actual)?;
        actual = a.get("parent")?.as_str()?.to_string();
        cadena.push(a.clone());
    }
    None
}

fn n(x: f64) -> String {
    let s = format!("{x:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Como `iconToSVG` de `@iconify/utils`: los volteos y giros van en un `<g>`.
fn a_svg(c: &Crudo) -> DatosIcono {
    let (mut l, mut t, mut w, mut h) = (c.left, c.top, c.width, c.height);
    let mut transformaciones = Vec::new();
    if c.h_flip {
        transformaciones.push(format!("translate({} {}) scale(-1 1)", n(w + 2.0 * l), 0));
    }
    if c.v_flip {
        transformaciones.push(format!("translate({} {}) scale(1 -1)", 0, n(h + 2.0 * t)));
    }
    match c.rotate {
        1 => {
            transformaciones.insert(
                0,
                format!("rotate(90 {} {})", n(h / 2.0 + t), n(h / 2.0 + t)),
            );
        }
        2 => {
            transformaciones.insert(
                0,
                format!("rotate(180 {} {})", n(w / 2.0 + l), n(h / 2.0 + t)),
            );
        }
        3 => {
            transformaciones.insert(
                0,
                format!("rotate(-90 {} {})", n(w / 2.0 + l), n(w / 2.0 + l)),
            );
        }
        _ => {}
    }
    if c.rotate % 2 == 1 {
        std::mem::swap(&mut w, &mut h);
        std::mem::swap(&mut l, &mut t);
    }
    let body = if transformaciones.is_empty() {
        c.body.clone()
    } else {
        format!(
            "<g transform=\"{}\">{}</g>",
            transformaciones.join(" "),
            c.body
        )
    };
    DatosIcono {
        body,
        vb: format!("{} {} {} {}", n(l), n(t), n(w), n(h)),
    }
}

/// ¿Se puede meter este SVG en la pantalla? Nada que ejecute ni que cargue de fuera.
fn seguro(body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    if b.contains("<script")
        || b.contains("foreignobject")
        || b.contains("javascript:")
        || b.contains("<image")
        || b.contains("<iframe")
        || b.contains("<!entity")
    {
        return false;
    }
    // Manejadores `on…=` en cualquier etiqueta.
    let bytes = b.as_bytes();
    for (i, w) in bytes.windows(2).enumerate() {
        if w == b"on" && i > 0 && bytes[i - 1].is_ascii_whitespace() {
            let resto = &b[i + 2..];
            let letras = resto
                .bytes()
                .take_while(|c| c.is_ascii_alphabetic())
                .count();
            if letras > 0 && resto[letras..].trim_start().starts_with('=') {
                return false;
            }
        }
    }
    // Enlaces solo dentro del propio SVG (`#id`).
    for clave in ["href=\"", "href='", "url("] {
        for (i, _) in b.match_indices(clave) {
            let resto = b[i + clave.len()..].trim_start_matches(['\'', '"', ' ']);
            if !resto.starts_with('#') {
                return false;
            }
        }
    }
    true
}

/// Los datos de estos iconos: de memoria, de los elegidos o de la API.
pub async fn datos(
    externo: &Externo,
    data: &Path,
    ids: &[String],
) -> AppResult<BTreeMap<String, DatosIcono>> {
    let mut out = BTreeMap::new();
    let mut faltan: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let elegidos = leer_elegidos(data);
    {
        let c = cache().lock().map_err(|_| red_mal("cache"))?;
        for id in ids.iter().take(2000) {
            let Some((p, nombre)) = partir(id) else {
                continue;
            };
            if let Some(d) = c.get(id).or_else(|| elegidos.get(id)) {
                out.insert(id.clone(), d.clone());
            } else {
                faltan
                    .entry(p.to_string())
                    .or_default()
                    .push(nombre.to_string());
            }
        }
    }
    for (prefijo, nombres) in faltan {
        for trozo in nombres.chunks(POR_PETICION) {
            let url = format!("{API}/{prefijo}.json?icons={}", trozo.join(","));
            let v = json(externo, &url).await?;
            let mut c = cache().lock().map_err(|_| red_mal("cache"))?;
            for nombre in trozo {
                let Some(d) = resolver(&v, nombre).map(|x| a_svg(&x)) else {
                    continue;
                };
                if !seguro(&d.body) {
                    continue;
                }
                let id = format!("{prefijo}:{nombre}");
                c.insert(id.clone(), d.clone());
                out.insert(id, d);
            }
        }
    }
    Ok(out)
}

/// Guarda un icono elegido para que se vea sin red.
pub async fn elegir(externo: &Externo, data: &Path, id: &str) -> AppResult<DatosIcono> {
    let d = datos(externo, data, &[id.to_string()])
        .await?
        .remove(id)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("icono", id))?;
    let mut elegidos = leer_elegidos(data);
    if elegidos.get(id) != Some(&d) {
        elegidos.insert(id.to_string(), d.clone());
        let ruta = fichero_elegidos(data);
        if let Some(dir) = ruta.parent() {
            std::fs::create_dir_all(dir)?;
        }
        escribir_atomico(&ruta, &serde_json::to_vec(&elegidos).map_err(red_mal)?)?;
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn los_ids_raros_no_llegan_a_la_url() {
        assert!(partir("mdi:home").is_some());
        assert!(partir("game-icons:crossed-swords").is_some());
        assert!(partir("mdi:home&x=1").is_none());
        assert!(partir("../x:y").is_none());
        assert!(partir("mdi").is_none());
    }

    #[test]
    fn el_svg_peligroso_se_rechaza() {
        assert!(seguro("<path fill=\"currentColor\" d=\"M0 0h24v24H0z\"/>"));
        assert!(seguro("<g mask=\"url(#a)\"><path d=\"M1 1\"/></g>"));
        assert!(!seguro("<path onload=\"x()\" d=\"M0 0\"/>"));
        assert!(!seguro("<script>x()</script>"));
        assert!(!seguro("<use href=\"https://x/y.svg#a\"/>"));
        assert!(!seguro("<path style=\"fill:url(https://x)\"/>"));
        // «on» dentro de un nombre no es un manejador.
        assert!(seguro("<polygon points=\"0 0 1 1\"/>"));
    }

    #[test]
    fn un_alias_hereda_del_padre_y_se_voltea() {
        let c = serde_json::json!({
            "prefix": "x",
            "width": 24,
            "height": 24,
            "icons": { "a": { "body": "<path d=\"M0 0\"/>" } },
            "aliases": { "b": { "parent": "a", "hFlip": true } }
        });
        let a = a_svg(&resolver(&c, "a").unwrap());
        assert_eq!(a.vb, "0 0 24 24");
        assert_eq!(a.body, "<path d=\"M0 0\"/>");
        let b = a_svg(&resolver(&c, "b").unwrap());
        assert!(b
            .body
            .starts_with("<g transform=\"translate(24 0) scale(-1 1)\">"));
        assert!(resolver(&c, "z").is_none());
    }
}
