//! **Geometría de página.** `grid` cuenta casillas; `plan` coloca cada carta
//! en cada cara de cada hoja. Ver `plan.rs` para las reglas de dúplex y DFC.

pub mod grid;
pub mod plan;

pub use grid::{Grid, RectMm};
pub use plan::{plan, LineMm, Page, PagePlan, PlanWarning, SheetSide, Slot, SlotFace};

/// Lo que puede fallar al calcular la geometría.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LayoutError {
    /// La carta no cabe en el papel con esos márgenes.
    #[error("la carta no cabe en el papel")]
    CardDoesNotFit,
    /// Medidas imposibles (negativas o cero).
    #[error("ajustes de página no válidos")]
    InvalidSettings,
}
