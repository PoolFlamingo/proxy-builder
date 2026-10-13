//! **Qué hace cada carta** en la partida (rampa, robo, eliminación…), leído
//! del texto de reglas en inglés de Scryfall. Es orientativo: lo que una
//! persona clasificaría a ojo, con patrones de texto. Una carta puede tener
//! varias funciones.

use std::sync::LazyLock;

use regex::Regex;

/// Las funciones, en el orden en que se enseñan.
pub const FUNCIONES: [&str; 14] = [
    "ramp",
    "draw",
    "removal",
    "wipe",
    "counter",
    "tutor",
    "protection",
    "recursion",
    "tokens",
    "anthem",
    "lifegain",
    "mill",
    "extraTurn",
    "landDenial",
];

/// Lo que hace falta de una carta para saber qué hace.
pub struct Texto<'a> {
    pub oracle: &'a str,
    pub keywords: &'a [String],
    pub produced_mana: &'a [String],
    pub es_tierra: bool,
}

static RECORDATORIO: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"\([^)]*\)").ok());

/// Los patrones de cada función, compilados una vez.
struct Patrones {
    rampa: Vec<Regex>,
    robo: Vec<Regex>,
    eliminacion: Vec<Regex>,
    barrido: Vec<Regex>,
    contra: Vec<Regex>,
    proteccion: Vec<Regex>,
    recursion: Vec<Regex>,
    fichas: Vec<Regex>,
    himno: Vec<Regex>,
    vida: Vec<Regex>,
    moler: Vec<Regex>,
    turno: Vec<Regex>,
    tierras: Vec<Regex>,
}

/// Los que no compilan se quedan fuera (un test comprueba que no hay ninguno).
fn compilar(p: &[&str]) -> Vec<Regex> {
    p.iter().filter_map(|p| Regex::new(p).ok()).collect()
}

const FUENTES: [&[&str]; 13] = [
    &[
        r"search your library for [^.]*?(land|forest|island|swamp|mountain|plains)[^.]*?onto the battlefield",
        r"put (?:a|an|up to \w+|\w+) (?:basic )?lands? cards? [^.]*?onto the battlefield",
        r"play an additional land|play two additional lands",
        r"create (?:a|an|one|two|three|x|\d+|that many) (?:tapped )?treasure tokens?",
        r"spells? you cast cost \{\d+\} less",
    ],
    &[
        r"\bdraws? (?:a|an|one|two|three|four|five|six|seven|eight|x|\d+|that many)(?: additional)? cards?",
        r"\bdraws? cards equal",
        r"exile the top (?:card|\w+ cards) of your library[^.]*\.[^.]*(?:you may (?:play|cast)|until)",
    ],
    &[
        r"(?:destroy|exile) (?:up to (?:one|two|three|x|\d+) )?(?:another )?target (?:[\w-]+ )*?(?:creature|permanent|artifact|enchantment|planeswalker|land|battle)s?(?P<tuyo> you control)?",
        r"deals? [^.]*?damage to (?:any target|(?:up to \w+ )?(?:another )?target (?:(?:attacking|blocking|tapped|or) )*(?:creature|planeswalker|permanent|battle))",
        r"target (?:[\w-]+ )*?creature (?:an opponent controls )?gets -\d+/-\d+|target (?:[\w-]+ )*?creature (?:an opponent controls )?gets -x/-x",
        r"\bfights? (?:up to one )?(?:another )?target",
        r"return (?:up to \w+ )?(?:another )?target (?:nonland )?(?:creature|permanent|artifact|enchantment|planeswalker)s?(?P<tuyo> you control)?[^.]*? to (?:its|their) owner(?:'s|s'|s)? hands?",
        r"target (?:player|opponent) sacrifices",
    ],
    &[
        r"(?:destroy|exile) all (?:other )?(?:[\w-]+ )*?(?:creatures|permanents|artifacts|enchantments|planeswalkers)(?P<tuyo> you control| blocked)?",
        r"deals? (?:\d+|x) damage to each (?:other )?creature",
        r"all (?:other )?creatures get -\d+/-\d+|all (?:other )?creatures get -x/-x",
        r"return all (?:other )?(?:nonland )?(?:permanents|creatures)[^.]* to (?:their|its) owner(?:'s|s'|s)? hands?",
        r"each (?:player|opponent) sacrifices (?:all|\w+) creatures",
        r"sacrifices all (?:other )?creatures",
    ],
    &[r"counter target (?:[\w,-]+ )*?(?:spell|ability)"],
    &[
        r"(?:gains?|have|has|gain) (?:[\w ,]*?)(?:hexproof|indestructible|shroud|protection from|ward)",
        r"\bphases? out",
        r"can't be the targets? of",
        r"exile (?:each|any number of target|target|up to \w+ target) creatures? you control[^.]*\. return",
        r"spells? you control can't be countered",
    ],
    &[
        r"from (?:your|a|any) graveyard (?:to|onto) (?:the battlefield|your hand)",
        r"from your graveyard[^.]*?(?:into|to) your hand",
        r"(?:you may )?(?:cast|play) [^.]*?from your graveyard",
        r"from (?:a|your) graveyard on top of",
    ],
    &[r"create (?:a|an|one|two|three|four|five|six|x|\d+|that many|a number of)\b[^.]*?\btokens?"],
    &[r"creatures you control(?: with [\w ]+?)? (?:get|have) \+\d+/\+\d+"],
    &[
        r"you gain (?:\d+|x|that much|life equal|an amount of)",
        r"(?:gains?|have|has) [\w ,]*?lifelink",
    ],
    &[
        r"\bmills?\b",
        r"puts? the top [^.]*? of (?:their|his or her|target player's) library into (?:their|his or her|that player's) graveyard",
    ],
    &[r"extra turn"],
    &[
        r"destroy all (?:nonbasic )?lands",
        r"each (?:player|opponent) sacrifices [^.]*?lands",
        r"lands (?:your opponents control |players control )?don't untap",
        r"(?:destroy|exile) all [^.]*?\blands\b",
    ],
];

static P: LazyLock<Patrones> = LazyLock::new(|| Patrones {
    rampa: compilar(FUENTES[0]),
    robo: compilar(FUENTES[1]),
    eliminacion: compilar(FUENTES[2]),
    barrido: compilar(FUENTES[3]),
    contra: compilar(FUENTES[4]),
    proteccion: compilar(FUENTES[5]),
    recursion: compilar(FUENTES[6]),
    fichas: compilar(FUENTES[7]),
    himno: compilar(FUENTES[8]),
    vida: compilar(FUENTES[9]),
    moler: compilar(FUENTES[10]),
    turno: compilar(FUENTES[11]),
    tierras: compilar(FUENTES[12]),
});

/// El texto, en minúsculas y sin los recordatorios entre paréntesis.
pub fn normalizar(oracle: &str) -> String {
    let t = oracle.to_lowercase();
    match RECORDATORIO.as_ref() {
        Some(r) => r.replace_all(&t, "").into_owned(),
        None => t,
    }
}

fn alguno(patrones: &[Regex], texto: &str) -> bool {
    patrones.iter().any(|r| r.is_match(texto))
}

/// Como `alguno`, pero no cuenta lo que se hace a lo propio («exile target
/// creature you control» es un parpadeo, no una eliminación).
fn alguno_ajeno(patrones: &[Regex], texto: &str) -> bool {
    patrones
        .iter()
        .any(|r| r.captures_iter(texto).any(|c| c.name("tuyo").is_none()))
}

/// ¿Busca en la biblioteca algo que no sea una tierra?
fn es_tutor(texto: &str) -> bool {
    texto
        .match_indices("search your library for")
        .any(|(i, f)| {
            let resto = &texto[i + f.len()..];
            let hasta = resto.find(['.', ',']).unwrap_or(resto.len());
            let busca = &resto[..hasta];
            !["land", "forest", "island", "swamp", "mountain", "plains"]
                .iter()
                .any(|t| busca.contains(t))
        })
}

/// Las funciones de una carta (claves de `FUNCIONES`).
pub fn funciones(c: &Texto) -> Vec<&'static str> {
    let t = normalizar(c.oracle);
    let palabra = |k: &str| c.keywords.iter().any(|w| w.eq_ignore_ascii_case(k));
    let mut f = Vec::new();
    // Las tierras dan maná, pero eso no es rampa.
    let rampa = !c.es_tierra && (!c.produced_mana.is_empty() || alguno(&P.rampa, &t));
    let pares: [(&str, bool); 14] = [
        ("ramp", rampa),
        ("draw", alguno(&P.robo, &t)),
        ("removal", alguno_ajeno(&P.eliminacion, &t)),
        ("wipe", alguno_ajeno(&P.barrido, &t)),
        ("counter", alguno(&P.contra, &t)),
        ("tutor", es_tutor(&t)),
        ("protection", alguno(&P.proteccion, &t)),
        ("recursion", alguno(&P.recursion, &t)),
        ("tokens", alguno(&P.fichas, &t)),
        ("anthem", alguno(&P.himno, &t)),
        ("lifegain", palabra("Lifelink") || alguno(&P.vida, &t)),
        ("mill", palabra("Mill") || alguno(&P.moler, &t)),
        ("extraTurn", alguno(&P.turno, &t)),
        ("landDenial", alguno(&P.tierras, &t)),
    ];
    for (k, si) in pares {
        if si {
            f.push(k);
        }
    }
    f
}

/// ¿Admite el mazo cualquier número de copias? («A deck can have any number
/// of cards named…», o hasta siete / nueve).
pub fn sin_limite_de_copias(oracle: &str) -> bool {
    let t = oracle.to_lowercase();
    t.contains("a deck can have any number of cards named") || t.contains("a deck can have up to")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(oracle: &str) -> Vec<&'static str> {
        funciones(&Texto {
            oracle,
            keywords: &[],
            produced_mana: &[],
            es_tierra: false,
        })
    }

    #[test]
    fn rampa() {
        assert!(funciones(&Texto {
            oracle: "{T}: Add {C}{C}.",
            keywords: &[],
            produced_mana: &["C".into()],
            es_tierra: false,
        })
        .contains(&"ramp"));
        assert!(f("Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.").contains(&"ramp"));
        assert!(!f("Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.").contains(&"tutor"));
        assert!(f("When this enters, create a Treasure token.").contains(&"ramp"));
        // Una tierra no es rampa.
        assert!(funciones(&Texto {
            oracle: "{T}: Add {G}.",
            keywords: &[],
            produced_mana: &["G".into()],
            es_tierra: true,
        })
        .is_empty());
    }

    #[test]
    fn robo_eliminacion_y_barrido() {
        assert_eq!(f("Draw two cards."), vec!["draw"]);
        assert_eq!(
            f("Destroy target creature. Its controller loses 2 life."),
            vec!["removal"]
        );
        assert_eq!(
            f("Swords to Plowshares deals 3 damage to any target."),
            vec!["removal"]
        );
        assert_eq!(
            f("Destroy all creatures. They can't be regenerated."),
            vec!["wipe"]
        );
        assert_eq!(f("Counter target spell."), vec!["counter"]);
        // Parpadear lo propio no es eliminar.
        assert!(!f("Exile target creature you control, then return it to the battlefield under its owner's control.").contains(&"removal"));
        // El recordatorio no cuenta.
        assert!(f("Cycling {2} (Pay {2}, Discard this card: Draw a card.)").is_empty());
    }

    #[test]
    fn tutores_y_lo_demas() {
        assert_eq!(f("Search your library for a card, put that card into your hand, then shuffle. You lose 2 life."), vec!["tutor"]);
        assert!(f("Target player takes an extra turn after this one.").contains(&"extraTurn"));
        assert!(f("Destroy all lands.").contains(&"landDenial"));
        assert!(f("Creatures you control get +1/+1.").contains(&"anthem"));
        assert!(f("Create two 1/1 white Soldier creature tokens.").contains(&"tokens"));
        assert!(f(
            "Target creature you control gains hexproof and indestructible until end of turn."
        )
        .contains(&"protection"));
        assert!(
            f("Return target creature card from your graveyard to your hand.")
                .contains(&"recursion")
        );
        assert!(f("You gain 3 life.").contains(&"lifegain"));
        assert!(f("Target player mills three cards.").contains(&"mill"));
    }

    #[test]
    fn casos_de_un_deck_real() {
        // Daño a «attacking or blocking», devolver a «its owner's hand».
        assert!(f("Channel — {2}{W}, Discard this card: It deals 4 damage to target attacking or blocking creature.").contains(&"removal"));
        assert!(f(
            "Return target artifact, creature, enchantment, or planeswalker to its owner's hand."
        )
        .contains(&"removal"));
        // Contrahechizos con varios tipos separados por comas.
        assert!(f("Counter target enchantment, instant, or sorcery spell. Its controller creates a 2/2 blue Bird creature token with flying.").contains(&"counter"));
        // El que crea la ficha es el rival: no son fichas propias.
        assert!(
            !f("Counter target spell. Its controller creates a 2/2 token.").contains(&"tokens")
        );
        // Exiliar lo que bloquea no es un barrido.
        assert!(
            !f("At end of combat, exile all creatures blocked by this creature.").contains(&"wipe")
        );
        assert!(f("Each player chooses any number of creatures they control with total power 4 or less, then sacrifices all other creatures they control.").contains(&"wipe"));
        // Parpadeo de protección y «no se puede contrarrestar».
        assert!(f("Exile any number of target creatures you control. Return those cards to the battlefield under their owner's control at the beginning of the next end step.").contains(&"protection"));
        assert!(f("Spells you control can't be countered this turn.").contains(&"protection"));
        assert!(
            f("Put target card from a graveyard on top of its owner's library.")
                .contains(&"recursion")
        );
        assert!(f("Other creatures you control with defender get +0/+2.").contains(&"anthem"));
        assert!(
            f("Green spells and blue spells you cast cost {1} less to cast.").contains(&"ramp")
        );
    }

    #[test]
    fn todos_los_patrones_compilan() {
        let total: usize = FUENTES.iter().map(|f| f.len()).sum();
        let p = &*P;
        let compilados = [
            p.rampa.len(),
            p.robo.len(),
            p.eliminacion.len(),
            p.barrido.len(),
            p.contra.len(),
            p.proteccion.len(),
            p.recursion.len(),
            p.fichas.len(),
            p.himno.len(),
            p.vida.len(),
            p.moler.len(),
            p.turno.len(),
            p.tierras.len(),
        ];
        assert_eq!(compilados.iter().sum::<usize>(), total);
    }

    #[test]
    fn copias_sin_limite() {
        assert!(sin_limite_de_copias(
            "A deck can have any number of cards named Relentless Rats."
        ));
        assert!(sin_limite_de_copias(
            "A deck can have up to seven cards named Seven Dwarves."
        ));
        assert!(!sin_limite_de_copias("Flying"));
    }
}
