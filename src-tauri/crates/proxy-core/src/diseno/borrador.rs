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
pub fn sustituir_nombre(texto: &str, originales: &[&str], propio: &str) -> String {
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
    let corto_propio = nombre_corto(propio).unwrap_or(propio);
    for o in &completos {
        if let Some(c) = nombre_corto(o) {
            t = cambiar_palabra(&t, c, &format!("{MARCA}1{MARCA}"));
        }
    }
    t.replace(&format!("{MARCA}0{MARCA}"), propio)
        .replace(&format!("{MARCA}1{MARCA}"), corto_propio)
}

/// Los nombres originales de una cara en un idioma: el de la cara, el de la
/// carta entera («Dusk // Dawn») y, en español, el impreso.
fn originales<'a>(card: &'a CardData, cara: &'a FaceData, lang: DesignLang) -> Vec<&'a str> {
    let mut v = vec![cara.name.as_str()];
    for f in &card.faces {
        v.push(f.name.as_str());
        if lang == DesignLang::Es {
            if let Some(n) = &f.printed_name_es {
                v.push(n);
            }
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
    let textos = d.faces.get(cara).map(|f| f.texts.get(lang));
    if let Some(manual) = textos.and_then(|t| t.rules_override.as_ref()) {
        return manual.clone();
    }
    let base = datos.rules.get(lang);
    let propio = textos.map(|t| t.name.as_str()).unwrap_or("");
    sustituir_nombre(base, &originales(&d.card, datos, lang), propio)
}

/// El nombre que se pinta: el temático, o el original si aún no hay.
pub fn nombre_efectivo(d: &CardDesign, cara: usize, lang: DesignLang) -> String {
    let propio = d
        .faces
        .get(cara)
        .map(|f| f.texts.get(lang).name.trim())
        .unwrap_or("");
    if !propio.is_empty() {
        return propio.to_string();
    }
    let Some(datos) = d.card.faces.get(cara) else {
        return String::new();
    };
    match lang {
        DesignLang::Es => datos.printed_name_es.clone().unwrap_or(datos.name.clone()),
        DesignLang::En => datos.name.clone(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::modelo::{ByLang, FaceTexts};

    #[test]
    fn progenitus_en_espanol_se_sustituye_dos_veces() {
        let t = "Protección contra todo.\nSi Progenitus fuera a ir a un cementerio desde cualquier lugar, revela a Progenitus y barájalo en la biblioteca de su propietario en su lugar.";
        let r = sustituir_nombre(t, &["Progenitus"], "Veldanava, Dragón de la Creación");
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
        );
        assert_eq!(r, "Whenever Milim attacks");
    }

    #[test]
    fn solo_palabras_enteras_y_sin_nombre_no_cambia_nada() {
        let r = sustituir_nombre("Ira de Ira", &["Ira"], "Furia");
        assert_eq!(r, "Furia de Furia");
        assert_eq!(sustituir_nombre("Iracundo", &["Ira"], "Furia"), "Iracundo");
        assert_eq!(sustituir_nombre("Ira", &["Ira"], "  "), "Ira");
    }

    #[test]
    fn dusk_dawn_cada_cara_con_su_texto() {
        let cara = |n: &str, es: &str, reglas: &str| FaceData {
            name: n.into(),
            printed_name_es: Some(es.into()),
            rules: ByLang {
                es: reglas.into(),
                en: String::new(),
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
