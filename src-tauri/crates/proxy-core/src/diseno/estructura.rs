//! **La estructura del texto de reglas** de los layouts especiales: las
//! habilidades de un planeswalker, los capítulos de una saga, los niveles de
//! una clase… Todo sale del texto oficial (Oracle o impresión española), que
//! sigue patrones fijos. Si un texto no los sigue, queda como texto normal.

use std::sync::LazyLock;

use regex::Regex;

/// Quita lo que no es texto de la carta: las marcas que Scryfall deja en
/// algunas impresiones (`//Level_2//`, `//PRT-Prototype_Mech//`).
pub fn limpiar(texto: &str) -> String {
    texto
        .lines()
        .filter(|l| {
            let t = l.trim();
            !(t.len() > 4 && t.starts_with("//") && t.ends_with("//"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Una habilidad de lealtad (o estática, sin coste).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Habilidad {
    /// `+1`, `−2`, `0`, `+X`… con el signo menos tipográfico.
    pub coste: Option<String>,
    pub texto: String,
}

static LEALTAD: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^([+−\-–]?(?:\d+|X)):\s*(.*)$").unwrap()
});

/// Las habilidades de un planeswalker, una por línea.
pub fn planeswalker(reglas: &str) -> Vec<Habilidad> {
    limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| match LEALTAD.captures(l) {
            Some(c) => Habilidad {
                coste: Some(c[1].replace(['-', '–'], "−")),
                texto: c[2].to_string(),
            },
            None => Habilidad {
                coste: None,
                texto: l.to_string(),
            },
        })
        .collect()
}

/// Un capítulo de una saga.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capitulo {
    /// Los números del capítulo (`I, II —` da `[1, 2]`).
    pub numeros: Vec<u32>,
    pub texto: String,
}

/// Una saga: el recordatorio de arriba y los capítulos.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Saga {
    pub intro: Vec<String>,
    pub capitulos: Vec<Capitulo>,
}

static CAPITULO: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^((?:[IVX]+)(?:\s*,\s*[IVX]+)*)\s*[—–-]\s*(.*)$").unwrap()
});

fn romano(s: &str) -> Option<u32> {
    let v = |c| match c {
        'I' => Some(1),
        'V' => Some(5),
        'X' => Some(10),
        _ => None,
    };
    let mut total = 0i64;
    let cs: Vec<u32> = s.chars().map(v).collect::<Option<_>>()?;
    for (i, &n) in cs.iter().enumerate() {
        if cs.get(i + 1).is_some_and(|&s| s > n) {
            total -= i64::from(n);
        } else {
            total += i64::from(n);
        }
    }
    u32::try_from(total).ok().filter(|n| *n > 0)
}

/// `1` → `I`, `4` → `IV`.
pub fn a_romano(mut n: u32) -> String {
    let mut s = String::new();
    for (v, r) in [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")] {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    s
}

/// Los capítulos de una saga.
pub fn saga(reglas: &str) -> Saga {
    let mut s = Saga::default();
    for l in limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        let cap = CAPITULO.captures(l).and_then(|c| {
            let numeros: Option<Vec<u32>> = c[1].split(',').map(|n| romano(n.trim())).collect();
            numeros.map(|numeros| Capitulo {
                numeros,
                texto: c[2].to_string(),
            })
        });
        match cap {
            Some(c) => s.capitulos.push(c),
            // Lo que va después de un capítulo sin número sigue en él.
            None => match s.capitulos.last_mut() {
                Some(ultimo) => {
                    ultimo.texto.push('\n');
                    ultimo.texto.push_str(l);
                }
                None => s.intro.push(l.to_string()),
            },
        }
    }
    s
}

/// Un tramo con cabecera: un nivel de clase, un umbral de estación, el
/// «Resolver» de un caso, un rango de nivel…
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tramo {
    /// La cabecera (`{1}{U}: Nivel 2`, `8+`, `NIVEL 2-6`…), si la hay.
    pub cabecera: Option<String>,
    /// Fuerza/resistencia del tramo (leveler, estación).
    pub fr: Option<String>,
    pub texto: Vec<String>,
}

/// La cabecera de un nivel de clase (`{2}{U}: Nivel 2`), en cualquier
/// idioma: de *Wizard Class* (AFR) en cada uno; el coreano pone el número
/// delante («2레벨») y el japonés, cifras de ancho completo.
static NIVEL_CLASE: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(
        r"^((?:\{[^}]+\})+)\s*[:：]\s*(?:(?:Level|Nivel|Niveau|Stufe|Livello|Nível|Уровень|Рівень|Poziom|レベル|等级|等級)\s*[\d０-９]+|[\d０-９]+\s*레벨)\s*$",
    )
    .unwrap()
});

/// Los niveles de una clase: el primero sin cabecera.
pub fn clase(reglas: &str) -> Vec<Tramo> {
    let mut t = vec![Tramo::default()];
    for l in limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        if NIVEL_CLASE.is_match(l) {
            t.push(Tramo {
                cabecera: Some(l.to_string()),
                ..Tramo::default()
            });
        } else if let Some(u) = t.last_mut() {
            u.texto.push(l.to_string());
        }
    }
    t
}

/// Un caso: lo de siempre, «Resolver —» y «Resuelto —».
pub fn caso(reglas: &str) -> Vec<Tramo> {
    let mut t = vec![Tramo::default()];
    for l in limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        // De *Case of the Shattered Pact* (MKM) en cada idioma.
        let cab = [
            "To solve",
            "Solved",
            "Resolver",
            "Para resolver",
            "Para resolverlo",
            "Resuelto",
            "À résoudre",
            "Résolue",
            "Lösen",
            "Gelöst",
            "Risolvere",
            "Risolto",
            "Para Solucionar",
            "Solucionado",
            "解明条件",
            "解明完了",
            "侦办",
            "侦结",
            "偵辦",
            "偵結",
        ]
        .iter()
        .find_map(|p| {
            let resto = l.strip_prefix(p)?;
            let resto = resto.trim_start();
            let resto = resto
                .strip_prefix('—')
                .or_else(|| resto.strip_prefix('–'))
                .or_else(|| resto.strip_prefix('-'))
                .or_else(|| resto.strip_prefix('～'))?;
            Some((p.to_string(), resto.trim().to_string()))
        });
        match cab {
            Some((c, resto)) => t.push(Tramo {
                cabecera: Some(c),
                fr: None,
                texto: vec![resto],
            }),
            None => {
                if let Some(u) = t.last_mut() {
                    u.texto.push(l.to_string());
                }
            }
        }
    }
    t.retain(|x| x.cabecera.is_some() || !x.texto.is_empty());
    t
}

/// Un rango de nivel («NIVEL 2-6», «等级2-6», «레벨 3 -7») con la palabra de
/// cualquier idioma (`textos_motor::palabra_nivel`).
static RANGO_NIVEL: LazyLock<Regex> = LazyLock::new(|| {
    let palabras: Vec<String> = super::textos_motor::palabras_nivel()
        .into_iter()
        .map(regex::escape)
        .collect();
    #[allow(clippy::unwrap_used)]
    Regex::new(&format!(
        r"^({})\s*(\d+\s*[-–]\s*\d+|\d+\+)\s*$",
        palabras.join("|")
    ))
    .unwrap()
});
static FR: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^[*\d+X−\-]+/[*\d+X−\-]+$").unwrap()
});

/// Un leveler: lo de arriba («Subir de nivel {W}») y los rangos de nivel con
/// su fuerza/resistencia.
pub fn leveler(reglas: &str) -> Vec<Tramo> {
    let mut t = vec![Tramo::default()];
    for l in limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        if let Some(c) = RANGO_NIVEL.captures(l) {
            t.push(Tramo {
                cabecera: Some(c[2].replace(' ', "")),
                ..Tramo::default()
            });
        } else if FR.is_match(l) && t.len() > 1 {
            if let Some(u) = t.last_mut() {
                u.fr = Some(l.to_string());
            }
        } else if let Some(u) = t.last_mut() {
            u.texto.push(l.to_string());
        }
    }
    t
}

static UMBRAL: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^(\d+\+)\s*\|\s*(.*)$").unwrap()
});

/// Una carta con Estación: lo de arriba y los umbrales (`8+ | Volar`). Una
/// fuerza/resistencia suelta en un umbral va aparte.
pub fn estacion(reglas: &str) -> Vec<Tramo> {
    let mut t = vec![Tramo::default()];
    for l in limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
    {
        if let Some(c) = UMBRAL.captures(l) {
            let resto = c[2].trim().to_string();
            let mut tramo = Tramo {
                cabecera: Some(c[1].to_string()),
                ..Tramo::default()
            };
            if FR.is_match(&resto) {
                tramo.fr = Some(resto);
            } else {
                tramo.texto.push(resto);
            }
            t.push(tramo);
        } else if FR.is_match(l) && t.len() > 1 {
            if let Some(u) = t.last_mut() {
                u.fr = Some(l.to_string());
            }
        } else if let Some(u) = t.last_mut() {
            u.texto.push(l.to_string());
        }
    }
    t
}

/// El prototipo: coste y fuerza/resistencia alternativos, y el resto.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Prototipo {
    pub coste: String,
    pub fr: String,
    /// El recordatorio (va en cursiva dentro del recuadro).
    pub recordatorio: Option<String>,
    pub resto: Vec<String>,
}

static PROTO_EN: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^(?:Prototype|Prototipo)\s*((?:\{[^}]+\})+)\s*[—–-]\s*(\S+/\S+)\s*(\(.*\))?\s*$")
        .unwrap()
});
static COSTE_SOLO: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^(?:\{[^}]+\})+$").unwrap()
});

/// Lee el prototipo en inglés («Prototype {1}{B}{B} — 3/3 (…)») o como lo
/// trae la impresión española (coste, «Prototipo. (…)» y la F/R en líneas
/// sueltas).
pub fn prototipo(reglas: &str) -> Option<Prototipo> {
    let lineas: Vec<String> = limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    let mut p = Prototipo::default();
    let mut encontrado = false;
    let mut i = 0;
    while i < lineas.len() {
        let l = &lineas[i];
        if let Some(c) = PROTO_EN.captures(l) {
            p.coste = c[1].to_string();
            p.fr = c[2].to_string();
            p.recordatorio = c.get(3).map(|m| m.as_str().to_string());
            encontrado = true;
        } else if COSTE_SOLO.is_match(l)
            && lineas
                .get(i + 1)
                .is_some_and(|s| s.starts_with("Prototipo") || s.starts_with("Prototype"))
        {
            p.coste = l.clone();
            let sig = &lineas[i + 1];
            p.recordatorio = sig.find('(').map(|j| sig[j..].to_string());
            i += 1;
            if lineas.get(i + 1).is_some_and(|s| FR.is_match(s)) {
                p.fr = lineas[i + 1].clone();
                i += 1;
            }
            encontrado = true;
        } else {
            p.resto.push(l.clone());
        }
        i += 1;
    }
    encontrado.then_some(p)
}

/// Una sala de una mazmorra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sala {
    pub nombre: String,
    pub texto: String,
    /// A qué salas lleva (por nombre).
    pub lleva_a: Vec<String>,
}

static SALA: LazyLock<Regex> = LazyLock::new(|| {
    #[allow(clippy::unwrap_used)]
    Regex::new(r"^(.+?)\s+[—–]\s+(.*?)\s*(?:\((?:Leads to|Lleva a):\s*([^)]*)\))?\s*$").unwrap()
});

/// Las salas de una mazmorra, con a dónde lleva cada una.
pub fn mazmorra(reglas: &str) -> Vec<Sala> {
    limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter_map(|l| {
            let c = SALA.captures(l)?;
            Some(Sala {
                nombre: c[1].to_string(),
                texto: c[2].to_string(),
                lleva_a: c
                    .get(3)
                    .map(|m| {
                        m.as_str()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Las filas de una mazmorra: cada sala en la fila de su camino más largo
/// desde la primera (las mazmorras oficiales son grafos sin ciclos).
pub fn filas_de_mazmorra(salas: &[Sala]) -> Vec<Vec<usize>> {
    let n = salas.len();
    let mut nivel = vec![0usize; n];
    // Relajación repetida: como mucho n vueltas.
    for _ in 0..n {
        let mut cambio = false;
        for (i, s) in salas.iter().enumerate() {
            for d in &s.lleva_a {
                if let Some(j) = salas.iter().position(|x| &x.nombre == d) {
                    if nivel[j] < nivel[i] + 1 && j != i {
                        nivel[j] = nivel[i] + 1;
                        cambio = true;
                    }
                }
            }
        }
        if !cambio {
            break;
        }
    }
    let filas = nivel.iter().copied().max().map_or(0, |m| m + 1);
    let mut out = vec![Vec::new(); filas];
    for (i, l) in nivel.iter().enumerate() {
        out[*l].push(i);
    }
    out
}

#[cfg(test)]
mod tests {

    #[test]
    fn clases_y_casos_en_otros_idiomas() {
        // De Wizard Class en francés, coreano y japonés.
        for (reglas, n) in [
            ("Il n'y a pas de limite.\n{2}{U} : Niveau 2\nQuand…\n{4}{U} : Niveau 3\nÀ chaque fois…", 3),
            ("손 제한이 없다.\n{2}{U}: 2레벨\n이 직업이…\n{4}{U}: 3레벨\n당신이…", 3),
            ("手札の上限はなくなる。\n{2}{U}：レベル２\nこのクラスが…\n{4}{U}：レベル３\nあなたが…", 3),
            ("你的手牌数量没有上限。\n{2}{U}：等级2\n当此职业…", 2),
        ] {
            assert_eq!(clase(reglas).len(), n, "{reglas}");
        }
        // Case of the Shattered Pact en francés y chino simplificado.
        let fr = caso("Quand cette affaire…\nÀ résoudre — Il y a cinq couleurs.\nRésolue — Au début du combat…");
        assert_eq!(fr.len(), 3);
        assert_eq!(fr[1].cabecera.as_deref(), Some("À résoudre"));
        let zh = caso("当此案件进战场时…\n侦办～由你操控之永久物…\n侦结～在你回合的战斗开始时…");
        assert_eq!(zh.len(), 3);
        assert_eq!(zh[2].cabecera.as_deref(), Some("侦结"));
    }
    use super::*;

    #[test]
    fn habilidades_de_liliana() {
        let h = planeswalker(
            "+1: Cada jugador descarta una carta.\n−2: El jugador objetivo sacrifica una criatura.\n-6: Separa todo.",
        );
        let costes: Vec<_> = h
            .iter()
            .map(|x| x.coste.clone().unwrap_or_default())
            .collect();
        assert_eq!(costes, ["+1", "−2", "−6"]);
        assert_eq!(h[1].texto, "El jugador objetivo sacrifica una criatura.");
        let estatica = planeswalker("Each creature gets +1/+0.\n0: Draw a card.");
        assert_eq!(estatica[0].coste, None);
        assert_eq!(estatica[1].coste.as_deref(), Some("0"));
    }

    #[test]
    fn capitulos_de_la_saga_de_urza() {
        let s = saga(
            "(En cuanto esta Saga entre…)\nI — La Saga de Urza gana \"{T}: Agrega {C}\".\nII — Otra cosa.\nIII — Busca en tu biblioteca.",
        );
        assert_eq!(s.intro.len(), 1);
        assert_eq!(s.capitulos.len(), 3);
        assert_eq!(s.capitulos[2].numeros, [3]);
        let doble = saga("I, II — Roba una carta.\nIII — Fin.");
        assert_eq!(doble.capitulos[0].numeros, [1, 2]);
        assert_eq!(a_romano(4), "IV");
    }

    #[test]
    fn niveles_de_la_clase_y_marcas_de_scryfall() {
        let t = clase(
            "(Gana un nivel…)\nTe cuesta {1} menos.\n{1}{U}: Nivel 2\n//Level_2//\nCuando esta Clase alcance el nivel 2…\n{5}{U}: Nivel 3\nAl comienzo…",
        );
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].cabecera, None);
        assert_eq!(t[0].texto.len(), 2);
        assert_eq!(t[1].cabecera.as_deref(), Some("{1}{U}: Nivel 2"));
        assert_eq!(t[1].texto.len(), 1, "sin la marca //Level_2//");
    }

    #[test]
    fn un_caso_en_ingles_y_en_espanol() {
        let en = caso("You may play an additional land.\nTo solve — You control seven lands. (If unsolved…)\nSolved — You may look…");
        assert_eq!(en.len(), 3);
        assert_eq!(en[1].cabecera.as_deref(), Some("To solve"));
        let es = caso("Puedes jugar una tierra adicional.\nResolver — Controlas siete tierras.\nResuelto — Puedes mirar…");
        assert_eq!(es[2].cabecera.as_deref(), Some("Resuelto"));
        assert_eq!(es[2].texto[0], "Puedes mirar…");
    }

    #[test]
    fn rangos_del_estudiante_de_la_guerra() {
        let t = leveler(
            "Subir de nivel {W}. ({W}: Pon un contador…)\nNIVEL 2-6\n3/3\nDaña primero.\nNIVEL 7+\n4/4\nDaña dos veces.",
        );
        assert_eq!(t.len(), 3);
        assert_eq!(t[1].cabecera.as_deref(), Some("2-6"));
        assert_eq!(t[1].fr.as_deref(), Some("3/3"));
        assert_eq!(t[2].texto, ["Daña dos veces."]);
    }

    #[test]
    fn umbrales_de_estacion() {
        let t = estacion(
            "When this enters…\nStation (Tap another creature…)\n8+ | Flying, trample\n12+ | 5/5",
        );
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].texto.len(), 2);
        assert_eq!(t[1].cabecera.as_deref(), Some("8+"));
        assert_eq!(t[2].fr.as_deref(), Some("5/5"));
    }

    #[test]
    fn prototipo_en_ingles_y_en_espanol() {
        let en = prototipo("Prototype {1}{B}{B} — 3/3 (You may cast…)\nMenace, lifelink").unwrap();
        assert_eq!((en.coste.as_str(), en.fr.as_str()), ("{1}{B}{B}", "3/3"));
        assert_eq!(en.resto, ["Menace, lifelink"]);
        let es = prototipo(
            "Amenaza, vínculo vital.\n//PRT-Prototype_Mech//\n{1}{B}{B}\nPrototipo. (Puedes lanzar…)\n3/3",
        )
        .unwrap();
        assert_eq!((es.coste.as_str(), es.fr.as_str()), ("{1}{B}{B}", "3/3"));
        assert_eq!(es.resto, ["Amenaza, vínculo vital."]);
        assert!(prototipo("Flying").is_none());
    }

    #[test]
    fn la_mazmorra_del_mago_loco_por_filas() {
        let s = mazmorra(
            "Yawning Portal — You gain 1 life. (Leads to: Dungeon Level)\n\
             Dungeon Level — Scry 1. (Leads to: Goblin Bazaar, Twisted Caverns)\n\
             Goblin Bazaar — Create a Treasure token. (Leads to: Lost Level)\n\
             Twisted Caverns — Target creature can't attack. (Leads to: Lost Level)\n\
             Lost Level — Scry 2. (Leads to: Runestone Caverns, Muiral's Graveyard)\n\
             Runestone Caverns — Exile. (Leads to: Deep Mines)\n\
             Muiral's Graveyard — Create two tokens. (Leads to: Deep Mines)\n\
             Deep Mines — Scry 3. (Leads to: Mad Wizard's Lair)\n\
             Mad Wizard's Lair — Draw three cards.",
        );
        assert_eq!(s.len(), 9);
        assert_eq!(s[1].lleva_a, ["Goblin Bazaar", "Twisted Caverns"]);
        let f = filas_de_mazmorra(&s);
        assert_eq!(
            f,
            vec![
                vec![0],
                vec![1],
                vec![2, 3],
                vec![4],
                vec![5, 6],
                vec![7],
                vec![8]
            ]
        );
    }
}
