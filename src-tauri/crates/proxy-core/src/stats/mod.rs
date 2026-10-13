//! **Estadísticas del deck**: curva de maná (por tipo y permanentes frente a
//! hechizos), símbolos de maná frente a fuentes, tipos, subtipos, habilidades,
//! funciones (rampa, robo, eliminación…), probabilidades de robo, precios,
//! sets, legalidad, problemas de construcción y, en Commander, una estimación
//! del bracket. La app le pasa una entrada por copia con los datos de Scryfall
//! que tenga; aquí solo se cuenta.

pub mod funciones;
pub mod mana;
pub mod prob;

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use funciones::{Texto, FUNCIONES};
use mana::COLORES;

/// Una copia del deck con lo que hace falta para contarla.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StatInput {
    /// El id de la copia en el deck (para seleccionarla desde las estadísticas).
    pub id: String,
    pub name: String,
    pub cmc: Option<f64>,
    /// El coste (`{2}{W}{U}`); en las de dos caras, el de la cara de delante.
    pub mana_cost: Option<String>,
    pub colors: Vec<String>,
    pub color_identity: Vec<String>,
    pub type_line: Option<String>,
    /// El texto de reglas en inglés (todas las caras).
    pub oracle_text: String,
    pub keywords: Vec<String>,
    /// Los colores de maná que puede producir (`W`, `U`, …, `C`).
    pub produced_mana: Vec<String>,
    pub power: Option<String>,
    pub toughness: Option<String>,
    pub rarity: Option<String>,
    pub set: Option<String>,
    pub set_name: Option<String>,
    pub released_at: Option<String>,
    /// Precios de Scryfall (texto, `"1.23"`).
    pub price_eur: Option<String>,
    pub price_usd: Option<String>,
    pub legalities: BTreeMap<String, String>,
    /// En la lista de Game Changers de Commander.
    pub game_changer: bool,
    pub is_token: bool,
}

/// Qué clase de mazo es: cambia las reglas de copias, el tamaño esperado y la
/// fórmula de tierras.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum DeckKind {
    Commander,
    Constructed,
    Limited,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Contador {
    pub key: String,
    pub count: u32,
}

/// Una carta del deck con cuántas copias tiene y los ids de esas copias.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartaCuenta {
    pub name: String,
    pub count: u32,
    pub ids: Vec<String>,
}

/// Un coste de la curva (0 a 7; el 7 es «7 o más»).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TramoCurva {
    pub cmc: u32,
    pub total: u32,
    /// Por tipo principal, en el orden de `TIPOS_CURVA` (con ceros).
    pub by_type: Vec<Contador>,
    pub permanents: u32,
    pub spells: u32,
}

/// Un color: cuántas cartas lo son, cuántos símbolos piden los costes y
/// cuántas cartas lo producen.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ColorMana {
    pub color: String,
    /// Cartas que no son tierra de ese color.
    pub cards: u32,
    pub pips: u32,
    pub land_sources: u32,
    pub other_sources: u32,
}

/// Una función (rampa, robo…) con las cartas que la cumplen.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Rol {
    pub key: String,
    pub count: u32,
    pub cards: Vec<CartaCuenta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartaPrecio {
    pub name: String,
    pub count: u32,
    /// Precio de una copia.
    pub eur: Option<f64>,
    pub usd: Option<f64>,
    pub ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PrecioTipo {
    pub key: String,
    pub eur: f64,
    pub usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Precios {
    pub total_eur: f64,
    pub total_usd: f64,
    /// Copias sin precio en cada moneda.
    pub without_eur: u32,
    pub without_usd: u32,
    /// Todas las cartas con precio, de la más cara a la más barata (por el
    /// total de sus copias, en euros y si no en dólares).
    pub cards: Vec<CartaPrecio>,
    pub by_type: Vec<PrecioTipo>,
    /// Copias por tramo de precio en euros: `<1`, `1-5`, `5-20`, `20-50`, `50+`.
    pub buckets: Vec<Contador>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SetCuenta {
    pub code: String,
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Criaturas {
    pub count: u32,
    pub average_power: Option<f64>,
    pub average_toughness: Option<f64>,
    /// Criaturas por fuerza, de 0 a 7+ (sin las de fuerza variable).
    pub power: Vec<u32>,
}

/// La probabilidad de haber visto al menos una copia de algo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ProbCarta {
    /// El nombre de la carta o la clave de la función.
    pub key: String,
    pub copies: u32,
    /// Con 7, 8, 9… cartas vistas (la mano inicial y cada robo).
    pub by_seen: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TurnoTierra {
    pub turn: u32,
    /// P(tener al menos `turn` tierras en ese turno) yendo primero y segundo.
    pub play: f64,
    pub draw: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Probabilidades {
    pub deck_size: u32,
    pub lands: u32,
    /// P(exactamente k tierras en la mano inicial), k = 0..=7.
    pub opening_lands: Vec<f64>,
    /// Tierras que se esperan en la mano inicial.
    pub expected_lands: f64,
    pub land_drops: Vec<TurnoTierra>,
    pub cards: Vec<ProbCarta>,
    pub roles: Vec<ProbCarta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Legalidad {
    pub format: String,
    pub legal: bool,
    /// Las cartas que lo impiden (sin repetir).
    pub offenders: Vec<String>,
}

/// Un problema de construcción según la clase de mazo.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Problema {
    /// `copies` (demasiadas copias de una carta) o `size` (tamaño).
    pub code: String,
    pub cards: Vec<CartaCuenta>,
    pub expected: Option<u32>,
    pub actual: Option<u32>,
}

/// Estimación del bracket de Commander por lo que se puede ver en las
/// cartas (no detecta combos).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Bracket {
    /// El bracket mínimo que permiten las cartas: 2, 3 o 4.
    pub level: u8,
    pub game_changers: Vec<CartaCuenta>,
    pub tutors: Vec<CartaCuenta>,
    pub extra_turns: Vec<CartaCuenta>,
    pub mass_land_denial: Vec<CartaCuenta>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckStats {
    pub kind: DeckKind,
    /// La que se deduce del deck (la elegida puede ser otra).
    pub detected_kind: DeckKind,
    /// Copias con datos, sin tokens.
    pub cards: u32,
    /// Todas las copias que no son token (también las que no tienen datos).
    pub deck_size: u32,
    pub lands: u32,
    pub nonlands: u32,
    pub tokens: u32,
    /// Copias sin datos de Scryfall (propias sin vincular, sin caché…).
    pub unknown: u32,
    pub unknown_cards: Vec<CartaCuenta>,
    pub token_cards: Vec<CartaCuenta>,
    /// Coste medio sin tierras.
    pub average_cmc: f64,
    pub average_cmc_with_lands: f64,
    pub median_cmc: f64,
    pub total_mana_value: f64,
    /// Curva de 0 a 7+, sin tierras.
    pub curve: Vec<TramoCurva>,
    /// W, U, B, R, G y C.
    pub colors: Vec<ColorMana>,
    pub color_identity: Vec<String>,
    /// Cartas que no son tierra con 0, 1, … 5 colores.
    pub color_count: Vec<u32>,
    /// Cartas con `{X}` en el coste.
    pub x_spells: u32,
    pub types: Vec<Contador>,
    pub supertypes: Vec<Contador>,
    pub subtypes: Vec<Contador>,
    pub keywords: Vec<Contador>,
    pub creatures: Criaturas,
    pub rarity: Vec<Contador>,
    pub sets: Vec<SetCuenta>,
    pub oldest: Option<String>,
    pub newest: Option<String>,
    pub roles: Vec<Rol>,
    pub prices: Precios,
    pub probability: Probabilidades,
    /// Tierras que recomienda la fórmula de Frank Karsten para esta curva.
    pub recommended_lands: f64,
    /// Rampa y robo de coste 2 o menos (lo que la fórmula descuenta).
    pub cheap_ramp_draw: u32,
    pub legality: Vec<Legalidad>,
    pub problems: Vec<Problema>,
    pub bracket: Option<Bracket>,
}

/// Formatos que se enseñan, en este orden.
pub const FORMATOS: [&str; 9] = [
    "standard",
    "pioneer",
    "modern",
    "legacy",
    "vintage",
    "commander",
    "pauper",
    "historic",
    "brawl",
];

const TIPOS: [&str; 8] = [
    "Creature",
    "Planeswalker",
    "Battle",
    "Instant",
    "Sorcery",
    "Artifact",
    "Enchantment",
    "Land",
];

/// Los tipos que se apilan en la curva (las tierras no van en ella).
pub const TIPOS_CURVA: [&str; 8] = [
    "Creature",
    "Planeswalker",
    "Battle",
    "Instant",
    "Sorcery",
    "Artifact",
    "Enchantment",
    "Other",
];

const SUPERTIPOS: [&str; 5] = ["Legendary", "Basic", "Snow", "World", "Ongoing"];

/// Cuántos subtipos y habilidades se devuelven como mucho.
const MAX_LISTA: usize = 30;

/// El tipo principal de una línea de tipo (`Legendary Artifact Creature — …` → `Creature`).
fn tipo_principal(linea: &str) -> String {
    let antes = linea.split('—').next().unwrap_or(linea);
    // De la cara frontal en las DFC (`A // B`).
    let frente = antes.split("//").next().unwrap_or(antes);
    TIPOS
        .iter()
        .find(|t| frente.split_whitespace().any(|w| w == **t))
        .map(|t| (*t).to_string())
        .unwrap_or_else(|| "Other".into())
}

/// Los subtipos de la cara de delante (lo que va tras la raya).
fn subtipos(linea: &str) -> Vec<String> {
    let frente = linea.split("//").next().unwrap_or(linea);
    frente
        .split('—')
        .nth(1)
        .map(|s| s.split_whitespace().map(str::to_string).collect())
        .unwrap_or_default()
}

fn es_tierra(c: &StatInput) -> bool {
    c.type_line
        .as_deref()
        .map(|l| tipo_principal(l) == "Land")
        .unwrap_or(false)
}

fn es_permanente(tipo: &str) -> bool {
    !matches!(tipo, "Instant" | "Sorcery")
}

fn es_basica(c: &StatInput) -> bool {
    c.type_line
        .as_deref()
        .is_some_and(|l| l.contains("Basic") && l.contains("Land"))
}

fn precio(p: &Option<String>) -> Option<f64> {
    p.as_deref().and_then(|p| p.parse::<f64>().ok())
}

fn centimos(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn numero(s: &Option<String>) -> Option<f64> {
    s.as_deref().and_then(|s| s.trim().parse::<f64>().ok())
}

/// Agrupa copias por nombre, en el orden en que aparecen.
#[derive(Default, Clone)]
struct Grupos {
    orden: Vec<String>,
    mapa: HashMap<String, CartaCuenta>,
}

impl Grupos {
    fn sumar(&mut self, c: &StatInput) {
        let e = self.mapa.entry(c.name.clone()).or_insert_with(|| {
            self.orden.push(c.name.clone());
            CartaCuenta {
                name: c.name.clone(),
                count: 0,
                ids: Vec::new(),
            }
        });
        e.count += 1;
        if !c.id.is_empty() {
            e.ids.push(c.id.clone());
        }
    }

    fn total(&self) -> u32 {
        self.mapa.values().map(|c| c.count).sum()
    }

    /// Las cartas, de más copias a menos y, a igualdad, por orden de aparición.
    fn lista(mut self) -> Vec<CartaCuenta> {
        let mut v: Vec<CartaCuenta> = self
            .orden
            .iter()
            .filter_map(|n| self.mapa.remove(n))
            .collect();
        v.sort_by_key(|c| std::cmp::Reverse(c.count));
        v
    }
}

fn contador(m: BTreeMap<String, u32>, max: usize) -> Vec<Contador> {
    let mut v: Vec<Contador> = m
        .into_iter()
        .map(|(key, count)| Contador { key, count })
        .collect();
    v.sort_by(|a, b| b.count.cmp(&a.count).then(a.key.cmp(&b.key)));
    v.truncate(max);
    v
}

/// La clase de mazo que parece: 90-110 cartas sin repetir (salvo básicas) es
/// Commander; menos de 50, limitado; si no, construido.
pub fn deducir(entrada: &[StatInput]) -> DeckKind {
    let mazo: Vec<&StatInput> = entrada.iter().filter(|c| !c.is_token).collect();
    let n = mazo.len();
    if n < 50 {
        return DeckKind::Limited;
    }
    if (90..=110).contains(&n) {
        let mut copias: HashMap<&str, u32> = HashMap::new();
        for c in &mazo {
            if !es_basica(c) && !funciones::sin_limite_de_copias(&c.oracle_text) {
                *copias.entry(c.name.as_str()).or_default() += 1;
            }
        }
        // Alguna repetida se perdona (una propia duplicada sin querer).
        if copias.values().filter(|n| **n > 1).count() <= 2 {
            return DeckKind::Commander;
        }
    }
    DeckKind::Constructed
}

/// Las tierras que recomienda Frank Karsten («How Many Lands Do You Need…»):
/// 31,42 + 3,13 × coste medio − 0,28 × rampa y robo baratos en Commander;
/// 19,59 + 1,90 × coste medio − 0,28 × … en 60 cartas; y esa misma escalada a
/// 40 en limitado (aproximado).
pub fn tierras_recomendadas(kind: DeckKind, coste_medio: f64, baratas: u32) -> f64 {
    let b = f64::from(baratas);
    let r = match kind {
        DeckKind::Commander => 31.42 + 3.13 * coste_medio - 0.28 * b,
        DeckKind::Constructed => 19.59 + 1.90 * coste_medio - 0.28 * b,
        DeckKind::Limited => (19.59 + 1.90 * coste_medio - 0.28 * b) * 40.0 / 60.0,
    };
    (r.max(0.0) * 10.0).round() / 10.0
}

pub fn stats(entrada: &[StatInput], kind: Option<DeckKind>) -> DeckStats {
    let detectada = deducir(entrada);
    let kind = kind.unwrap_or(detectada);

    let mut curva: Vec<TramoCurva> = (0..8)
        .map(|cmc| TramoCurva {
            cmc,
            total: 0,
            by_type: TIPOS_CURVA
                .iter()
                .map(|t| Contador {
                    key: (*t).to_string(),
                    count: 0,
                })
                .collect(),
            permanents: 0,
            spells: 0,
        })
        .collect();
    let mut colores: Vec<ColorMana> = COLORES
        .iter()
        .map(|c| ColorMana {
            color: (*c).to_string(),
            cards: 0,
            pips: 0,
            land_sources: 0,
            other_sources: 0,
        })
        .collect();
    let mut identidad: Vec<String> = Vec::new();
    let mut num_colores = vec![0u32; 6];
    let mut x_spells = 0;
    let mut tipos: BTreeMap<String, u32> = BTreeMap::new();
    let mut supertipos: BTreeMap<String, u32> = BTreeMap::new();
    let mut subtipos_m: BTreeMap<String, u32> = BTreeMap::new();
    let mut palabras: BTreeMap<String, u32> = BTreeMap::new();
    let mut rareza: BTreeMap<String, u32> = BTreeMap::new();
    let mut sets: BTreeMap<String, (String, u32)> = BTreeMap::new();
    let (mut mas_viejo, mut mas_nuevo): (Option<String>, Option<String>) = (None, None);
    let mut roles: BTreeMap<&'static str, Grupos> =
        FUNCIONES.iter().map(|f| (*f, Grupos::default())).collect();
    let mut tutores = Grupos::default();
    let mut game_changers = Grupos::default();
    let mut desconocidas = Grupos::default();
    let mut fichas = Grupos::default();
    let mut todas = Grupos::default();
    let mut copias_por_nombre: HashMap<String, (bool, Grupos)> = HashMap::new();
    let mut costes: Vec<f64> = Vec::new();
    let (mut cartas, mut tierras, mut tamano) = (0u32, 0u32, 0u32);
    let mut suma_tierras_cmc = 0.0;
    let mut baratas = 0;
    let (mut cr_n, mut fuerzas, mut resistencias) = (0u32, Vec::new(), Vec::new());
    let mut por_fuerza = vec![0u32; 8];
    // Precios.
    let (mut total_eur, mut total_usd, mut sin_eur, mut sin_usd) = (0.0, 0.0, 0u32, 0u32);
    let mut precio_carta: HashMap<String, (Option<f64>, Option<f64>)> = HashMap::new();
    let mut por_tipo: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    let mut tramos = [0u32; 5];
    let mut legalidad: BTreeMap<&str, Vec<String>> =
        FORMATOS.iter().map(|f| (*f, Vec::new())).collect();

    for c in entrada {
        if c.is_token {
            fichas.sumar(c);
            continue;
        }
        tamano += 1;
        todas.sumar(c);
        if c.type_line.is_none() && c.cmc.is_none() {
            desconocidas.sumar(c);
            continue;
        }
        cartas += 1;
        let tierra = es_tierra(c);
        let tipo = c
            .type_line
            .as_deref()
            .map(tipo_principal)
            .unwrap_or_else(|| "Other".into());
        *tipos.entry(tipo.clone()).or_default() += 1;
        if let Some(l) = &c.type_line {
            let frente = l.split("//").next().unwrap_or(l);
            let antes = frente.split('—').next().unwrap_or(frente);
            for s in SUPERTIPOS {
                if antes.split_whitespace().any(|w| w == s) {
                    *supertipos.entry(s.to_string()).or_default() += 1;
                }
            }
            for s in subtipos(l) {
                *subtipos_m.entry(s).or_default() += 1;
            }
        }
        for k in &c.keywords {
            *palabras.entry(k.clone()).or_default() += 1;
        }
        if let Some(r) = &c.rarity {
            *rareza.entry(r.clone()).or_default() += 1;
        }
        if let Some(code) = &c.set {
            let e = sets
                .entry(code.to_lowercase())
                .or_insert_with(|| (c.set_name.clone().unwrap_or_default(), 0));
            e.1 += 1;
        }
        if let Some(f) = &c.released_at {
            if mas_viejo.as_ref().is_none_or(|v| f < v) {
                mas_viejo = Some(f.clone());
            }
            if mas_nuevo.as_ref().is_none_or(|v| f > v) {
                mas_nuevo = Some(f.clone());
            }
        }

        // Curva y costes.
        let coste = c.mana_cost.as_deref().map(mana::leer).unwrap_or_default();
        if tierra {
            tierras += 1;
            suma_tierras_cmc += c.cmc.unwrap_or(0.0);
        } else if let Some(cmc) = c.cmc {
            let i = (cmc.max(0.0).floor() as usize).min(7);
            let tramo = &mut curva[i];
            tramo.total += 1;
            let clave = if TIPOS_CURVA.contains(&tipo.as_str()) {
                tipo.as_str()
            } else {
                "Other"
            };
            if let Some(t) = tramo.by_type.iter_mut().find(|t| t.key == clave) {
                t.count += 1;
            }
            if es_permanente(&tipo) {
                tramo.permanents += 1;
            } else {
                tramo.spells += 1;
            }
            costes.push(cmc);
        }
        if coste.x {
            x_spells += 1;
        }
        for (k, n) in coste.pips.iter().enumerate() {
            colores[k].pips += n;
        }

        // Colores de la carta y maná que produce.
        if !tierra {
            let n = c
                .colors
                .iter()
                .filter(|col| COLORES[..5].contains(&col.as_str()))
                .count();
            num_colores[n.min(5)] += 1;
            if n == 0 {
                colores[5].cards += 1;
            }
            for col in &c.colors {
                if let Some(k) = COLORES[..5].iter().position(|k| k == col) {
                    colores[k].cards += 1;
                }
            }
        }
        for col in &c.produced_mana {
            if let Some(k) = COLORES.iter().position(|k| k == col) {
                if tierra {
                    colores[k].land_sources += 1;
                } else {
                    colores[k].other_sources += 1;
                }
            }
        }
        for col in &c.color_identity {
            if !identidad.contains(col) {
                identidad.push(col.clone());
            }
        }

        // Criaturas.
        if tipo == "Creature" {
            cr_n += 1;
            if let Some(p) = numero(&c.power) {
                fuerzas.push(p);
                por_fuerza[(p.max(0.0).floor() as usize).min(7)] += 1;
            }
            if let Some(t) = numero(&c.toughness) {
                resistencias.push(t);
            }
        }

        // Funciones.
        let f = funciones::funciones(&Texto {
            oracle: &c.oracle_text,
            keywords: &c.keywords,
            produced_mana: &c.produced_mana,
            es_tierra: tierra,
        });
        for k in &f {
            if let Some(g) = roles.get_mut(k) {
                g.sumar(c);
            }
        }
        if f.contains(&"tutor") {
            tutores.sumar(c);
        }
        if !tierra && c.cmc.unwrap_or(9.0) <= 2.0 && (f.contains(&"ramp") || f.contains(&"draw")) {
            baratas += 1;
        }
        if c.game_changer {
            game_changers.sumar(c);
        }

        // Copias.
        let libre = es_basica(c) || funciones::sin_limite_de_copias(&c.oracle_text);
        copias_por_nombre
            .entry(c.name.clone())
            .or_insert_with(|| (libre, Grupos::default()))
            .1
            .sumar(c);

        // Precios.
        let (eur, usd) = (precio(&c.price_eur), precio(&c.price_usd));
        precio_carta.entry(c.name.clone()).or_insert((eur, usd));
        match eur {
            Some(p) => {
                total_eur += p;
                let i = match p {
                    p if p < 1.0 => 0,
                    p if p < 5.0 => 1,
                    p if p < 20.0 => 2,
                    p if p < 50.0 => 3,
                    _ => 4,
                };
                tramos[i] += 1;
            }
            None => sin_eur += 1,
        }
        match usd {
            Some(p) => total_usd += p,
            None => sin_usd += 1,
        }
        let e = por_tipo.entry(tipo.clone()).or_default();
        e.0 += eur.unwrap_or(0.0);
        e.1 += usd.unwrap_or(0.0);

        for (formato, malas) in legalidad.iter_mut() {
            let estado = c.legalities.get(*formato).map(String::as_str);
            if !matches!(estado, Some("legal" | "restricted")) && !malas.contains(&c.name) {
                malas.push(c.name.clone());
            }
        }
    }

    let orden = ["W", "U", "B", "R", "G"];
    identidad.sort_by_key(|c| orden.iter().position(|o| o == c).unwrap_or(9));

    let no_tierras = costes.len() as u32;
    let suma_cmc: f64 = costes.iter().sum();
    let media = if no_tierras > 0 {
        suma_cmc / f64::from(no_tierras)
    } else {
        0.0
    };
    costes.sort_by(|a, b| a.total_cmp(b));
    let mediana = match costes.len() {
        0 => 0.0,
        n if n % 2 == 1 => costes[n / 2],
        n => (costes[n / 2 - 1] + costes[n / 2]) / 2.0,
    };
    let media_f = |v: &[f64]| -> Option<f64> {
        (!v.is_empty()).then(|| centimos(v.iter().sum::<f64>() / v.len() as f64))
    };

    // Precios: todas las cartas, de la más cara a la más barata.
    let mut con_precio: Vec<CartaPrecio> = todas
        .orden
        .iter()
        .filter_map(|n| {
            let g = todas.mapa.get(n)?;
            let (eur, usd) = precio_carta.get(n).copied().unwrap_or((None, None));
            (eur.is_some() || usd.is_some()).then(|| CartaPrecio {
                name: n.clone(),
                count: g.count,
                eur,
                usd,
                ids: g.ids.clone(),
            })
        })
        .collect();
    let clave_precio = |c: &CartaPrecio| f64::from(c.count) * c.eur.or(c.usd).unwrap_or(0.0);
    con_precio.sort_by(|a, b| clave_precio(b).total_cmp(&clave_precio(a)));
    let mut por_tipo_v: Vec<PrecioTipo> = por_tipo
        .into_iter()
        .map(|(key, (eur, usd))| PrecioTipo {
            key,
            eur: centimos(eur),
            usd: centimos(usd),
        })
        .collect();
    por_tipo_v.sort_by(|a, b| b.eur.total_cmp(&a.eur));

    // Probabilidades.
    let mut probs_cartas: Vec<ProbCarta> = todas
        .orden
        .iter()
        .filter_map(|n| todas.mapa.get(n))
        .map(|g| ProbCarta {
            key: g.name.clone(),
            copies: g.count,
            by_seen: prob::por_vistas(tamano, g.count),
        })
        .collect();
    probs_cartas.sort_by(|a, b| a.key.cmp(&b.key));
    let probs_roles: Vec<ProbCarta> = FUNCIONES
        .iter()
        .filter_map(|k| {
            let n = roles.get(k)?.total();
            (n > 0).then(|| ProbCarta {
                key: (*k).to_string(),
                copies: n,
                by_seen: prob::por_vistas(tamano, n),
            })
        })
        .collect();
    let probabilidad = Probabilidades {
        deck_size: tamano,
        lands: tierras,
        opening_lands: (0..=prob::MANO)
            .map(|k| prob::redondear(prob::exacta(tamano, tierras, prob::MANO, k)))
            .collect(),
        expected_lands: if tamano > 0 {
            centimos(f64::from(prob::MANO.min(tamano)) * f64::from(tierras) / f64::from(tamano))
        } else {
            0.0
        },
        land_drops: (1..=8)
            .map(|t| TurnoTierra {
                turn: t,
                play: prob::redondear(prob::al_menos(tamano, tierras, prob::MANO + t - 1, t)),
                draw: prob::redondear(prob::al_menos(tamano, tierras, prob::MANO + t, t)),
            })
            .collect(),
        cards: probs_cartas,
        roles: probs_roles,
    };

    // Problemas de construcción.
    let mut problemas = Vec::new();
    let max_copias = match kind {
        DeckKind::Commander => Some(1),
        DeckKind::Constructed => Some(4),
        DeckKind::Limited => None,
    };
    if let Some(max) = max_copias {
        let mut demasiadas: Vec<CartaCuenta> = copias_por_nombre
            .into_values()
            .filter(|(libre, g)| !libre && g.total() > max)
            .flat_map(|(_, g)| g.lista())
            .collect();
        demasiadas.sort_by(|a, b| a.name.cmp(&b.name));
        if !demasiadas.is_empty() {
            problemas.push(Problema {
                code: "copies".into(),
                cards: demasiadas,
                expected: Some(max),
                actual: None,
            });
        }
    }
    let (minimo, exacto) = match kind {
        DeckKind::Commander => (100, true),
        DeckKind::Constructed => (60, false),
        DeckKind::Limited => (40, false),
    };
    if tamano > 0 && (tamano < minimo || (exacto && tamano != minimo)) {
        problemas.push(Problema {
            code: "size".into(),
            cards: Vec::new(),
            expected: Some(minimo),
            actual: Some(tamano),
        });
    }

    let bracket = (kind == DeckKind::Commander).then(|| {
        let gc = game_changers.lista();
        let turnos = roles.get("extraTurn").cloned().unwrap_or_default().lista();
        let mld = roles.get("landDenial").cloned().unwrap_or_default().lista();
        let tut = tutores.lista();
        let level = if !mld.is_empty() || gc.len() > 3 {
            4
        } else if !gc.is_empty() || turnos.len() >= 2 || tut.len() > 3 {
            3
        } else {
            2
        };
        Bracket {
            level,
            game_changers: gc,
            tutors: tut,
            extra_turns: turnos,
            mass_land_denial: mld,
        }
    });

    let roles_v: Vec<Rol> = FUNCIONES
        .iter()
        .map(|k| {
            let g = roles.remove(k).unwrap_or_default();
            Rol {
                key: (*k).to_string(),
                count: g.total(),
                cards: g.lista(),
            }
        })
        .collect();

    let mut sets_v: Vec<SetCuenta> = sets
        .into_iter()
        .map(|(code, (name, count))| SetCuenta { code, name, count })
        .collect();
    sets_v.sort_by(|a, b| b.count.cmp(&a.count).then(a.code.cmp(&b.code)));

    DeckStats {
        kind,
        detected_kind: detectada,
        cards: cartas,
        deck_size: tamano,
        lands: tierras,
        nonlands: no_tierras,
        tokens: fichas.total(),
        unknown: desconocidas.total(),
        unknown_cards: desconocidas.lista(),
        token_cards: fichas.lista(),
        average_cmc: centimos(media),
        average_cmc_with_lands: if no_tierras + tierras > 0 {
            centimos((suma_cmc + suma_tierras_cmc) / f64::from(no_tierras + tierras))
        } else {
            0.0
        },
        median_cmc: mediana,
        total_mana_value: centimos(suma_cmc),
        curve: curva,
        colors: colores,
        color_identity: identidad,
        color_count: num_colores,
        x_spells,
        types: contador(tipos, usize::MAX),
        supertypes: contador(supertipos, usize::MAX),
        subtypes: contador(subtipos_m, MAX_LISTA),
        keywords: contador(palabras, MAX_LISTA),
        creatures: Criaturas {
            count: cr_n,
            average_power: media_f(&fuerzas),
            average_toughness: media_f(&resistencias),
            power: por_fuerza,
        },
        rarity: contador(rareza, usize::MAX),
        sets: sets_v,
        oldest: mas_viejo,
        newest: mas_nuevo,
        roles: roles_v,
        prices: Precios {
            total_eur: centimos(total_eur),
            total_usd: centimos(total_usd),
            without_eur: sin_eur,
            without_usd: sin_usd,
            cards: con_precio,
            by_type: por_tipo_v,
            buckets: ["<1", "1-5", "5-20", "20-50", "50+"]
                .iter()
                .zip(tramos)
                .map(|(k, count)| Contador {
                    key: (*k).to_string(),
                    count,
                })
                .collect(),
        },
        probability: probabilidad,
        recommended_lands: tierras_recomendadas(kind, media, baratas),
        cheap_ramp_draw: baratas,
        legality: FORMATOS
            .iter()
            .map(|f| {
                let malas = legalidad.remove(f).unwrap_or_default();
                Legalidad {
                    format: (*f).to_string(),
                    legal: cartas > 0 && malas.is_empty(),
                    offenders: malas,
                }
            })
            .collect(),
        problems: problemas,
        bracket,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn carta(nombre: &str, cmc: f64, tipo: &str, colores: &[&str], eur: Option<&str>) -> StatInput {
        StatInput {
            id: format!("id-{nombre}"),
            name: nombre.into(),
            cmc: Some(cmc),
            colors: colores.iter().map(|c| (*c).to_string()).collect(),
            color_identity: colores.iter().map(|c| (*c).to_string()).collect(),
            type_line: Some(tipo.into()),
            rarity: Some("common".into()),
            price_eur: eur.map(Into::into),
            legalities: FORMATOS
                .iter()
                .map(|f| ((*f).to_string(), "legal".to_string()))
                .collect(),
            ..StatInput::default()
        }
    }

    fn deck() -> Vec<StatInput> {
        let mut caro = carta("Sol Ring", 1.0, "Artifact", &[], Some("2.50"));
        caro.legalities.insert("modern".into(), "not_legal".into());
        caro.produced_mana = vec!["C".into()];
        caro.oracle_text = "{T}: Add {C}{C}.".into();
        caro.game_changer = true;
        let mut rayo = carta("Lightning Bolt", 1.0, "Instant", &["R"], Some("0.80"));
        rayo.mana_cost = Some("{R}".into());
        rayo.oracle_text = "Lightning Bolt deals 3 damage to any target.".into();
        let mut isla = carta("Island", 0.0, "Basic Land — Island", &[], None);
        isla.produced_mana = vec!["U".into()];
        let mut titan = carta("Grave Titan", 6.0, "Creature — Giant", &["B"], Some("3"));
        titan.mana_cost = Some("{4}{B}{B}".into());
        titan.power = Some("6".into());
        titan.toughness = Some("6".into());
        titan.keywords = vec!["Deathtouch".into()];
        vec![
            rayo.clone(),
            rayo,
            isla,
            titan,
            carta(
                "Emrakul",
                15.0,
                "Legendary Creature — Eldrazi",
                &[],
                Some("10"),
            ),
            caro,
            StatInput {
                name: "Treasure".into(),
                is_token: true,
                ..StatInput::default()
            },
            StatInput {
                id: "x".into(),
                name: "Propia".into(),
                ..StatInput::default()
            },
        ]
    }

    #[test]
    fn cuenta_curva_colores_tipos_y_precio() {
        let s = stats(&deck(), None);
        assert_eq!(s.cards, 6);
        assert_eq!(s.deck_size, 7);
        assert_eq!(s.lands, 1);
        assert_eq!(s.nonlands, 5);
        assert_eq!(s.tokens, 1);
        assert_eq!(s.unknown, 1);
        assert_eq!(s.unknown_cards[0].ids, vec!["x"]);
        let totales: Vec<u32> = s.curve.iter().map(|t| t.total).collect();
        assert_eq!(totales, vec![0, 3, 0, 0, 0, 0, 1, 1]);
        let uno = &s.curve[1];
        assert_eq!((uno.permanents, uno.spells), (1, 2));
        let inst = uno.by_type.iter().find(|t| t.key == "Instant").unwrap();
        assert_eq!(inst.count, 2);
        assert_eq!(s.prices.total_eur, 17.1);
        assert_eq!(s.prices.without_eur, 1);
        assert_eq!(s.prices.cards[0].name, "Emrakul");
        assert_eq!(s.color_identity, vec!["B", "R"]);
        let r = s.colors.iter().find(|c| c.color == "R").unwrap();
        assert_eq!((r.cards, r.pips), (2, 2));
        let b = s.colors.iter().find(|c| c.color == "B").unwrap();
        assert_eq!(b.pips, 2);
        let u = s.colors.iter().find(|c| c.color == "U").unwrap();
        assert_eq!(u.land_sources, 1);
        let c = s.colors.iter().find(|c| c.color == "C").unwrap();
        assert_eq!((c.cards, c.other_sources), (2, 1), "Emrakul y Sol Ring");
        assert_eq!(s.types[0].key, "Creature");
        assert_eq!(s.creatures.count, 2);
        assert_eq!(s.creatures.average_power, Some(6.0));
        assert_eq!(s.keywords[0].key, "Deathtouch");
        assert!(s.subtypes.iter().any(|t| t.key == "Giant"));
        assert!(s.supertypes.iter().any(|t| t.key == "Legendary"));
        let modern = s.legality.iter().find(|l| l.format == "modern").unwrap();
        assert!(!modern.legal);
        assert_eq!(modern.offenders, vec!["Sol Ring"]);
        let ramp = s.roles.iter().find(|r| r.key == "ramp").unwrap();
        assert_eq!(ramp.cards[0].name, "Sol Ring");
        let removal = s.roles.iter().find(|r| r.key == "removal").unwrap();
        assert_eq!(removal.count, 2);
        assert_eq!(removal.cards[0].ids.len(), 2);
        assert_eq!(s.cheap_ramp_draw, 1);
        assert_eq!(s.token_cards[0].name, "Treasure");
    }

    #[test]
    fn probabilidades_del_deck() {
        let s = stats(&deck(), None);
        let p = &s.probability;
        assert_eq!(p.deck_size, 7);
        // Con 7 cartas vistas de 7, la tierra sale seguro.
        assert_eq!(p.opening_lands[1], 1.0);
        let bolt = p.cards.iter().find(|c| c.key == "Lightning Bolt").unwrap();
        assert_eq!(bolt.copies, 2);
        assert_eq!(bolt.by_seen[0], 1.0);
        assert!(p.roles.iter().any(|r| r.key == "removal"));
    }

    #[test]
    fn la_clase_de_mazo_y_sus_problemas() {
        let mut cien: Vec<StatInput> = (0..60)
            .map(|i| carta(&format!("C{i}"), 2.0, "Creature — Elf", &["G"], None))
            .collect();
        cien.extend((0..40).map(|_| carta("Forest", 0.0, "Basic Land — Forest", &[], None)));
        assert_eq!(deducir(&cien), DeckKind::Commander);
        let s = stats(&cien, None);
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert!(s.bracket.is_some());
        assert_eq!(s.bracket.as_ref().unwrap().level, 2);

        // Como construido, 100 cartas valen y las copias de Forest son libres.
        let s = stats(&cien, Some(DeckKind::Constructed));
        assert!(s.problems.is_empty());
        assert!(s.bracket.is_none());

        // Dos copias de una no básica en Commander.
        cien[1].name = "C0".into();
        let s = stats(&cien, Some(DeckKind::Commander));
        assert_eq!(s.problems[0].code, "copies");
        assert_eq!(s.problems[0].cards[0].name, "C0");

        // 61 cartas → construido; 40 → limitado; 99 en Commander → falta una.
        assert_eq!(deducir(&cien[..61]), DeckKind::Constructed);
        assert_eq!(deducir(&cien[..40]), DeckKind::Limited);
        let s = stats(&cien[..99], Some(DeckKind::Commander));
        assert!(s
            .problems
            .iter()
            .any(|p| p.code == "size" && p.actual == Some(99)));
    }

    #[test]
    fn el_bracket() {
        let mut v: Vec<StatInput> = (0..100)
            .map(|i| carta(&format!("C{i}"), 2.0, "Instant", &["U"], None))
            .collect();
        v[0].game_changer = true;
        assert_eq!(stats(&v, None).bracket.unwrap().level, 3);
        for c in v.iter_mut().take(4) {
            c.game_changer = true;
        }
        assert_eq!(stats(&v, None).bracket.unwrap().level, 4);
        for c in v.iter_mut().take(4) {
            c.game_changer = false;
        }
        v[5].oracle_text = "Destroy all lands.".into();
        let b = stats(&v, None).bracket.unwrap();
        assert_eq!(b.level, 4);
        assert_eq!(b.mass_land_denial[0].name, "C5");
    }

    #[test]
    fn las_tierras_de_karsten() {
        assert_eq!(tierras_recomendadas(DeckKind::Commander, 3.0, 10), 38.0);
        assert_eq!(tierras_recomendadas(DeckKind::Constructed, 2.0, 0), 23.4);
    }

    #[test]
    fn el_tipo_principal() {
        assert_eq!(
            tipo_principal("Legendary Artifact Creature — Golem"),
            "Creature"
        );
        assert_eq!(tipo_principal("Artifact — Equipment"), "Artifact");
        assert_eq!(tipo_principal("Instant // Sorcery"), "Instant");
        assert_eq!(tipo_principal("Tribal Kindred"), "Other");
        assert_eq!(
            subtipos("Creature — Human Wizard // Land"),
            vec!["Human", "Wizard"]
        );
    }

    #[test]
    fn un_deck_vacio_no_es_legal_en_nada() {
        let s = stats(&[], None);
        assert!(s.legality.iter().all(|l| !l.legal));
        assert_eq!(s.average_cmc, 0.0);
        assert!(s.problems.is_empty());
        assert_eq!(s.probability.deck_size, 0);
    }
}
