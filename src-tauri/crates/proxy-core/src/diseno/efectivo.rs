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
        template: c
            .map(|c| c.footer_template.clone())
            .filter(|t| !t.es.trim().is_empty() || !t.en.trim().is_empty())
            .unwrap_or_else(|| ByLang {
                es: PIE_DEFECTO.into(),
                en: PIE_DEFECTO.into(),
            }),
    };
    let squeeze_min = d
        .options
        .title_squeeze_min
        .or_else(|| c.map(|c| c.title_squeeze_min))
        .unwrap_or(ESTRECHADO_DEFECTO)
        .clamp(ESTRECHADO_MIN, ESTRECHADO_MAX);
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
        coleccion_perdida: perdida,
    }
}

/// La huella de un render: el diseño efectivo, el idioma y las huellas de
/// los recursos (letras, pack, símbolos) y la versión del motor.
pub fn huella(e: &DisenoEfectivo, lang: DesignLang, cara: usize, recursos: &str) -> String {
    let mut h = Sha256::new();
    h.update(serde_json::to_vec(e).unwrap_or_default());
    h.update(format!(
        "|{lang:?}|{cara}|{recursos}|{}",
        super::VERSION_MOTOR
    ));
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
}
