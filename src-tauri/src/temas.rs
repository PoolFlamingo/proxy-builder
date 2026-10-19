//! **Temas de tweakcn** aparte de los que trae la app (`src/themes.css`): el
//! registro oficial (`/r/themes/registry.json`) y cualquier tema guardado en
//! tweakcn por su id o su enlace (`/r/themes/<id>`, también los de la
//! comunidad, que no tiene listado público: su web va por *server actions*).
//!
//! Un tema se guarda en `app_data_dir/temas/<id>.json` ya convertido a CSS con
//! el formato de `src/themes.css` (`[data-theme="tw-<id>"]` y su `.dark`), y
//! sus letras de Google Fonts se bajan a `app_data_dir/temas/fuentes/` y se
//! sirven por el protocolo (`/f/<huella>.woff2`): la vista web no sale a
//! internet. Del JSON solo se toman variables (`cssVars`) con nombre y valor
//! comprobados: nada de CSS libre.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::fuentes::Externo;
use crate::store::blobs::escribir_atomico;

const REGISTRO: &str = "https://tweakcn.com/r/themes/registry.json";
const TEMA: &str = "https://tweakcn.com/r/themes/";
/// La galería de la comunidad (sin API pública: se abre en el navegador).
pub const COMUNIDAD: &str = "https://tweakcn.com/community";
const MAX_JSON: usize = 4 * 1024 * 1024;
const MAX_LETRA: usize = 2 * 1024 * 1024;
/// Lo que se pone en el CSS en lugar de la raíz del protocolo, que cambia
/// según la plataforma: la pantalla lo cambia por la suya.
pub const BASE: &str = "{{PBIMG}}";

/// Las familias que ya van en la app (`src/fonts.css`) o son del sistema: no
/// se bajan.
const YA_ESTAN: &[&str] = &[
    "antic",
    "architects daughter",
    "dm sans",
    "fira code",
    "geist",
    "geist mono",
    "ibm plex mono",
    "inter",
    "jetbrains mono",
    "libre baskerville",
    "lora",
    "merriweather",
    "montserrat",
    "open sans",
    "plus jakarta sans",
    "poppins",
    "roboto mono",
    "source code pro",
    "source serif 4",
    "serif",
    "sans-serif",
    "monospace",
    "system-ui",
    "ui-sans-serif",
    "ui-serif",
    "ui-monospace",
    "-apple-system",
    "blinkmacsystemfont",
    "segoe ui",
    "georgia",
    "menlo",
    "monaco",
    "consolas",
    "courier new",
    "times new roman",
    "arial",
    "helvetica",
    "helvetica neue",
    "cambria",
    "times",
    "courier",
    "sfmono-regular",
    "liberation mono",
];

#[derive(Debug, Clone, Deserialize)]
struct Registro {
    items: Vec<ItemRegistro>,
}

#[derive(Debug, Clone, Deserialize)]
struct ItemRegistro {
    name: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(rename = "cssVars")]
    css_vars: CssVars,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct CssVars {
    #[serde(default)]
    theme: BTreeMap<String, String>,
    #[serde(default)]
    light: BTreeMap<String, String>,
    #[serde(default)]
    dark: BTreeMap<String, String>,
}

/// Un tema del catálogo, con sus colores para la tarjeta.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemaCatalogo {
    pub id: String,
    pub nombre: String,
    /// Fondo, primario, acento y texto en claro y en oscuro.
    pub claro: Vec<String>,
    pub oscuro: Vec<String>,
    /// La letra principal (`font-sans`), por su nombre.
    pub letra: Option<String>,
    /// El CSS para probarlo sin instalarlo (sin sus letras).
    pub css: String,
}

/// Un tema instalado.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TemaInstalado {
    /// El de `data-theme` (`tw-<id>`).
    pub id: String,
    pub nombre: String,
    /// Las variables del tema y sus `@font-face` (con `{{PBIMG}}` en las URL).
    pub css: String,
    /// El color de la muestra del selector (el primario en oscuro).
    pub muestra: String,
}

fn malo(motivo: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput).with("motivo", motivo)
}

/// El id de un tema a partir de lo que pegue el usuario: el id tal cual o un
/// enlace de tweakcn (`/themes/<id>`, `/r/themes/<id>.json`,
/// `/editor/theme?theme=<id>`).
pub fn id_de(texto: &str) -> Option<String> {
    let t = texto.trim();
    // Un enlace solo vale si es de tweakcn.
    if (t.contains('/') || t.contains('=')) && !t.contains("tweakcn.com/") {
        return None;
    }
    let candidato = if let Some(i) = t.find("theme=") {
        t[i + 6..].split(['&', '#']).next().unwrap_or("")
    } else if t.contains('/') {
        t.trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .split(['?', '#'])
            .next()
            .unwrap_or("")
    } else {
        t
    };
    let id = candidato.trim_end_matches(".json");
    let valido = (2..=64).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    valido.then(|| id.to_string())
}

/// ¿Es aceptable como nombre de variable CSS?
fn nombre_ok(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 40
        && n.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// ¿Es aceptable como valor? Nada que cierre la declaración o el bloque, ni
/// `url(`, ni `@`.
fn valor_ok(v: &str) -> bool {
    v.len() <= 400
        && !v.contains([';', '{', '}', '<', '>', '\\', '@'])
        && !v.to_lowercase().contains("url(")
}

fn bloque(selector: &str, vars: &[(&String, &String)]) -> AppResult<String> {
    let mut s = format!("{selector} {{\n");
    for (k, v) in vars {
        if !nombre_ok(k) || !valor_ok(v) {
            return Err(malo("tema").with("variable", k.as_str()));
        }
        s.push_str(&format!("  --{k}: {v};\n"));
    }
    s.push_str("}\n");
    Ok(s)
}

/// El CSS de un tema con el formato de `src/themes.css`.
fn a_css(id: &str, v: &CssVars) -> AppResult<String> {
    let sel = format!("[data-theme=\"{id}\"]");
    let claro: Vec<_> = v.theme.iter().chain(v.light.iter()).collect();
    let oscuro: Vec<_> = v.theme.iter().chain(v.dark.iter()).collect();
    Ok(format!(
        "{}{}",
        bloque(&sel, &claro)?,
        bloque(&format!("{sel}.dark"), &oscuro)?
    ))
}

/// La primera familia de una pila de letras (`"Fira Code", monospace` →
/// `Fira Code`), si no va ya en la app.
fn familia_a_bajar(pila: &str) -> Option<String> {
    let f = pila
        .split(',')
        .next()?
        .trim()
        .trim_matches(['"', '\''])
        .trim();
    let ok = !f.is_empty()
        && f.len() <= 60
        && f.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-');
    (ok && !YA_ESTAN.contains(&f.to_lowercase().as_str())).then(|| f.to_string())
}

fn muestras(v: &BTreeMap<String, String>) -> Vec<String> {
    ["background", "primary", "accent", "foreground"]
        .iter()
        .filter_map(|k| v.get(*k).cloned())
        .filter(|c| valor_ok(c))
        .collect()
}

fn carpeta(datos: &Path) -> PathBuf {
    datos.join("temas")
}

fn huella(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// El catálogo del registro oficial de tweakcn.
pub async fn catalogo(externo: &Externo) -> AppResult<Vec<TemaCatalogo>> {
    let b = externo.bajar(REGISTRO, MAX_JSON).await?;
    let r: Registro = serde_json::from_slice(&b).map_err(|_| malo("tema"))?;
    Ok(r.items
        .into_iter()
        .filter_map(|i| {
            let id = id_de(&i.name)?;
            let css = a_css(&format!("tw-{id}"), &i.css_vars).ok()?;
            Some(TemaCatalogo {
                nombre: i.title.unwrap_or_else(|| i.name.clone()),
                claro: muestras(&i.css_vars.light),
                oscuro: muestras(&i.css_vars.dark),
                letra: i.css_vars.theme.get("font-sans").and_then(|p| {
                    p.split(',')
                        .next()
                        .map(|f| f.trim().trim_matches(['"', '\'']).to_string())
                }),
                css,
                id,
            })
        })
        .collect())
}

/// Un tema cualquiera de tweakcn por su id o su enlace, para verlo antes de
/// instalarlo.
pub async fn traer(externo: &Externo, texto: &str) -> AppResult<TemaCatalogo> {
    let id = id_de(texto).ok_or_else(|| malo("tema_enlace"))?;
    let item = item(externo, &id).await?;
    Ok(TemaCatalogo {
        nombre: item.title.clone().unwrap_or_else(|| item.name.clone()),
        claro: muestras(&item.css_vars.light),
        oscuro: muestras(&item.css_vars.dark),
        letra: item.css_vars.theme.get("font-sans").cloned(),
        css: a_css(&format!("tw-{id}"), &item.css_vars)?,
        id,
    })
}

async fn item(externo: &Externo, id: &str) -> AppResult<ItemRegistro> {
    let b = externo.bajar(&format!("{TEMA}{id}"), MAX_JSON).await?;
    serde_json::from_slice(&b).map_err(|_| malo("tema"))
}

/// Las `@font-face` (latín) de una familia de Google Fonts, con sus woff2
/// bajados a `fuentes/` y la URL cambiada por la del protocolo.
async fn letras_de(externo: &Externo, dir: &Path, familia: &str) -> AppResult<String> {
    // La API v1 da los pesos que haya (la v2 falla si se pide uno que no está).
    let url = format!(
        "https://fonts.googleapis.com/css?family={}:300,400,500,600,700&subset=latin,latin-ext&display=swap",
        familia.replace(' ', "+")
    );
    let css = String::from_utf8(externo.bajar_como_navegador(&url, MAX_JSON).await?)
        .map_err(|_| malo("letra"))?;
    let mut out = String::new();
    let mut bajadas: BTreeMap<String, String> = BTreeMap::new();
    for trozo in css.split("/* ").skip(1) {
        let subconjunto = trozo.split(" */").next().unwrap_or("");
        if subconjunto != "latin" && subconjunto != "latin-ext" {
            continue;
        }
        let Some(cara) = trozo.split(" */").nth(1) else {
            continue;
        };
        let Some(ini) = cara.find("url(") else {
            continue;
        };
        let resto = &cara[ini + 4..];
        let Some(fin) = resto.find(')') else {
            continue;
        };
        let woff = &resto[..fin];
        let local = match bajadas.get(woff) {
            Some(l) => l.clone(),
            None => {
                let b = externo.bajar(woff, MAX_LETRA).await?;
                // Solo woff2 de verdad (la cabecera), nada que no sea una letra.
                if !b.starts_with(b"wOF2") {
                    return Err(malo("letra"));
                }
                let h = huella(&b);
                escribir_atomico(&dir.join(format!("{h}.woff2")), &b)?;
                bajadas.insert(woff.to_string(), h.clone());
                h
            }
        };
        out.push_str(&cara.replace(woff, &format!("{BASE}f/{local}.woff2")));
    }
    Ok(out)
}

/// Instala un tema (lo baja con sus letras y lo guarda).
pub async fn instalar(externo: &Externo, datos: &Path, texto: &str) -> AppResult<TemaInstalado> {
    let id = id_de(texto).ok_or_else(|| malo("tema_enlace"))?;
    let item = item(externo, &id).await?;
    let data_theme = format!("tw-{id}");
    let mut css = a_css(&data_theme, &item.css_vars)?;
    let dir = carpeta(datos);
    let fuentes = dir.join("fuentes");
    let mut vistas = HashSet::new();
    for k in ["font-sans", "font-serif", "font-mono"] {
        if let Some(f) = item.css_vars.theme.get(k).and_then(|p| familia_a_bajar(p)) {
            if vistas.insert(f.clone()) {
                match letras_de(externo, &fuentes, &f).await {
                    Ok(caras) => css.push_str(&caras),
                    // Una letra que no está en Google Fonts: se queda la de reserva.
                    Err(e) => {
                        tracing::warn!(codigo = ?e.code, familia = %f, "letra del tema sin bajar")
                    }
                }
            }
        }
    }
    let t = TemaInstalado {
        nombre: item.title.clone().unwrap_or_else(|| item.name.clone()),
        muestra: item
            .css_vars
            .dark
            .get("primary")
            .or_else(|| item.css_vars.light.get("primary"))
            .filter(|c| valor_ok(c))
            .cloned()
            .unwrap_or_else(|| "#888".into()),
        id: data_theme,
        css,
    };
    let json = serde_json::to_vec(&t)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    escribir_atomico(&dir.join(format!("{id}.json")), &json)?;
    Ok(t)
}

/// Los temas instalados.
pub fn instalados(datos: &Path) -> Vec<TemaInstalado> {
    let Ok(entradas) = std::fs::read_dir(carpeta(datos)) else {
        return Vec::new();
    };
    let mut v: Vec<TemaInstalado> = entradas
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
        .collect();
    v.sort_by(|a, b| a.nombre.cmp(&b.nombre));
    v
}

/// Quita un tema instalado (sus letras se quedan: pueden ser de otro).
pub fn quitar(datos: &Path, id: &str) -> AppResult<()> {
    let id = id.trim_start_matches("tw-");
    let id = id_de(id).ok_or_else(|| malo("tema_enlace"))?;
    let f = carpeta(datos).join(format!("{id}.json"));
    if f.exists() {
        std::fs::remove_file(f)?;
    }
    Ok(())
}

/// Los bytes de una letra bajada (para el protocolo).
pub fn letra(datos: &Path, nombre: &str) -> AppResult<Vec<u8>> {
    let h = nombre.trim_end_matches(".woff2");
    if !crate::store::blobs::es_huella(h) {
        return Err(AppError::new(ErrorCode::NotFound));
    }
    Ok(std::fs::read(
        carpeta(datos).join("fuentes").join(format!("{h}.woff2")),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = include_str!("fixtures/tweakcn-amethyst-haze.json");

    #[test]
    fn el_id_de_lo_que_se_pegue() {
        for (t, id) in [
            ("amethyst-haze", "amethyst-haze"),
            ("https://tweakcn.com/themes/cmlq1abc23def", "cmlq1abc23def"),
            (
                "https://tweakcn.com/r/themes/cmlq1abc23def.json",
                "cmlq1abc23def",
            ),
            (
                "https://tweakcn.com/editor/theme?theme=claude&x=1",
                "claude",
            ),
            ("https://tweakcn.com/themes/cmlq1abc23def/", "cmlq1abc23def"),
        ] {
            assert_eq!(id_de(t).as_deref(), Some(id), "{t}");
        }
        for malo in [
            "",
            "a",
            "../etc/passwd",
            "hola mundo",
            "x;y",
            "https://malo.com/themes/x1",
        ] {
            assert_eq!(id_de(malo), None, "{malo}");
        }
    }

    #[test]
    fn el_css_con_el_formato_de_la_app() {
        let item: ItemRegistro = serde_json::from_str(JSON).unwrap();
        let css = a_css("tw-amethyst-haze", &item.css_vars).unwrap();
        assert!(css.starts_with("[data-theme=\"tw-amethyst-haze\"] {\n"));
        assert!(css.contains("[data-theme=\"tw-amethyst-haze\"].dark {\n"));
        assert!(css.contains("  --background: oklch(0.9777 0.0041 301.4256);\n"));
        assert!(css.contains("  --font-sans: Geist, sans-serif;\n"));
    }

    #[test]
    fn nada_de_css_libre() {
        let mut v = CssVars::default();
        v.light
            .insert("background".into(), "red; } body { display: none".into());
        assert!(a_css("tw-x", &v).is_err());
        let mut v = CssVars::default();
        v.light
            .insert("background".into(), "url(https://malo.com/x.png)".into());
        assert!(a_css("tw-x", &v).is_err());
        let mut v = CssVars::default();
        v.light.insert("Background{".into(), "red".into());
        assert!(a_css("tw-x", &v).is_err());
    }

    #[test]
    fn solo_se_bajan_las_letras_que_faltan() {
        assert_eq!(familia_a_bajar("\"Fira Code\", monospace"), None);
        assert_eq!(familia_a_bajar("Geist, sans-serif"), None);
        assert_eq!(familia_a_bajar("ui-sans-serif, system-ui"), None);
        assert_eq!(
            familia_a_bajar("Outfit, sans-serif").as_deref(),
            Some("Outfit")
        );
        assert_eq!(
            familia_a_bajar("'Space Grotesk', sans-serif").as_deref(),
            Some("Space Grotesk")
        );
        assert_eq!(familia_a_bajar("x\"; } body {"), None);
    }
}

/// Prueba en vivo: `cargo test --lib temas::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn catalogo_e_instalar() {
        let e = Externo::nuevo().unwrap();
        let c = catalogo(&e).await.unwrap();
        println!("{} temas en el catálogo", c.len());
        let dir = tempfile::tempdir().unwrap();
        let t = instalar(&e, dir.path(), "https://tweakcn.com/themes/soft-pop")
            .await
            .unwrap();
        println!("{} ({}): {} bytes de CSS", t.nombre, t.id, t.css.len());
        assert!(t.css.contains("@font-face"));
        println!(
            "{:?}",
            instalados(dir.path())
                .iter()
                .map(|t| &t.id)
                .collect::<Vec<_>>()
        );
    }
}
