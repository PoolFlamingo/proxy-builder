//! **Volver a una partida en red** de la que se cayó un invitado (se cortó la
//! red, se cerró la app, se durmió el equipo…). El anfitrión ya readmite a
//! quien vuelve por su nodo (`Anfitrion::admitir`) y le manda la partida
//! entera; lo que faltaba era recordar con qué código se entró.
//!
//! Se apunta en `app_settings` al recibir la partida (también al reconectar
//! o tras un traspaso, con el código del anfitrión de ahora) y se olvida
//! cuando ya no hay a qué volver: el anfitrión la cerró, rechazó la vuelta o
//! la persona salió a propósito estando conectada.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::codigo::Codigo;
use super::invitado::Union;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::db::Db;
use crate::store::settings;

const CLAVE: &str = "red.vuelta";

/// Lo que se guarda (el código va en texto, como se pega).
#[derive(Serialize, Deserialize)]
struct Guardada {
    codigo: String,
    apodo: String,
    cuando_ms: u64,
}

/// La partida a la que se puede volver, para la pantalla.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VueltaRed {
    pub apodo: String,
    /// Cuándo se recibió la partida por última vez (ms desde 1970).
    #[ts(type = "number")]
    pub cuando_ms: u64,
}

fn ahora_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Apunta la unión de un jugador (los espectadores no vuelven: entran a mirar).
pub fn recordar(db: &Db, u: &Union) -> AppResult<()> {
    let g = Guardada {
        codigo: u.codigo.a_texto()?,
        apodo: u.apodo.clone(),
        cuando_ms: ahora_ms(),
    };
    let j = serde_json::to_string(&g)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    settings::set(db, CLAVE, &j)
}

pub fn olvidar(db: &Db) -> AppResult<()> {
    settings::set(db, CLAVE, "")
}

fn guardada(db: &Db) -> AppResult<Option<Guardada>> {
    Ok(settings::get(db, CLAVE)?
        .filter(|v| !v.is_empty())
        .and_then(|v| serde_json::from_str(&v).ok()))
}

pub fn leer(db: &Db) -> AppResult<Option<VueltaRed>> {
    Ok(guardada(db)?.map(|g| VueltaRed {
        apodo: g.apodo,
        cuando_ms: g.cuando_ms,
    }))
}

/// El código y el apodo con los que volver (un código que ya no se lee, nada).
pub fn para_volver(db: &Db) -> AppResult<Option<(Codigo, String)>> {
    Ok(guardada(db)?.and_then(|g| Codigo::de_texto(&g.codigo).ok().map(|c| (c, g.apodo))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::red::protocolo::Rol;

    fn union() -> Union {
        let k = iroh::SecretKey::generate();
        Union {
            codigo: Codigo {
                addr: iroh::EndpointAddr::new(k.public()),
                secreto: crate::red::codigo::secreto_nuevo(),
            },
            apodo: "Ana".into(),
            rol: Rol::Jugador,
            deck: None,
        }
    }

    #[test]
    fn se_recuerda_y_se_olvida() {
        let db = Db::in_memory().unwrap();
        assert_eq!(leer(&db).unwrap(), None);
        let u = union();
        recordar(&db, &u).unwrap();
        assert_eq!(leer(&db).unwrap().unwrap().apodo, "Ana");
        let (c, apodo) = para_volver(&db).unwrap().unwrap();
        assert_eq!(c.secreto, u.codigo.secreto);
        assert_eq!(apodo, "Ana");
        olvidar(&db).unwrap();
        assert_eq!(leer(&db).unwrap(), None);
        assert!(para_volver(&db).unwrap().is_none());
    }
}
