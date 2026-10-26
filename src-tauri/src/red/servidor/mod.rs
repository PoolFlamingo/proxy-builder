//! **El servidor de salas** (Fase F): playgroups, invitaciones y salas por un
//! WebSocket (`server/docs/protocolo.md`). La partida sigue siendo P2P; el
//! servidor junta a la gente, reparte las direcciones de iroh, elige el host
//! y nombra otro si se cae.
//!
//! - `mensajes.rs`: el protocolo v1 (con sus pruebas de contrato).
//! - `cliente.rs`: el WebSocket, el reto firmado y la reconexión.
//! - `sala.rs`: la sala en la app (malla, host, migración y avisos).

pub mod cliente;
pub mod mensajes;
pub mod sala;

/// La URL del servidor. En producción, la de Pablo (aún no hay dominio: sin
/// ella, las salas dicen que no hay servidor). En debug, `PB_SERVIDOR` la
/// cambia (`ws://127.0.0.1:8787/v1/ws` con el servidor local).
pub fn url() -> Option<String> {
    const PRODUCCION: Option<&str> = None;
    if cfg!(debug_assertions) {
        if let Ok(u) = std::env::var("PB_SERVIDOR") {
            let u = u.trim().to_string();
            if !u.is_empty() {
                return Some(u);
            }
        }
    }
    PRODUCCION.map(str::to_string)
}
