//! **Diseño de cartas** (la 2.0): el motor que pinta una carta proxy a partir
//! de los datos de la carta real y de lo temático que pone el usuario.
//!
//! - `letras`: las letras cargadas (las de la casa y las importadas).
//! - `texto`: el texto enriquecido (reglas, sabor, símbolos) y su maquetación
//!   con `parley`, incluido el ajuste para que quepa.
//! - `simbolos`: los símbolos de maná (SVG de Scryfall) y cómo se pintan.
//! - `pintar`: pasar un bloque maquetado a píxeles con `tiny-skia`.
//!
//! Todo trabaja en píxeles de la imagen de salida: quien llama convierte de
//! milímetros con `px_por_mm`. Nada lee letras del sistema: solo las que se
//! cargan aquí, para que el resultado sea igual en cualquier equipo.

pub mod borrador;
pub mod efectivo;
pub mod letras;
pub mod modelo;
pub mod pintar;
pub mod simbolos;
pub mod texto;

/// Lo que puede fallar al diseñar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DisenoError {
    #[error("el fichero no es una letra válida")]
    LetraNoValida,
    #[error("símbolo dañado: {0}")]
    SimboloNoValido(String),
    #[error("lienzo de tamaño imposible")]
    Lienzo,
}

/// Versión del motor de render: entra en la huella, así que subirla rehace
/// todos los renders de diseños.
pub const VERSION_MOTOR: u32 = 1;

/// Puntos tipográficos a milímetros.
pub const MM_POR_PT: f32 = 25.4 / 72.0;

/// Un color RGBA sin premultiplicar.
pub type Color = [u8; 4];
