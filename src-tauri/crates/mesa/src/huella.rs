//! **La huella de la partida**: comprobar rápido que dos copias son iguales
//! (red, repetición, tests).
//!
//! SHA-256 del JSON de la partida, en 16 caracteres hexadecimales. Todo en
//! la partida son `BTreeMap` y `Vec` (nunca `HashMap`), así el JSON sale
//! siempre igual. El `seq` no cuenta: deshacer deja la partida igual que si
//! no se hubiera hecho, aunque el contador de eventos siga subiendo.

use sha2::{Digest, Sha256};

use crate::modelo::Partida;

pub fn huella(p: &Partida) -> String {
    let mut sin_seq = p.clone();
    sin_seq.seq = 0;
    let json = serde_json::to_vec(&sin_seq).unwrap_or_default();
    let h = Sha256::digest(&json);
    h.iter().take(8).map(|b| format!("{b:02x}")).collect()
}
