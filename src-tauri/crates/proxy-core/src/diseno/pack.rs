//! **Packs de marcos**: cómo es cada estilo de carta. Un pack es un
//! `pack.json` y sus capas SVG. El de la casa (`builtin`) va dentro de la app
//! y se escribe en este mismo formato, así que un pack hecho por cualquiera
//! funciona igual. El formato se documenta en `docs/packs-de-marcos.md`.
//!
//! - Medidas en **milímetros** sobre la carta de 63×88; el motor las escala
//!   al tamaño de la carta y a los píxeles de salida.
//! - Las capas SVG llevan variables `{{nombre}}` que el motor rellena antes
//!   de pintarlas: colores de la paleta, la geometría de la caja de reglas
//!   que crece y los degradados (`{{defs}}`).
//! - Los huecos pueden usar expresiones sencillas con esas variables
//!   (`"yTipo+1.5"`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Versión del formato que entiende esta app.
pub const VERSION_PACK: u32 = 1;

/// Lo que puede fallar al cargar un pack (códigos, no prosa).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackError {
    #[error("pack.json no se entiende: {0}")]
    Json(String),
    #[error("versión de pack no admitida: {0}")]
    Version(u32),
    #[error("falta el fichero {0}")]
    FaltaFichero(String),
    #[error("falta el hueco {hueco} en el estilo {estilo}")]
    FaltaHueco { estilo: String, hueco: String },
    #[error("falta el estilo {0}")]
    FaltaEstilo(String),
    #[error("color mal escrito: {0}")]
    Color(String),
    #[error("expresión no válida: {0}")]
    Expresion(String),
}

/// Un número o una expresión con variables (`"yTipo+1.5"`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Valor {
    Num(f32),
    Expr(String),
}

impl Valor {
    pub fn evaluar(&self, vars: &BTreeMap<&str, f32>) -> Result<f32, PackError> {
        match self {
            Valor::Num(n) => Ok(*n),
            Valor::Expr(e) => evaluar(e, vars),
        }
    }
}

/// Suma y resta de números y variables, sin paréntesis.
pub fn evaluar(e: &str, vars: &BTreeMap<&str, f32>) -> Result<f32, PackError> {
    let mut total = 0.0;
    let mut signo = 1.0;
    let mut actual = String::new();
    let cerrar = |actual: &mut String, signo: f32, total: &mut f32| -> Result<(), PackError> {
        let t = actual.trim();
        if t.is_empty() {
            return Err(PackError::Expresion(e.to_string()));
        }
        let v = match t.parse::<f32>() {
            Ok(n) => n,
            Err(_) => *vars
                .get(t)
                .ok_or_else(|| PackError::Expresion(e.to_string()))?,
        };
        *total += signo * v;
        actual.clear();
        Ok(())
    };
    for c in e.chars() {
        match c {
            '+' | '-' if !actual.trim().is_empty() => {
                cerrar(&mut actual, signo, &mut total)?;
                signo = if c == '+' { 1.0 } else { -1.0 };
            }
            c => actual.push(c),
        }
    }
    cerrar(&mut actual, signo, &mut total)?;
    Ok(total)
}

/// Un color de una paleta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaletaDef {
    /// Brillo, cuerpo y sombra del marco (degradado vertical).
    pub claro: String,
    pub medio: String,
    pub oscuro: String,
    /// Tinte de la caja de reglas translúcida (despejado).
    pub tinte: String,
    /// Caja de reglas clara (clásico, extendido) y su tinta.
    pub caja: String,
    pub tinta: String,
}

/// La letra de un papel (título, reglas…).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LetraDef {
    pub familia: String,
    #[serde(default = "peso_normal")]
    pub peso: f32,
    /// Altura de mayúscula / cuerpo de esta letra (para medir el texto por
    /// la altura de las mayúsculas, que es lo que se ve).
    pub mayuscula: f32,
}

fn peso_normal() -> f32 {
    400.0
}

/// Cuándo se pinta una capa.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Condicion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corona: Option<bool>,
    /// Lleva fuerza/resistencia.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fr: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cinta: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emblema: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capa {
    /// Ruta del SVG dentro del pack.
    pub svg: String,
    #[serde(default)]
    pub si: Condicion,
}

/// Alineación de un texto en su hueco.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Alineacion {
    #[default]
    Inicio,
    Centro,
    Fin,
}

/// Un rectángulo de la carta, con lo que se escribe en él.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hueco {
    pub x: Valor,
    pub y: Valor,
    pub w: Valor,
    pub h: Valor,
    /// Papel de la letra (`titulo`, `tipo`, `reglas`, `fr`, `pie`, `cinta`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rol: Option<String>,
    /// Altura de mayúscula en mm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mayuscula: Option<f32>,
    /// Color del texto: `#rrggbb[aa]` o una variable (`{{tintaCaja}}`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default)]
    pub alinear: Alineacion,
    #[serde(default)]
    pub sombra: bool,
    /// Tabuladores en mm desde el borde izquierdo (pie).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tabs: Vec<f32>,
    /// Separación entre líneas en mm (pie de dos líneas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interlinea: Option<f32>,
    /// Versalitas (títulos de las fichas).
    #[serde(default)]
    pub versalitas: bool,
}

/// La caja de reglas que crece (§3.7 del plan): de `altoMin` a `altoMax`
/// con el borde de abajo fijo en `fondo`. La barra de tipo va encima.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CajaDinamica {
    pub fondo: f32,
    pub alto_min: f32,
    pub alto_max: f32,
    pub barra_tipo: f32,
    pub pad_arriba: f32,
    pub pad_abajo: f32,
}

/// Qué caja de reglas lleva el estilo de serie.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CajaPorDefecto {
    /// Translúcida y teñida del color de la carta (despejado).
    #[default]
    Tinte,
    /// Clara y opaca (clásico, extendido).
    Clara,
}

/// Cambios de un layout (ficha…) sobre el estilo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capas: Option<Vec<Capa>>,
    #[serde(default)]
    pub huecos: BTreeMap<String, Hueco>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstiloDef {
    /// Dónde va la ilustración.
    pub arte: Hueco,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caja: Option<CajaDinamica>,
    #[serde(default)]
    pub caja_por_defecto: CajaPorDefecto,
    pub capas: Vec<Capa>,
    pub huecos: BTreeMap<String, Hueco>,
    #[serde(default)]
    pub layouts: BTreeMap<String, LayoutDef>,
    /// Radio de las esquinas de la carta en pantalla (mm).
    #[serde(default = "radio_defecto")]
    pub radio: f32,
}

fn radio_defecto() -> f32 {
    3.0
}

/// El `pack.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackDef {
    pub id: String,
    pub nombre: String,
    pub version: u32,
    pub paletas: BTreeMap<String, PaletaDef>,
    pub letras: BTreeMap<String, LetraDef>,
    pub estilos: BTreeMap<String, EstiloDef>,
}

/// Un pack cargado: la definición y sus ficheros.
#[derive(Debug, Clone)]
pub struct Pack {
    pub def: PackDef,
    pub ficheros: BTreeMap<String, String>,
    /// Huella de todo el pack (entra en la huella del render).
    pub huella: String,
}

/// Los huecos que todo estilo tiene que tener.
pub const HUECOS_OBLIGATORIOS: &[&str] = &["titulo", "tipo", "reglas", "pie"];
/// Las paletas que el motor elige solo.
pub const PALETAS: &[&str] = &[
    "W",
    "U",
    "B",
    "R",
    "G",
    "gold",
    "artifact",
    "land",
    "colorless",
];
/// Los papeles de letra.
pub const ROLES: &[&str] = &["titulo", "tipo", "reglas", "fr", "pie", "cinta"];

/// ¿`#rgb`, `#rrggbb` o `#rrggbbaa`?
pub fn color_valido(c: &str) -> bool {
    let Some(h) = c.strip_prefix('#') else {
        return false;
    };
    matches!(h.len(), 3 | 6 | 8) && h.chars().all(|c| c.is_ascii_hexdigit())
}

/// Un color `#rrggbb[aa]` a RGBA.
pub fn rgba(c: &str) -> Option<[u8; 4]> {
    let h = c.strip_prefix('#')?;
    let n = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        3 => {
            let d = |i: usize| u8::from_str_radix(&h[i..=i].repeat(2), 16).ok();
            Some([d(0)?, d(1)?, d(2)?, 255])
        }
        6 => Some([n(0)?, n(2)?, n(4)?, 255]),
        8 => Some([n(0)?, n(2)?, n(4)?, n(6)?]),
        _ => None,
    }
}

impl Pack {
    /// Carga y valida un pack desde sus ficheros (ruta relativa → contenido).
    pub fn cargar(ficheros: BTreeMap<String, String>) -> Result<Pack, PackError> {
        let json = ficheros
            .get("pack.json")
            .ok_or_else(|| PackError::FaltaFichero("pack.json".into()))?;
        let def: PackDef =
            serde_json::from_str(json).map_err(|e| PackError::Json(e.to_string()))?;
        if def.version == 0 || def.version > VERSION_PACK {
            return Err(PackError::Version(def.version));
        }
        for (nombre, p) in &def.paletas {
            for c in [&p.claro, &p.medio, &p.oscuro, &p.tinte, &p.caja, &p.tinta] {
                if !color_valido(c) {
                    return Err(PackError::Color(format!("{nombre}: {c}")));
                }
            }
        }
        for p in PALETAS {
            if !def.paletas.contains_key(*p) {
                return Err(PackError::Color(format!("falta la paleta {p}")));
            }
        }
        for (nombre, e) in &def.estilos {
            for h in HUECOS_OBLIGATORIOS {
                if !e.huecos.contains_key(*h) {
                    return Err(PackError::FaltaHueco {
                        estilo: nombre.clone(),
                        hueco: (*h).into(),
                    });
                }
            }
            let capas = e
                .capas
                .iter()
                .chain(e.layouts.values().flat_map(|l| l.capas.iter().flatten()));
            for c in capas {
                if !ficheros.contains_key(&c.svg) {
                    return Err(PackError::FaltaFichero(c.svg.clone()));
                }
            }
            for h in e
                .huecos
                .values()
                .chain(e.layouts.values().flat_map(|l| l.huecos.values()))
            {
                if let Some(c) = &h.color {
                    if !c.starts_with("{{") && !color_valido(c) {
                        return Err(PackError::Color(c.clone()));
                    }
                }
            }
        }
        let mut h = Sha256::new();
        for (k, v) in &ficheros {
            h.update(k.as_bytes());
            h.update([0]);
            h.update(v.as_bytes());
            h.update([0]);
        }
        let huella = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        Ok(Pack {
            def,
            ficheros,
            huella,
        })
    }

    /// El pack de la casa, dentro de la app.
    pub fn de_fabrica() -> Result<Pack, PackError> {
        let ficheros = BUILTIN
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Pack::cargar(ficheros)
    }

    pub fn estilo(&self, nombre: &str) -> Result<&EstiloDef, PackError> {
        self.def
            .estilos
            .get(nombre)
            .ok_or_else(|| PackError::FaltaEstilo(nombre.into()))
    }
}

/// Rellena las variables `{{x}}` de una plantilla. Las que no están se
/// quedan vacías.
pub fn rellenar(plantilla: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(plantilla.len() + 256);
    let mut resto = plantilla;
    while let Some(i) = resto.find("{{") {
        out.push_str(&resto[..i]);
        let tras = &resto[i + 2..];
        match tras.find("}}") {
            Some(f) => {
                let nombre = tras[..f].trim();
                if let Some(v) = vars.get(nombre) {
                    out.push_str(v);
                }
                resto = &tras[f + 2..];
            }
            None => {
                out.push_str(&resto[i..]);
                return out;
            }
        }
    }
    out.push_str(resto);
    out
}

macro_rules! fichero {
    ($ruta:literal) => {
        (
            $ruta,
            include_str!(concat!("../../assets/marcos/builtin/", $ruta)),
        )
    };
}

/// Los ficheros del pack de la casa.
const BUILTIN: &[(&str, &str)] = &[
    fichero!("pack.json"),
    fichero!("despejado/mascara.svg"),
    fichero!("despejado/corona.svg"),
    fichero!("despejado/titulo.svg"),
    fichero!("despejado/reglas.svg"),
    fichero!("despejado/fr.svg"),
    fichero!("extendido/marco.svg"),
    fichero!("extendido/cinta.svg"),
    fichero!("extendido/corona.svg"),
    fichero!("extendido/fr.svg"),
    fichero!("clasico/marco.svg"),
    fichero!("clasico/cinta.svg"),
    fichero!("clasico/corona.svg"),
    fichero!("clasico/fr.svg"),
    fichero!("paisaje/franja.svg"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expresiones() {
        let mut v = BTreeMap::new();
        v.insert("yTipo", 50.0);
        assert_eq!(evaluar("yTipo+1.5", &v).unwrap(), 51.5);
        assert_eq!(evaluar("yTipo - 2 + 0.5", &v).unwrap(), 48.5);
        assert_eq!(evaluar("-3", &v).unwrap(), -3.0);
        assert!(evaluar("yNada+1", &v).is_err());
        assert!(evaluar("", &v).is_err());
    }

    #[test]
    fn colores() {
        assert_eq!(rgba("#ff0080"), Some([255, 0, 128, 255]));
        assert_eq!(rgba("#fff"), Some([255, 255, 255, 255]));
        assert_eq!(rgba("#00000080"), Some([0, 0, 0, 128]));
        assert!(!color_valido("rojo"));
        assert!(!color_valido("#12345"));
    }

    #[test]
    fn plantillas() {
        let mut v = BTreeMap::new();
        v.insert("a".to_string(), "1".to_string());
        assert_eq!(rellenar("x{{a}}y{{ b }}z{{", &v), "x1yz{{");
    }

    #[test]
    fn el_pack_de_la_casa_carga_y_sus_svg_se_entienden() {
        let p = Pack::de_fabrica().unwrap();
        for e in ["borderless", "extended", "classic", "fullArt"] {
            p.estilo(e).unwrap();
        }
        // Cada SVG, con sus variables de ejemplo, lo entiende usvg sin quejas.
        let vars = super::super::render::vars_de_ejemplo();
        for (ruta, svg) in &p.ficheros {
            if ruta.ends_with(".svg") {
                let s = rellenar(svg, &vars);
                resvg::usvg::Tree::from_str(&s, &resvg::usvg::Options::default())
                    .unwrap_or_else(|e| panic!("{ruta}: {e}"));
            }
        }
    }

    fn minimo() -> BTreeMap<String, String> {
        let p = Pack::de_fabrica().unwrap();
        p.ficheros
    }

    /// Cambia el `pack.json` como JSON (no como texto: el formato da igual).
    fn con_json(f: &mut BTreeMap<String, String>, cambio: impl FnOnce(&mut serde_json::Value)) {
        let mut v: serde_json::Value = serde_json::from_str(&f["pack.json"]).unwrap();
        cambio(&mut v);
        f.insert("pack.json".into(), serde_json::to_string(&v).unwrap());
    }

    #[test]
    fn cada_error_da_su_codigo() {
        let mut f = minimo();
        f.remove("despejado/corona.svg");
        assert_eq!(
            Pack::cargar(f).unwrap_err(),
            PackError::FaltaFichero("despejado/corona.svg".into())
        );

        let mut f = minimo();
        con_json(&mut f, |v| {
            v["estilos"]["borderless"]["huecos"]
                .as_object_mut()
                .unwrap()
                .remove("titulo");
        });
        assert_eq!(
            Pack::cargar(f).unwrap_err(),
            PackError::FaltaHueco {
                estilo: "borderless".into(),
                hueco: "titulo".into()
            }
        );

        let mut f = minimo();
        con_json(&mut f, |v| v["paletas"]["W"]["claro"] = "#zzzzzz".into());
        assert!(matches!(Pack::cargar(f).unwrap_err(), PackError::Color(_)));

        let mut f = minimo();
        con_json(&mut f, |v| v["version"] = 99.into());
        assert_eq!(Pack::cargar(f).unwrap_err(), PackError::Version(99));

        let mut f = minimo();
        f.insert("pack.json".into(), "{".into());
        assert!(matches!(Pack::cargar(f).unwrap_err(), PackError::Json(_)));
    }
}
