//! **Cuánto amenaza una carta**: el peso con el que el fantasma la quiere
//! quitar. Playgroup lo mide en partidas reales (cuántas veces se elimina
//! cada carta al llegar); aquí no hay esos datos y se estima con lo que la
//! app sabe de ella (`InfoCarta`): el coste, si es el comandante y sus
//! funciones (`proxy_core::stats::funciones`). Las rocas de maná y las
//! criaturas que dan maná pesan algo menos que las demás (Playgroup no las
//! quita; en las mesas de Pablo, sí). De las tierras, solo las muy fuertes:
//! las legendarias (Gaea's Cradle, Serra's Sanctum…) y unas pocas más
//! (`TIERRAS_FUERTES`); las básicas y las normales, nunca.

use crate::modelo::{Carta, InfoCarta};

/// Los tipos de la cara de delante de una carta.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tipos {
    pub criatura: bool,
    pub artefacto: bool,
    pub encantamiento: bool,
    pub planeswalker: bool,
    pub batalla: bool,
    pub instantaneo: bool,
    pub conjuro: bool,
    pub tierra: bool,
    pub basica: bool,
}

pub fn tipos(info: &InfoCarta) -> Tipos {
    let t = info.tipo.as_deref().unwrap_or_default().to_lowercase();
    let delante = t.split(" // ").next().unwrap_or_default();
    let tiene = |p: &str| delante.contains(p);
    Tipos {
        criatura: info.es_criatura || tiene("creature"),
        artefacto: tiene("artifact"),
        encantamiento: tiene("enchantment"),
        planeswalker: tiene("planeswalker"),
        batalla: tiene("battle"),
        instantaneo: tiene("instant"),
        conjuro: tiene("sorcery"),
        tierra: info.es_tierra || tiene("land"),
        basica: tiene("basic"),
    }
}

fn tiene_palabra(info: &InfoCarta, carta: Option<&Carta>, palabra: &str) -> bool {
    info.palabras_clave
        .iter()
        .any(|k| k.eq_ignore_ascii_case(palabra))
        || carta.is_some_and(|c| {
            c.palabras_clave
                .iter()
                .any(|k| k.eq_ignore_ascii_case(palabra))
        })
}

/// Antimaleficio o velo: no se le puede apuntar.
pub fn protegida(info: &InfoCarta, carta: Option<&Carta>) -> bool {
    tiene_palabra(info, carta, "Hexproof") || tiene_palabra(info, carta, "Shroud")
}

pub fn indestructible(info: &InfoCarta, carta: Option<&Carta>) -> bool {
    tiene_palabra(info, carta, "Indestructible")
}

/// Tierras que no son legendarias pero se quitan igual (las que más pesan en
/// Commander), por su nombre en inglés.
const TIERRAS_FUERTES: [&str; 14] = [
    "Cabal Coffers",
    "Cabal Stronghold",
    "Field of the Dead",
    "Maze of Ith",
    "Glacial Chasm",
    "Kor Haven",
    "Volrath's Stronghold",
    "Thespian's Stage",
    "Ancient Tomb",
    "Mishra's Workshop",
    "Gemstone Caverns",
    "Homeward Path",
    "Yavimaya, Cradle of Growth",
    "Dark Depths",
];

/// Si una tierra es de las que el fantasma quita.
pub fn tierra_fuerte(info: &InfoCarta, nombre: &str) -> bool {
    let n = nombre.split(" // ").next().unwrap_or(nombre).trim();
    info.es_legendaria
        || info
            .tipo
            .as_deref()
            .is_some_and(|t| t.to_lowercase().contains("legendary"))
        || TIERRAS_FUERTES.iter().any(|t| t.eq_ignore_ascii_case(n))
}

/// Una roca de maná o una criatura pequeña que da maná: pesa menos.
fn solo_da_mana(info: &InfoCarta, t: &Tipos) -> bool {
    info.funciones.iter().any(|f| f == "ramp") && (!t.criatura || info.cmc() <= 2)
}

/// Cuánto multiplica cada función el peso de una carta.
const POR_FUNCION: [(&str, f64); 9] = [
    ("extraTurn", 1.4),
    ("tutor", 1.3),
    ("draw", 1.2),
    ("wipe", 1.2),
    ("anthem", 1.15),
    ("protection", 1.15),
    ("removal", 1.1),
    ("tokens", 1.1),
    ("recursion", 1.1),
];

/// Lo que pesa una tierra fuerte.
const PESO_TIERRA: f64 = 1.5;
/// Cuánto pesa lo que solo da maná respecto a otra carta igual.
const POR_MANA: f64 = 0.7;

/// El peso de una carta como amenaza: ~0,4 (una roca barata) a 2,5 (el
/// comandante caro que roba y busca). 0 = no se quita nunca (las tierras
/// que no son fuertes).
pub fn peso(info: &InfoCarta, nombre: &str, es_ficha: bool) -> f64 {
    let t = tipos(info);
    if t.tierra && !t.criatura && !t.artefacto {
        return if !t.basica && tierra_fuerte(info, nombre) {
            PESO_TIERRA
        } else {
            0.0
        };
    }
    let mv = f64::from(info.cmc().min(7));
    let mut p = 0.55 + 0.15 * mv;
    if info.es_comandante {
        p *= 1.7;
    }
    if t.planeswalker {
        p *= 1.2;
    }
    for (f, k) in POR_FUNCION {
        if info.funciones.iter().any(|x| x == f) {
            p *= k;
        }
    }
    if solo_da_mana(info, &t) {
        p *= POR_MANA;
    }
    if es_ficha {
        p *= 0.6;
    }
    p.min(2.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(tipo: &str, cmc: u16) -> InfoCarta {
        InfoCarta {
            tipo: Some(tipo.into()),
            cmc_decimas: Some(cmc * 10),
            es_criatura: tipo.contains("Creature"),
            es_tierra: tipo.contains("Land"),
            ..InfoCarta::default()
        }
    }

    #[test]
    fn de_las_tierras_solo_las_fuertes() {
        assert_eq!(peso(&info("Basic Land — Forest", 0), "Forest", false), 0.0);
        assert_eq!(peso(&info("Land", 0), "Command Tower", false), 0.0);
        let cuna = InfoCarta {
            es_legendaria: true,
            ..info("Legendary Land", 0)
        };
        assert!(peso(&cuna, "Gaea's Cradle", false) > 1.0);
        assert!(peso(&info("Land", 0), "Cabal Coffers", false) > 1.0);
        // Una legendaria básica (Snow-Covered…) no: solo las fuertes.
        let basica = InfoCarta {
            es_legendaria: true,
            ..info("Legendary Basic Land — Forest", 0)
        };
        assert_eq!(peso(&basica, "Forest", false), 0.0);
    }

    #[test]
    fn rocas_y_elfos_pesan_menos_pero_se_quitan() {
        let roca = InfoCarta {
            funciones: vec!["ramp".into()],
            ..info("Artifact", 2)
        };
        let elfo = InfoCarta {
            funciones: vec!["ramp".into()],
            ..info("Creature — Elf Druid", 1)
        };
        let artefacto = peso(&info("Artifact", 2), "Otro", false);
        assert!(peso(&roca, "Sol Ring", false) > 0.0);
        assert!(peso(&roca, "Sol Ring", false) < artefacto);
        assert!(peso(&elfo, "Llanowar Elves", false) > 0.0);
    }

    #[test]
    fn el_comandante_y_las_funciones_pesan_mas() {
        let oso = peso(&info("Creature — Bear", 2), "Oso", false);
        let jefe = InfoCarta {
            es_comandante: true,
            funciones: vec!["draw".into(), "tutor".into()],
            ..info("Legendary Creature — Human", 4)
        };
        assert!(peso(&jefe, "Jefe", false) > 2.0 * oso);
        assert!(peso(&info("Creature — Bear", 2), "Oso", true) < oso);
        assert!(peso(&jefe, "Jefe", false) <= 2.5);
    }

    #[test]
    fn los_tipos_son_los_de_la_cara_de_delante() {
        let mdfc = InfoCarta {
            es_tierra: false,
            ..info("Creature — Human // Land", 2)
        };
        let t = tipos(&mdfc);
        assert!(t.criatura && !t.tierra);
        assert!(tipos(&info("Artifact Creature — Golem", 3)).artefacto);
    }
}
