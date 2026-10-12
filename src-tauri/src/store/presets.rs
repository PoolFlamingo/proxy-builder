//! **Presets**: desplazamientos de fila por impresora (frente/reverso) y
//! ajustes de impresión guardados con nombre. Se usan en la Fase 7.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::db::Db;
use crate::error::AppResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PresetKind {
    FrontOffsets,
    BackOffsets,
    Print,
}

impl PresetKind {
    fn texto(self) -> &'static str {
        match self {
            PresetKind::FrontOffsets => "frontOffsets",
            PresetKind::BackOffsets => "backOffsets",
            PresetKind::Print => "print",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Preset {
    pub id: String,
    pub kind: PresetKind,
    pub name: String,
    /// El contenido (JSON de `Vec<Offset>` o de `PrintSettings`).
    pub json: String,
}

pub fn list(db: &Db, kind: PresetKind) -> AppResult<Vec<Preset>> {
    let c = db.conn()?;
    let mut st = c.prepare("SELECT id, name, json FROM presets WHERE kind = ?1 ORDER BY name")?;
    let filas = st.query_map([kind.texto()], |r| {
        Ok(Preset {
            id: r.get(0)?,
            kind,
            name: r.get(1)?,
            json: r.get(2)?,
        })
    })?;
    Ok(filas.collect::<Result<_, _>>()?)
}

/// Guarda un preset; si ya hay uno del mismo tipo y nombre, lo reemplaza.
pub fn save(db: &Db, kind: PresetKind, name: &str, json: &str) -> AppResult<Preset> {
    let c = db.conn()?;
    let existente: Option<String> = rusqlite::OptionalExtension::optional(c.query_row(
        "SELECT id FROM presets WHERE kind = ?1 AND name = ?2",
        (kind.texto(), name.trim()),
        |r| r.get(0),
    ))?;
    let id = existente.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    c.execute(
        "INSERT INTO presets (id, kind, name, json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at",
        (&id, kind.texto(), name.trim(), json, super::ahora_ms()),
    )?;
    Ok(Preset {
        id,
        kind,
        name: name.trim().to_string(),
        json: json.to_string(),
    })
}

pub fn delete(db: &Db, id: &str) -> AppResult<()> {
    db.conn()?
        .execute("DELETE FROM presets WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardar_con_el_mismo_nombre_reemplaza() {
        let db = Db::in_memory().unwrap();
        let a = save(&db, PresetKind::BackOffsets, "Brother", "[]").unwrap();
        let b = save(
            &db,
            PresetKind::BackOffsets,
            "Brother",
            "[{\"xMm\":0.5,\"yMm\":0}]",
        )
        .unwrap();
        assert_eq!(a.id, b.id);
        let l = list(&db, PresetKind::BackOffsets).unwrap();
        assert_eq!(l.len(), 1);
        assert!(l[0].json.contains("0.5"));
        assert!(list(&db, PresetKind::FrontOffsets).unwrap().is_empty());
        delete(&db, &a.id).unwrap();
        assert!(list(&db, PresetKind::BackOffsets).unwrap().is_empty());
    }
}
