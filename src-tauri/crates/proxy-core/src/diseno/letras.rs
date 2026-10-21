//! **Letras**: el catálogo de familias que usa el motor. Solo las que se
//! cargan aquí (nunca las del sistema), para que una carta salga igual en
//! cualquier equipo.

use std::collections::HashSet;

use parley::fontique::{
    Blob, Collection, CollectionOptions, FallbackKey, FamilyId, GenericFamily, Language, Script,
    SourceCache,
};
use parley::FontContext;

use super::modelo::Escritura;
use super::DisenoError;

/// Las letras cargadas, listas para maquetar.
pub struct Letras {
    pub(crate) fcx: FontContext,
    /// Las escrituras de Asia oriental con letras cargadas (`cargar_cjk`).
    cjk: HashSet<Escritura>,
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
            cjk: HashSet::new(),
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

    /// Carga la letra de emoji (en color) y la deja como la familia genérica
    /// de emoji: `parley` la usa para los emoji que la letra del texto no
    /// tiene, que es todos.
    pub fn cargar_emoji(&mut self, datos: Vec<u8>) -> Result<(), DisenoError> {
        let mut ids: Vec<FamilyId> = Vec::new();
        for (id, _) in self.fcx.collection.register_fonts(Blob::from(datos), None) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        if ids.is_empty() {
            return Err(DisenoError::LetraNoValida);
        }
        self.fcx
            .collection
            .set_generic_families(GenericFamily::Emoji, ids.into_iter());
        Ok(())
    }

    /// Deja `familias` (ya cargadas) de respaldo para el latín y el
    /// cirílico: una letra de un pack sin «ș», «ł» o el alfabeto ruso los toma
    /// de ellas en vez de pintar cuadrados. Solo actúa en los caracteres que
    /// la letra pedida no tiene.
    pub fn respaldos(&mut self, familias: &[&str]) {
        let ids: Vec<FamilyId> = familias
            .iter()
            .filter_map(|f| self.fcx.collection.family_id(f))
            .collect();
        for escritura in [*b"Latn", *b"Cyrl"] {
            self.fcx.collection.append_fallbacks(
                FallbackKey::new(Script::from_bytes(escritura), None),
                ids.iter().copied(),
            );
        }
    }

    /// Carga una letra de Asia oriental (Noto CJK, bajada aparte) y la deja de
    /// respaldo de su escritura y su idioma: así el mismo ideograma sale con
    /// la forma japonesa o china según el idioma del texto.
    pub fn cargar_cjk(&mut self, datos: Vec<u8>, escritura: Escritura) -> Result<(), DisenoError> {
        let mut ids: Vec<FamilyId> = Vec::new();
        for (id, _) in self.fcx.collection.register_fonts(Blob::from(datos), None) {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        if ids.is_empty() {
            return Err(DisenoError::LetraNoValida);
        }
        let (locale, escrituras): (&str, &[[u8; 4]]) = match escritura {
            Escritura::Japonesa => ("ja", &[*b"Hani", *b"Hira", *b"Kana"]),
            Escritura::Coreana => ("ko", &[*b"Hang", *b"Hani"]),
            Escritura::ChinaSimplificada => ("zh-Hans", &[*b"Hani", *b"Bopo"]),
            Escritura::ChinaTradicional => ("zh-Hant", &[*b"Hani", *b"Bopo"]),
            Escritura::Latina | Escritura::Cirilica => return Ok(()),
        };
        let lengua = Language::parse(locale).ok();
        for e in escrituras {
            let script = Script::from_bytes(*e);
            self.fcx.collection.append_fallbacks(
                FallbackKey::new(script, lengua.as_ref()),
                ids.iter().copied(),
            );
            // Sin idioma (un ideograma suelto en una carta en español, por
            // ejemplo), la primera que se cargó.
            self.fcx
                .collection
                .append_fallbacks(FallbackKey::new(script, None), ids.iter().copied());
        }
        self.cjk.insert(escritura);
        Ok(())
    }

    /// Si hay letras para pintar `escritura` (las latinas y cirílicas, siempre).
    pub fn tiene_escritura(&self, escritura: Escritura) -> bool {
        matches!(escritura, Escritura::Latina | Escritura::Cirilica)
            || self.cjk.contains(&escritura)
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
#[cfg(test)]
mod cobertura {
    use skrifa::{FontRef, MetadataProvider};

    /// Lo que piden el rumano, el polaco, el ruso y el ucraniano (y las
    /// comillas y rayas de todos). Las letras de la casa lo tienen todo: son
    /// el respaldo (`Letras::respaldos`) de las de los packs que no.
    const TEXTO: &str = "șțȘȚăâîĂÂÎąęłśźżćńóĄĘŁŚŹŻĆŃÓ\
        АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюя\
        іїєґІЇЄҐ’—«»„“”";

    #[test]
    fn las_letras_de_la_casa_tienen_rumano_polaco_y_cirilico() {
        for (n, d) in [
            (
                "Spectral-Regular",
                &include_bytes!("../../assets/fuentes/Spectral-Regular.ttf")[..],
            ),
            (
                "Spectral-Italic",
                &include_bytes!("../../assets/fuentes/Spectral-Italic.ttf")[..],
            ),
            (
                "Spectral-SemiBold",
                &include_bytes!("../../assets/fuentes/Spectral-SemiBold.ttf")[..],
            ),
            (
                "Spectral-Bold",
                &include_bytes!("../../assets/fuentes/Spectral-Bold.ttf")[..],
            ),
            (
                "Montserrat",
                &include_bytes!("../../assets/fuentes/Montserrat-Variable.ttf")[..],
            ),
        ] {
            let f = FontRef::new(d).unwrap();
            let faltan: String = TEXTO
                .chars()
                .filter(|c| f.charmap().map(*c).is_none())
                .collect();
            assert!(faltan.is_empty(), "a {n} le faltan «{faltan}»");
        }
    }
}
