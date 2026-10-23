//! **El modelo de datos de un deck.** Es el contrato entre Rust, la base de
//! datos y la pantalla: cada tipo se exporta a TypeScript con `ts-rs`
//! (`pnpm bindings` → `src/bindings/`).
//!
//! 🛑 Una fila por **copia física** (`DeckCard`), no una por carta con
//! cantidad: cada copia puede tener su variante, sus retoques y su sitio.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::paper::PaperFormat;

/// Versión del esquema de `Deck` (base de datos y paquetes `.pbdeck`).
/// La 2 añade `DeckCard.design` (el editor de cartas): una 1.x que guardara
/// un deck v2 perdería los diseños, por eso sube.
pub const DECK_SCHEMA_VERSION: u32 = 2;

// ── Imágenes ────────────────────────────────────────────────────────────────

/// De dónde salió una imagen. Decide, entre otras cosas, si se le aplica el
/// oscurecido de bordes (`apply_scryfall` / `apply_uploads`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImageOrigin {
    /// PNG de Scryfall (745×1040, esquinas transparentes).
    ScryfallPng,
    /// JPG `large` de Scryfall (672×936), cuando no hay PNG.
    ScryfallLarge,
    /// Subida por el usuario.
    Upload,
    /// Generada por la app (el reverso genérico).
    Generated,
    /// La miniatura de un diseño del editor de cartas (la 2.0). El PDF no la
    /// usa: pinta desde el diseño a la resolución de exportación.
    Designed,
}

/// Una imagen guardada en el almacén de blobs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FaceImage {
    /// SHA-256 del fichero, en hexadecimal (64 caracteres).
    pub blob: String,
    pub width: u32,
    pub height: u32,
    pub origin: ImageOrigin,
}

// ── Cartas ──────────────────────────────────────────────────────────────────

/// Qué es una carta del deck.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum CardSource {
    /// Una impresión concreta de Scryfall.
    Scryfall {
        #[serde(rename = "scryfallId")]
        #[ts(rename = "scryfallId")]
        scryfall_id: String,
    },
    /// Un token añadido desde `all_parts` de otra carta.
    Token {
        #[serde(rename = "scryfallId")]
        #[ts(rename = "scryfallId")]
        scryfall_id: String,
    },
    /// Una imagen subida por el usuario. Puede ir vinculada a la carta real
    /// de Scryfall que representa (para las estadísticas y la info de carta);
    /// la imagen sigue siendo la propia.
    Custom {
        #[serde(
            rename = "scryfallId",
            default,
            skip_serializing_if = "Option::is_none"
        )]
        #[ts(rename = "scryfallId", optional)]
        scryfall_id: Option<String>,
        /// O vinculada a una carta de **otra fuente** (una ficha de Cockatrice
        /// que Scryfall no tiene: «Kelp»). Es también lo que es una carta
        /// añadida desde esa fuente: una propia con la imagen bajada de allí.
        /// Un campo opcional (no una variante nueva) para que una versión
        /// anterior de la app siga abriendo el deck: la ve como propia.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        externa: Option<RefExterna>,
    },
    /// Un hueco en blanco (lo que deja «+ Añadir página»).
    Blank,
}

/// De dónde sale una carta que no es de Scryfall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FuenteCarta {
    /// La base de fichas de Cockatrice (`Cockatrice/Magic-Token`).
    Cockatrice,
}

/// Una carta de otra fuente: la fuente y su id allí.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RefExterna {
    pub fuente: FuenteCarta,
    pub id: String,
}

/// Qué va al dorso de la carta cuando se imprime a doble cara.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export)]
pub enum BackSpec {
    /// El reverso por defecto del deck.
    DeckDefault,
    /// La cara trasera de una carta de doble cara (DFC).
    Dfc { image: FaceImage },
    /// Un reverso elegido para esta carta.
    Custom { image: FaceImage },
}

/// Giro de la imagen dentro del marco de la carta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Rotation {
    #[default]
    R0,
    R90,
    R180,
    R270,
}

/// Encuadre: zoom ≥ 1 y desplazamiento en [-1, 1] dentro del margen que deja
/// el zoom (0 = centrado).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Crop {
    pub zoom: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl Default for Crop {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

/// Los once retoques de fabricard. 0 = sin efecto. Fórmulas y orden en
/// `imaging::adjust` (sub-plan de la Fase 4, Step 4.5).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Adjustments {
    /// [-100, 100]
    pub brightness: f64,
    /// [-100, 100]
    pub exposure: f64,
    /// [-100, 100]
    pub contrast: f64,
    /// [-100, 100]
    pub saturation: f64,
    /// [-100, 100]
    pub warmth: f64,
    /// [-100, 100]
    pub vibrance: f64,
    /// [0, 100]
    pub sharpness: f64,
    /// [0, 100]
    pub clarity: f64,
    /// [0, 100]
    pub noise_reduction: f64,
    /// [0, 100]
    pub vignette: f64,
    /// [0, 100]
    pub fade: f64,
}

impl Adjustments {
    pub fn is_identity(&self) -> bool {
        *self == Adjustments::default()
    }
}

/// Lo que el usuario ha retocado en una carta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardEdits {
    pub rotation: Rotation,
    pub crop: Crop,
    pub adjust: Adjustments,
}

impl CardEdits {
    pub fn is_identity(&self) -> bool {
        *self == CardEdits::default()
    }
}

/// Datos de la carta para enseñar y para decidir (nombre, set, idioma…).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardMeta {
    pub name: String,
    pub set: Option<String>,
    pub collector_number: Option<String>,
    pub lang: Option<String>,
    /// Se pidió otro idioma y no existía: se quedó en inglés («↩ EN»).
    pub lang_fallback: bool,
    /// `layout` de Scryfall (`normal`, `transform`, `modal_dfc`…).
    pub layout: Option<String>,
    pub is_dfc: bool,
    /// `image_status` de Scryfall (`highres_scan`, `lowres`, `placeholder`, `missing`).
    pub image_status: Option<String>,
    pub highres: bool,
    pub type_line: Option<String>,
    /// En las cartas propias, el idioma de **la imagen**, que no tiene por qué
    /// ser el de la carta vinculada (`lang`): una propia en español vinculada
    /// a la impresión inglesa. Lo pone quien vincula (el pie leído por el OCR)
    /// o se elige en «Editar carta».
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub image_lang: Option<String>,
}

/// Una copia física del deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckCard {
    /// uuid v4.
    pub id: String,
    pub source: CardSource,
    /// `None` = hueco en blanco o imagen todavía sin descargar.
    pub front: Option<FaceImage>,
    pub back: BackSpec,
    pub edits: CardEdits,
    /// Fijada: «Auto ordenar» y «Reaplicar idioma» no la tocan.
    pub locked: bool,
    pub meta: CardMeta,
    /// El diseño del editor de cartas, si lo tiene (la 2.0). Opcional: los
    /// decks 1.x se leen igual y las cartas subidas a mano no lo llevan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub design: Option<crate::diseno::modelo::CardDesign>,
    /// Una marca fija para organizar el deck (no cambia lo que se imprime):
    /// variante de diseño, opcional o banquillo. Las marcadas no cuentan para
    /// el tamaño del mazo, las copias ni las estadísticas (`cuentan`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub marca: Option<MarcaCarta>,
}

/// Las marcas fijas de una carta del deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MarcaCarta {
    /// Otra versión (de diseño, de arte…) de una carta que ya está: se
    /// escogerá una. De las copias de esa carta solo cuenta una.
    Variante,
    /// Puede ir o no: no cuenta.
    Opcional,
    /// Del banquillo (sideboard): no cuenta para el mazo.
    Banquillo,
}

/// Cuántas copias se quedan fuera de las cuentas del mazo por su marca.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Apartadas {
    pub variantes: u32,
    pub opcionales: u32,
    pub banquillo: u32,
}

impl Apartadas {
    pub fn total(&self) -> u32 {
        self.variantes + self.opcionales + self.banquillo
    }
}

/// Las copias que cuentan para el mazo (tamaño, copias, estadísticas) y las
/// que se quedan fuera. Sin las opcionales ni las del banquillo; de las
/// variantes de diseño, ninguna si la carta también está sin marcar, y una
/// si todas sus copias son variantes (es la carta que irá, falta escoger
/// cuál).
pub fn cuentan(cards: &[DeckCard]) -> (Vec<&DeckCard>, Apartadas) {
    let mut ap = Apartadas::default();
    let nombre = |c: &DeckCard| c.meta.name.trim().to_lowercase();
    let sin_marcar: std::collections::HashSet<String> = cards
        .iter()
        .filter(|c| c.marca.is_none())
        .map(nombre)
        .collect();
    let mut variante_puesta: std::collections::HashSet<String> = Default::default();
    let mut out = Vec::new();
    for c in cards {
        match c.marca {
            None => out.push(c),
            Some(MarcaCarta::Opcional) => ap.opcionales += 1,
            Some(MarcaCarta::Banquillo) => ap.banquillo += 1,
            Some(MarcaCarta::Variante) => {
                let n = nombre(c);
                if !sin_marcar.contains(&n) && variante_puesta.insert(n) {
                    out.push(c);
                } else {
                    ap.variantes += 1;
                }
            }
        }
    }
    (out, ap)
}

// ── Ajustes de impresión ────────────────────────────────────────────────────

/// Tamaño de la carta. Por defecto 63×88 mm (lo que publica Wizards); las
/// alternativas son 63,5×88,9 (2,5×3,5″) y 65×90, que llena justa una funda:
/// sale como con 1 mm de sangrado, pero con la imagen entera a ese tamaño en
/// vez del borde repetido.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CardSize {
    pub width_mm: f64,
    pub height_mm: f64,
}

impl CardSize {
    pub const MTG: CardSize = CardSize {
        width_mm: 63.0,
        height_mm: 88.0,
    };
    pub const POKER: CardSize = CardSize {
        width_mm: 63.5,
        height_mm: 88.9,
    };
    pub const FUNDA: CardSize = CardSize {
        width_mm: 65.0,
        height_mm: 90.0,
    };
}

impl Default for CardSize {
    fn default() -> Self {
        CardSize::MTG
    }
}

/// Cómo se rellena el sangrado: repitiendo el píxel del borde o en espejo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum BleedMode {
    #[default]
    Clamp,
    Mirror,
}

/// Guías de corte (marcas en los márgenes, a la altura de cada línea de corte).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CutGuides {
    pub enabled: bool,
    /// Color CSS en hexadecimal (`#00c853`).
    pub color: String,
    /// Grosor en píxeles CSS (1 px = 0,75 pt).
    pub width_px: f64,
    /// Separación desde el borde de la carta hacia fuera.
    pub offset_mm: f64,
    pub length_mm: f64,
}

impl Default for CutGuides {
    fn default() -> Self {
        Self {
            // Apagadas por defecto: preferencia de Pablo (13/10/2026).
            enabled: false,
            color: "#00c853".to_string(),
            width_px: 0.5,
            offset_mm: 0.0,
            length_mm: 5.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EdgeMode {
    #[default]
    ContrastEdges,
    SolidBlack,
    Threshold,
}

/// Oscurecer los bordes grisáceos de escaneos viejos.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EdgeDarkening {
    pub enabled: bool,
    pub mode: EdgeMode,
    /// [0, 100]
    pub amount: f64,
    /// Ancho de la banda, en % del ancho de la carta.
    pub width_pct: f64,
    /// No tocar cartas de borde blanco, sin borde o de arte completo.
    pub auto_detect: bool,
    pub apply_scryfall: bool,
    pub apply_uploads: bool,
    pub apply_backs: bool,
}

impl Default for EdgeDarkening {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: EdgeMode::ContrastEdges,
            amount: 80.0,
            width_pct: 8.0,
            auto_detect: true,
            apply_scryfall: true,
            apply_uploads: false,
            apply_backs: false,
        }
    }
}

/// Impresión a doble cara.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Duplex {
    #[default]
    Off,
    LongEdge,
    ShortEdge,
}

/// Qué hacer con las cartas de doble cara.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DfcFilter {
    /// Se imprimen todas.
    #[default]
    All,
    /// Las DFC se quitan del plan (y del PDF), para imprimirlas aparte.
    HideDfc,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Quality {
    Dpi300,
    #[default]
    Dpi600,
    Dpi900,
}

impl Quality {
    pub fn dpi(self) -> f64 {
        match self {
            Quality::Dpi300 => 300.0,
            Quality::Dpi600 => 600.0,
            Quality::Dpi900 => 900.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ImageFormat {
    #[default]
    Jpeg,
    Png,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Encoding {
    pub format: ImageFormat,
    /// 1..=100 (solo JPEG).
    pub jpeg_quality: u8,
}

impl Default for Encoding {
    fn default() -> Self {
        Self {
            format: ImageFormat::Jpeg,
            jpeg_quality: 95,
        }
    }
}

/// Desplazamiento de una fila, en milímetros.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Offset {
    pub x_mm: f64,
    pub y_mm: f64,
}

/// Desplazamientos por fila. 🛑 Los del reverso se expresan **en el marco de
/// la cara frontal** e indexados por la fila FRONTAL (lo que se ve mirando la
/// hoja al trasluz desde delante); `layout::plan` los aplica antes del espejo.
/// Las filas que falten valen 0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct RowOffsets {
    pub front: Vec<Offset>,
    pub back: Vec<Offset>,
}

impl RowOffsets {
    pub fn front_row(&self, r: usize) -> Offset {
        self.front.get(r).copied().unwrap_or_default()
    }
    pub fn back_row(&self, r: usize) -> Offset {
        self.back.get(r).copied().unwrap_or_default()
    }
}

/// Todo lo que decide cómo sale el papel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PrintSettings {
    pub paper: PaperFormat,
    pub landscape: bool,
    pub card_size: CardSize,
    pub min_margin_mm: f64,
    pub gap_mm: f64,
    pub bleed_mm: f64,
    pub bleed_on_backs: bool,
    pub bleed_mode: BleedMode,
    /// Pintar del color del borde las esquinas redondeadas (transparentes o
    /// claras) de las imágenes. Apagado de fábrica (Pablo, 13/10/2026): las
    /// esquinas transparentes salen del color del papel.
    #[serde(default)]
    pub fill_corners: bool,
    pub cut_guides: CutGuides,
    pub edge_darkening: EdgeDarkening,
    pub duplex: Duplex,
    pub interleave_dfc: bool,
    pub dfc_filter: DfcFilter,
    pub quality: Quality,
    pub encoding: Encoding,
    pub row_offsets: RowOffsets,
    /// Partir el PDF cada N hojas (`None` = un solo fichero).
    pub split_every: Option<u32>,
    /// En qué idioma salen las cartas diseñadas (inglés de fábrica, como la
    /// app; los decks guardados sin este campo siguen en español).
    #[serde(default)]
    pub design_lang: crate::diseno::modelo::DesignLang,
    /// Las cartas grandes (planos, planes, Vanguard) llevan en dúplex el
    /// reverso del deck; si no, el dorso va en blanco (sus reversos son otros).
    #[serde(default)]
    pub big_cards_back: bool,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            paper: PaperFormat::A4,
            landscape: false,
            card_size: CardSize::MTG,
            min_margin_mm: 5.0,
            // Separación en blanco entre cartas, como fabricard: cada carta
            // queda a su tamaño exacto (63×88) para las fundas y el sangrado
            // no tapa el hueco (Pablo, 13/10/2026). El sangrado sigue
            // disponible para quien lo quiera.
            gap_mm: 2.0,
            bleed_mm: 0.0,
            bleed_on_backs: true,
            bleed_mode: BleedMode::Clamp,
            fill_corners: false,
            cut_guides: CutGuides::default(),
            edge_darkening: EdgeDarkening::default(),
            duplex: Duplex::Off,
            interleave_dfc: false,
            dfc_filter: DfcFilter::All,
            quality: Quality::Dpi600,
            encoding: Encoding::default(),
            row_offsets: RowOffsets::default(),
            split_every: None,
            design_lang: crate::diseno::modelo::DesignLang::En,
            big_cards_back: false,
        }
    }
}

// ── El deck ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Deck {
    pub id: String,
    pub name: String,
    /// El texto de la lista tal como lo dejó el usuario (es una entrada; el
    /// deck son las cartas).
    pub decklist_text: String,
    pub cards: Vec<DeckCard>,
    pub settings: PrintSettings,
    /// Reverso por defecto (el genérico de la app si no se ha subido otro).
    pub default_back: Option<FaceImage>,
    pub schema_version: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn copia(nombre: &str, marca: Option<MarcaCarta>) -> DeckCard {
        DeckCard {
            id: nombre.into(),
            source: CardSource::Blank,
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: nombre.into(),
                ..CardMeta::default()
            },
            design: None,
            marca,
        }
    }

    #[test]
    fn las_marcadas_no_cuentan() {
        use MarcaCarta::*;
        let cartas = vec![
            copia("Sol Ring", None),
            // Variante de una que ya está: no cuenta.
            copia("Sol Ring", Some(Variante)),
            // Solo variantes: cuenta una.
            copia("Yusri", Some(Variante)),
            copia("yusri", Some(Variante)),
            copia("Counterspell", Some(Opcional)),
            copia("Pyroblast", Some(Banquillo)),
        ];
        let (dentro, ap) = cuentan(&cartas);
        let nombres: Vec<&str> = dentro.iter().map(|c| c.meta.name.as_str()).collect();
        assert_eq!(nombres, ["Sol Ring", "Yusri"]);
        assert_eq!(
            ap,
            Apartadas {
                variantes: 2,
                opcionales: 1,
                banquillo: 1
            }
        );
        assert_eq!(ap.total(), 4);
    }

    #[test]
    fn los_ajustes_por_defecto_son_los_de_fabricard() {
        let s = PrintSettings::default();
        assert_eq!(s.paper, PaperFormat::A4);
        assert_eq!(s.gap_mm, 2.0);
        // Hueco en blanco entre cartas; guías apagadas (Pablo, 13/10/2026).
        assert_eq!(s.bleed_mm, 0.0);
        assert!(!s.cut_guides.enabled);
        assert_eq!(s.quality.dpi(), 600.0);
        assert_eq!(s.encoding.jpeg_quality, 95);
        assert_eq!(s.edge_darkening.amount, 80.0);
        assert_eq!(s.cut_guides.length_mm, 5.0);
    }

    #[test]
    fn el_origen_de_una_carta_viaja_etiquetado() {
        let c = CardSource::Scryfall {
            scryfall_id: "abc".into(),
        };
        let v = serde_json::to_value(&c).unwrap();
        assert_eq!(v["kind"], "scryfall");
        assert_eq!(v["scryfallId"], "abc");
        let ida_y_vuelta: CardSource = serde_json::from_value(v).unwrap();
        assert_eq!(ida_y_vuelta, c);
    }

    #[test]
    fn los_campos_van_en_camel_case() {
        let v = serde_json::to_value(PrintSettings::default()).unwrap();
        assert!(v.get("minMarginMm").is_some());
        assert_eq!(v["duplex"], "off");
        assert_eq!(v["quality"], "dpi600");
    }

    #[test]
    fn faltan_filas_de_desplazamiento_y_valen_cero() {
        let r = RowOffsets {
            front: vec![Offset {
                x_mm: 1.0,
                y_mm: 0.0,
            }],
            back: vec![],
        };
        assert_eq!(r.front_row(0).x_mm, 1.0);
        assert_eq!(r.front_row(5), Offset::default());
        assert_eq!(r.back_row(0), Offset::default());
    }
}
