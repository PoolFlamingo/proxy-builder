//! **El código directo** (sin servidor): lo que el anfitrión pasa a los demás
//! para que se unan. Lleva su dirección de iroh (`EndpointAddr`: su clave y
//! por dónde llegar) y un secreto de 16 bytes que el anfitrión pide en el
//! `Hola`: sin él no admite a nadie. Es texto: `PB1-` + base64url del JSON.

use base64::Engine;
use iroh::EndpointAddr;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorCode};

const PREFIJO: &str = "PB1-";
/// Lo más largo que se acepta pegar.
const MAX: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Codigo {
    #[serde(rename = "a")]
    pub addr: EndpointAddr,
    /// El secreto, en hexadecimal (32 caracteres).
    #[serde(rename = "s")]
    pub secreto: String,
}

/// Un secreto nuevo de 16 bytes, en hexadecimal.
pub fn secreto_nuevo() -> String {
    let mut b = [0u8; 16];
    if getrandom::fill(&mut b).is_err() {
        // Sin entropía (no pasa en la práctica): el reloj y la dirección.
        let t = crate::store::ahora_ms().to_le_bytes();
        b[..8].copy_from_slice(&t);
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Compara dos secretos sin cortar en la primera diferencia.
pub fn mismo_secreto(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

impl Codigo {
    pub fn a_texto(&self) -> AppResult<String> {
        let j = serde_json::to_vec(self)
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
        Ok(format!(
            "{PREFIJO}{}",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(j)
        ))
    }

    pub fn de_texto(t: &str) -> AppResult<Codigo> {
        let malo = || AppError::new(ErrorCode::InvalidInput).with("motivo", "codigo");
        let t: String = t.chars().filter(|c| !c.is_whitespace()).collect();
        if t.len() > MAX {
            return Err(malo());
        }
        let cuerpo = t.strip_prefix(PREFIJO).ok_or_else(malo)?;
        let j = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(cuerpo)
            .map_err(|_| malo())?;
        let c: Codigo = serde_json::from_slice(&j).map_err(|_| malo())?;
        if c.secreto.len() != 32 || !c.secreto.chars().all(|x| x.is_ascii_hexdigit()) {
            return Err(malo());
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_y_codigos_malos() {
        let k = iroh::SecretKey::generate();
        let c = Codigo {
            addr: EndpointAddr::new(k.public()),
            secreto: secreto_nuevo(),
        };
        let t = c.a_texto().unwrap();
        assert!(t.starts_with("PB1-"));
        // Con espacios o saltos de línea al pegarlo, igual.
        let pegado = format!("  {}\n{} ", &t[..10], &t[10..]);
        assert_eq!(Codigo::de_texto(&pegado).unwrap(), c);
        for malo in [
            "",
            "PB1-",
            "PB2-abc",
            "PB1-%%%",
            &format!("PB1-{}", "a".repeat(5_000)),
        ] {
            assert!(Codigo::de_texto(malo).is_err(), "{malo}");
        }
    }

    #[test]
    fn secretos() {
        let a = secreto_nuevo();
        assert_eq!(a.len(), 32);
        assert_ne!(a, secreto_nuevo());
        assert!(mismo_secreto(&a, &a.clone()));
        assert!(!mismo_secreto(&a, &secreto_nuevo()));
        assert!(!mismo_secreto(&a, "corto"));
    }
}
