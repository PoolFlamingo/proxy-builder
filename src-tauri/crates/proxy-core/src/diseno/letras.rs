//! **Letras**: el catálogo de familias que usa el motor. Solo las que se
//! cargan aquí (nunca las del sistema), para que una carta salga igual en
//! cualquier equipo.

use parley::fontique::{Blob, Collection, CollectionOptions, SourceCache};
use parley::FontContext;

use super::DisenoError;

/// Las letras cargadas, listas para maquetar.
pub struct Letras {
    pub(crate) fcx: FontContext,
}

impl Default for Letras {
    fn default() -> Self {
        Letras::nuevas()
    }
}

impl Letras {
    /// Un catálogo vacío, sin letras del sistema.
    pub fn nuevas() -> Letras {
        Letras {
            fcx: FontContext {
                collection: Collection::new(CollectionOptions {
                    shared: false,
                    system_fonts: false,
                }),
                source_cache: SourceCache::default(),
            },
        }
    }

    /// Carga un fichero de letra (TTF/OTF, también variable) y devuelve los
    /// nombres de familia que trae.
    pub fn cargar(&mut self, datos: Vec<u8>) -> Result<Vec<String>, DisenoError> {
        let registradas = self.fcx.collection.register_fonts(Blob::from(datos), None);
        if registradas.is_empty() {
            return Err(DisenoError::LetraNoValida);
        }
        let mut nombres = Vec::new();
        for (id, _) in registradas {
            if let Some(n) = self.fcx.collection.family_name(id) {
                if !nombres.iter().any(|x: &String| x == n) {
                    nombres.push(n.to_string());
                }
            }
        }
        Ok(nombres)
    }

    /// Si hay alguna familia con ese nombre.
    pub fn tiene(&mut self, familia: &str) -> bool {
        self.fcx.collection.family_id(familia).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carga_la_letra_de_la_casa_y_rechaza_basura() {
        let mut l = Letras::nuevas();
        let n = l
            .cargar(include_bytes!("../../assets/fuentes/Caladea-Regular.ttf").to_vec())
            .unwrap();
        assert_eq!(n, vec!["Caladea".to_string()]);
        assert!(l.tiene("Caladea"));
        assert!(!l.tiene("Arial"));
        assert_eq!(
            l.cargar(b"esto no es una letra".to_vec()),
            Err(DisenoError::LetraNoValida)
        );
    }
}
