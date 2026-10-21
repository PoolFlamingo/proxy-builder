//! **Del dato de Scryfall al texto de la carta**: la sustitución del nombre
//! original por el temático en las reglas, el texto efectivo de cada cara y
//! la numeración automática del pie. Todo puro.

use super::modelo::{CardData, CardDesign, DesignLang, FaceData};

/// La forma corta de un nombre: lo que va antes de la coma («Arcades» en
/// «Arcades, the Strategist»). `None` si no tiene coma o queda muy corta.
pub fn nombre_corto(nombre: &str) -> Option<&str> {
    let (corto, _) = nombre.split_once(',')?;
    let corto = corto.trim();
    (corto.chars().count() >= 3).then_some(corto)
}

fn es_letra(c: char) -> bool {
    c.is_alphanumeric() || c == '\'' || c == '’'
}

/// Cambia `buscado` por `nuevo` solo como palabra entera (no dentro de otra).
fn cambiar_palabra(texto: &str, buscado: &str, nuevo: &str) -> String {
    if buscado.is_empty() {
        return texto.to_string();
    }
    let mut out = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(i) = resto.find(buscado) {
        let antes = resto[..i].chars().next_back();
        let despues = resto[i + buscado.len()..].chars().next();
        let entera = !antes.is_some_and(es_letra) && !despues.is_some_and(es_letra);
        out.push_str(&resto[..i]);
        out.push_str(if entera { nuevo } else { buscado });
        resto = &resto[i + buscado.len()..];
    }
    out.push_str(resto);
    out
}

/// Sustituye en `texto` los nombres originales por el temático: primero los
/// completos y después sus formas cortas (con la forma corta del temático,
/// si la tiene). Es lo que hace Pablo: «Si Veldanava, Dragón de la Creación
/// fuera a ir a un cementerio…».
pub fn sustituir_nombre(texto: &str, originales: &[&str], propio: &str, espanol: bool) -> String {
    let propio = propio.trim();
    if propio.is_empty() {
        return texto.to_string();
    }
    // Los más largos primero, para no partir «Arcades, the Strategist» al
    // cambiar «Arcades».
    let mut completos: Vec<&str> = originales
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    completos.sort_by_key(|s| std::cmp::Reverse(s.len()));
    completos.dedup();
    // Marcadores intermedios para que un nombre ya puesto no se vuelva a tocar.
    const MARCA: char = '\u{E000}';
    let mut t = texto.to_string();
    for o in &completos {
        t = cambiar_palabra(&t, o, &format!("{MARCA}0{MARCA}"));
    }
    if espanol {
        t = quitar_articulos(&t, &format!("{MARCA}0{MARCA}"));
    }
    let corto_propio = nombre_corto(propio).unwrap_or(propio);
    for o in &completos {
        if let Some(c) = nombre_corto(o) {
            t = cambiar_palabra(&t, c, &format!("{MARCA}1{MARCA}"));
        }
    }
    t.replace(&format!("{MARCA}0{MARCA}"), propio)
        .replace(&format!("{MARCA}1{MARCA}"), corto_propio)
}

/// En español, el texto impreso pone artículo delante de los nombres que no
/// son de persona («sacrificar la Llave de plata lunar»). Con un nombre
/// temático (casi siempre de personaje) sobra: «del» pasa a «de», «al» a
/// «a» y «el/la/los/las» se quitan. Como hace Pablo: «sacrificar a Zegion».
fn quitar_articulos(texto: &str, marca: &str) -> String {
    let mut out = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(i) = resto.find(marca) {
        let antes = &resto[..i];
        let recortado = antes.trim_end_matches(' ');
        let palabra_ini = recortado
            .rfind(|c: char| !c.is_alphabetic())
            .map_or(0, |p| {
                p + recortado[p..].chars().next().map_or(1, char::len_utf8)
            });
        let palabra = &recortado[palabra_ini..];
        let hay_espacio = recortado.len() < antes.len();
        let cambio = if hay_espacio {
            match palabra.to_lowercase().as_str() {
                "del" => Some("de "),
                "al" => Some("a "),
                "el" | "la" | "los" | "las" => Some(""),
                _ => None,
            }
        } else {
            None
        };
        match cambio {
            Some(nuevo) => {
                out.push_str(&recortado[..palabra_ini]);
                // Mayúscula si el artículo empezaba frase.
                let mayus = palabra.chars().next().is_some_and(char::is_uppercase);
                if mayus && !nuevo.is_empty() {
                    let mut c = nuevo.chars();
                    if let Some(p) = c.next() {
                        out.extend(p.to_uppercase());
                        out.push_str(c.as_str());
                    }
                } else {
                    out.push_str(nuevo);
                }
            }
            None => out.push_str(antes),
        }
        out.push_str(marca);
        resto = &resto[i + marca.len()..];
    }
    out.push_str(resto);
    out
}

/// Los nombres originales de una cara en un idioma: el de la cara, el de la
/// carta entera («Dusk // Dawn») y el impreso en ese idioma.
fn originales<'a>(card: &'a CardData, cara: &'a FaceData, lang: DesignLang) -> Vec<&'a str> {
    let mut v = vec![cara.name.as_str()];
    for f in &card.faces {
        v.push(f.name.as_str());
        if let Some(n) = f.printed_name(lang) {
            v.push(n);
        }
    }
    v
}

/// El texto de reglas que se pinta: el escrito a mano si lo hay; si no, el
/// de Scryfall con el nombre temático sustituido.
pub fn reglas_efectivas(d: &CardDesign, cara: usize, lang: DesignLang) -> String {
    let Some(datos) = d.card.faces.get(cara) else {
        return String::new();
    };
    let textos = d.faces.get(cara).map(|f| f.textos(lang));
    // Las reglas a mano mandan, pero el nombre original se cambia igual por
    // el propio (la traducción sugerida lo deja con el original).
    let base = textos
        .and_then(|t| t.rules_override.as_ref())
        .unwrap_or_else(|| datos.rules.get(lang));
    let propio = textos.map(|t| t.name.as_str()).unwrap_or("");
    sustituir_nombre(
        base,
        &originales(&d.card, datos, lang),
        propio,
        lang == DesignLang::Es,
    )
}

/// El nombre que se pinta: el temático, o el original si aún no hay.
pub fn nombre_efectivo(d: &CardDesign, cara: usize, lang: DesignLang) -> String {
    let propio = d
        .faces
        .get(cara)
        .map(|f| f.textos(lang).name.trim())
        .unwrap_or("");
    if !propio.is_empty() {
        return propio.to_string();
    }
    let Some(datos) = d.card.faces.get(cara) else {
        return String::new();
    };
    datos
        .printed_name(lang)
        .map_or_else(|| datos.name.clone(), str::to_string)
}

/// El nombre original que va en la cinta o en el pie: el inglés de la carta
/// entera («Arcades, the Strategist», «Dusk // Dawn»).
pub fn nombre_original(card: &CardData) -> String {
    card.faces
        .iter()
        .map(|f| f.name.as_str())
        .collect::<Vec<_>>()
        .join(" // ")
}

/// El siguiente número libre del pie: `0001…` para cartas y `T001…` para
/// fichas, por encima del mayor que ya haya en el deck.
pub fn siguiente_numero<'a>(usados: impl IntoIterator<Item = &'a str>, ficha: bool) -> String {
    let max = usados
        .into_iter()
        .filter_map(|n| {
            let n = n.trim();
            let (es_ficha, cifras) = match n.strip_prefix('T') {
                Some(r) => (true, r),
                None => (false, n),
            };
            (es_ficha == ficha)
                .then(|| cifras.parse::<u32>().ok())
                .flatten()
        })
        .max()
        .unwrap_or(0);
    numero(max + 1, ficha)
}

/// Un número del pie con su formato.
pub fn numero(n: u32, ficha: bool) -> String {
    if ficha {
        format!("T{n:03}")
    } else {
        format!("{n:04}")
    }
}

/// (tipo en inglés, español, ¿femenino?)
const TIPOS: [(&str, &str, bool); 11] = [
    ("Land", "Tierra", true),
    ("Creature", "Criatura", true),
    ("Artifact", "Artefacto", false),
    ("Enchantment", "Encantamiento", false),
    ("Instant", "Instantáneo", false),
    ("Sorcery", "Conjuro", false),
    ("Planeswalker", "Planeswalker", false),
    ("Battle", "Batalla", true),
    ("Plane", "Plano", false),
    ("Phenomenon", "Fenómeno", false),
    ("Scheme", "Maquinación", true),
];

/// Los subtipos que se saben sin red (los que Scryfall no suele dejar en
/// blanco no hacen falta: los de criatura son demasiados).
const SUBTIPOS: [(&str, &str); 19] = [
    ("Plains", "Llanura"),
    ("Island", "Isla"),
    ("Swamp", "Pantano"),
    ("Mountain", "Montaña"),
    ("Forest", "Bosque"),
    ("Equipment", "Equipo"),
    ("Vehicle", "Vehículo"),
    ("Treasure", "Tesoro"),
    ("Clue", "Pista"),
    ("Food", "Comida"),
    ("Blood", "Sangre"),
    ("Gold", "Oro"),
    ("Aura", "Aura"),
    ("Saga", "Saga"),
    ("Class", "Clase"),
    ("Curse", "Maldición"),
    ("Shrine", "Santuario"),
    ("Lesson", "Lección"),
    ("Trap", "Trampa"),
];

/// La línea de tipo en español de una en inglés, sin red: un solo tipo de
/// carta (con «Basic» o «Legendary») y subtipos conocidos. `None` si algo no
/// se sabe (criaturas, fichas, tipos mezclados): ahí manda lo que imprimió
/// Scryfall. Evita que salga «Land» en una carta en español.
pub fn tipo_en_espanol(en: &str) -> Option<String> {
    let (tipos, subtipos) = match en.split_once('—') {
        Some((a, b)) => (a.trim(), Some(b.trim())),
        None => (en.trim(), None),
    };
    let mut basica = false;
    let mut legendaria = false;
    let mut tipo = None;
    for p in tipos.split_whitespace() {
        match p {
            "Basic" => basica = true,
            "Legendary" => legendaria = true,
            _ => {
                let t = TIPOS.iter().find(|t| t.0 == p)?;
                if tipo.replace(t).is_some() {
                    return None;
                }
            }
        }
    }
    let (_, nombre, femenino) = tipo?;
    if basica && *nombre != "Tierra" {
        return None;
    }
    let mut r = nombre.to_string();
    if basica {
        r.push_str(" básica");
    }
    if legendaria {
        r.push_str(if *femenino {
            " legendaria"
        } else {
            " legendario"
        });
    }
    if let Some(sub) = subtipos {
        let palabras: Vec<&str> = sub.split_whitespace().collect();
        if palabras.is_empty() {
            return None;
        }
        let es: Option<Vec<&str>> = palabras
            .iter()
            .map(|p| SUBTIPOS.iter().find(|s| s.0 == *p).map(|s| s.1))
            .collect();
        r.push_str(" — ");
        r.push_str(&es?.join(" "));
    }
    Some(r)
}

/// La línea de tipo en `lang` de una en inglés, sin red (`None` si no se
/// sabe, y en inglés). El español con su gramática; los demás, con las
/// frases de `tipos_idiomas`.
pub fn tipo_traducido(en: &str, lang: DesignLang) -> Option<String> {
    match lang {
        DesignLang::Es => tipo_en_espanol(en),
        DesignLang::En => None,
        l => super::tipos_idiomas::tipo_en(en, l),
    }
}

/// Rellena el tipo en `lang` de las caras que lo traen en inglés (diseños
/// guardados antes de saber traducirlo). Devuelve si cambió algo.
pub fn arreglar_tipos(card: &mut CardData, lang: DesignLang) -> bool {
    let mut cambio = false;
    for f in &mut card.faces {
        if !f.type_line.tiene(lang) {
            continue;
        }
        let actual = f.type_line.get(lang);
        if actual.is_empty() || *actual != f.type_line.en {
            continue;
        }
        if let Some(t) = tipo_traducido(&f.type_line.en, lang) {
            *f.type_line.get_mut(lang) = t;
            cambio = true;
        }
    }
    cambio
}

/// `arreglar_tipos` en español.
pub fn arreglar_tipos_es(card: &mut CardData) -> bool {
    arreglar_tipos(card, DesignLang::Es)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::modelo::{ByLang, FaceTexts};

    #[test]
    fn tipos_sin_red() {
        let t = |s| tipo_en_espanol(s).unwrap();
        assert_eq!(t("Land"), "Tierra");
        assert_eq!(t("Legendary Land"), "Tierra legendaria");
        assert_eq!(t("Basic Land — Island"), "Tierra básica — Isla");
        assert_eq!(
            t("Legendary Artifact — Equipment"),
            "Artefacto legendario — Equipo"
        );
        assert_eq!(t("Instant"), "Instantáneo");
        assert_eq!(tipo_en_espanol("Creature — Elf"), None);
        assert_eq!(tipo_en_espanol("Artifact Creature"), None);
        assert_eq!(tipo_en_espanol("Token Creature — Bird"), None);
    }

    #[test]
    fn arregla_los_tipos_guardados_en_ingles() {
        let mut c = crate::diseno::modelo::tests::diseno_de_prueba().card;
        c.faces[0].type_line = ByLang {
            es: "Land".into(),
            en: "Land".into(),
            otros: Default::default(),
        };
        assert!(arreglar_tipos_es(&mut c));
        assert_eq!(c.faces[0].type_line.es, "Tierra");
        assert!(!arreglar_tipos_es(&mut c));
    }

    #[test]
    fn progenitus_en_espanol_se_sustituye_dos_veces() {
        let t = "Protección contra todo.\nSi Progenitus fuera a ir a un cementerio desde cualquier lugar, revela a Progenitus y barájalo en la biblioteca de su propietario en su lugar.";
        let r = sustituir_nombre(t, &["Progenitus"], "Veldanava, Dragón de la Creación", true);
        assert_eq!(r.matches("Veldanava, Dragón de la Creación").count(), 2);
        assert!(!r.contains("Progenitus"));
    }

    #[test]
    fn arcades_suelto_va_con_la_forma_corta() {
        let t = "Arcades, the Strategist enters. Whenever Arcades attacks, draw.";
        let r = sustituir_nombre(
            t,
            &["Arcades, the Strategist"],
            "Milim, Señora Demonio Dragonoide",
            false,
        );
        assert_eq!(
            r,
            "Milim, Señora Demonio Dragonoide enters. Whenever Milim attacks, draw."
        );
        // Sin coma en el propio, la forma corta es el nombre entero.
        let r = sustituir_nombre(
            "Whenever Arcades attacks",
            &["Arcades, the Strategist"],
            "Milim",
            false,
        );
        assert_eq!(r, "Whenever Milim attacks");
    }

    #[test]
    fn solo_palabras_enteras_y_sin_nombre_no_cambia_nada() {
        let r = sustituir_nombre("Ira de Ira", &["Ira"], "Furia", false);
        assert_eq!(r, "Furia de Furia");
        assert_eq!(
            sustituir_nombre("Iracundo", &["Ira"], "Furia", false),
            "Iracundo"
        );
        assert_eq!(sustituir_nombre("Ira", &["Ira"], "  ", false), "Ira");
    }

    #[test]
    fn en_espanol_sobran_los_articulos() {
        let n = ["Llave de plata lunar"];
        let z = "Zegion, General Insecto";
        assert_eq!(
            sustituir_nombre(
                "{1}, {T}, sacrificar la Llave de plata lunar: Busca.",
                &n,
                z,
                true
            ),
            "{1}, {T}, sacrificar Zegion, General Insecto: Busca."
        );
        assert_eq!(
            sustituir_nombre("La Llave de plata lunar entra. Gira del Llave de plata lunar al Llave de plata lunar.", &n, "Zegion", true),
            "Zegion entra. Gira de Zegion a Zegion."
        );
        // «dela» no es un artículo; y en inglés no se toca nada.
        assert_eq!(
            sustituir_nombre("sacrificar the Llave de plata lunar", &n, "Zegion", false),
            "sacrificar the Zegion"
        );
    }

    #[test]
    fn dusk_dawn_cada_cara_con_su_texto() {
        let cara = |n: &str, es: &str, reglas: &str| FaceData {
            name: n.into(),
            printed_name_es: Some(es.into()),
            rules: ByLang {
                es: reglas.into(),
                en: String::new(),
                otros: Default::default(),
            },
            ..FaceData::default()
        };
        let mut d = crate::diseno::modelo::tests::diseno_de_prueba();
        d.card.faces = vec![
            cara("Dusk", "Ocaso", "Destruye todas las criaturas con fuerza de 3 o más."),
            cara(
                "Dawn",
                "Alba",
                "Secuela. (Lanza este hechizo solo desde tu cementerio. Luego exílialo.)\nRegresa a Alba a tu mano.",
            ),
        ];
        d.faces[1].texts.es = FaceTexts {
            name: "Amanecer de Ciel".into(),
            ..FaceTexts::default()
        };
        let r = reglas_efectivas(&d, 1, DesignLang::Es);
        assert!(r.starts_with("Secuela."));
        assert!(r.contains("Regresa a Amanecer de Ciel a tu mano."));
        assert_eq!(nombre_original(&d.card), "Dusk // Dawn");
        // Las reglas a mano mandan.
        d.faces[1].texts.es.rules_override = Some("A mano.".into());
        assert_eq!(reglas_efectivas(&d, 1, DesignLang::Es), "A mano.");
        // ...pero el nombre original también se cambia en ellas.
        d.faces[1].texts.es.rules_override = Some("Regresa a Alba a tu mano.".into());
        assert_eq!(
            reglas_efectivas(&d, 1, DesignLang::Es),
            "Regresa a Amanecer de Ciel a tu mano."
        );
        // Sin nombre temático, el impreso.
        d.faces[0].texts.es.name.clear();
        assert_eq!(nombre_efectivo(&d, 0, DesignLang::Es), "Ocaso");
        assert_eq!(nombre_efectivo(&d, 0, DesignLang::En), "Dusk");
    }

    #[test]
    fn numeracion_del_pie() {
        assert_eq!(siguiente_numero([], false), "0001");
        assert_eq!(siguiente_numero(["0001", "0003", "T001"], false), "0004");
        assert_eq!(siguiente_numero(["0001", "T001", "T002"], true), "T003");
        assert_eq!(siguiente_numero(["raro", "0002"], false), "0003");
    }
}
