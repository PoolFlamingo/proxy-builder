//! **Packs de marcos**: cómo es cada estilo de carta. Un pack es un
//! `pack.json` y sus capas, SVG o PNG (marcos de imagen, uno por color). El de la casa (`builtin`) va dentro de la app
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
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Versión del formato que entiende esta app.
/// La 2 (19/10/2026) trae estilos propios, variantes, símbolos, etiquetas,
/// colores de texto, capas de fondo, escalado `pixel` y mezclas: una app
/// anterior rechaza un pack que los use en vez de pintarlo mal sin avisar.
/// La 3 añade a las capas PNG el trozo (`desde`) y la máscara (`mascara`), y
/// la condición de la caja de reglas elegida (`si.caja`).
pub const VERSION_PACK: u32 = 3;

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
    /// Alguno de estos tipos de carta (en inglés y minúsculas: `land`,
    /// `creature`, `planeswalker`, `artifact`, `enchantment`, `instant`,
    /// `sorcery`, `battle`, `token`…).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tipo: Option<Vec<String>>,
    /// Tierra básica (o no).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basica: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legendaria: Option<bool>,
    /// Cuántos colores tiene la carta, como rango `[min, max]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colores: Option<[u8; 2]>,
    /// Cuántos símbolos tiene el coste, como rango (`[3, 3]`: solo con tres).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simbolos: Option<[u8; 2]>,
    /// La variante del estilo que ha elegido el usuario (`variantes`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variante: Option<String>,
    /// Alguna de estas cajas de reglas elegidas en el diseño (`auto`,
    /// `dark`, `light`, `tint`): el pergamino del despejado Original solo
    /// con la automática o la clara, para que la oscura y la de tinte sigan
    /// dejando ver la ilustración.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caja: Option<Vec<String>>,
}

/// Lo que se sabe de la carta que se pinta, para las condiciones (`si`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Hechos {
    pub corona: bool,
    pub fr: bool,
    pub cinta: bool,
    pub emblema: bool,
    /// Los tipos (en inglés y minúsculas).
    pub tipos: Vec<String>,
    pub basica: bool,
    pub legendaria: bool,
    pub colores: u8,
    pub simbolos: u8,
    pub variante: Option<String>,
    /// La caja de reglas del diseño (`auto`, `dark`, `light` o `tint`).
    pub caja: String,
}

impl Condicion {
    /// ¿Se cumple con estos hechos? Lo que la condición no pide, vale.
    pub fn cumple(&self, h: &Hechos) -> bool {
        let igual = |pide: Option<bool>, hay: bool| pide.is_none_or(|p| p == hay);
        let rango = |pide: Option<[u8; 2]>, hay: u8| pide.is_none_or(|[a, b]| hay >= a && hay <= b);
        igual(self.corona, h.corona)
            && igual(self.fr, h.fr)
            && igual(self.cinta, h.cinta)
            && igual(self.emblema, h.emblema)
            && igual(self.basica, h.basica)
            && igual(self.legendaria, h.legendaria)
            && rango(self.colores, h.colores)
            && rango(self.simbolos, h.simbolos)
            && self.tipo.as_ref().is_none_or(|t| {
                t.iter()
                    .any(|x| h.tipos.iter().any(|y| y.eq_ignore_ascii_case(x)))
            })
            && self
                .variante
                .as_ref()
                .is_none_or(|v| h.variante.as_deref() == Some(v.as_str()))
            && self.caja.as_ref().is_none_or(|c| c.contains(&h.caja))
    }
}

/// Un rectángulo en mm.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Zona {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// Un rectángulo en mm cuyos lados pueden ser expresiones (`"yTipo-1.3"`):
/// la zona de una capa que se mueve con la caja de reglas que crece.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZonaV {
    pub x: Valor,
    pub y: Valor,
    pub w: Valor,
    pub h: Valor,
}

impl ZonaV {
    /// La zona en mm con las variables de la carta (las que falten, error).
    pub fn evaluar(&self, vars: &BTreeMap<&str, f32>) -> Result<Zona, PackError> {
        Ok(Zona {
            x: self.x.evaluar(vars)?,
            y: self.y.evaluar(vars)?,
            w: self.w.evaluar(vars)?,
            h: self.h.evaluar(vars)?,
        })
    }
}

/// Cómo se reescala una capa PNG a su medida en la carta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Escalado {
    /// Suave (Lanczos3): para marcos pintados o fotografiados.
    #[default]
    Suave,
    /// Para pixel art: al ampliar, cada píxel se repite (vecino más cercano);
    /// al reducir, la media de lo que cubre cada píxel. Nada de halos ni de
    /// bordes emborronados.
    Pixel,
}

/// Cómo se mezcla una capa con lo que hay debajo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mezcla {
    #[default]
    Normal,
    /// Se queda el más oscuro de los dos (el «Oscurecer» de Photoshop: las
    /// líneas de barrido de un monitor viejo).
    Oscurecer,
    /// Multiplicar.
    Multiplicar,
}

/// Una capa del marco: un SVG (con variables) o una imagen PNG.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capa {
    /// Ruta del SVG dentro del pack.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub svg: String,
    /// Ruta de un PNG dentro del pack. `{color}` se cambia por la letra del
    /// color del marco (ver `colores`): `m15/m15Frame{color}.png`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub png: Option<String>,
    /// Dónde va el PNG (estirado a esa zona); sin ella, la carta entera. Sus
    /// lados admiten las variables de la caja que crece (`yTipo`, `yReglas`,
    /// `altoReglas`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub en: Option<ZonaV>,
    /// Estirar solo por el medio: los mm de arriba y de abajo (a la escala
    /// de su ancho) se pintan sin deformar y lo de en medio se estira hasta
    /// el alto de la zona. Para las cajas de pixel art que crecen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tramos: Option<[f32; 2]>,
    /// Solo este trozo del PNG: un rectángulo en mm medido como si la imagen
    /// cubriera la carta entera. Sin `en`, va en ese mismo sitio de la
    /// carta; con ella, estirado a esa zona. Para sacar una pieza de otro
    /// marco (el pergamino de la caja de reglas del clásico en el despejado).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desde: Option<ZonaV>,
    /// Un PNG del pack de la medida de la carta cuya transparencia recorta
    /// la capa (las máscaras de Card Conjurer: solo la barra del tipo). Con
    /// `desde`, se recorta el mismo trozo de la máscara.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mascara: Option<String>,
    /// No es una imagen: aquí va el tinte de la caja de reglas (en la zona
    /// `en`) cuando el diseño pide uno (oscura, clara u otro tinte). Para los
    /// marcos de imagen, que traen la caja pintada; con ella, el texto de las
    /// reglas cambia de color para que se lea.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub caja_reglas: bool,
    /// Letras de color propias de esta capa (paleta → texto de `{color}`);
    /// lo que no diga, como el pack.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colores: BTreeMap<String, String>,
    #[serde(default)]
    pub si: Condicion,
    /// Cómo se reescala el PNG (si no lo dice, como el pack).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalado: Option<Escalado>,
    /// Cómo se mezcla con lo de debajo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mezcla: Option<Mezcla>,
    /// Opacidad del PNG (0–1; de partida, entera): el barrido de Balatro va
    /// al 5 %.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opacidad: Option<f32>,
}

impl Capa {
    /// La ruta del PNG para la paleta `clave` (`W`, `gold`…), si es de PNG.
    /// `{color1}` y `{color2}` son el primer y el segundo color de la carta
    /// (`colores`, en orden WUBRG; sin colores, la letra de incoloro; con uno
    /// solo, los dos el mismo): los cuadros de color junto al título.
    pub fn png_para(&self, pack: &PackDef, clave: &str, colores: &[String]) -> Option<String> {
        let plantilla = self.png.as_ref()?;
        let letra_de = |clave: &str| -> String {
            self.colores
                .get(clave)
                .or_else(|| pack.colores.get(clave))
                .cloned()
                .unwrap_or_else(|| letra_por_defecto(clave).to_string())
        };
        let primero = colores
            .first()
            .map_or_else(|| letra_de("colorless"), |c| letra_de(c));
        let segundo = colores
            .get(1)
            .map_or_else(|| primero.clone(), |c| letra_de(c));
        Some(
            plantilla
                .replace("{color}", &letra_de(clave))
                .replace("{color1}", &primero)
                .replace("{color2}", &segundo),
        )
    }
}

/// La letra de cada paleta si el pack no dice otra (la de Card Conjurer).
pub fn letra_por_defecto(clave: &str) -> &str {
    match clave {
        "gold" => "M",
        "artifact" => "A",
        "land" => "L",
        "colorless" => "C",
        otra => otra,
    }
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
    /// Todo en mayúsculas (los títulos de algunos marcos).
    #[serde(default)]
    pub mayusculas: bool,
    /// Dónde va el texto en vertical en una caja fija (las reglas): de
    /// partida, centrado.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alinear_v: Option<Alineacion>,
    /// En el hueco `etiqueta`, qué lleva: `rareza` (de partida),
    /// `supertipo` («Legendaria») o `subtipoTierra` («Montaña»).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contenido: Option<String>,
}

/// Una etiqueta de la línea de tipo (`etiquetas` del pack, por valor:
/// `common`, `uncommon`, `rare`, `mythic`, `special`, `bonus`, `legendary`,
/// `plains`…).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EtiquetaDef {
    /// El texto por idioma (`es`, `en`). Sin el del idioma, el inglés.
    #[serde(default)]
    pub texto: BTreeMap<String, String>,
    /// El fondo: un PNG del pack (estirado al hueco) o un color.
    pub fondo: String,
    /// El color del texto (si no, el del hueco).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Cómo se pintan las insignias de las secciones (lealtad, capítulos,
/// niveles) en un pack.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InsigniasDef {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fondo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub borde: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texto: Option<String>,
    /// Un PNG del pack en lugar de la forma de la insignia de lealtad.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub png_lealtad: Option<String>,
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
    /// Hasta dónde crece la caja (mm) antes de bajar un poco la letra (hasta
    /// el 75 %): así un texto largo no se come el arte. Sin él, la caja crece
    /// hasta `altoMax` con la letra entera.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alto_comodo: Option<f32>,
    /// La caja mide siempre `altoMax` mientras el usuario no pida la altura
    /// según el texto (ajuste fino `rulesBoxDynamic`). Sin esto, de serie
    /// se adapta al texto.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub fija_de_serie: bool,
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
    /// Otra ventana para el arte (la variante alta la acorta).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arte: Option<Hueco>,
    /// Otra caja de reglas que crece (la variante alta la deja crecer más).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caja: Option<CajaDinamica>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capas: Option<Vec<Capa>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fondo: Option<Vec<Capa>>,
    #[serde(default)]
    pub huecos: BTreeMap<String, Hueco>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstiloDef {
    /// Medida del lienzo en mm (`[ancho, alto]`); sin ella, 63×88. Los
    /// layouts apaisados (batalla, plano) llevan la suya.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tamano: Option<[f32; 2]>,
    /// Dónde va la ilustración.
    pub arte: Hueco,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caja: Option<CajaDinamica>,
    #[serde(default)]
    pub caja_por_defecto: CajaPorDefecto,
    pub capas: Vec<Capa>,
    /// Capas que van **debajo** de la ilustración (el fondo de la ventana
    /// del arte, que se ve a través de una ilustración con transparencia).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fondo: Vec<Capa>,
    pub huecos: BTreeMap<String, Hueco>,
    #[serde(default)]
    pub layouts: BTreeMap<String, LayoutDef>,
    /// Radio de las esquinas de la carta en pantalla (mm).
    #[serde(default = "radio_defecto")]
    pub radio: f32,
    /// ¿Las legendarias llevan corona sin pedirla? Sin decirlo, solo en el
    /// despejado (como el pack de la casa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corona_legendaria: Option<bool>,
    /// Cuánto (mm) se funde con negro el borde de abajo de la ilustración
    /// (el extendido de la casa, para que no acabe en seco). Los ajustes
    /// finos de la carta o de la colección lo cambian.
    #[serde(default, skip_serializing_if = "es_cero")]
    pub difuminado_arte: f32,
    /// Cuánto (mm) sigue la ilustración por debajo de su hueco, perdiéndose
    /// (la extensión del difuminado). Va con `difuminadoArte`: si los ajustes
    /// finos ponen un difuminado propio, la extensión es la suya (o ninguna).
    #[serde(default, skip_serializing_if = "es_cero")]
    pub extension_difuminado: f32,
    /// La ventana de la ilustración: fuera de ella, negro, con las esquinas
    /// redondeadas a su radio (o al de los ajustes finos). Se pinta encima
    /// de la ilustración y debajo de las capas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ventana_arte: Option<VentanaArte>,
    /// El nombre que se enseña de un estilo propio del pack, por idioma
    /// (`{"es": "Tarot", "en": "Tarot"}`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub nombre: BTreeMap<String, String>,
    /// En un estilo propio del pack (no uno de los cuatro de serie), de cuál
    /// de serie hereda el comportamiento (`borderless`, `extended`,
    /// `classic` o `fullArt`): lo que el motor decide por el estilo (esquinas
    /// redondeadas, símbolo del paisaje…) y a qué estilo cae si el pack no
    /// está. Sin decirlo, `classic`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    /// Variantes que elige el usuario (id → nombre por idioma): encienden las
    /// capas con `si.variante` y, si hay un layout con su mismo id, ese.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub variantes: BTreeMap<String, BTreeMap<String, String>>,
    /// Layouts que se eligen por la carta (la primera regla que se cumpla):
    /// `[{"si": {"tipo": ["land"], "basica": false}, "layout": "noBasica"}]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layout_si: Vec<ReglaLayout>,
}

/// Un layout del estilo que toca cuando la carta cumple una condición.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReglaLayout {
    pub si: Condicion,
    pub layout: String,
}

/// Los estilos de serie (los de `DesignStyle`).
pub const ESTILOS_DE_SERIE: &[&str] = &["borderless", "extended", "classic", "fullArt"];

/// La ventana de la ilustración (mm).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VentanaArte {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub radio: f32,
    /// Solo las esquinas de abajo (las de arriba son las de la carta).
    #[serde(default)]
    pub solo_abajo: bool,
    /// Zonas (mm: `[x, y, ancho, alto]`) donde la curva de la ventana viene
    /// dibujada en una capa de imagen: con un radio en los ajustes finos, ahí
    /// se vuelve a pintar la ilustración (o el negro) encima de las capas,
    /// para que la esquina quede con ese radio, mayor o menor.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repintar: Vec<[f32; 4]>,
    /// Tras cuántas capas se repinta (para no tapar las de encima, como la
    /// caja de F/R). Sin decirlo, tras todas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repintar_tras: Option<usize>,
}

fn radio_defecto() -> f32 {
    3.0
}

fn es_cero(v: &f32) -> bool {
    *v == 0.0
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
    /// Layouts completos que valen para cualquier estilo (saga, batalla,
    /// plano…): se usan cuando el estilo no trae el suyo en `layouts`.
    #[serde(default)]
    pub layouts: BTreeMap<String, EstiloDef>,
    /// Qué se pone en `{color}` de las capas PNG para cada paleta (`W`,
    /// `gold`…). Lo que no se diga: `W U B R G`, `M`, `A`, `L` y `C`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colores: BTreeMap<String, String>,
    /// De dónde salen las imágenes, para decirlo en la app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origen: Option<String>,
    /// Cómo se reescalan sus capas PNG si la capa no dice otra cosa
    /// (`pixel` para un pack de pixel art).
    #[serde(default)]
    pub escalado: Escalado,
    /// Símbolos propios (código sin llaves → ruta de un SVG o PNG del pack):
    /// tapan a los de Scryfall en el coste y en el texto (`"W"`, `"2"`,
    /// `"T"`…). Los que no trae, los de Scryfall.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub simbolos: BTreeMap<String, String>,
    /// Las etiquetas de la línea de tipo (hueco `etiqueta`), por valor.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub etiquetas: BTreeMap<String, EtiquetaDef>,
    /// Los colores (y la imagen de la de lealtad) de las insignias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insignias: Option<InsigniasDef>,
    /// Colores dentro de las reglas: lo marcado con `[[…]]` y las palabras de
    /// habilidad. Sin ellos, todo del color normal (y sin las marcas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colores_texto: Option<ColoresTexto>,
    /// Caracteres que su letra no tiene y por cuáles se cambian al escribir
    /// con este pack (`{"—": "-"}`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sustituir: BTreeMap<String, String>,
    /// El pie de fábrica de este pack por idioma (`es`, `en`), con las mismas
    /// claves que el de las colecciones (`{num}`, `{set}`…). Solo cuando la
    /// colección no pone uno propio.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub plantilla_pie: BTreeMap<String, String>,
    /// Separación entre los símbolos del coste, en proporción a su lado (de
    /// partida, 0,05).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separacion_coste: Option<f32>,
}

/// Los colores de texto de las reglas de un pack.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColoresTexto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resalte: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub habilidad: Option<String>,
}

/// Un pack cargado: la definición y sus ficheros.
#[derive(Debug, Clone)]
pub struct Pack {
    pub def: PackDef,
    pub ficheros: BTreeMap<String, String>,
    /// Los PNG de las capas de imagen, tal cual (se decodifican al usarlos).
    pub imagenes: BTreeMap<String, Arc<Vec<u8>>>,
    /// Huella de todo el pack (entra en la huella del render).
    pub huella: String,
    /// Sus símbolos propios, ya cargados (si trae).
    pub simbolos: Option<Arc<super::simbolos::SimbolosPropios>>,
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
        Pack::cargar_con_imagenes(ficheros, BTreeMap::new())
    }

    /// Como `cargar`, con los PNG de las capas de imagen.
    pub fn cargar_con_imagenes(
        ficheros: BTreeMap<String, String>,
        imagenes: BTreeMap<String, Arc<Vec<u8>>>,
    ) -> Result<Pack, PackError> {
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
        // Los fondos de las etiquetas y la imagen de la insignia de lealtad.
        let imagenes_pedidas = def
            .etiquetas
            .values()
            .map(|e| &e.fondo)
            .filter(|f| f.ends_with(".png"))
            .chain(def.insignias.iter().flat_map(|i| i.png_lealtad.iter()));
        for ruta in imagenes_pedidas {
            if !imagenes.contains_key(ruta) {
                return Err(PackError::FaltaFichero(ruta.clone()));
            }
        }
        for e in def.etiquetas.values() {
            for c in std::iter::once(&e.fondo)
                .filter(|f| !f.ends_with(".png"))
                .chain(e.color.iter())
            {
                if !color_valido(c) {
                    return Err(PackError::Color(c.clone()));
                }
            }
        }
        for (nombre, e) in &def.estilos {
            if let Some(b) = &e.base {
                if !ESTILOS_DE_SERIE.contains(&b.as_str()) {
                    return Err(PackError::Json(format!(
                        "el estilo {nombre} tiene una base que no existe: {b}"
                    )));
                }
            }
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
                .chain(e.fondo.iter())
                .chain(e.layouts.values().flat_map(|l| l.capas.iter().flatten()))
                .chain(e.layouts.values().flat_map(|l| l.fondo.iter().flatten()));
            for c in capas {
                comprobar_capa(c, &def, &ficheros, &imagenes)?;
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
        for (nombre, e) in &def.layouts {
            for c in &e.capas {
                comprobar_capa(c, &def, &ficheros, &imagenes)?;
            }
            for h in e.huecos.values() {
                if let Some(c) = &h.color {
                    if !c.starts_with("{{") && !color_valido(c) {
                        return Err(PackError::Color(format!("{nombre}: {c}")));
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
        for (k, v) in &imagenes {
            h.update(k.as_bytes());
            h.update([0]);
            h.update(v.as_slice());
            h.update([0]);
        }
        let huella = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        let simbolos = if def.simbolos.is_empty() {
            None
        } else {
            let mut s = super::simbolos::SimbolosPropios::nuevos(def.escalado == Escalado::Pixel);
            for (codigo, ruta) in &def.simbolos {
                let r = if ruta.ends_with(".svg") {
                    let svg = ficheros
                        .get(ruta)
                        .ok_or_else(|| PackError::FaltaFichero(ruta.clone()))?;
                    s.cargar_svg(codigo, svg.as_bytes())
                } else {
                    let png = imagenes
                        .get(ruta)
                        .ok_or_else(|| PackError::FaltaFichero(ruta.clone()))?;
                    s.cargar_png(codigo, png)
                };
                r.map_err(|e| PackError::Json(e.to_string()))?;
            }
            Some(Arc::new(s))
        };
        Ok(Pack {
            def,
            ficheros,
            imagenes,
            huella,
            simbolos,
        })
    }

    /// El pack de la casa, cargado una vez (para lo que un pack importado no
    /// trae: los layouts de la Fase 5 en packs de antes).
    pub fn de_fabrica_compartido() -> Option<&'static Pack> {
        static FABRICA: std::sync::LazyLock<Option<Pack>> =
            std::sync::LazyLock::new(|| Pack::de_fabrica().ok());
        FABRICA.as_ref()
    }

    /// El pack de la casa, dentro de la app.
    pub fn de_fabrica() -> Result<Pack, PackError> {
        let ficheros = BUILTIN
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Pack::cargar(ficheros)
    }

    /// Un layout completo del pack (`saga`, `batalla`…), si lo trae.
    pub fn layout(&self, nombre: &str) -> Option<&EstiloDef> {
        self.def.layouts.get(nombre)
    }

    pub fn estilo(&self, nombre: &str) -> Result<&EstiloDef, PackError> {
        self.def
            .estilos
            .get(nombre)
            .ok_or_else(|| PackError::FaltaEstilo(nombre.into()))
    }
}

/// Una capa es SVG o PNG, y lo que nombra está en el pack (los PNG, para
/// cada color que puede tocar).
fn comprobar_capa(
    c: &Capa,
    def: &PackDef,
    ficheros: &BTreeMap<String, String>,
    imagenes: &BTreeMap<String, Arc<Vec<u8>>>,
) -> Result<(), PackError> {
    if c.caja_reglas {
        return if c.png.is_none() && c.svg.is_empty() && c.en.is_some() {
            Ok(())
        } else {
            Err(PackError::Json(
                "la caja de reglas va sola y con su zona".into(),
            ))
        };
    }
    // El trozo y la máscara son de los PNG, y no se mezclan con los tramos.
    if (c.desde.is_some() || c.mascara.is_some()) && (c.png.is_none() || c.tramos.is_some()) {
        return Err(PackError::Json(
            "desde y mascara van en capas png sin tramos".into(),
        ));
    }
    if let Some(m) = &c.mascara {
        if !imagenes.contains_key(m) {
            return Err(PackError::FaltaFichero(m.clone()));
        }
    }
    match (&c.png, c.svg.is_empty()) {
        (None, false) => {
            if !ficheros.contains_key(&c.svg) {
                return Err(PackError::FaltaFichero(c.svg.clone()));
            }
        }
        (Some(_), true) => {
            // Cada paleta y, si la ruta lleva `{color1}`/`{color2}`, cada color.
            let colores: Vec<Vec<String>> = std::iter::once(Vec::new())
                .chain(["W", "U", "B", "R", "G"].map(|c| vec![c.to_string()]))
                .collect();
            for clave in PALETAS {
                for cs in &colores {
                    if let Some(ruta) = c.png_para(def, clave, cs) {
                        if !imagenes.contains_key(&ruta) {
                            return Err(PackError::FaltaFichero(ruta));
                        }
                    }
                }
            }
        }
        // Las dos o ninguna: no se sabe qué pintar.
        _ => return Err(PackError::Json("capa con svg y png, o sin ninguno".into())),
    }
    Ok(())
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
                if let Some(expr) = nombre.strip_prefix('=') {
                    // `{{= 56.25 - subeCaja}}`: la cuenta con las variables
                    // que son números (la caja de reglas a medida).
                    let nums: BTreeMap<&str, f32> = vars
                        .iter()
                        .filter_map(|(k, v)| Some((k.as_str(), v.parse::<f32>().ok()?)))
                        .collect();
                    if let Ok(v) = evaluar(expr, &nums) {
                        out.push_str(&format!("{v}"));
                    }
                } else if let Some(v) = vars.get(nombre) {
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
    fichero!("despejado/cinta.svg"),
    fichero!("despejado/reglas.svg"),
    fichero!("despejado/fr.svg"),
    fichero!("extendido/marco.svg"),
    fichero!("extendido/marco-alto.svg"),
    fichero!("extendido/cinta.svg"),
    fichero!("extendido/corona.svg"),
    fichero!("extendido/fr.svg"),
    fichero!("clasico/marco.svg"),
    fichero!("clasico/marco-alto.svg"),
    fichero!("clasico/cinta.svg"),
    fichero!("clasico/corona.svg"),
    fichero!("clasico/fr.svg"),
    fichero!("paisaje/franja.svg"),
    fichero!("layouts/saga.svg"),
    fichero!("layouts/mazmorra.svg"),
    fichero!("layouts/secuela-arriba.svg"),
    fichero!("layouts/secuela-abajo.svg"),
    fichero!("layouts/fusionar.svg"),
    fichero!("layouts/flip.svg"),
    fichero!("layouts/batalla.svg"),
    fichero!("layouts/plano.svg"),
    fichero!("layouts/plan.svg"),
    fichero!("layouts/vanguard.svg"),
];

/// El pack **«Original»**: los marcos de imagen de Card Conjurer. Aquí van
/// solo su definición y sus SVG; los PNG se descargan la primera vez que se
/// usan (la lista, con la huella de cada uno, en `ORIGINAL_IMAGENES`).
pub const ORIGINAL: &[(&str, &str)] = &[
    (
        "pack.json",
        include_str!("../../assets/marcos/original/pack.json"),
    ),
    (
        "cinta.svg",
        include_str!("../../assets/marcos/original/cinta.svg"),
    ),
    (
        "cubierta-corona.svg",
        include_str!("../../assets/marcos/original/cubierta-corona.svg"),
    ),
    (
        "sombra-pie.svg",
        include_str!("../../assets/marcos/original/sombra-pie.svg"),
    ),
    (
        "refuerzo-caja.svg",
        include_str!("../../assets/marcos/original/refuerzo-caja.svg"),
    ),
];

/// `{ commit, base, imagenes: { ruta: sha256 } }` del pack «Original».
pub const ORIGINAL_IMAGENES: &str = include_str!("../../assets/marcos/original/imagenes.json");

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn la_zona_de_una_capa_admite_expresiones() {
        let c: Capa = serde_json::from_str(
            r#"{"png": "caja.png", "en": {"x": 2, "y": "yTipo+1.5", "w": 59, "h": "80-yTipo"},
                "tramos": [1.6, 0.5]}"#,
        )
        .unwrap();
        let mut v = BTreeMap::new();
        v.insert("yTipo", 50.0);
        let z = c.en.as_ref().unwrap().evaluar(&v).unwrap();
        assert_eq!((z.x, z.y, z.w, z.h), (2.0, 51.5, 59.0, 30.0));
        assert_eq!(c.tramos, Some([1.6, 0.5]));
        // Sin la variable (caja fija), error: la capa no se pinta.
        assert!(c.en.unwrap().evaluar(&BTreeMap::new()).is_err());
    }

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
        assert_eq!(rellenar("{{= 5.5 - a}} {{= 2 + b}}", &v), "4.5 ");
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

    pub(crate) fn minimo() -> BTreeMap<String, String> {
        let p = Pack::de_fabrica().unwrap();
        p.ficheros
    }

    /// Cambia el `pack.json` como JSON (no como texto: el formato da igual).
    pub(crate) fn con_json(
        f: &mut BTreeMap<String, String>,
        cambio: impl FnOnce(&mut serde_json::Value),
    ) {
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

    /// Un PNG de 2×2 de un color.
    fn png(c: [u8; 4]) -> Arc<Vec<u8>> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba(c));
        Arc::new(crate::imaging::encode::png(&img).unwrap())
    }

    /// El pack de la casa con una capa PNG por color en el despejado.
    pub(crate) type Imagenes = BTreeMap<String, Arc<Vec<u8>>>;

    pub(crate) fn con_png() -> (BTreeMap<String, String>, Imagenes) {
        let mut f = minimo();
        con_json(&mut f, |v| {
            v["estilos"]["borderless"]["capas"] = serde_json::json!([
                { "png": "marco{color}.png" },
                { "png": "fr-{color}.png", "en": { "x": 40, "y": 70, "w": 10, "h": 5 },
                  "colores": { "colorless": "c", "land": "c" }, "si": { "fr": true } }
            ]);
        });
        let mut im = BTreeMap::new();
        for l in ["W", "U", "B", "R", "G", "M", "A", "L", "C"] {
            im.insert(format!("marco{l}.png"), png([10, 20, 30, 255]));
        }
        for l in ["W", "U", "B", "R", "G", "M", "A", "c"] {
            im.insert(format!("fr-{l}.png"), png([200, 0, 0, 255]));
        }
        (f, im)
    }

    #[test]
    fn condiciones_por_los_datos_de_la_carta() {
        let tierra = Hechos {
            tipos: vec!["land".into()],
            basica: false,
            colores: 2,
            simbolos: 0,
            ..Hechos::default()
        };
        let si: Condicion =
            serde_json::from_str(r#"{"tipo": ["land"], "basica": false, "colores": [2, 5]}"#)
                .unwrap();
        assert!(si.cumple(&tierra));
        assert!(!si.cumple(&Hechos {
            basica: true,
            ..tierra.clone()
        }));
        assert!(!si.cumple(&Hechos {
            tipos: vec!["creature".into()],
            ..tierra.clone()
        }));
        let tres: Condicion = serde_json::from_str(r#"{"simbolos": [3, 3]}"#).unwrap();
        assert!(!tres.cumple(&tierra));
        assert!(tres.cumple(&Hechos {
            simbolos: 3,
            ..Hechos::default()
        }));
        let caja: Condicion = serde_json::from_str(r#"{"caja": ["auto", "light"]}"#).unwrap();
        assert!(caja.cumple(&Hechos {
            caja: "light".into(),
            ..Hechos::default()
        }));
        assert!(!caja.cumple(&Hechos {
            caja: "tint".into(),
            ..Hechos::default()
        }));
        let variante: Condicion = serde_json::from_str(r#"{"variante": "sinDados"}"#).unwrap();
        assert!(!variante.cumple(&Hechos::default()));
        assert!(variante.cumple(&Hechos {
            variante: Some("sinDados".into()),
            ..Hechos::default()
        }));
        // Sin pedir nada, vale siempre.
        assert!(Condicion::default().cumple(&Hechos::default()));
    }

    #[test]
    fn el_trozo_y_la_mascara_van_en_capas_png_y_la_mascara_existe() {
        let prueba = |capa: serde_json::Value| {
            let (mut f, im) = con_png();
            con_json(&mut f, |v| {
                v["estilos"]["borderless"]["capas"] = serde_json::json!([capa]);
            });
            Pack::cargar_con_imagenes(f, im).map(|_| ())
        };
        let zona = serde_json::json!({ "x": 0, "y": 0, "w": 10, "h": 10 });
        assert!(prueba(serde_json::json!({ "png": "marco{color}.png", "desde": zona })).is_ok());
        assert!(prueba(serde_json::json!({ "svg": "despejado/fr.svg", "desde": zona })).is_err());
        assert!(prueba(
            serde_json::json!({ "png": "marco{color}.png", "desde": zona, "tramos": [1, 1] })
        )
        .is_err());
        assert_eq!(
            prueba(serde_json::json!({ "png": "marco{color}.png", "mascara": "no.png" })),
            Err(PackError::FaltaFichero("no.png".into()))
        );
        assert!(
            prueba(serde_json::json!({ "png": "marco{color}.png", "mascara": "frW.png" })).is_err()
        );
        assert!(
            prueba(serde_json::json!({ "png": "marco{color}.png", "mascara": "fr-W.png" })).is_ok()
        );
    }

    #[test]
    fn cuadros_de_color_con_color1_y_color2() {
        let (f, im) = con_png();
        let p = Pack::cargar_con_imagenes(f, im).unwrap();
        let capa = Capa {
            png: Some("cuadro{color1}-{color2}.png".into()),
            ..Capa::default()
        };
        let c = |v: &[&str]| -> Option<String> {
            let v: Vec<String> = v.iter().map(|s| s.to_string()).collect();
            capa.png_para(&p.def, "gold", &v)
        };
        assert_eq!(c(&["U", "R"]).as_deref(), Some("cuadroU-R.png"));
        assert_eq!(c(&["G"]).as_deref(), Some("cuadroG-G.png"));
        assert_eq!(c(&[]).as_deref(), Some("cuadroC-C.png"));
    }

    #[test]
    fn capas_png_por_color() {
        let (f, im) = con_png();
        let p = Pack::cargar_con_imagenes(f, im).unwrap();
        let capas = &p.estilo("borderless").unwrap().capas;
        assert_eq!(
            capas[0].png_para(&p.def, "gold", &[]).as_deref(),
            Some("marcoM.png")
        );
        assert_eq!(
            capas[0].png_para(&p.def, "U", &[]).as_deref(),
            Some("marcoU.png")
        );
        assert_eq!(
            capas[1].png_para(&p.def, "land", &[]).as_deref(),
            Some("fr-c.png")
        );
        assert_eq!(
            capas[1].png_para(&p.def, "artifact", &[]).as_deref(),
            Some("fr-A.png")
        );
    }

    #[test]
    fn falta_el_png_de_un_color() {
        let (f, mut im) = con_png();
        im.remove("marcoL.png");
        assert_eq!(
            Pack::cargar_con_imagenes(f, im).unwrap_err(),
            PackError::FaltaFichero("marcoL.png".into())
        );
    }

    #[test]
    fn una_capa_es_svg_o_png_pero_no_las_dos() {
        let (mut f, im) = con_png();
        con_json(&mut f, |v| {
            v["estilos"]["borderless"]["capas"][0]["svg"] = "despejado/titulo.svg".into();
        });
        assert!(matches!(
            Pack::cargar_con_imagenes(f, im.clone()).unwrap_err(),
            PackError::Json(_)
        ));
        let (mut f, _) = con_png();
        con_json(&mut f, |v| {
            v["estilos"]["borderless"]["capas"][0] = serde_json::json!({});
        });
        assert!(Pack::cargar_con_imagenes(f, im).is_err());
    }

    #[test]
    fn las_imagenes_entran_en_la_huella() {
        let (f, im) = con_png();
        let a = Pack::cargar_con_imagenes(f.clone(), im.clone()).unwrap();
        let mut im2 = im;
        im2.insert("marcoW.png".into(), png([1, 2, 3, 255]));
        let b = Pack::cargar_con_imagenes(f, im2).unwrap();
        assert_ne!(a.huella, b.huella);
    }

    #[test]
    fn el_pack_original_es_valido_con_sus_imagenes() {
        let ficheros: BTreeMap<String, String> = ORIGINAL
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        let lista: serde_json::Value = serde_json::from_str(ORIGINAL_IMAGENES).unwrap();
        let rutas: Vec<String> = lista["imagenes"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let im: BTreeMap<String, Arc<Vec<u8>>> = rutas
            .iter()
            .map(|r| (r.clone(), png([0, 0, 0, 255])))
            .collect();
        let p = Pack::cargar_con_imagenes(ficheros.clone(), im.clone()).unwrap();
        assert_eq!(p.def.id, "original");
        for e in ["borderless", "extended", "classic", "fullArt"] {
            p.estilo(e).unwrap();
        }
        // Las tierras básicas (paisaje) llevan el marco del pack, no el de la
        // casa: la barra de mármol, sin la franja oscura.
        let paisaje = p.estilo("fullArt").unwrap();
        assert!(paisaje.capas.iter().any(|c| c.png.is_some()));
        assert!(paisaje.capas.iter().all(|c| c.svg != "paisaje/franja.svg"));
        // La lista trae justo las que piden las capas: ni una de más ni de menos.
        let mut pedidas = std::collections::BTreeSet::new();
        for e in p.def.estilos.values() {
            for c in &e.capas {
                for k in PALETAS {
                    pedidas.extend(c.png_para(&p.def, k, &[]));
                }
                pedidas.extend(c.mascara.clone());
            }
        }
        let lista: std::collections::BTreeSet<String> = rutas.into_iter().collect();
        assert_eq!(pedidas, lista);
        // Sin imágenes no carga.
        assert!(Pack::cargar(ficheros).is_err());
    }
}
