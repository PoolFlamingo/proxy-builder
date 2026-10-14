//! **El contrato del diseño de una carta** (§3.2 del plan de la 2.0): lo que
//! se guarda en `DeckCard.design`, lo que viaja a la pantalla (`ts-rs`) y lo
//! que el motor pinta.
//!
//! 🛑 Lo mecánico (`card`) es una **instantánea** de Scryfall: coste, tipo,
//! reglas, fuerza/resistencia, rareza y nombre original salen siempre de
//! ahí y el formulario no los edita (§3.7). Lo único que pone el usuario es
//! lo temático: nombre, sabor, ilustración, marco y pie.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{BackSpec, CardEdits, CardSource, FaceImage};

/// Versión del formato de `CardDesign`.
pub const VERSION_DISENO: u32 = 1;

/// Límite de estrechado del título por defecto y su rango (§3.7).
pub const ESTRECHADO_DEFECTO: f64 = 0.65;
pub const ESTRECHADO_MIN: f64 = 0.55;
pub const ESTRECHADO_MAX: f64 = 0.75;

/// En qué idioma sale un diseño.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DesignLang {
    #[default]
    Es,
    En,
}

/// Un valor por idioma.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ByLang<T> {
    pub es: T,
    pub en: T,
}

impl<T> ByLang<T> {
    pub fn get(&self, lang: DesignLang) -> &T {
        match lang {
            DesignLang::Es => &self.es,
            DesignLang::En => &self.en,
        }
    }
    pub fn get_mut(&mut self, lang: DesignLang) -> &mut T {
        match lang {
            DesignLang::Es => &mut self.es,
            DesignLang::En => &mut self.en,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DesignStatus {
    #[default]
    Draft,
    Ready,
}

/// Los estilos base. «Ficha» no es un estilo sino un layout que cada estilo
/// sabe pintar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DesignStyle {
    /// Despejado: la ilustración ocupa toda la carta (01–04 de Pablo).
    #[default]
    Borderless,
    /// Enmarcado extendido: arte a sangre arriba y caja opaca abajo (09).
    Extended,
    /// Enmarcado clásico: ventana de arte y marco completo (05–08).
    Classic,
    /// Paisaje: tierras básicas con el arte entero y una franja abajo (13–14).
    FullArt,
}

/// Una paleta de marco de las de serie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum Palette {
    W,
    U,
    B,
    R,
    G,
    #[serde(rename = "gold")]
    Gold,
    #[serde(rename = "artifact")]
    Artifact,
    #[serde(rename = "land")]
    Land,
    #[serde(rename = "colorless")]
    Colorless,
}

/// El color del marco.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum FrameColor {
    /// Según la carta: un color, el suyo; dos, degradado; tres o más, oro.
    #[default]
    Auto,
    Palette {
        palette: Palette,
    },
    Gradient {
        a: Palette,
        b: Palette,
    },
    /// Color propio (`#rrggbb`).
    Custom {
        hex: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OnOffAuto {
    #[default]
    Auto,
    On,
    Off,
}

/// La caja de reglas (en los estilos con arte detrás).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum RulesBox {
    /// Teñida del color de la carta.
    #[default]
    Auto,
    Dark,
    Light,
    Tint {
        hex: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FrameOptions {
    /// `builtin` o el id de un pack importado.
    pub pack: String,
    pub color: FrameColor,
    pub crown: OnOffAuto,
    pub rules_box: RulesBox,
}

impl Default for FrameOptions {
    fn default() -> Self {
        FrameOptions {
            pack: "builtin".into(),
            color: FrameColor::Auto,
            crown: OnOffAuto::Auto,
            rules_box: RulesBox::Auto,
        }
    }
}

/// De dónde salió la ilustración.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum ArtSource {
    Upload,
    ScryfallArtCrop {
        url: String,
    },
    /// Recortada de una carta ya hecha (el blob de esa carta).
    CardCrop {
        blob: String,
    },
}

/// El encuadre de la ilustración en su hueco: zoom ≥ 1, desplazamiento en
/// [-1, 1] y giro libre en grados.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtFraming {
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
    pub rotation: f64,
}

impl Default for ArtFraming {
    fn default() -> Self {
        ArtFraming {
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            rotation: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ArtRef {
    pub image: FaceImage,
    pub source: ArtSource,
    pub framing: ArtFraming,
}

/// Lo que el usuario escribe de una cara, en un idioma.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FaceTexts {
    /// El nombre temático.
    pub name: String,
    pub flavor: String,
    /// Reglas escritas a mano (erratas, ajustes). Si está, manda y la carta
    /// sale marcada como «editada a mano».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rules_override: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FaceDesign {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub art: Option<ArtRef>,
    pub texts: ByLang<FaceTexts>,
}

/// Lo mecánico de una cara (o mitad), tal como vino de Scryfall.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FaceData {
    /// Nombre en inglés (el original).
    pub name: String,
    pub mana_cost: String,
    /// Línea de tipo y reglas por idioma: `en` del Oracle, `es` de la
    /// impresión española (o el Oracle si no hay, con aviso).
    pub type_line: ByLang<String>,
    pub rules: ByLang<String>,
    /// Nombre impreso en español (para sustituirlo en las reglas).
    #[serde(default)]
    pub printed_name_es: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub power: Option<String>,
    #[serde(default)]
    pub toughness: Option<String>,
    #[serde(default)]
    pub loyalty: Option<String>,
    #[serde(default)]
    pub defense: Option<String>,
}

/// La instantánea mecánica de la carta.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardData {
    pub scryfall_id: String,
    #[serde(default)]
    pub oracle_id: Option<String>,
    /// El `layout` de Scryfall.
    pub layout: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    /// Rareza de Scryfall (`common`, `uncommon`, `rare`, `mythic`, `special`…).
    pub rarity: String,
    #[serde(default)]
    pub frame_effects: Vec<String>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub color_identity: Vec<String>,
    pub cmc: f64,
    pub set: String,
    pub collector_number: String,
    #[serde(default)]
    pub artist: Option<String>,
    pub faces: Vec<FaceData>,
    /// No hay impresión en español: el texto `es` es el Oracle en inglés.
    #[serde(default)]
    pub sin_texto_es: bool,
    /// Impresa en tamaño grande (planos, planes, Vanguard).
    #[serde(default)]
    pub oversized: bool,
    /// Modificadores de mano y vida de una Vanguard (`+1`, `-3`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub hand_modifier: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub life_modifier: Option<String>,
    /// Meld: la carta fundida va como segunda cara y aquí qué mitad de ella
    /// lleva esta carta en el dorso (`true`, la de arriba).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub meld_arriba: Option<bool>,
    /// Cuándo se tomó la instantánea (ms desde 1970).
    #[ts(type = "number")]
    pub fetched_at: i64,
}

impl CardData {
    pub fn es_legendaria(&self) -> bool {
        self.frame_effects.iter().any(|f| f == "legendary")
            || self
                .faces
                .first()
                .is_some_and(|f| f.type_line.en.starts_with("Legendary"))
    }
    pub fn es_ficha(&self) -> bool {
        self.layout == "token"
            || self.layout == "double_faced_token"
            || self
                .faces
                .first()
                .is_some_and(|f| f.type_line.en.contains("Token"))
    }
}

/// Cómo se restaura la carta al quitar el diseño.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct BeforeDesign {
    pub source: CardSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub front: Option<FaceImage>,
    pub back: BackSpec,
    pub edits: CardEdits,
}

/// Lo que la carta cambia sobre el pie de la colección.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FooterOptions {
    /// Número de colección («0002», «T001»). Se asigna al crear el diseño y
    /// se guarda; «Renumerar» lo recalcula si no es manual.
    pub number: String,
    pub number_manual: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rarity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub set_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub credit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub universe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub year: Option<String>,
}

/// Dónde va el nombre original.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OriginalName {
    /// Lo que diga la colección (y si no, el pie).
    #[default]
    Auto,
    Ribbon,
    Footer,
    Hidden,
}

/// El adorno a la derecha de la línea de tipo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum EmblemChoice {
    /// El de la colección.
    #[default]
    Inherit,
    None,
    Own {
        image: FaceImage,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DesignOptions {
    pub original_name: OriginalName,
    /// Tamaño manual de las reglas en puntos (`None` = automático).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rules_size_pt: Option<f64>,
    pub show_flavor: bool,
    /// Límite de estrechado del título (`None` = el de la colección).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title_squeeze_min: Option<f64>,
}

/// El diseño de una carta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardDesign {
    pub version: u32,
    pub status: DesignStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub collection: Option<String>,
    pub style: DesignStyle,
    /// El marco propio de esta carta; `None` = el de la colección (y si no
    /// hay, el de fábrica). Así, cambiar la colección cambia sus cartas.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub frame: Option<FrameOptions>,
    /// Una por cara o mitad (1 en las normales).
    pub faces: Vec<FaceDesign>,
    pub before: BeforeDesign,
    pub card: CardData,
    pub footer: FooterOptions,
    pub options: DesignOptions,
    pub emblem: EmblemChoice,
    /// La huella con la que se pintó la miniatura `front` de la carta. Si la
    /// de ahora es otra (cambió la colección, una letra, el pack…), la
    /// miniatura está desfasada y se rehace (`design_rerender`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub thumb_key: Option<String>,
}

impl CardDesign {
    /// Todas las imágenes del diseño (para `blob_refs` y los paquetes): el
    /// arte de cada cara, el emblema propio y lo de antes.
    pub fn blobs(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .faces
            .iter()
            .filter_map(|f| f.art.as_ref())
            .flat_map(|a| {
                let mut x = vec![a.image.blob.as_str()];
                if let ArtSource::CardCrop { blob } = &a.source {
                    x.push(blob.as_str());
                }
                x
            })
            .collect();
        if let EmblemChoice::Own { image } = &self.emblem {
            v.push(&image.blob);
        }
        if let Some(f) = &self.before.front {
            v.push(&f.blob);
        }
        match &self.before.back {
            BackSpec::Dfc { image } | BackSpec::Custom { image } => v.push(&image.blob),
            BackSpec::DeckDefault => {}
        }
        v.sort_unstable();
        v.dedup();
        v
    }
}

/// Un aviso sobre un diseño. Rust nunca escribe prosa: la pantalla lo
/// traduce por su `kind`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum Aviso {
    /// No hay impresión en español: las reglas `es` son el Oracle en inglés.
    SinTextoOficial,
    /// El texto no cabe ni a la letra mínima (`sobra` en mm).
    TextoNoCabe {
        hueco: String,
        sobra: f64,
    },
    /// El título o el tipo no cabían ni estrechados y se ha bajado la letra.
    TituloReducido {
        hueco: String,
    },
    /// Reglas escritas a mano en esa cara e idioma.
    ReglasEditadas {
        cara: u32,
        lang: DesignLang,
    },
    /// Falta el nombre temático en un idioma.
    FaltaNombre {
        cara: u32,
        lang: DesignLang,
    },
    LetraNoEncontrada {
        familia: String,
    },
    FaltanSimbolos {
        simbolos: Vec<String>,
    },
    PackSinLayout {
        layout: String,
    },
    LayoutDesconocido {
        layout: String,
    },
    /// La colección del diseño ya no existe: se usan los valores de fábrica.
    ColeccionPerdida,
}

/// Lo que devuelve un borrador: el diseño y sus avisos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DesignDraft {
    pub design: CardDesign,
    pub avisos: Vec<Aviso>,
}

/// Una colección: lo compartido por un grupo de diseños (marco, emblema,
/// pie…). Se guarda en `presets` con `kind = collection`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub default_style: DesignStyle,
    pub frame: FrameOptions,
    /// El emblema del mazo o de la serie (el slime de Pablo).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub emblem: Option<FaceImage>,
    pub set_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rarity_letter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub year: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub artist: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub credit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub universe: Option<String>,
    pub original_name: OriginalName,
    /// Plantillas del pie con fichas: `{num} {rarity} {original}` / `{set}{sep}{lang} {brush} {artist}`…
    pub footer_template: ByLang<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub font_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub font_rules: Option<String>,
    pub title_squeeze_min: f64,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::model::{DeckCard, ImageOrigin};

    pub(crate) fn imagen(h: &str) -> FaceImage {
        FaceImage {
            blob: h.repeat(64 / h.len()),
            width: 10,
            height: 10,
            origin: ImageOrigin::Upload,
        }
    }

    pub(crate) fn diseno_de_prueba() -> CardDesign {
        let cara = |n: &str| FaceData {
            name: n.into(),
            mana_cost: "{2}{W}{W}".into(),
            type_line: ByLang {
                es: "Conjuro".into(),
                en: "Sorcery".into(),
            },
            rules: ByLang {
                es: "Destruye todas las criaturas con fuerza de 3 o más.".into(),
                en: "Destroy all creatures with power 3 or greater.".into(),
            },
            printed_name_es: Some("Ocaso".into()),
            ..FaceData::default()
        };
        CardDesign {
            version: VERSION_DISENO,
            status: DesignStatus::Draft,
            collection: Some("c1".into()),
            style: DesignStyle::Borderless,
            frame: None,
            faces: vec![
                FaceDesign {
                    art: Some(ArtRef {
                        image: imagen("a"),
                        source: ArtSource::CardCrop {
                            blob: "c".repeat(64),
                        },
                        framing: ArtFraming::default(),
                    }),
                    texts: ByLang {
                        es: FaceTexts {
                            name: "Crepúsculo de Veldora".into(),
                            ..FaceTexts::default()
                        },
                        en: FaceTexts::default(),
                    },
                },
                FaceDesign::default(),
            ],
            before: BeforeDesign {
                source: CardSource::Scryfall {
                    scryfall_id: "x".into(),
                },
                front: Some(imagen("b")),
                back: BackSpec::DeckDefault,
                edits: CardEdits::default(),
            },
            card: CardData {
                scryfall_id: "x".into(),
                layout: "split".into(),
                rarity: "rare".into(),
                cmc: 9.0,
                set: "akh".into(),
                collector_number: "210".into(),
                faces: vec![cara("Dusk"), cara("Dawn")],
                ..CardData::default()
            },
            footer: FooterOptions {
                number: "0002".into(),
                ..FooterOptions::default()
            },
            options: DesignOptions::default(),
            emblem: EmblemChoice::Own { image: imagen("e") },
            thumb_key: None,
        }
    }

    #[test]
    fn ida_y_vuelta_de_un_diseno_de_dos_caras() {
        let d = diseno_de_prueba();
        let json = serde_json::to_string(&d).unwrap();
        let vuelta: CardDesign = serde_json::from_str(&json).unwrap();
        assert_eq!(vuelta, d);
        assert!(json.contains("\"style\":\"borderless\""));
        assert!(json.contains("\"kind\":\"cardCrop\""));
        // Todas las imágenes que hay que proteger de la limpieza.
        let b = d.blobs();
        assert_eq!(b.len(), 4);
        assert!(b.contains(&"c".repeat(64).as_str()));
    }

    #[test]
    fn una_carta_1x_se_lee_igual_y_no_escribe_design() {
        let json = r#"{"id":"1","source":{"kind":"blank"},"front":null,"back":{"kind":"deckDefault"},
            "edits":{"rotation":"r0","crop":{"zoom":1.0,"offsetX":0.0,"offsetY":0.0},
            "adjust":{"brightness":0,"exposure":0,"contrast":0,"saturation":0,"warmth":0,"vibrance":0,
            "sharpness":0,"clarity":0,"noiseReduction":0,"vignette":0,"fade":0}},
            "locked":false,"meta":{"name":"","set":null,"collectorNumber":null,"lang":null,
            "langFallback":false,"layout":null,"isDfc":false,"imageStatus":null,"highres":false,"typeLine":null}}"#;
        let c: DeckCard = serde_json::from_str(json).unwrap();
        assert!(c.design.is_none());
        assert!(!serde_json::to_string(&c).unwrap().contains("design"));
    }

    #[test]
    fn legendaria_y_ficha() {
        let mut d = diseno_de_prueba().card;
        assert!(!d.es_legendaria());
        d.frame_effects.push("legendary".into());
        assert!(d.es_legendaria());
        assert!(!d.es_ficha());
        d.layout = "token".into();
        assert!(d.es_ficha());
    }

    /// R-8: lo que ocupa un deck de 108 cartas diseñadas (ES + EN).
    #[test]
    fn peso_de_un_deck_disenado() {
        let mut d = diseno_de_prueba();
        d.faces.truncate(1);
        d.card.faces.truncate(1);
        let largo = "Siempre que una criatura con defensor entre al campo de batalla bajo tu control, roba una carta. ".repeat(4);
        d.card.faces[0].rules = ByLang {
            es: largo.clone(),
            en: largo.clone(),
        };
        d.faces[0].texts.es.flavor = "“Te daré mi destrucción y la llamaré una bendición.”".into();
        d.faces[0].texts.en.flavor = d.faces[0].texts.es.flavor.clone();
        let una = serde_json::to_string(&d).unwrap().len();
        let total = una * 108;
        assert!(total < 2_000_000, "{total} bytes");
    }
}
