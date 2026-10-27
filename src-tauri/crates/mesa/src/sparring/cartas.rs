//! **Las cartas del fantasma**: unas 120 cartas reales de interacción de
//! Commander (`assets/sparring/cartas.json`), cada una con lo que hace, a
//! qué tipos de carta puede responder y lo que cuesta. Van por nombre: la
//! app busca la imagen en Scryfall al enseñarla.

use std::sync::OnceLock;

use serde::Deserialize;

use super::amenaza::{tipos, Tipos};
use super::TipoReaccion;
use crate::modelo::InfoCarta;

/// A qué responde una carta del fantasma.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Categoria {
    Criatura,
    Artefacto,
    Encantamiento,
    Planeswalker,
    Batalla,
    Instantaneo,
    Conjuro,
    /// Cualquier permanente que no sea tierra.
    Permanente,
    /// Cualquier hechizo que no sea de criatura.
    NoCriatura,
    /// Cualquier hechizo.
    Todo,
    /// Cualquier tierra (las que se quitan: las fuertes).
    Tierra,
    /// Una tierra que no es básica.
    TierraNoBasica,
}

impl Categoria {
    fn incluye(self, t: &Tipos) -> bool {
        match self {
            Categoria::Criatura => t.criatura,
            Categoria::Artefacto => t.artefacto,
            Categoria::Encantamiento => t.encantamiento,
            Categoria::Planeswalker => t.planeswalker,
            Categoria::Batalla => t.batalla,
            Categoria::Instantaneo => t.instantaneo,
            Categoria::Conjuro => t.conjuro,
            Categoria::Permanente => !t.tierra && !t.instantaneo && !t.conjuro,
            Categoria::NoCriatura => !t.criatura,
            Categoria::Todo => true,
            Categoria::Tierra => t.tierra,
            Categoria::TierraNoBasica => t.tierra && !t.basica,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CartaDelFantasma {
    pub nombre: String,
    pub coste: String,
    /// El valor de maná.
    pub mv: u16,
    pub tipo: TipoReaccion,
    pub responde: Vec<Categoria>,
    /// Lo que quita va al exilio (si no, al cementerio).
    #[serde(default)]
    pub exilia: bool,
    /// Se puede lanzar sin pagar su coste (Force of Will, Deadly Rollick).
    #[serde(default)]
    pub gratis: bool,
    /// No apunta (sacrificios, Council's Judgment): el antimaleficio no la para.
    #[serde(default)]
    pub no_apunta: bool,
    /// Solo cartas de un color (Vanishing Verse).
    #[serde(default)]
    pub monocolor: bool,
    /// Valor de maná mínimo o máximo de lo que puede quitar (Despark, Abrupt Decay).
    #[serde(default)]
    pub mv_min: Option<u16>,
    #[serde(default)]
    pub mv_max: Option<u16>,
    /// Lo que de verdad paga si se abarata sola (Blasphemous Act).
    #[serde(default)]
    pub paga: Option<u16>,
}

impl CartaDelFantasma {
    /// En la ronda N el fantasma tiene N manás (las gratis, siempre).
    pub fn se_puede_pagar(&self, ronda: u32) -> bool {
        self.gratis || u32::from(self.paga.unwrap_or(self.mv)) <= ronda
    }

    /// Si puede responder a esta carta (por sus tipos, su coste y sus colores).
    pub fn vale_para(&self, info: &InfoCarta) -> bool {
        let t = tipos(info);
        let mv = info.cmc();
        self.responde.iter().any(|c| c.incluye(&t))
            && self.mv_min.is_none_or(|m| mv >= m)
            && self.mv_max.is_none_or(|m| mv <= m)
            && (!self.monocolor || info.colores.len() == 1)
    }
}

/// Todas las cartas del fantasma. Si el JSON no se pudiera leer, ninguna (y
/// el fantasma se calla): nunca un pánico por un dato.
pub fn todas() -> &'static [CartaDelFantasma] {
    static CARTAS: OnceLock<Vec<CartaDelFantasma>> = OnceLock::new();
    CARTAS.get_or_init(|| {
        serde_json::from_str(include_str!("../../assets/sparring/cartas.json")).unwrap_or_default()
    })
}

pub fn por_nombre(nombre: &str) -> Option<&'static CartaDelFantasma> {
    todas().iter().find(|c| c.nombre == nombre)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_json_se_lee_entero_y_sin_repetidas() {
        let v = todas();
        assert!(v.len() >= 110, "{} cartas", v.len());
        let mut nombres: Vec<&str> = v.iter().map(|c| c.nombre.as_str()).collect();
        nombres.sort_unstable();
        nombres.dedup();
        assert_eq!(nombres.len(), v.len());
        for tipo in [
            TipoReaccion::Eliminar,
            TipoReaccion::Contrarrestar,
            TipoReaccion::Barrer,
            TipoReaccion::Devolver,
            TipoReaccion::Sacrificio,
        ] {
            assert!(v.iter().any(|c| c.tipo == tipo), "{tipo:?}");
        }
        // Los sacrificios nunca apuntan.
        assert!(v
            .iter()
            .filter(|c| c.tipo == TipoReaccion::Sacrificio)
            .all(|c| c.no_apunta));
    }
}
