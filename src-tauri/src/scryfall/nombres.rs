//! **Todos los nombres de carta de Scryfall** (`/catalog/card-names`, ~30 000
//! nombres en inglés), para reconocer en local cualquier línea que el OCR lea
//! en una carta propia: el título, el pie (donde muchas ponen el nombre
//! original) o cualquier otra.
//!
//! Se descarga la primera vez que hace falta a `app_cache_dir` (se puede
//! borrar sin perder nada) y se renueva si tiene más de una semana.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::OnceCell;

use crate::commands::identify::{normalizar, parecido};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::Scryfall;
use crate::store::blobs::escribir_atomico;

const FICHERO: &str = "scryfall-card-names.json";
const CADUCA: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Desde cuánto parecido una línea mal leída cuenta como ese nombre.
const UMBRAL: f64 = 0.85;

static INDICE: OnceCell<Arc<Indice>> = OnceCell::const_new();

/// Los nombres, por su forma normalizada. Las cartas de dos caras entran
/// enteras («A // B») y por cada cara.
#[derive(Debug, Default)]
pub struct Indice {
    exactos: HashMap<String, String>,
    /// Los mismos, con las letras que el OCR confunde igualadas (`plegar`).
    plegados: HashMap<String, String>,
}

/// Iguala lo que el OCR confunde: `l`, `1` y `|` con la `i` («lsland»), y el
/// `0` con la `o`.
pub fn plegar(normalizado: &str) -> String {
    normalizado
        .chars()
        .map(|c| match c {
            'l' | '1' | '|' => 'i',
            '0' => 'o',
            c => c,
        })
        .collect()
}

/// Cómo se ha encontrado un nombre en una línea.
#[derive(Debug, Clone, PartialEq)]
pub struct Hallazgo {
    pub nombre: String,
    pub parecido: f64,
}

impl Indice {
    pub fn nuevo(nombres: &[String]) -> Indice {
        let mut exactos = HashMap::new();
        for n in nombres {
            exactos.insert(normalizar(n), n.clone());
            if n.contains("//") {
                for cara in n.split("//") {
                    exactos.entry(normalizar(cara)).or_insert_with(|| n.clone());
                }
            }
        }
        let plegados = exactos
            .iter()
            .map(|(k, v)| (plegar(k), v.clone()))
            .collect();
        Indice { exactos, plegados }
    }

    pub fn len(&self) -> usize {
        self.exactos.len()
    }

    pub fn is_empty(&self) -> bool {
        self.exactos.is_empty()
    }

    /// El nombre que es exactamente esta línea (sin tildes ni signos).
    pub fn exacto(&self, linea: &str) -> Option<String> {
        let n = normalizar(linea);
        self.exactos
            .get(&n)
            .or_else(|| self.plegados.get(&plegar(&n)))
            .cloned()
    }

    /// El nombre con el que empieza la línea, el más largo («Jodah, Archmage
    /// Eternal S J» → «Jodah, Archmage Eternal»). Solo nombres de dos
    /// palabras o más, para no confundir «Island» con el principio de una
    /// frase.
    pub fn al_principio(&self, linea: &str) -> Option<String> {
        let n = normalizar(linea);
        let palabras: Vec<&str> = n.split(' ').collect();
        (2..=palabras.len())
            .rev()
            .find_map(|k| self.exactos.get(&palabras[..k].join(" ")).cloned())
    }

    /// El nombre que más se parece, si se parece lo bastante (una letra mal
    /// leída, una que falta…). Recorre todo el catálogo: solo para unas
    /// pocas líneas.
    pub fn parecido(&self, linea: &str) -> Option<Hallazgo> {
        let n = normalizar(linea);
        let largo = n.chars().count();
        if largo < 4 {
            return None;
        }
        let mut mejor: Option<(&str, f64)> = None;
        for (clave, nombre) in &self.exactos {
            let otro = clave.chars().count();
            // Con más de un 15 % de diferencia de largo no puede llegar al umbral.
            if (otro as f64 - largo as f64).abs() > largo.max(otro) as f64 * (1.0 - UMBRAL) {
                continue;
            }
            let p = parecido(&n, clave);
            if p >= UMBRAL && mejor.is_none_or(|(_, m)| p > m) {
                mejor = Some((nombre, p));
            }
        }
        mejor.map(|(nombre, parecido)| Hallazgo {
            nombre: nombre.to_string(),
            parecido,
        })
    }
}

/// El índice, cargado una vez por sesión (del disco o de Scryfall). Si falla
/// (sin red y sin copia), no se queda guardado el fallo: se reintenta la
/// próxima vez.
pub async fn indice(scryfall: &Scryfall, cache: &Path) -> AppResult<Arc<Indice>> {
    INDICE
        .get_or_try_init(|| async {
            let ruta = cache.join(FICHERO);
            let fresca = std::fs::metadata(&ruta)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| SystemTime::now().duration_since(t).ok())
                .is_some_and(|edad| edad < CADUCA);
            let leida = if fresca {
                std::fs::read(&ruta)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
            } else {
                None
            };
            let nombres = match leida {
                Some(n) => n,
                None => match scryfall.card_names().await {
                    Ok(n) if !n.is_empty() => {
                        let json = serde_json::to_vec(&n)
                            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
                        escribir_atomico(&ruta, &json)?;
                        n
                    }
                    // Sin red, la copia vieja vale.
                    otro => std::fs::read(&ruta)
                        .ok()
                        .and_then(|b| serde_json::from_slice::<Vec<String>>(&b).ok())
                        .ok_or_else(|| {
                            otro.err()
                                .unwrap_or_else(|| AppError::new(ErrorCode::Network))
                        })?,
                },
            };
            Ok(Arc::new(Indice::nuevo(&nombres)))
        })
        .await
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indice() -> Indice {
        Indice::nuevo(&[
            "Jodah, Archmage Eternal".into(),
            "Sol Ring".into(),
            "Island".into(),
            "Delver of Secrets // Insectile Aberration".into(),
        ])
    }

    #[test]
    fn exacto_sin_signos_ni_mayusculas() {
        let i = indice();
        assert_eq!(i.len(), 6);
        assert_eq!(
            i.exacto("jodah archmage eternal").as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        assert_eq!(
            i.exacto("Insectile Aberration").as_deref(),
            Some("Delver of Secrets // Insectile Aberration")
        );
        assert_eq!(i.exacto("Rimuru, Rey Demonio"), None);
        // Lo que el OCR confunde.
        assert_eq!(i.exacto("lsland").as_deref(), Some("Island"));
        assert_eq!(i.exacto("S0l Ring").as_deref(), Some("Sol Ring"));
    }

    #[test]
    fn al_principio_de_una_linea() {
        let i = indice();
        assert_eq!(
            i.al_principio("Jodah, Archmage Eternal S J").as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        // Una sola palabra no basta.
        assert_eq!(i.al_principio("Island of the dead"), None);
    }

    #[test]
    fn una_letra_mal_leida() {
        let i = indice();
        let h = i.parecido("Jodah, Archmage Eterna").unwrap();
        assert_eq!(h.nombre, "Jodah, Archmage Eternal");
        assert!(h.parecido > 0.9);
        assert_eq!(i.parecido("Criatura legendaria"), None);
    }
}

/// Prueba en vivo (red): `cargo test --lib nombres::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn descarga_y_busca() {
        let dir = std::env::temp_dir().join("pb-nombres");
        std::fs::create_dir_all(&dir).unwrap();
        let s = Scryfall::new().unwrap();
        let i = indice(&s, &dir).await.unwrap();
        println!("{} nombres", i.len());
        assert_eq!(
            i.exacto("Jodah, Archmage Eternal").as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        let t = std::time::Instant::now();
        let h = i.parecido("Jodah, Archmage Eterna").unwrap();
        println!("{h:?} en {:?}", t.elapsed());
        assert_eq!(h.nombre, "Jodah, Archmage Eternal");
    }
}
