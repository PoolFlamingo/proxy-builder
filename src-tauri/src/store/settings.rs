//! Ajustes sueltos de la app (clave → valor): el último deck abierto, etc.

use rusqlite::OptionalExtension;

use super::db::Db;
use crate::error::AppResult;

pub fn get(db: &Db, clave: &str) -> AppResult<Option<String>> {
    Ok(db
        .conn()?
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [clave],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn set(db: &Db, clave: &str, valor: &str) -> AppResult<()> {
    db.conn()?.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (clave, valor),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardar_y_leer_un_ajuste() {
        let db = Db::in_memory().unwrap();
        assert_eq!(get(&db, "x").unwrap(), None);
        set(&db, "x", "1").unwrap();
        set(&db, "x", "2").unwrap();
        assert_eq!(get(&db, "x").unwrap().as_deref(), Some("2"));
    }
}
