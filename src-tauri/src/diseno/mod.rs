//! **Editor de cartas en la app** (la 2.0): lo que necesita red, disco o
//! base de datos alrededor del motor puro de `proxy_core::diseno`.
//!
//! - `borrador`: de la carta de Scryfall (inglés + impresión española) a un
//!   `CardDesign` con su instantánea mecánica.

pub mod arte;
pub mod borrador;
pub mod descargables;
pub mod letras_libres;
pub mod lote;
pub mod original;
pub mod pintar;
pub mod png;
pub mod recursos;
pub mod traduccion;
pub mod vista;
