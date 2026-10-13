//! Lo que se lee de Scryfall. Solo los campos que usa la app; el resto se
//! ignora (y se guarda entero en la caché como JSON).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageUris {
    pub png: Option<String>,
    pub large: Option<String>,
    pub normal: Option<String>,
    pub small: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardFace {
    #[serde(default)]
    pub name: String,
    pub image_uris: Option<ImageUris>,
    pub type_line: Option<String>,
    #[serde(default)]
    pub colors: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelatedPart {
    pub id: String,
    pub component: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prices {
    pub eur: Option<String>,
    pub eur_foil: Option<String>,
    pub usd: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScryCard {
    pub id: String,
    pub oracle_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub lang: String,
    #[serde(default)]
    pub set: String,
    #[serde(default)]
    pub set_name: String,
    #[serde(default)]
    pub collector_number: String,
    #[serde(default)]
    pub layout: String,
    pub image_uris: Option<ImageUris>,
    #[serde(default)]
    pub card_faces: Vec<CardFace>,
    #[serde(default)]
    pub highres_image: bool,
    pub image_status: Option<String>,
    #[serde(default)]
    pub all_parts: Vec<RelatedPart>,
    #[serde(default)]
    pub prices: Prices,
    pub cmc: Option<f64>,
    #[serde(default)]
    pub colors: Vec<String>,
    #[serde(default)]
    pub color_identity: Vec<String>,
    pub type_line: Option<String>,
    pub rarity: Option<String>,
    #[serde(default)]
    pub legalities: BTreeMap<String, String>,
    pub printed_name: Option<String>,
    pub released_at: Option<String>,
}

impl ScryCard {
    /// Imagen de la cara `i` (0 = frente). Las cartas de una cara tienen los
    /// `image_uris` arriba; las DFC, en cada `card_faces[i]`.
    pub fn face_uris(&self, i: usize) -> Option<&ImageUris> {
        if i == 0 {
            if let Some(u) = &self.image_uris {
                return Some(u);
            }
        }
        self.card_faces.get(i).and_then(|f| f.image_uris.as_ref())
    }

    /// ¿Tiene dos caras con imagen propia? (transform, modal_dfc, reversible…)
    pub fn is_dfc(&self) -> bool {
        self.image_uris.is_none()
            && self.card_faces.len() >= 2
            && self
                .card_faces
                .iter()
                .take(2)
                .all(|f| f.image_uris.is_some())
    }

    /// La mejor imagen para imprimir: el PNG (745×1040); si no, `large`.
    pub fn print_url(&self, cara: usize) -> Option<(String, bool)> {
        let u = self.face_uris(cara)?;
        u.png
            .clone()
            .map(|p| (p, true))
            .or_else(|| u.large.clone().map(|l| (l, false)))
    }

    pub fn tiene_imagen_buena(&self) -> bool {
        !matches!(
            self.image_status.as_deref(),
            Some("missing") | Some("placeholder")
        ) && self.face_uris(0).is_some()
    }

    /// ¿Responde a este nombre? (el completo, o el de alguna de sus caras,
    /// sin distinguir mayúsculas).
    pub fn responde_a(&self, nombre: &str) -> bool {
        let n = nombre.trim().to_lowercase();
        self.name.to_lowercase() == n
            || self
                .printed_name
                .as_deref()
                .map(str::to_lowercase)
                .as_deref()
                == Some(n.as_str())
            || self.card_faces.iter().any(|f| f.name.to_lowercase() == n)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Collection {
    #[serde(default)]
    pub data: Vec<ScryCard>,
    #[serde(default)]
    pub not_found: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchPage {
    #[serde(default)]
    pub data: Vec<ScryCard>,
    #[serde(default)]
    pub has_more: bool,
    pub next_page: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Catalog {
    #[serde(default)]
    pub data: Vec<String>,
}

/// Un set, para el explorador de sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetInfo {
    pub code: String,
    pub name: String,
    pub set_type: String,
    pub released_at: Option<String>,
    pub card_count: u32,
}

/// Un set tal como lo manda Scryfall (en snake_case).
#[derive(Debug, Clone, Deserialize)]
pub struct SetRaw {
    pub code: String,
    pub name: String,
    #[serde(default)]
    pub set_type: String,
    pub released_at: Option<String>,
    #[serde(default)]
    pub card_count: u32,
}

impl From<SetRaw> for SetInfo {
    fn from(s: SetRaw) -> Self {
        SetInfo {
            code: s.code,
            name: s.name,
            set_type: s.set_type,
            released_at: s.released_at,
            card_count: s.card_count,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SetList {
    #[serde(default)]
    pub data: Vec<SetRaw>,
}

/// Una forma de pedir una carta a `/cards/collection`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(untagged)]
pub enum Identifier {
    Id {
        id: String,
    },
    SetNumber {
        set: String,
        collector_number: String,
    },
    NameSet {
        name: String,
        set: String,
    },
    Name {
        name: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_identificador_se_escribe_como_pide_scryfall() {
        let v = serde_json::to_value(Identifier::SetNumber {
            set: "cmm".into(),
            collector_number: "410".into(),
        })
        .unwrap();
        assert_eq!(v, serde_json::json!({"set":"cmm","collector_number":"410"}));
        let v = serde_json::to_value(Identifier::Name {
            name: "Sol Ring".into(),
        })
        .unwrap();
        assert_eq!(v, serde_json::json!({"name":"Sol Ring"}));
    }

    #[test]
    fn una_dfc_lleva_sus_imagenes_en_las_caras() {
        let c: ScryCard = serde_json::from_value(serde_json::json!({
            "id": "x", "name": "Delver of Secrets // Insectile Aberration", "layout": "transform",
            "card_faces": [
                {"name": "Delver of Secrets", "image_uris": {"png": "https://p/0.png"}},
                {"name": "Insectile Aberration", "image_uris": {"png": "https://p/1.png"}}
            ]
        }))
        .unwrap();
        assert!(c.is_dfc());
        assert_eq!(c.print_url(1), Some(("https://p/1.png".into(), true)));
        assert!(c.responde_a("delver of secrets"));
        assert!(c.responde_a("Delver of Secrets // Insectile Aberration"));
    }
}
