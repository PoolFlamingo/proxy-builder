//! **Cuánto amenaza una carta**: el peso con el que el fantasma la quiere
//! quitar. Playgroup lo mide en partidas reales (cuántas veces se elimina
//! cada carta al llegar); aquí no hay esos datos y se estima con lo que la
//! app sabe de ella (`InfoCarta`): el coste, si es el comandante y sus
//! funciones (`proxy_core::stats::funciones`). Las rocas de maná y las
//! criaturas que solo dan maná pesan 0 (Playgroup tampoco las quita: están
//! por debajo de su 1 %), y las tierras nunca.

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

/// Una roca de maná o una criatura que solo da maná: no se quita.
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

/// El peso de una carta como amenaza: ~0,5 (barata y sin más) a 2,5 (el
/// comandante caro que roba y busca). 0 = no se quita nunca.
pub fn peso(info: &InfoCarta, es_ficha: bool) -> f64 {
    let t = tipos(info);
    if t.tierra || solo_da_mana(info, &t) {
        return 0.0;
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
    fn tierras_rocas_y_elfos_no_se_quitan() {
        assert_eq!(peso(&info("Basic Land — Forest", 0), false), 0.0);
        let roca = InfoCarta {
            funciones: vec!["ramp".into()],
            ..info("Artifact", 1)
        };
        assert_eq!(peso(&roca, false), 0.0);
        let elfo = InfoCarta {
            funciones: vec!["ramp".into()],
            ..info("Creature — Elf Druid", 1)
        };
        assert_eq!(peso(&elfo, false), 0.0);
        // Una criatura grande que además da maná sí cuenta.
        let grande = InfoCarta {
            funciones: vec!["ramp".into()],
            ..info("Creature — Dragon", 6)
        };
        assert!(peso(&grande, false) > 1.0);
    }

    #[test]
    fn el_comandante_y_las_funciones_pesan_mas() {
        let oso = peso(&info("Creature — Bear", 2), false);
        let jefe = InfoCarta {
            es_comandante: true,
            funciones: vec!["draw".into(), "tutor".into()],
            ..info("Legendary Creature — Human", 4)
        };
        assert!(peso(&jefe, false) > 2.0 * oso);
        assert!(peso(&info("Creature — Bear", 2), true) < oso);
        assert!(peso(&jefe, false) <= 2.5);
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
