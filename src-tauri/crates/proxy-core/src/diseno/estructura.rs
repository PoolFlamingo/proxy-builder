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

/// Las líneas de una mazmorra que no son salas (la de Undercity: «You can't
/// enter this dungeon unless…»). Van encima del plano.
pub fn nota_de_mazmorra(reglas: &str) -> Vec<String> {
    limpiar(reglas)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !SALA.is_match(l))
        .map(str::to_string)
        .collect()
}

/// Dónde va una sala en el plano: de qué fila a qué fila (las dos incluidas)
/// y qué tramo del ancho ocupa (fracciones de 0 a 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SitioSala {
    pub fila0: usize,
    pub fila1: usize,
    pub x0: f32,
    pub x1: f32,
}

/// Una puerta: en la pared de debajo de la fila `fila`, a `x` del ancho.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Puerta {
    pub fila: usize,
    pub x: f32,
}

/// El plano de una mazmorra como en las cartas impresas: salas pegadas por
/// filas, sin huecos entre ellas, y una puerta en cada pared que se cruza.
#[derive(Debug, Clone, PartialEq)]
pub struct Plano {
    pub filas: usize,
    /// Uno por sala, en el orden del texto.
    pub sitios: Vec<SitioSala>,
    pub puertas: Vec<Puerta>,
}

/// Monta el plano. Cada sala va en la fila de su camino más largo desde la
/// primera y baja hasta la fila de encima de su primer destino (la Oubliette
/// de Tomb of Annihilation ocupa dos filas, al lado de las dos salas de su
/// izquierda); la última llega hasta abajo. En cada fila, las salas nuevas
/// se reparten a partes iguales el ancho que dejan libre las que vienen de
/// arriba, cada una lo más cerca posible de las salas de las que viene. Sin
/// ningún «Leads to» (Baldur's Gate Wilderness), una rejilla sin puertas.
pub fn plano_de_mazmorra(salas: &[Sala]) -> Plano {
    let n = salas.len();
    let destinos: Vec<Vec<usize>> = salas
        .iter()
        .enumerate()
        .map(|(i, s)| {
            s.lleva_a
                .iter()
                .filter_map(|d| salas.iter().position(|x| &x.nombre == d))
                .filter(|&j| j != i)
                .collect()
        })
        .collect();
    if destinos.iter().all(Vec::is_empty) {
        return rejilla(n);
    }
    // El camino más largo; relajación repetida, como mucho n vueltas (las
    // mazmorras oficiales son grafos sin ciclos).
    let mut nivel = vec![0usize; n];
    for _ in 0..n {
        let mut cambio = false;
        for (i, ds) in destinos.iter().enumerate() {
            for &j in ds {
                if nivel[j] < nivel[i] + 1 {
                    nivel[j] = nivel[i] + 1;
                    cambio = true;
                }
            }
        }
        if !cambio {
            break;
        }
    }
    let filas = nivel.iter().copied().max().map_or(0, |m| m + 1);
    let mut fin: Vec<usize> = (0..n)
        .map(|i| match destinos[i].iter().map(|&j| nivel[j]).min() {
            Some(m) => m.saturating_sub(1).max(nivel[i]),
            None => filas - 1,
        })
        .collect();
    let padres: Vec<Vec<usize>> = (0..n)
        .map(|j| (0..n).filter(|&i| destinos[i].contains(&j)).collect())
        .collect();
    let mut x = vec![(0.0f32, 1.0f32); n];
    for r in 0..filas {
        let nuevas: Vec<usize> = (0..n).filter(|&i| nivel[i] == r).collect();
        if nuevas.is_empty() {
            continue;
        }
        let siguen: Vec<usize> = (0..n).filter(|&i| nivel[i] < r && fin[i] >= r).collect();
        let mut huecos = tramos_libres(&siguen.iter().map(|&i| x[i]).collect::<Vec<_>>());
        if huecos.is_empty() {
            // No queda sitio: las que venían de arriba acaban aquí.
            for &i in &siguen {
                fin[i] = r - 1;
            }
            huecos = vec![(0.0, 1.0)];
        }
        let centro = |j: usize, x: &[(f32, f32)]| {
            let p = &padres[j];
            if p.is_empty() {
                0.5
            } else {
                p.iter().map(|&i| (x[i].0 + x[i].1) / 2.0).sum::<f32>() / p.len() as f32
            }
        };
        let mut por_hueco: Vec<Vec<(f32, usize)>> = vec![Vec::new(); huecos.len()];
        for &j in &nuevas {
            let c = centro(j, &x);
            let distancia = |&(a, b): &(f32, f32)| {
                if c < a {
                    a - c
                } else if c > b {
                    c - b
                } else {
                    0.0
                }
            };
            let k = (0..huecos.len())
                .min_by(|&p, &q| distancia(&huecos[p]).total_cmp(&distancia(&huecos[q])))
                .unwrap_or(0);
            por_hueco[k].push((c, j));
        }
        for (k, mut v) in por_hueco.into_iter().enumerate() {
            if v.is_empty() {
                continue;
            }
            v.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
            let (a, b) = huecos[k];
            let w = (b - a) / v.len() as f32;
            for (m, &(_, j)) in v.iter().enumerate() {
                x[j] = (a + w * m as f32, a + w * (m + 1) as f32);
            }
        }
    }
    // Una puerta por cada paso de una sala a la de debajo: en medio del
    // tramo que comparten o, si no comparten, en el borde más cercano.
    let mut puertas: Vec<Puerta> = Vec::new();
    for (i, ds) in destinos.iter().enumerate() {
        for &j in ds {
            if fin[i] + 1 != nivel[j] {
                continue;
            }
            let (a, b) = (x[i].0.max(x[j].0), x[i].1.min(x[j].1));
            let px = if b > a {
                (a + b) / 2.0
            } else {
                let m = ((x[i].1 - x[i].0) * 0.2).min(0.05);
                ((x[j].0 + x[j].1) / 2.0).clamp(x[i].0 + m, x[i].1 - m)
            };
            let p = Puerta {
                fila: fin[i],
                x: px,
            };
            if !puertas
                .iter()
                .any(|q| q.fila == p.fila && (q.x - p.x).abs() < 0.01)
            {
                puertas.push(p);
            }
        }
    }
    Plano {
        filas,
        sitios: (0..n)
            .map(|i| SitioSala {
                fila0: nivel[i],
                fila1: fin[i],
                x0: x[i].0,
                x1: x[i].1,
            })
            .collect(),
        puertas,
    }
}

/// Los tramos de 0 a 1 que no tapan los dados.
pub fn tramos_libres(ocupados: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut o = ocupados.to_vec();
    o.sort_by(|p, q| p.0.total_cmp(&q.0));
    let mut out = Vec::new();
    let mut desde = 0.0f32;
    for (a, b) in o {
        if a - desde > 0.001 {
            out.push((desde, a));
        }
        desde = desde.max(b);
    }
    if 1.0 - desde > 0.001 {
        out.push((desde, 1.0));
    }
    out
}

/// Las salas sin conexiones, en filas de tres (de dos con tres o cuatro
/// salas); la última fila se reparte el ancho entero.
fn rejilla(n: usize) -> Plano {
    let cols = match n {
        0..=2 => 1,
        3..=4 => 2,
        _ => 3,
    };
    let filas = n.div_ceil(cols);
    let sitios = (0..n)
        .map(|i| {
            let fila = i / cols;
            let en_fila = (n - fila * cols).min(cols);
            let k = (i % cols) as f32;
            let w = 1.0 / en_fila as f32;
            SitioSala {
                fila0: fila,
                fila1: fila,
                x0: k * w,
                x1: (k + 1.0) * w,
            }
        })
        .collect();
    Plano {
        filas,
        sitios,
        puertas: Vec::new(),
    }
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
        let p = plano_de_mazmorra(&s);
        assert_eq!(p.filas, 7);
        let filas: Vec<(usize, usize)> = p.sitios.iter().map(|x| (x.fila0, x.fila1)).collect();
        assert_eq!(
            filas,
            [
                (0, 0),
                (1, 1),
                (2, 2),
                (2, 2),
                (3, 3),
                (4, 4),
                (4, 4),
                (5, 5),
                (6, 6)
            ]
        );
        // Las dos salas de en medio, cada una en su mitad.
        assert_eq!((p.sitios[2].x0, p.sitios[2].x1), (0.0, 0.5));
        assert_eq!((p.sitios[3].x0, p.sitios[3].x1), (0.5, 1.0));
        // Dungeon Level abre dos puertas, a la cuarta parte y a las tres
        // cuartas; Goblin Bazaar y Twisted Caverns, una cada una a Lost Level.
        let en = |fila: usize| -> Vec<f32> {
            p.puertas
                .iter()
                .filter(|q| q.fila == fila)
                .map(|q| q.x)
                .collect()
        };
        assert_eq!(en(0), [0.5]);
        assert_eq!(en(1), [0.25, 0.75]);
        assert_eq!(en(2), [0.25, 0.75]);
        assert_eq!(en(5), [0.5]);
    }

    fn cerca(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.001
    }

    #[test]
    fn la_oubliette_ocupa_dos_filas_como_en_la_carta() {
        let s = mazmorra(
            "Trapped Entry — Each player loses 1 life. (Leads to: Veils of Fear, Oubliette)\n\
             Veils of Fear — Each player loses 2 life. (Leads to: Sandfall Cell)\n\
             Sandfall Cell — Each player loses 2 life. (Leads to: Cradle of the Death God)\n\
             Oubliette — Discard a card. (Leads to: Cradle of the Death God)\n\
             Cradle of the Death God — Create The Atropal.",
        );
        let p = plano_de_mazmorra(&s);
        assert_eq!(p.filas, 4);
        let o = p.sitios[3];
        assert_eq!((o.fila0, o.fila1, o.x0, o.x1), (1, 2, 0.5, 1.0));
        // Sandfall Cell, debajo de Veils of Fear, en el sitio que queda.
        let c = p.sitios[2];
        assert_eq!((c.fila0, c.fila1, c.x0, c.x1), (2, 2, 0.0, 0.5));
        // La puerta de la Oubliette a la Cradle está bajo la fila 2.
        assert!(p.puertas.iter().any(|q| q.fila == 2 && cerca(q.x, 0.75)));
        assert!(p.puertas.iter().any(|q| q.fila == 1 && cerca(q.x, 0.25)));
    }

    #[test]
    fn las_salas_de_phandelver_se_abren_donde_se_tocan() {
        let s = mazmorra(
            "Cave Entrance — Scry 1. (Leads to: Goblin Lair, Mine Tunnels)\n\
             Goblin Lair — Create a Goblin. (Leads to: Storeroom, Dark Pool)\n\
             Mine Tunnels — Create a Treasure token. (Leads to: Dark Pool, Fungi Cavern)\n\
             Storeroom — Put a counter. (Leads to: Temple of Dumathoin)\n\
             Dark Pool — Each opponent loses 1 life. (Leads to: Temple of Dumathoin)\n\
             Fungi Cavern — Target creature gets -4/-0. (Leads to: Temple of Dumathoin)\n\
             Temple of Dumathoin — Draw a card.",
        );
        let p = plano_de_mazmorra(&s);
        let tercio = |i: usize| (p.sitios[i].x0 * 3.0).round() as u32;
        assert_eq!([tercio(3), tercio(4), tercio(5)], [0, 1, 2]);
        // Goblin Lair (la mitad izquierda) da al Storeroom y al Dark Pool en
        // medio de lo que comparten; lo mismo Mine Tunnels.
        let fila1: Vec<f32> = p
            .puertas
            .iter()
            .filter(|q| q.fila == 1)
            .map(|q| q.x)
            .collect();
        assert_eq!(fila1.len(), 4);
        for (a, b) in fila1
            .iter()
            .zip([1.0 / 6.0, 5.0 / 12.0, 7.0 / 12.0, 5.0 / 6.0])
        {
            assert!(cerca(*a, b), "{fila1:?}");
        }
    }

    #[test]
    fn undercity_con_su_nota_y_sin_ella_en_las_salas() {
        let reglas = "You can't enter this dungeon unless you \"venture into Undercity.\"\n\
             Secret Entrance — Search your library. (Leads to: Forge, Lost Well)\n\
             Forge — Put two counters. (Leads to: Trap!, Arena)\n\
             Lost Well — Scry 2. (Leads to: Arena, Stash)\n\
             Trap! — Target player loses 5 life. (Leads to: Archives)\n\
             Arena — Goad target creature. (Leads to: Archives, Catacombs)\n\
             Stash — Create a Treasure token. (Leads to: Catacombs)\n\
             Archives — Draw a card. (Leads to: Throne of the Dead Three)\n\
             Catacombs — Create a Skeleton. (Leads to: Throne of the Dead Three)\n\
             Throne of the Dead Three — Reveal the top ten cards.";
        assert_eq!(nota_de_mazmorra(reglas).len(), 1);
        let s = mazmorra(reglas);
        assert_eq!(s.len(), 9);
        let p = plano_de_mazmorra(&s);
        assert_eq!(p.filas, 5);
        // Archives a la izquierda y Catacombs a la derecha.
        assert!(p.sitios[6].x1 <= p.sitios[7].x0 + 0.001);
        // Ninguna puerta cae fuera de las dos salas que une.
        assert_eq!(p.puertas.iter().filter(|q| q.fila == 2).count(), 4);
    }

    #[test]
    fn sin_conexiones_las_salas_van_en_rejilla() {
        let texto: Vec<String> = (0..19)
            .map(|i| format!("Room {i} — Draw a card."))
            .collect();
        let p = plano_de_mazmorra(&mazmorra(&texto.join("\n")));
        assert_eq!(p.filas, 7);
        assert!(p.puertas.is_empty());
        let ultima = p.sitios[18];
        assert_eq!((ultima.fila0, ultima.x0, ultima.x1), (6, 0.0, 1.0));
    }
}
