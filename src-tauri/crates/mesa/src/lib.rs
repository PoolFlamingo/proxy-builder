//! **El motor de la mesa de juego.**
//!
//! Qué es: la cuenta de una partida de Magic como en la mesa de la cocina.
//! Sabe dónde está cada carta y qué se le ha hecho (girada, contadores,
//! boca abajo…), apunta cada acción como un evento, decide el azar
//! (barajar, dados) y le dice a cada jugador qué puede ver.
//!
//! Qué no es: un juez. 🛑 No hay reglas de Magic (ni maná, ni fases, ni
//! objetivos legales, ni combate): solo comprueba que la acción tenga
//! sentido (la carta existe, el actor puede tocarla) y ofrece ayudas que se
//! pueden apagar (robar y enderezar al empezar el turno, el impuesto del
//! comandante).
//!
//! Puro y determinista, como `proxy-core`: sin Tauri, sin red, sin base de
//! datos, sin reloj y sin azar de fuera. El ciclo es siempre el mismo:
//!
//! `Intencion` → `resolver` → `Evento` → `aplicar` → `Partida` → `vista`
//!
//! y el mismo registro de eventos sirve para el solitario, la red, deshacer
//! y la repetición.

#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod accion;
pub mod aplicar;
pub mod azar;
pub mod error;
pub mod formato;
pub mod huella;
pub mod ids;
pub mod lista;
pub mod modelo;
pub mod preparar;
pub mod registro;
pub mod resolver;
pub mod sparring;
pub mod visibilidad;
pub mod vista;

pub use accion::{Accion, Cambio, Evento, Intencion, VERSION_CATALOGO};
pub use aplicar::{aplicar, Tocado};
pub use azar::Azar;
pub use error::Rechazo;
pub use huella::huella;
pub use ids::{IdCarta, IdEquipo, IdJugador};
pub use modelo::Partida;
pub use registro::{ModoDeshacer, Registro};
pub use resolver::resolver;
pub use visibilidad::Mirador;
pub use vista::{parche, vista, ParcheVista, VistaMesa};
