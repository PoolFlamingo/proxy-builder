//! **Fichas en español → su nombre en Scryfall.** Scryfall casi no tiene
//! fichas en otros idiomas, y las fichas se llaman por su tipo («Warrior»,
//! «Treasure»). Una carta propia de ficha en español («Ficha de criatura —
//! Guerrero») se busca por la traducción de su tipo.

use crate::commands::identify::normalizar;

/// Los tipos de ficha más comunes (sin tildes, en minúsculas → inglés).
const TIPOS: &[(&str, &str)] = &[
    ("angel", "Angel"),
    ("ardilla", "Squirrel"),
    ("ave", "Bird"),
    ("avatar", "Avatar"),
    ("bestia", "Beast"),
    ("caballero", "Knight"),
    ("caballo", "Horse"),
    ("centauro", "Centaur"),
    ("ciudadano", "Citizen"),
    ("clerigo", "Cleric"),
    ("comida", "Food"),
    ("conejo", "Rabbit"),
    ("constructo", "Construct"),
    ("construccion", "Construct"),
    ("demonio", "Demon"),
    ("dinosaurio", "Dinosaur"),
    ("dragon", "Dragon"),
    ("elefante", "Elephant"),
    ("elemental", "Elemental"),
    ("elfo", "Elf"),
    ("esfinge", "Sphinx"),
    ("espiritu", "Spirit"),
    ("esqueleto", "Skeleton"),
    ("explorador", "Scout"),
    ("fenix", "Phoenix"),
    ("gato", "Cat"),
    ("germen", "Germ"),
    ("gigante", "Giant"),
    ("golem", "Golem"),
    ("goblin", "Goblin"),
    ("grifo", "Griffin"),
    ("guerrero", "Warrior"),
    ("hada", "Faerie"),
    ("hidra", "Hydra"),
    ("hongo", "Fungus"),
    ("humano", "Human"),
    ("ilusion", "Illusion"),
    ("insecto", "Insect"),
    ("lobo", "Wolf"),
    ("mago", "Wizard"),
    ("mapa", "Map"),
    ("minotauro", "Minotaur"),
    ("monje", "Monk"),
    ("murcielago", "Bat"),
    ("ogro", "Ogre"),
    ("oso", "Bear"),
    ("pajaro", "Bird"),
    ("pegaso", "Pegasus"),
    ("perro", "Dog"),
    ("pez", "Fish"),
    ("pirata", "Pirate"),
    ("pista", "Clue"),
    ("planta", "Plant"),
    ("rana", "Frog"),
    ("rata", "Rat"),
    ("rinoceronte", "Rhino"),
    ("sabueso", "Hound"),
    ("sangre", "Blood"),
    ("saprolin", "Saproling"),
    ("serpiente", "Snake"),
    ("servo", "Servo"),
    ("simio", "Ape"),
    ("soldado", "Soldier"),
    ("tesoro", "Treasure"),
    ("tiburon", "Shark"),
    ("toptero", "Thopter"),
    ("trasgo", "Goblin"),
    ("triton", "Merfolk"),
    ("vampiro", "Vampire"),
    ("zombi", "Zombie"),
];

/// El nombre en Scryfall de una ficha a partir de una palabra (su tipo, en
/// español o ya en inglés).
pub fn nombre_en_ingles(palabra: &str) -> Option<String> {
    let n = normalizar(palabra);
    if n.len() < 3 || n.contains(' ') {
        return None;
    }
    if let Some((_, en)) = TIPOS.iter().find(|(es, _)| *es == n) {
        return Some((*en).to_string());
    }
    // Ya en inglés («Token Creature — Warrior»).
    TIPOS
        .iter()
        .find(|(_, en)| normalizar(en) == n)
        .map(|(_, en)| (*en).to_string())
}

/// Los nombres de ficha que se deducen de lo leído: la última palabra de la
/// línea de tipo («Ficha de criatura Guerrero» → `Warrior`) y el título.
pub fn candidatas<'a>(lineas: impl IntoIterator<Item = &'a String>) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for l in lineas {
        let palabras: Vec<&str> = l
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let probar = if palabras.len() == 1 {
            palabras.first().copied()
        } else if crate::ocr::es_de_ficha(l) {
            palabras.last().copied()
        } else {
            None
        };
        if let Some(en) = probar.and_then(nombre_en_ingles) {
            if !v.contains(&en) {
                v.push(en);
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traduce_el_tipo_de_la_ficha() {
        assert_eq!(nombre_en_ingles("Guerrero").as_deref(), Some("Warrior"));
        assert_eq!(nombre_en_ingles("TESORO").as_deref(), Some("Treasure"));
        assert_eq!(nombre_en_ingles("Dragón").as_deref(), Some("Dragon"));
        assert_eq!(nombre_en_ingles("Warrior").as_deref(), Some("Warrior"));
        assert_eq!(nombre_en_ingles("Copia"), None);
    }

    #[test]
    fn de_lo_leido() {
        let l: Vec<String> = [
            "GUERRERO",
            "Ficha de criatura Guerrero",
            "Ficha de artefacto -Tesoro",
            "Defendemos Con honor lo que juramos",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
        assert_eq!(candidatas(&l), vec!["Warrior", "Treasure"]);
    }
}
