//! Abrir la base de datos y aplicar las migraciones.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

use crate::error::{AppError, AppResult, ErrorCode};

/// Migraciones en orden. 🛑 Solo se añaden al final; nunca se edita una que
/// ya haya salido en una versión.
const MIGRACIONES: &[(i64, &str)] = &[
    (1, include_str!("migrations/0001_init.sql")),
    (2, include_str!("migrations/0002_blob_refs.sql")),
    (3, include_str!("migrations/0003_biblioteca.sql")),
];

/// La conexión, protegida por un candado (rusqlite no es `Sync`). Los
/// trabajos largos sueltan el candado entre pasos.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(ruta: &Path) -> AppResult<Db> {
        if let Some(dir) = ruta.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(ruta)?;
        Self::preparar(conn)
    }

    /// Base en memoria, para los tests.
    pub fn in_memory() -> AppResult<Db> {
        Self::preparar(Connection::open_in_memory()?)
    }

    fn preparar(mut conn: Connection) -> AppResult<Db> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        migrar(&mut conn)?;
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    pub fn conn(&self) -> AppResult<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal).with("detalle", "candado de la base"))
    }
}

fn migrar(conn: &mut Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);",
    )?;
    for (version, sql) in MIGRACIONES {
        let hecha: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |r| r.get(0),
        )?;
        if hecha {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
            (version, super::ahora_ms()),
        )?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abrir_dos_veces_aplica_la_migracion_una_sola_vez() {
        let dir = tempfile::tempdir().unwrap();
        let ruta = dir.path().join("library.db");
        drop(Db::open(&ruta).unwrap());
        let db = Db::open(&ruta).unwrap();
        let c = db.conn().unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, MIGRACIONES.len() as i64);
        let ok: String = c
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(ok, "ok");
    }
}
