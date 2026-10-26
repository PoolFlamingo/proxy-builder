//! **Las imágenes de la partida en red** (`proxy-builder/img/1`): las cartas
//! propias y los tapetes de cada jugador, que los demás piden por su huella.
//! (Fase E, paso E8.)

use iroh::endpoint::Connection;

use super::mesa::Mesa;

/// Atiende a quien pide imágenes a este nodo.
pub async fn servir<M: Mesa>(conn: Connection, _mesa: M) {
    conn.close(0u32.into(), b"pronto");
}
