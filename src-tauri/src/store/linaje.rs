//! **El linaje de cada deck** (`deck_linaje`, migración 0009): qué decks de
//! equipos distintos son el mismo, para que pasar un deck pregunte si ya lo
//! hay en vez de duplicarlo. Aparte del deck, como las etiquetas: no cambia
//! `updated_at` y se borra con él.
//!
//! - Un deck nuevo toma su propio id (lo hace `decks::save`).
//! - Un duplicado es otro deck: otro linaje (también `decks::save`).
//! - Uno importado o recibido conserva el linaje del que vino (`poner`).
//! - Un deck sin fila (de antes de la migración, o metido a mano en un test)
//!   cuenta con su id como linaje.

use rusqlite::OptionalExtension;

use super::db::Db;
use crate::error::AppResult;

/// El linaje de una copia y cuándo se cambió su contenido por última vez
/// (ms, en el equipo donde se cambió).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linaje {
    pub linaje: String,
    pub actualizado: i64,
}

/// El linaje de un deck (el suyo propio si no tiene fila).
pub fn de(db: &Db, deck_id: &str) -> AppResult<Linaje> {
    let c = db.conn()?;
    let fila: Option<Linaje> = c
        .query_row(
            "SELECT linaje, actualizado FROM deck_linaje WHERE deck_id = ?1",
            [deck_id],
            |r| {
                Ok(Linaje {
                    linaje: r.get(0)?,
                    actualizado: r.get(1)?,
                })
            },
        )
        .optional()?;
    if let Some(l) = fila {
        return Ok(l);
    }
    let updated: Option<i64> = c
        .query_row(
            "SELECT updated_at FROM decks WHERE id = ?1",
            [deck_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(Linaje {
        linaje: deck_id.to_string(),
        actualizado: updated.unwrap_or(0),
    })
}

/// Fija el linaje de un deck (el de donde vino) y la hora de su versión.
pub fn poner(db: &Db, deck_id: &str, linaje: &str, actualizado: i64) -> AppResult<()> {
    db.conn()?.execute(
        "INSERT INTO deck_linaje (deck_id, linaje, actualizado) VALUES (?1, ?2, ?3)
         ON CONFLICT(deck_id) DO UPDATE SET linaje = excluded.linaje,
           actualizado = excluded.actualizado",
        (deck_id, linaje, actualizado),
    )?;
    Ok(())
}

/// Los decks de esta biblioteca de ese linaje, el más reciente primero.
pub fn decks_de(db: &Db, linaje: &str) -> AppResult<Vec<(String, i64)>> {
    let c = db.conn()?;
    // Los sin fila cuentan con su id: se buscan por los dos caminos.
    let mut st = c.prepare(
        "SELECT d.id, COALESCE(l.actualizado, d.updated_at)
         FROM decks d LEFT JOIN deck_linaje l ON l.deck_id = d.id
         WHERE COALESCE(l.linaje, d.id) = ?1
         ORDER BY 2 DESC, d.id",
    )?;
    let v = st
        .query_map([linaje], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::decks;

    #[test]
    fn un_deck_nuevo_es_su_propio_linaje_y_un_duplicado_otro() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "A", None).unwrap();
        let l = de(&db, &d.id).unwrap();
        assert_eq!(l.linaje, d.id);
        let copia = decks::duplicate(&db, &d.id, "A (copia)").unwrap();
        let lc = de(&db, &copia.id).unwrap();
        assert_eq!(lc.linaje, copia.id);
        assert_ne!(lc.linaje, l.linaje);
        assert_eq!(decks_de(&db, &d.id).unwrap().len(), 1);
    }

    #[test]
    fn borrar_el_deck_borra_su_linaje() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "A", None).unwrap();
        poner(&db, &d.id, "otro", 5).unwrap();
        assert_eq!(decks_de(&db, "otro").unwrap().len(), 1);
        decks::delete(&db, &d.id).unwrap();
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM deck_linaje", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
        assert!(decks_de(&db, "otro").unwrap().is_empty());
    }

    #[test]
    fn guardar_apunta_la_hora_del_contenido_y_no_cambia_el_linaje() {
        let db = Db::in_memory().unwrap();
        let mut d = decks::create(&db, "A", None).unwrap();
        poner(&db, &d.id, "lejano", 1).unwrap();
        d.name = "B".into();
        let t = decks::save(&db, &d).unwrap();
        let l = de(&db, &d.id).unwrap();
        assert_eq!(l.linaje, "lejano");
        assert_eq!(l.actualizado, t);
    }

    #[test]
    fn un_deck_sin_fila_cuenta_con_su_id() {
        let db = Db::in_memory().unwrap();
        db.conn()
            .unwrap()
            .execute(
                "INSERT INTO decks (id, name, settings_json, schema_version, created_at, updated_at) VALUES ('x', 'x', '{}', 1, 0, 7)",
                [],
            )
            .unwrap();
        assert_eq!(
            de(&db, "x").unwrap(),
            Linaje {
                linaje: "x".into(),
                actualizado: 7
            }
        );
        assert_eq!(decks_de(&db, "x").unwrap(), vec![("x".to_string(), 7)]);
    }

    #[test]
    fn la_migracion_da_a_cada_deck_su_linaje() {
        // Una base con la migración 8 y un deck; luego se aplica la 9.
        let c = rusqlite::Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", "ON").unwrap();
        for sql in [
            include_str!("migrations/0001_init.sql"),
            include_str!("migrations/0002_blob_refs.sql"),
            include_str!("migrations/0003_biblioteca.sql"),
            include_str!("migrations/0004_idioma_deck.sql"),
            include_str!("migrations/0005_listas_juego.sql"),
            include_str!("migrations/0006_partidas.sql"),
            include_str!("migrations/0007_icono_etiqueta.sql"),
            include_str!("migrations/0008_partida_imagenes.sql"),
        ] {
            c.execute_batch(sql).unwrap();
        }
        c.execute(
            "INSERT INTO decks (id, name, settings_json, schema_version, created_at, updated_at) VALUES ('viejo', 'v', '{}', 1, 0, 42)",
            [],
        )
        .unwrap();
        c.execute_batch(include_str!("migrations/0009_linaje.sql"))
            .unwrap();
        let (l, a): (String, i64) = c
            .query_row(
                "SELECT linaje, actualizado FROM deck_linaje WHERE deck_id = 'viejo'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((l.as_str(), a), ("viejo", 42));
    }
}
