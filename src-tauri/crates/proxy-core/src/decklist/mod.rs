//! **Lector de listas de cartas.**
//!
//! Acepta lo que acepta fabricard: texto plano (`Sol Ring`, `2x Sol Ring`), el
//! formato de MTG Arena (`4 Lightning Bolt (M10) 146`), las exportaciones de
//! Moxfield (`*F*`, `*E*`, `*CMDR*`) y de Archidekt (`1x Sol Ring (cmm) 410
//! [Commander{top}] ^Tag^`), y las secciones habituales (Deck, Commander,
//! Sideboard, Companion, Maybeboard, `SB:`).
//!
//! No adivina nada: cada línea sale con su tipo y, si no se entiende, con el
//! motivo (`LineError`) para marcarla en pantalla.

mod exportar;

pub use exportar::{exportar, FormatoLista};

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Sección del deck a la que pertenece una carta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Section {
    #[default]
    Main,
    Commander,
    Sideboard,
    Companion,
    Maybeboard,
    Tokens,
}

impl Section {
    /// Lo que se imprime por defecto: todo menos la maybeboard.
    pub fn included_by_default(self) -> bool {
        self != Section::Maybeboard
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LineKind {
    Card,
    Section,
    Comment,
    Empty,
    Invalid,
}

/// Por qué no se entiende una línea (la pantalla tiene un texto para cada uno).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum LineError {
    /// `0 Sol Ring`.
    ZeroQuantity,
    /// `x3 Sol Ring`, `3xx Sol Ring`…
    BadQuantity,
    /// Más de 999 copias.
    TooMany,
    /// Solo hay cantidad o marcas, sin nombre.
    EmptyName,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParsedLine {
    /// Número de línea, empezando en 1.
    pub line_no: u32,
    pub raw: String,
    pub kind: LineKind,
    pub qty: u32,
    pub name: String,
    /// Código del set en minúsculas (`cmm`).
    pub set: Option<String>,
    pub collector: Option<String>,
    pub foil: bool,
    pub section: Section,
    /// `false` para la maybeboard y lo marcado `{noDeck}` en Archidekt.
    pub included: bool,
    pub error: Option<LineError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ParsedDecklist {
    pub lines: Vec<ParsedLine>,
    /// Suma de cantidades de las cartas incluidas.
    pub card_count: u32,
    pub invalid_count: u32,
}

const MAX_COPIAS: u32 = 999;

static RE_CANTIDAD: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^(\d+)\s*[xX]?\s+(\S.*)$").ok());
static RE_CANTIDAD_MAL: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^(?:[xX]\d+|\d+[xX]{2,}|\d+[xX]?)(?:\s|$)").ok());
static RE_SET: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"^(?P<name>.*?)\s+\((?P<set>[A-Za-z0-9]{2,8})\)(?:\s+(?P<num>\S+))?$").ok()
});
static RE_MARCAS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)\s*\*(?:F|E|CMDR|Foil|Etched)\*").ok());

fn seccion_de_cabecera(texto: &str) -> Option<Section> {
    let t = texto.trim().trim_end_matches(':').trim().to_lowercase();
    // «Commander (1)», «Sideboard (15)»: Moxfield y otros ponen el recuento.
    let t = match t.find('(') {
        Some(i) if t.ends_with(')') => t[..i].trim().to_string(),
        _ => t,
    };
    match t.as_str() {
        "deck" | "main" | "mainboard" | "main deck" | "maindeck" | "creatures" | "lands"
        | "spells" | "about" => Some(Section::Main),
        "commander" | "commanders" => Some(Section::Commander),
        "sideboard" | "side" | "sb" => Some(Section::Sideboard),
        "companion" | "companions" => Some(Section::Companion),
        "maybeboard" | "maybe" | "considering" => Some(Section::Maybeboard),
        "tokens" | "token" => Some(Section::Tokens),
        _ => None,
    }
}

/// Normaliza el nombre: comillas tipográficas, espacios repetidos.
fn limpiar_nombre(s: &str) -> String {
    let s = s.replace(['’', '‘', '`'], "'").replace(['“', '”'], "\"");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Quita y devuelve las categorías de Archidekt (`[Commander{top}]`) y las
/// etiquetas (`^Have,#37d67a^`) del final de la línea.
fn quitar_archidekt(s: &str) -> (String, Option<String>) {
    let mut resto = s.trim_end().to_string();
    // `^…^` al final.
    if resto.ends_with('^') {
        if let Some(i) = resto[..resto.len() - 1].rfind('^') {
            resto = resto[..i].trim_end().to_string();
        }
    }
    let mut categoria = None;
    if resto.ends_with(']') {
        if let Some(i) = resto.rfind('[') {
            categoria = Some(resto[i + 1..resto.len() - 1].to_string());
            resto = resto[..i].trim_end().to_string();
        }
    }
    (resto, categoria)
}

fn linea(line_no: u32, raw: &str, seccion: &mut Section, en_about: &mut bool) -> ParsedLine {
    let mut p = ParsedLine {
        line_no,
        raw: raw.to_string(),
        kind: LineKind::Empty,
        qty: 0,
        name: String::new(),
        set: None,
        collector: None,
        foil: false,
        section: *seccion,
        included: false,
        error: None,
    };
    let t = raw.trim();
    if t.is_empty() {
        *en_about = false;
        return p;
    }
    // Hay sitios que ponen la cabecera como comentario al copiar la lista como
    // texto («#SIDEBOARD», «// Sideboard (15)»): si lo de detrás es una
    // cabecera, cuenta como tal; si no, es un comentario.
    let comentario = t.starts_with("//") || t.starts_with('#');
    let cabecera = if comentario {
        t.trim_start_matches(['/', '#']).trim()
    } else {
        t
    };
    if let Some(s) = seccion_de_cabecera(cabecera) {
        *seccion = s;
        *en_about = cabecera.trim_end_matches(':').eq_ignore_ascii_case("about");
        p.kind = LineKind::Section;
        p.section = s;
        return p;
    }
    if comentario {
        p.kind = LineKind::Comment;
        return p;
    }
    // MTG Arena: «About» / «Name Mi mazo».
    if *en_about && t.len() > 5 && t[..5].eq_ignore_ascii_case("name ") {
        p.kind = LineKind::Comment;
        return p;
    }

    let mut cuerpo = t.to_string();
    let mut seccion_linea = *seccion;
    if cuerpo.len() >= 3 && cuerpo[..3].eq_ignore_ascii_case("sb:") {
        seccion_linea = Section::Sideboard;
        cuerpo = cuerpo[3..].trim_start().to_string();
    }
    p.section = seccion_linea;

    let (sin_archidekt, categoria) = quitar_archidekt(&cuerpo);
    cuerpo = sin_archidekt;
    let mut incluida = seccion_linea.included_by_default();
    if let Some(c) = &categoria {
        let c = c.to_lowercase();
        if c.contains("{nodeck}") || c.starts_with("maybeboard") {
            incluida = false;
            if c.starts_with("maybeboard") {
                p.section = Section::Maybeboard;
            }
        } else if c.starts_with("commander") {
            p.section = Section::Commander;
        } else if c.starts_with("sideboard") {
            p.section = Section::Sideboard;
        }
    }

    if let Some(re) = RE_MARCAS.as_ref() {
        if re.is_match(&cuerpo) {
            p.foil = true;
            cuerpo = re.replace_all(&cuerpo, "").trim().to_string();
        }
    }

    // Cantidad.
    let (qty, resto) = match RE_CANTIDAD.as_ref().and_then(|re| re.captures(&cuerpo)) {
        Some(c) => {
            let n: u64 = c[1].parse().unwrap_or(u64::MAX);
            (n, c[2].to_string())
        }
        None => {
            if RE_CANTIDAD_MAL
                .as_ref()
                .is_some_and(|re| re.is_match(&cuerpo))
            {
                p.kind = LineKind::Invalid;
                p.error = Some(
                    if cuerpo.split_whitespace().count() <= 1
                        && cuerpo
                            .chars()
                            .all(|c| c.is_ascii_digit() || c == 'x' || c == 'X')
                    {
                        LineError::EmptyName
                    } else {
                        LineError::BadQuantity
                    },
                );
                return p;
            }
            (1, cuerpo.clone())
        }
    };
    if qty == 0 {
        p.kind = LineKind::Invalid;
        p.error = Some(LineError::ZeroQuantity);
        return p;
    }
    if qty > u64::from(MAX_COPIAS) {
        p.kind = LineKind::Invalid;
        p.error = Some(LineError::TooMany);
        return p;
    }

    // Set y número.
    let (nombre, set, num) = match RE_SET.as_ref().and_then(|re| re.captures(&resto)) {
        Some(c) => (
            c["name"].to_string(),
            Some(c["set"].to_lowercase()),
            c.name("num").map(|m| m.as_str().to_string()),
        ),
        None => (resto.clone(), None, None),
    };
    let nombre = limpiar_nombre(&nombre);
    if nombre.is_empty() {
        p.kind = LineKind::Invalid;
        p.error = Some(LineError::EmptyName);
        return p;
    }

    p.kind = LineKind::Card;
    p.qty = qty as u32;
    p.name = nombre;
    p.set = set;
    p.collector = num;
    p.included = incluida;
    p
}

/// Lee una lista entera.
pub fn parse(texto: &str) -> ParsedDecklist {
    let mut seccion = Section::Main;
    let mut en_about = false;
    let lines: Vec<ParsedLine> = texto
        .lines()
        .enumerate()
        .map(|(i, l)| linea(i as u32 + 1, l, &mut seccion, &mut en_about))
        .collect();
    let card_count = lines
        .iter()
        .filter(|l| l.kind == LineKind::Card && l.included)
        .map(|l| l.qty)
        .sum();
    let invalid_count = lines.iter().filter(|l| l.kind == LineKind::Invalid).count() as u32;
    ParsedDecklist {
        lines,
        card_count,
        invalid_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (línea, cantidad, nombre, set, número)
    fn carta(l: &str) -> (u32, String, Option<String>, Option<String>) {
        let p = parse(l);
        let x = &p.lines[0];
        assert_eq!(x.kind, LineKind::Card, "«{l}» → {x:?}");
        (x.qty, x.name.clone(), x.set.clone(), x.collector.clone())
    }

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn cabeceras_escritas_como_comentario() {
        for cab in [
            "SIDEBOARD",
            "#SIDEBOARD",
            "# Sideboard:",
            "//Sideboard (15)",
            "// SIDEBOARD",
        ] {
            let p = parse(&format!("4 Island\n{cab}\n2 Duress"));
            assert_eq!(p.lines[1].kind, LineKind::Section, "«{cab}»");
            assert_eq!(p.lines[2].section, Section::Sideboard, "«{cab}»");
            assert_eq!(p.lines[0].section, Section::Main);
        }
        let p = parse("#Commander\n1 Atraxa, Praetors' Voice\n# mi nota\n// mi mazo\n1 Sol Ring");
        assert_eq!(p.lines[1].section, Section::Commander);
        assert_eq!(p.lines[2].kind, LineKind::Comment);
        assert_eq!(p.lines[3].kind, LineKind::Comment);
        // Los comentarios no cambian de sección.
        assert_eq!(p.lines[4].section, Section::Commander);
    }

    #[test]
    fn about_tras_almohadilla() {
        let p = parse("# About\nName Mi mazo\n\nDeck\n1 Island");
        assert_eq!(p.lines[0].kind, LineKind::Section);
        assert_eq!(p.lines[1].kind, LineKind::Comment);
        assert_eq!(p.lines[4].kind, LineKind::Card);
    }

    #[test]
    fn los_formatos_de_la_pantalla_de_fabricard() {
        assert_eq!(carta("Sol Ring"), (1, "Sol Ring".into(), None, None));
        assert_eq!(carta("2x Sol Ring"), (2, "Sol Ring".into(), None, None));
        assert_eq!(
            carta("1 Sol Ring (CMM) 410"),
            (1, "Sol Ring".into(), s("cmm"), s("410"))
        );
        assert_eq!(carta("Bird (teoc)"), (1, "Bird".into(), s("teoc"), None));
    }

    /// (línea, cantidad, nombre, set, número) esperados.
    type Caso<'a> = (&'a str, u32, &'a str, Option<&'a str>, Option<&'a str>);

    #[test]
    fn formatos_de_arena_moxfield_y_archidekt() {
        let casos: &[Caso] = &[
            (
                "4 Lightning Bolt (M10) 146",
                4,
                "Lightning Bolt",
                Some("m10"),
                Some("146"),
            ),
            (
                "1 Fire // Ice (MH2) 290 *F*",
                1,
                "Fire // Ice",
                Some("mh2"),
                Some("290"),
            ),
            (
                "1x Sol Ring (cmm) 410 [Commander{top}]",
                1,
                "Sol Ring",
                Some("cmm"),
                Some("410"),
            ),
            (
                "1x Sol Ring (cmm) 410 [Ramp] ^Have,#37d67a^",
                1,
                "Sol Ring",
                Some("cmm"),
                Some("410"),
            ),
            ("2X Counterspell", 2, "Counterspell", None, None),
            ("3 x Island", 3, "Island", None, None),
            (
                "10 Island (UNF) 236",
                10,
                "Island",
                Some("unf"),
                Some("236"),
            ),
            (
                "1 Delver of Secrets // Insectile Aberration (ISD) 51",
                1,
                "Delver of Secrets // Insectile Aberration",
                Some("isd"),
                Some("51"),
            ),
            (
                "1 Sol Ring (SLD) 2063★",
                1,
                "Sol Ring",
                Some("sld"),
                Some("2063★"),
            ),
            (
                "1 Llanowar Elves (PLST) DOM-168",
                1,
                "Llanowar Elves",
                Some("plst"),
                Some("DOM-168"),
            ),
            (
                "1 Atraxa, Praetors' Voice *CMDR*",
                1,
                "Atraxa, Praetors' Voice",
                None,
                None,
            ),
            ("1 Lim-Dûl's Vault", 1, "Lim-Dûl's Vault", None, None),
            ("1 Lim-Dûl’s Vault", 1, "Lim-Dûl's Vault", None, None),
            (
                "   2    Serra   Ascendant   ",
                2,
                "Serra Ascendant",
                None,
                None,
            ),
            ("1 The One Ring", 1, "The One Ring", None, None),
            (
                "1 Jace, the Mind Sculptor (A25) 62 *E*",
                1,
                "Jace, the Mind Sculptor",
                Some("a25"),
                Some("62"),
            ),
            (
                "1 Forest (THB) 254p",
                1,
                "Forest",
                Some("thb"),
                Some("254p"),
            ),
            (
                "1 Ancestral Recall (VMA) 1 *F*",
                1,
                "Ancestral Recall",
                Some("vma"),
                Some("1"),
            ),
            (
                "1 Kongming, \"Sleeping Dragon\"",
                1,
                "Kongming, \"Sleeping Dragon\"",
                None,
                None,
            ),
            (
                "1 Hymn to Tourach (FEM) 38a",
                1,
                "Hymn to Tourach",
                Some("fem"),
                Some("38a"),
            ),
            ("999 Relentless Rats", 999, "Relentless Rats", None, None),
            (
                "1 Who // What // When // Where // Why",
                1,
                "Who // What // When // Where // Why",
                None,
                None,
            ),
            (
                "1 Treasure (TCMM) 21",
                1,
                "Treasure",
                Some("tcmm"),
                Some("21"),
            ),
            ("1 Goblin (TFDN)", 1, "Goblin", Some("tfdn"), None),
            (
                "1 Sol Ring (cmm)  410",
                1,
                "Sol Ring",
                Some("cmm"),
                Some("410"),
            ),
            (
                "1 Phyrexian Arena (ONE) 104 *F*  ",
                1,
                "Phyrexian Arena",
                Some("one"),
                Some("104"),
            ),
        ];
        for (l, q, n, st, num) in casos {
            assert_eq!(
                carta(l),
                (
                    *q,
                    n.to_string(),
                    st.map(String::from),
                    num.map(String::from)
                ),
                "«{l}»"
            );
        }
    }

    #[test]
    fn la_marca_de_foil_se_anota() {
        let p = parse("1 Fire // Ice (MH2) 290 *F*");
        assert!(p.lines[0].foil);
        assert!(!parse("1 Sol Ring").lines[0].foil);
    }

    #[test]
    fn lineas_que_no_se_entienden() {
        let casos = [
            ("0 Sol Ring", LineError::ZeroQuantity),
            ("x3 Sol Ring", LineError::BadQuantity),
            ("3xx Sol Ring", LineError::BadQuantity),
            ("1000 Island", LineError::TooMany),
            ("4", LineError::EmptyName),
            ("4x", LineError::EmptyName),
        ];
        for (l, e) in casos {
            let p = parse(l);
            assert_eq!(p.lines[0].kind, LineKind::Invalid, "«{l}»");
            assert_eq!(p.lines[0].error, Some(e), "«{l}»");
        }
        assert_eq!(parse("0 Sol Ring\n1 Island").invalid_count, 1);
    }

    #[test]
    fn secciones_comentarios_y_vacias() {
        let p = parse(
            "// mi mazo\nCommander\n1 Atraxa, Praetors' Voice\n\nDeck\n4 Island\nSideboard\n2 Duress\nMaybeboard\n1 Sol Ring\n# nota",
        );
        let k: Vec<LineKind> = p.lines.iter().map(|l| l.kind).collect();
        let (c, s, k_, e) = (
            LineKind::Comment,
            LineKind::Section,
            LineKind::Card,
            LineKind::Empty,
        );
        assert_eq!(k, vec![c, s, k_, e, s, k_, s, k_, s, k_, c]);
        assert_eq!(p.lines[2].section, Section::Commander);
        assert_eq!(p.lines[7].section, Section::Sideboard);
        assert!(p.lines[7].included);
        assert_eq!(p.lines[9].section, Section::Maybeboard);
        assert!(!p.lines[9].included);
        assert_eq!(p.card_count, 1 + 4 + 2);
    }

    #[test]
    fn cabeceras_con_recuento_y_dos_puntos() {
        let p = parse("Commander (1)\n1 Atraxa\nSideboard:\n1 Duress");
        assert_eq!(p.lines[0].kind, LineKind::Section);
        assert_eq!(p.lines[1].section, Section::Commander);
        assert_eq!(p.lines[3].section, Section::Sideboard);
    }

    #[test]
    fn prefijo_sb_de_arena_antiguo() {
        let p = parse("SB: 2 Duress\n4 Island");
        assert_eq!(p.lines[0].section, Section::Sideboard);
        assert_eq!(p.lines[0].qty, 2);
        assert_eq!(p.lines[1].section, Section::Main);
    }

    #[test]
    fn about_de_arena_no_cuenta_como_carta() {
        let p = parse("About\nName Mono Red\n\nDeck\n20 Mountain");
        assert_eq!(p.lines[1].kind, LineKind::Comment);
        assert_eq!(p.card_count, 20);
    }

    #[test]
    fn archidekt_maybeboard_y_nodeck_no_se_incluyen() {
        let p = parse("1x Sol Ring (cmm) 410 [Maybeboard{noDeck}{noPrice}]\n1x Island [Land{noDeck}]\n1x Atraxa [Commander{top}]");
        assert!(!p.lines[0].included);
        assert_eq!(p.lines[0].section, Section::Maybeboard);
        assert!(!p.lines[1].included);
        assert_eq!(p.lines[2].section, Section::Commander);
        assert_eq!(p.card_count, 1);
    }

    #[test]
    fn los_numeros_de_linea_empiezan_en_uno_y_se_conserva_el_texto() {
        let p = parse("Sol Ring\r\n\r\n2 Island");
        assert_eq!(p.lines[0].line_no, 1);
        assert_eq!(p.lines[2].line_no, 3);
        assert_eq!(p.lines[2].raw, "2 Island");
    }
}
