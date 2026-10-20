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
/// Estrechado mínimo (el que se aplica siempre al título y al tipo, quepan o
/// no): 1 = sin estrechar, como hasta ahora.
pub const ESTRECHADO_BASE_DEFECTO: f64 = 1.0;
/// Hasta dónde se mueve cada texto con los ajustes finos (mm, arriba o abajo).
pub const AJUSTE_MAX_MM: f64 = 3.0;

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
    /// Estrechado mínimo del título y el tipo (`None` = el de la colección).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title_squeeze_base: Option<f64>,
    /// Ajustes finos propios de esta carta (`None` = los de la colección).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fine_tuning: Option<FineTuning>,
}

/// Ajustes finos de la carta: cuánto se baja cada texto o símbolo (mm;
/// negativo, sube), lo opaco que es el fondo de la caja de reglas y el radio
/// de las esquinas de la ilustración y de la carta. Todo a cero, como viene
/// en el marco.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct FineTuning {
    pub title: f64,
    pub cost: f64,
    pub type_line: f64,
    pub emblem: f64,
    pub rules: f64,
    pub pt: f64,
    pub footer: f64,
    /// Cuánto más opaco el fondo de la caja de reglas: 0 = el del marco,
    /// 1 = del todo opaco.
    pub rules_box_opacity: f64,
    /// Radio (mm) de las esquinas de la ventana de la ilustración (`None` =
    /// el del marco). Solo en los marcos que la dibujan (despejado).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub art_radius: Option<f64>,
    /// Radio (mm) de las esquinas de la carta (`None` = redondeadas en
    /// pantalla y en ángulo recto al imprimir). Con él, igual en las dos.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub card_radius: Option<f64>,
    /// Cuánto (mm) se funde con negro el borde de abajo de la ilustración
    /// (`None` = lo del marco: `difuminadoArte` del estilo; 0 = nada).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub art_fade: Option<f64>,
    /// Cuánto (mm) sigue la ilustración por debajo de su ventana, perdiéndose
    /// (el difuminado acaba ahí). Se ve donde el marco no la tapa: en el
    /// extendido, a los lados de la caja y debajo de ella. `None` = nada.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub art_fade_extension: Option<f64>,
    /// Cuánto (mm) más alta la caja de reglas: crece hacia arriba (y la
    /// ilustración se acorta); negativo, más baja. `None` = la del marco.
    /// En los marcos de imagen (Original, Balatro) no cambia.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rules_box_height: Option<f64>,
    /// Tamaño de cada texto en proporción al del marco (`None` = 1, el del
    /// marco). En las reglas, `None` es «automático»: en las cajas que
    /// crecen, la letra baja un poco antes de comerse el arte; con un valor,
    /// la letra es esa y la caja crece lo que haga falta (y solo baja si ni
    /// al máximo cabe). El título y el tipo se estrechan como siempre.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub title_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub type_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rules_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub pt_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub footer_size: Option<f64>,
    /// Interlineado de las reglas y del pie, en proporción al del marco
    /// (`None` = 1).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub rules_line_spacing: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub footer_line_spacing: Option<f64>,
}

/// Lo más que se puede fundir con negro el borde de abajo de la ilustración (mm).
pub const DIFUMINADO_MAX_MM: f64 = 25.0;
/// Lo más que puede seguir la ilustración por debajo de su ventana (mm).
pub const EXTENSION_DIFUMINADO_MAX_MM: f64 = 35.0;
/// Lo más que se puede subir o bajar el borde de arriba de la caja de reglas (mm).
pub const ALTO_CAJA_MAX_MM: f64 = 10.0;
/// Lo que se puede agrandar o achicar cada texto.
pub const TAMANO_TEXTO_MIN: f64 = 0.5;
pub const TAMANO_TEXTO_MAX: f64 = 1.6;
/// Lo que se puede apretar o separar el interlineado.
pub const INTERLINEADO_MIN: f64 = 0.7;
pub const INTERLINEADO_MAX: f64 = 1.6;

impl FineTuning {
    /// Dentro de los límites.
    pub fn limitado(self) -> FineTuning {
        let m = |v: f64| v.clamp(-AJUSTE_MAX_MM, AJUSTE_MAX_MM);
        let tam = |v: Option<f64>| v.map(|v| v.clamp(TAMANO_TEXTO_MIN, TAMANO_TEXTO_MAX));
        let lin = |v: Option<f64>| v.map(|v| v.clamp(INTERLINEADO_MIN, INTERLINEADO_MAX));
        FineTuning {
            title: m(self.title),
            cost: m(self.cost),
            type_line: m(self.type_line),
            emblem: m(self.emblem),
            rules: m(self.rules),
            pt: m(self.pt),
            footer: m(self.footer),
            rules_box_opacity: self.rules_box_opacity.clamp(0.0, 1.0),
            art_radius: self.art_radius.map(|r| r.clamp(0.0, 20.0)),
            card_radius: self.card_radius.map(|r| r.clamp(0.0, 6.0)),
            art_fade: self.art_fade.map(|r| r.clamp(0.0, DIFUMINADO_MAX_MM)),
            art_fade_extension: self
                .art_fade_extension
                .map(|r| r.clamp(0.0, EXTENSION_DIFUMINADO_MAX_MM)),
            rules_box_height: self
                .rules_box_height
                .map(|r| r.clamp(-ALTO_CAJA_MAX_MM, ALTO_CAJA_MAX_MM)),
            title_size: tam(self.title_size),
            type_size: tam(self.type_size),
            rules_size: tam(self.rules_size),
            pt_size: tam(self.pt_size),
            footer_size: tam(self.footer_size),
            rules_line_spacing: lin(self.rules_line_spacing),
            footer_line_spacing: lin(self.footer_line_spacing),
        }
    }
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
    /// Un estilo propio del pack de marcos (no uno de los de serie): su id en
    /// el pack («tarot»). `style` es entonces el de serie del que hereda, al
    /// que se cae si el pack no lo tiene. Sin `skip_serializing_if` cambiaría
    /// la huella de todos los diseños de antes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub estilo_pack: Option<String>,
    /// La variante elegida del estilo (`variantes` del pack). Como
    /// `estilo_pack`, sin escribirse si no hay, por la huella.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub variante_pack: Option<String>,
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
    /// El estilo propio de la carta no está en el pack de marcos (se cambió
    /// de pack o no está descargado): se usa su estilo de serie.
    EstiloNoEnElPack {
        estilo: String,
    },
    LayoutDesconocido {
        layout: String,
    },
    /// La colección del diseño ya no existe: se usan los valores de fábrica.
    ColeccionPerdida,
    /// Se pidió el nombre original en una cinta, pero el marco de este estilo
    /// no la tiene: va en el pie.
    SinCinta,
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
    /// El estilo propio del pack para las cartas nuevas (con `default_style`
    /// como su base).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub estilo_pack_defecto: Option<String>,
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
    /// Estrechado mínimo: el título y el tipo van siempre al menos así de
    /// estrechos (1 = sin estrechar). No baja del límite de arriba.
    #[serde(default = "estrechado_base_defecto")]
    pub title_squeeze_base: f64,
    /// Grosor (100–900) del título, la línea de tipo y las reglas. `None` =
    /// el del pack (negrita en el título y el tipo, normal en las reglas):
    /// una letra que ya es gruesa puede ir sin negrita.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub weight_title: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub weight_type: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub weight_rules: Option<f64>,
    /// Ajustes finos de sus cartas (posición de los textos, caja, radios).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fine_tuning: Option<FineTuning>,
}

fn estrechado_base_defecto() -> f64 {
    ESTRECHADO_BASE_DEFECTO
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn los_tamanos_e_interlineados_tienen_limites() {
        let f = FineTuning {
            rules_size: Some(0.1),
            title_size: Some(9.0),
            rules_line_spacing: Some(0.2),
            footer_line_spacing: Some(3.0),
            ..FineTuning::default()
        }
        .limitado();
        assert_eq!(f.rules_size, Some(TAMANO_TEXTO_MIN));
        assert_eq!(f.title_size, Some(TAMANO_TEXTO_MAX));
        assert_eq!(f.rules_line_spacing, Some(INTERLINEADO_MIN));
        assert_eq!(f.footer_line_spacing, Some(INTERLINEADO_MAX));
        assert_eq!(f.type_size, None);
    }
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
            estilo_pack: None,
            variante_pack: None,
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
