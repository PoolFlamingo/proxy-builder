//! **La base de fichas de Cockatrice** (`Cockatrice/Magic-Token`,
//! `tokens.xml`, licencia Unlicense): unas 1000 fichas, y unas 80 que Scryfall
//! no tiene («Kelp», la Planta Muro 0/1 de *Wall of Kelp*), con su tipo,
//! texto, F/R, imagen y las cartas que las crean (`reverse-related`).
//!
//! Solo se guardan las que **no** tienen ninguna imagen en Scryfall: las demás
//! ya las da Scryfall, y saldrían repetidas. Se baja a `app_cache_dir` y se
//! renueva cada semana, como el catálogo de nombres de Scryfall.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use proxy_core::model::FuenteCarta;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

use super::Externo;
use crate::commands::identify::normalizar;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::blobs::escribir_atomico;

const URL: &str = "https://raw.githubusercontent.com/Cockatrice/Magic-Token/master/tokens.xml";
const FICHERO: &str = "cockatrice-tokens.xml";
const CADUCA: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Tope del catálogo (hoy pesa 1,3 MB).
const MAX_CATALOGO: usize = 8 * 1024 * 1024;

/// Una ficha de Cockatrice que Scryfall no tiene.
#[derive(Debug, Clone, PartialEq)]
pub struct FichaCockatrice {
    /// Estable entre descargas (el XML no trae ids): huella de nombre, tipo,
    /// texto y F/R.
    pub id: String,
    pub nombre: String,
    pub tipo: String,
    pub texto: String,
    pub fr: Option<String>,
    /// Letras de color («U», «WB»).
    pub colores: String,
    /// La colección y la URL de cada imagen (puede no haber ninguna).
    pub imagenes: Vec<(String, String)>,
    /// Las cartas que la crean.
    pub creadores: Vec<String>,
}

impl FichaCockatrice {
    pub const FUENTE: FuenteCarta = FuenteCarta::Cockatrice;

    /// La línea de tipo con «Token» delante si no lo lleva (como Scryfall).
    pub fn linea_de_tipo(&self) -> String {
        if self.tipo.to_lowercase().contains("token") {
            self.tipo.clone()
        } else {
            format!("Token {}", self.tipo)
        }
    }
}

/// Las fichas y sus índices.
#[derive(Debug, Default)]
pub struct Catalogo {
    fichas: Vec<FichaCockatrice>,
    por_nombre: HashMap<String, Vec<usize>>,
    por_creador: HashMap<String, Vec<usize>>,
}

fn huella(partes: &[&str]) -> String {
    let h = Sha256::digest(partes.join("\u{1f}").as_bytes());
    h.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

impl Catalogo {
    /// Lee `tokens.xml`. Tolerante: lo que falte en una ficha queda vacío.
    pub fn leer(xml: &str) -> AppResult<Catalogo> {
        let doc = roxmltree::Document::parse(xml).map_err(|e| {
            AppError::new(ErrorCode::InvalidInput)
                .with("motivo", "cockatrice")
                .with("detalle", e)
        })?;
        let hijo = |n: roxmltree::Node, nombre: &str| -> String {
            n.children()
                .find(|c| c.has_tag_name(nombre))
                .and_then(|c| c.text())
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let mut c = Catalogo::default();
        for card in doc.descendants().filter(|n| n.has_tag_name("card")) {
            let nombre = hijo(card, "name");
            if nombre.is_empty() {
                continue;
            }
            let imagenes: Vec<(String, String)> = card
                .children()
                .filter(|n| n.has_tag_name("set"))
                .filter_map(|n| {
                    let url = n.attribute("picURL")?.trim().to_string();
                    (!url.is_empty()).then(|| (n.text().unwrap_or("").trim().to_string(), url))
                })
                .collect();
            // Si Scryfall tiene alguna imagen suya, la ficha ya está en Scryfall.
            if imagenes.iter().any(|(_, u)| u.contains("scryfall.io")) {
                continue;
            }
            let prop = card.children().find(|n| n.has_tag_name("prop"));
            let (tipo, colores, fr) = match prop {
                Some(p) => (hijo(p, "type"), hijo(p, "colors"), hijo(p, "pt")),
                None => (String::new(), String::new(), String::new()),
            };
            let texto = hijo(card, "text");
            let fr = (!fr.is_empty()).then_some(fr);
            let id = huella(&[&nombre, &tipo, &texto, fr.as_deref().unwrap_or("")]);
            let creadores: Vec<String> = card
                .children()
                .filter(|n| n.has_tag_name("reverse-related"))
                .filter_map(|n| n.text())
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            let i = c.fichas.len();
            c.por_nombre.entry(normalizar(&nombre)).or_default().push(i);
            for cr in &creadores {
                c.por_creador.entry(normalizar(cr)).or_default().push(i);
            }
            c.fichas.push(FichaCockatrice {
                id,
                nombre,
                tipo,
                texto,
                fr,
                colores,
                imagenes,
                creadores,
            });
        }
        Ok(c)
    }

    pub fn len(&self) -> usize {
        self.fichas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fichas.is_empty()
    }

    pub fn por_id(&self, id: &str) -> Option<&FichaCockatrice> {
        self.fichas.iter().find(|f| f.id == id)
    }

    /// Las fichas que se llaman exactamente así (sin tildes ni mayúsculas).
    pub fn exactas(&self, nombre: &str) -> Vec<&FichaCockatrice> {
        self.por_nombre
            .get(&normalizar(nombre))
            .map(|v| v.iter().map(|&i| &self.fichas[i]).collect())
            .unwrap_or_default()
    }

    /// Las que llevan el texto en el nombre: primero las exactas, luego las
    /// que empiezan así y luego el resto.
    pub fn buscar(&self, texto: &str) -> Vec<&FichaCockatrice> {
        let t = normalizar(texto);
        if t.chars().count() < 3 {
            return Vec::new();
        }
        let mut v: Vec<(u8, &FichaCockatrice)> = self
            .fichas
            .iter()
            .filter_map(|f| {
                let n = normalizar(&f.nombre);
                let rango = if n == t {
                    0
                } else if n.starts_with(&t) {
                    1
                } else if n.split(' ').any(|p| p.starts_with(&t)) {
                    2
                } else if n.contains(&t) {
                    3
                } else {
                    return None;
                };
                Some((rango, f))
            })
            .collect();
        v.sort_by_key(|(r, f)| (*r, f.nombre.clone()));
        v.into_iter().map(|(_, f)| f).collect()
    }

    /// Las fichas que crea una carta (por su nombre en inglés).
    pub fn creadas_por(&self, carta: &str) -> Vec<&FichaCockatrice> {
        let mut v: Vec<&FichaCockatrice> = Vec::new();
        let caras = std::iter::once(carta).chain(carta.split(" // "));
        for cara in caras {
            for &i in self
                .por_creador
                .get(&normalizar(cara))
                .into_iter()
                .flatten()
            {
                if !v.iter().any(|f| f.id == self.fichas[i].id) {
                    v.push(&self.fichas[i]);
                }
            }
        }
        v
    }
}

/// El catálogo de la sesión (se carga una vez; si falla, se reintenta).
static CATALOGO: Mutex<Option<Arc<Catalogo>>> = Mutex::const_new(None);

/// El catálogo: el del disco si tiene menos de una semana; si no, se baja (y
/// sin red vale el viejo).
pub async fn catalogo(externo: &Externo, cache: &Path) -> AppResult<Arc<Catalogo>> {
    let mut guardado = CATALOGO.lock().await;
    if let Some(c) = guardado.as_ref() {
        return Ok(c.clone());
    }
    let ruta = cache.join(FICHERO);
    let fresco = std::fs::metadata(&ruta)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|edad| edad < CADUCA);
    let leer = || std::fs::read_to_string(&ruta).ok();
    let xml = match (fresco, leer()) {
        (true, Some(x)) => x,
        (_, viejo) => match externo.bajar(URL, MAX_CATALOGO).await {
            Ok(b) => {
                let x = String::from_utf8(b).map_err(|_| {
                    AppError::new(ErrorCode::InvalidInput).with("motivo", "cockatrice")
                })?;
                // Solo se guarda si se entiende.
                Catalogo::leer(&x)?;
                escribir_atomico(&ruta, x.as_bytes())?;
                x
            }
            Err(e) => viejo.ok_or(e)?,
        },
    };
    let c = Arc::new(Catalogo::leer(&xml)?);
    *guardado = Some(c.clone());
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const XML: &str = include_str!("../fixtures/cockatrice-tokens.xml");

    #[test]
    fn solo_las_que_scryfall_no_tiene() {
        let c = Catalogo::leer(XML).unwrap();
        // Goblin tiene imagen de Scryfall: fuera. Kelp y la ficha sin imagen, dentro.
        assert_eq!(c.len(), 2);
        let kelp = &c.exactas("kelp")[0];
        assert_eq!(kelp.tipo, "Token Creature — Plant Wall");
        assert_eq!(kelp.fr.as_deref(), Some("0/1"));
        assert_eq!(kelp.texto, "Defender");
        assert_eq!(kelp.colores, "U");
        assert_eq!(kelp.creadores, vec!["Wall of Kelp"]);
        assert!(kelp.imagenes[0].1.ends_with("Kelp.jpeg"));
        assert!(c.exactas("Goblin").is_empty());
        // El id no cambia entre lecturas.
        assert_eq!(Catalogo::leer(XML).unwrap().exactas("Kelp")[0].id, kelp.id);
        assert_eq!(c.por_id(&kelp.id).map(|f| f.nombre.as_str()), Some("Kelp"));
    }

    #[test]
    fn buscar_y_creadas_por() {
        let c = Catalogo::leer(XML).unwrap();
        assert_eq!(c.buscar("Kel")[0].nombre, "Kelp");
        assert!(c.buscar("ke").is_empty());
        assert_eq!(c.creadas_por("Wall of Kelp")[0].nombre, "Kelp");
        assert!(c.creadas_por("Dragon Fodder").is_empty());
        let sin = &c.exactas("Hechizo de prueba")[0];
        assert!(sin.imagenes.is_empty());
        assert_eq!(sin.linea_de_tipo(), "Token Sorcery");
    }

    #[test]
    fn un_xml_roto_no_vale() {
        assert!(Catalogo::leer("<cards><card>").is_err());
    }
}

/// Prueba en vivo: `cargo test --lib cockatrice::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn baja_el_catalogo_y_la_imagen_de_kelp() {
        let dir = tempfile::tempdir().unwrap();
        let e = Externo::nuevo().unwrap();
        let c = catalogo(&e, dir.path()).await.unwrap();
        println!("{} fichas que Scryfall no tiene", c.len());
        let kelp = c.exactas("Kelp")[0].clone();
        println!("{kelp:?}");
        let b = e
            .bajar(&kelp.imagenes[0].1, super::super::MAX_IMAGEN)
            .await
            .unwrap();
        let limpia = super::super::limpiar_imagen(&b).unwrap();
        println!("imagen: {} bytes → {} bytes", b.len(), limpia.len());
    }
}
