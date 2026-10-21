//! **El diseño efectivo**: el de la carta con la colección aplicada y los
//! valores de fábrica donde ninguno de los dos dice nada. Es lo que pinta el
//! motor y lo que entra en la huella de un render (así, cambiar la colección
//! cambia la huella y nunca se sirve un render viejo).
//!
//! Orden: la carta pisa a la colección y la colección pisa a la fábrica.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::modelo::*;
use crate::model::FaceImage;

/// Plantilla del pie de fábrica (dos líneas): la de las cartas de Pablo.
pub const PIE_DEFECTO: &str = "{num}\t{rarity}\t{original}\n{set} • {lang}\t{brush} {artist}";

/// El pie ya resuelto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PieEfectivo {
    pub number: String,
    /// La letra de rareza («S», «C», «M»…).
    pub rarity: String,
    pub set_code: String,
    pub artist: String,
    pub credit: String,
    pub universe: String,
    pub year: String,
    pub template: ByLang<String>,
}

/// Grosores pedidos por la colección (`None` = el del pack).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Pesos {
    pub titulo: Option<f64>,
    pub tipo: Option<f64>,
    pub reglas: Option<f64>,
}

/// Lo que pinta el motor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisenoEfectivo {
    pub design: CardDesign,
    pub frame: FrameOptions,
    pub emblem: Option<FaceImage>,
    /// Nunca `Auto`.
    pub original_name: OriginalName,
    pub pie: PieEfectivo,
    pub font_title: Option<String>,
    pub font_rules: Option<String>,
    pub squeeze_min: f64,
    /// Estrechado que llevan siempre el título y el tipo (1 = ninguno).
    pub squeeze_base: f64,
    pub pesos: Pesos,
    pub retoques: FineTuning,
    /// La colección pedida no existe (se borró): se usa la fábrica y se avisa.
    pub coleccion_perdida: bool,
}

/// La letra de rareza de Scryfall.
pub fn letra_rareza(rareza: &str) -> &'static str {
    match rareza {
        "common" => "C",
        "uncommon" => "U",
        "rare" => "R",
        "mythic" => "M",
        "bonus" => "B",
        _ => "S",
    }
}

fn o(a: &Option<String>, b: Option<&Option<String>>, defecto: &str) -> String {
    a.clone()
        .or_else(|| b.and_then(|x| x.clone()))
        .unwrap_or_else(|| defecto.to_string())
}

/// Aplica la colección (si la hay y existe) y la fábrica.
pub fn efectivo(d: &CardDesign, coleccion: Option<&Collection>) -> DisenoEfectivo {
    let c = coleccion.filter(|c| d.collection.as_deref() == Some(c.id.as_str()));
    let perdida = d.collection.is_some() && c.is_none();
    let frame = d
        .frame
        .clone()
        .or_else(|| c.map(|c| c.frame.clone()))
        .unwrap_or_default();
    let emblem = match &d.emblem {
        EmblemChoice::None => None,
        EmblemChoice::Own { image } => Some(image.clone()),
        EmblemChoice::Inherit => c.and_then(|c| c.emblem.clone()),
    };
    let original_name = match d.options.original_name {
        OriginalName::Auto => match c.map(|c| c.original_name) {
            Some(OriginalName::Auto) | None => OriginalName::Footer,
            Some(x) => x,
        },
        x => x,
    };
    let f = &d.footer;
    let pie = PieEfectivo {
        number: f.number.clone(),
        rarity: f
            .rarity
            .clone()
            .or_else(|| c.and_then(|c| c.rarity_letter.clone()))
            .unwrap_or_else(|| letra_rareza(&d.card.rarity).to_string()),
        set_code: f
            .set_code
            .clone()
            .or_else(|| c.map(|c| c.set_code.clone()).filter(|s| !s.is_empty()))
            .unwrap_or_else(|| d.card.set.to_uppercase()),
        artist: o(
            &f.artist,
            c.map(|c| &c.artist),
            d.card.artist.as_deref().unwrap_or(""),
        ),
        credit: o(&f.credit, c.map(|c| &c.credit), ""),
        universe: o(&f.universe, c.map(|c| &c.universe), ""),
        year: o(&f.year, c.map(|c| &c.year), ""),
        // Cada idioma por su lado: el que se deja vacío usa el de fábrica.
        template: {
            let plantilla = |t: Option<&String>| match t {
                Some(t) if !t.trim().is_empty() => t.clone(),
                _ => PIE_DEFECTO.into(),
            };
            ByLang {
                es: plantilla(c.map(|c| &c.footer_template.es)),
                en: plantilla(c.map(|c| &c.footer_template.en)),
                otros: c
                    .map(|c| {
                        c.footer_template
                            .otros
                            .iter()
                            .map(|(l, t)| (*l, plantilla(Some(t))))
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        },
    };
    let squeeze_min = d
        .options
        .title_squeeze_min
        .or_else(|| c.map(|c| c.title_squeeze_min))
        .unwrap_or(ESTRECHADO_DEFECTO)
        .clamp(ESTRECHADO_MIN, ESTRECHADO_MAX);
    // El mínimo nunca pasa del límite (nunca más estrecho que él).
    let squeeze_base = d
        .options
        .title_squeeze_base
        .or_else(|| c.map(|c| c.title_squeeze_base))
        .unwrap_or(ESTRECHADO_BASE_DEFECTO)
        .clamp(squeeze_min, 1.0);
    // Lo que no cambia los píxeles no entra en la huella.
    let mut design = d.clone();
    design.thumb_key = None;
    design.status = DesignStatus::Draft;
    DisenoEfectivo {
        design,
        frame,
        emblem,
        original_name,
        pie,
        font_title: c.and_then(|c| c.font_title.clone()),
        font_rules: c.and_then(|c| c.font_rules.clone()),
        squeeze_min,
        squeeze_base,
        pesos: Pesos {
            titulo: c
                .and_then(|c| c.weight_title)
                .map(|p| p.clamp(100.0, 900.0)),
            tipo: c.and_then(|c| c.weight_type).map(|p| p.clamp(100.0, 900.0)),
            reglas: c
                .and_then(|c| c.weight_rules)
                .map(|p| p.clamp(100.0, 900.0)),
        },
        retoques: d
            .options
            .fine_tuning
            .or_else(|| c.and_then(|c| c.fine_tuning))
            .unwrap_or_default()
            .limitado(),
        coleccion_perdida: perdida,
    }
}

/// El diseño sin los textos de los idiomas que no se pintan: así, editar el
/// inglés no cambia la huella (ni repinta) la vista en español, y al revés.
/// De las reglas a mano solo cuenta si las hay (las compuestas lo miran en
/// los dos idiomas). La línea de tipo inglesa se queda: decide cosas
/// (criatura, sala…) en todos.
///
/// 🛑 En español e inglés el resultado es el de antes de haber más idiomas
/// (los demás no se guardan en la huella): sus cartas no se vuelven a pintar.
fn solo_su_idioma(e: &DisenoEfectivo, lang: DesignLang) -> DisenoEfectivo {
    let mut e = e.clone();
    match lang {
        DesignLang::Es | DesignLang::En => {
            let otro = if lang == DesignLang::Es {
                DesignLang::En
            } else {
                DesignLang::Es
            };
            for f in &mut e.design.faces {
                let t = f.texts.get_mut(otro);
                *t = FaceTexts {
                    rules_override: t.rules_override.as_ref().map(|_| String::new()),
                    ..FaceTexts::default()
                };
                f.texts.otros.clear();
            }
            for f in &mut e.design.card.faces {
                f.rules.get_mut(otro).clear();
                f.rules.otros.clear();
                f.type_line.otros.clear();
                f.printed_names.clear();
            }
            e.design.card.sin_texto.clear();
            e.pie.template.get_mut(otro).clear();
            e.pie.template.otros.clear();
        }
        _ => {
            // Lo del idioma pintado (o lo inglés de respaldo, si aún no lo
            // tiene) se queda en su sitio; lo demás, fuera.
            for f in &mut e.design.faces {
                let propio = f.texts.otros.remove(&lang);
                let hechas = f.reglas_a_mano();
                f.texts = ByLang {
                    es: FaceTexts {
                        rules_override: hechas.then(String::new),
                        ..FaceTexts::default()
                    },
                    ..ByLang::default()
                };
                if let Some(t) = propio {
                    f.texts.otros.insert(lang, t);
                }
            }
            for f in &mut e.design.card.faces {
                let reglas = f.rules.get(lang).clone();
                let tipo = f.type_line.get(lang).clone();
                let nombre = f.printed_name(lang).map(str::to_string);
                f.rules = ByLang {
                    otros: [(lang, reglas)].into(),
                    ..ByLang::default()
                };
                f.type_line.es.clear();
                f.type_line.otros = [(lang, tipo)].into();
                f.printed_name_es = None;
                f.printed_names.clear();
                f.poner_printed_name(lang, nombre);
            }
            let sin = e.design.card.sin_texto_en(lang);
            e.design.card.sin_texto_es = false;
            e.design.card.sin_texto.clear();
            e.design.card.poner_sin_texto(lang, sin);
            let plantilla = e.pie.template.get(lang).clone();
            e.pie.template = ByLang {
                otros: [(lang, plantilla)].into(),
                ..ByLang::default()
            };
        }
    }
    e
}

/// La huella de un render: el diseño efectivo, el idioma y las huellas de
/// los recursos (letras, pack, símbolos) y la versión del motor.
///
/// Si algún texto lleva emoji, además la letra de emoji: así, al cambiar de
/// Twemoji a Noto solo se vuelven a pintar las cartas con emoji, y no todas
/// (como pasaría subiendo `VERSION_MOTOR`).
pub fn huella(e: &DisenoEfectivo, lang: DesignLang, cara: usize, recursos: &str) -> String {
    let mut h = Sha256::new();
    let json = serde_json::to_string(&solo_su_idioma(e, lang)).unwrap_or_default();
    h.update(json.as_bytes());
    h.update(format!(
        "|{lang:?}|{cara}|{recursos}|{}",
        super::VERSION_MOTOR
    ));
    if json.chars().any(super::emoji::es_emoji) {
        h.update("|emoji-noto");
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::modelo::tests::{diseno_de_prueba, imagen};

    fn coleccion() -> Collection {
        Collection {
            id: "c1".into(),
            name: "Tensura 2".into(),
            default_style: DesignStyle::Borderless,
            estilo_pack_defecto: None,
            frame: FrameOptions {
                color: FrameColor::Palette {
                    palette: Palette::Gold,
                },
                ..FrameOptions::default()
            },
            emblem: Some(imagen("9")),
            set_code: "RTS".into(),
            rarity_letter: Some("S".into()),
            year: None,
            artist: Some("S.J.".into()),
            credit: None,
            universe: None,
            original_name: OriginalName::Footer,
            footer_template: ByLang::default(),
            font_title: None,
            font_rules: None,
            title_squeeze_min: 0.6,
            title_squeeze_base: ESTRECHADO_BASE_DEFECTO,
            weight_title: None,
            weight_type: None,
            weight_rules: None,
            fine_tuning: None,
        }
    }

    #[test]
    fn la_carta_pisa_a_la_coleccion_y_la_coleccion_a_la_fabrica() {
        let mut d = diseno_de_prueba();
        d.emblem = EmblemChoice::Inherit;
        let c = coleccion();
        let e = efectivo(&d, Some(&c));
        assert_eq!(e.frame, c.frame);
        assert_eq!(e.emblem, c.emblem);
        assert_eq!(e.pie.set_code, "RTS");
        assert_eq!(e.pie.rarity, "S");
        assert_eq!(e.pie.artist, "S.J.");
        assert_eq!(e.squeeze_min, 0.6);
        assert_eq!(e.pie.template.es, PIE_DEFECTO);
        // La carta pisa.
        d.footer.artist = Some("Pablo".into());
        d.options.title_squeeze_min = Some(0.9); // se recorta al rango
        d.emblem = EmblemChoice::None;
        let e = efectivo(&d, Some(&c));
        assert_eq!(e.pie.artist, "Pablo");
        assert_eq!(e.squeeze_min, ESTRECHADO_MAX);
        assert!(e.emblem.is_none());
        // Sin colección, la fábrica.
        let e = efectivo(&d, None);
        assert!(e.coleccion_perdida); // pedía «c1» y no está
        assert_eq!(e.frame, FrameOptions::default());
        assert_eq!(e.pie.set_code, "AKH");
        assert_eq!(e.pie.rarity, "R");
        assert_eq!(e.original_name, OriginalName::Footer);
    }

    /// 🛑 Las huellas de la 2.5.9 (calculadas antes de los idiomas nuevos):
    /// una carta en español o inglés de antes no se vuelve a pintar ni cambia.
    #[test]
    fn las_huellas_en_espanol_e_ingles_son_las_de_antes() {
        let d = diseno_de_prueba();
        let c = coleccion();
        let h = |c: Option<&Collection>, l, cara| huella(&efectivo(&d, c), l, cara, "r");
        assert_eq!(
            h(None, DesignLang::Es, 0),
            "86a126e8ad16c7f75fc082c8d1b650d79c6baf6c30c58f046e2337e56a218df8"
        );
        assert_eq!(
            h(None, DesignLang::Es, 1),
            "c5722df8bcd52b26a4216543c73b6b6265d57d4483aed3175f1782c8aa996b97"
        );
        assert_eq!(
            h(None, DesignLang::En, 0),
            "a72dfae88af9007735357f2b81d11b8ede00e8098e7fee737aa67a145c574c9a"
        );
        assert_eq!(
            h(None, DesignLang::En, 1),
            "7a151574de87a6572b1a0817b23a6bdba95c4c1c8f7f166941dc954100bdd103"
        );
        assert_eq!(
            h(Some(&c), DesignLang::Es, 0),
            "0d9e033b8987919a222ede3d46b53bc4642097e8e53d5cfb4f57aac3effea19a"
        );
        assert_eq!(
            h(Some(&c), DesignLang::En, 0),
            "dc973db2ab8aab3618d85a6ce2278ad70d73337c3521e306fe3246c9b0e7c580"
        );
    }

    #[test]
    fn cambiar_la_coleccion_cambia_la_huella() {
        let d = diseno_de_prueba();
        let mut c = coleccion();
        let a = huella(&efectivo(&d, Some(&c)), DesignLang::Es, 0, "r");
        assert_eq!(a, huella(&efectivo(&d, Some(&c)), DesignLang::Es, 0, "r"));
        c.set_code = "RT2".into();
        assert_ne!(a, huella(&efectivo(&d, Some(&c)), DesignLang::Es, 0, "r"));
        assert_ne!(
            a,
            huella(&efectivo(&d, Some(&coleccion())), DesignLang::En, 0, "r")
        );
        assert_ne!(
            a,
            huella(&efectivo(&d, Some(&coleccion())), DesignLang::Es, 0, "s")
        );
    }

    #[test]
    fn los_textos_del_otro_idioma_no_cambian_la_huella() {
        let c = coleccion();
        let d = diseno_de_prueba();
        let h = |d: &CardDesign, l| huella(&efectivo(d, Some(&c)), l, 0, "r");
        let mut en = d.clone();
        en.faces[0].texts.en.name = "Other".into();
        en.faces[0].texts.en.flavor = "Flavor".into();
        assert_eq!(h(&d, DesignLang::Es), h(&en, DesignLang::Es));
        assert_ne!(h(&d, DesignLang::En), h(&en, DesignLang::En));
        // Reglas a mano en el otro idioma: cuenta que las haya, no qué dicen.
        en.faces[0].texts.en.rules_override = Some("A".into());
        let a = h(&en, DesignLang::Es);
        assert_ne!(h(&d, DesignLang::Es), a);
        en.faces[0].texts.en.rules_override = Some("B".into());
        assert_eq!(a, h(&en, DesignLang::Es));
    }
}
