//! **Núcleo puro de Proxy Builder.**
//!
//! Todo lo determinista del producto vive aquí: el modelo de datos, las
//! unidades, los formatos de papel, la geometría de página (`PagePlan`), el
//! parser de listas de cartas y, más adelante, el procesado de imagen y el
//! PDF. Sin Tauri, sin red y sin base de datos: se prueba con
//! `cargo test -p proxy-core` en segundos.
//!
//! 🛑 Este crate no conoce `AppError` (vive en el crate de la app). Cada
//! módulo tiene su propio error y la app los convierte con `From`.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod decklist;
pub mod imaging;
pub mod layout;
pub mod model;
pub mod paper;
pub mod units;

/// Versión del núcleo, para los metadatos del PDF y los paquetes de deck.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
