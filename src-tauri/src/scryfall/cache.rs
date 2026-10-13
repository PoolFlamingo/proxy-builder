//! Caché local de los datos de Scryfall (`scryfall_cards`): lo que ya se ha
//! pedido no se vuelve a pedir, y las estadísticas funcionan sin red.

use rusqlite::OptionalExtension;

use super::models::ScryCard;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::db::Db;

/// Datos de juego: una semana. Los precios se refrescan aparte (24 h).
pub const TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

pub fn guardar(db: &Db, c: &ScryCard) -> AppResult<()> {
    let json = serde_json::to_string(c)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    db.conn()?.execute(
        "INSERT INTO scryfall_cards (id, oracle_id, name, lang, set_code, collector_number, json, fetched_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET json = excluded.json, fetched_at = excluded.fetched_at",
        (
            &c.id,
            &c.oracle_id,
            &c.name,
            &c.lang,
            &c.set,
            &c.collector_number,
            json,
            crate::store::ahora_ms(),
        ),
    )?;
    Ok(())
}

/// Una carta de la caché y cuándo se pidió (ms).
pub fn leer(db: &Db, id: &str) -> AppResult<Option<(ScryCard, i64)>> {
    let fila: Option<(String, i64)> = db
        .conn()?
        .query_row(
            "SELECT json, fetched_at FROM scryfall_cards WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    fila.map(|(j, t)| {
        serde_json::from_str(&j)
            .map(|c| (c, t))
            .map_err(|e| AppError::new(ErrorCode::Db).with("detalle", e))
    })
    .transpose()
}

/// Una impresión concreta (set, número e idioma) de la caché, la más reciente.
pub fn leer_impresion(
    db: &Db,
    set: &str,
    numero: &str,
    lang: &str,
) -> AppResult<Option<(ScryCard, i64)>> {
    let fila: Option<(String, i64)> = db
        .conn()?
        .query_row(
            "SELECT json, fetched_at FROM scryfall_cards
             WHERE set_code = ?1 AND collector_number = ?2 AND lang = ?3
             ORDER BY fetched_at DESC LIMIT 1",
            (set, numero, lang),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    fila.map(|(j, t)| {
        serde_json::from_str(&j)
            .map(|c| (c, t))
            .map_err(|e| AppError::new(ErrorCode::Db).with("detalle", e))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::resolve::tests::carta;

    #[test]
    fn guardar_y_leer() {
        let db = Db::in_memory().unwrap();
        let c = carta(
            serde_json::json!({"id":"a","name":"Sol Ring","lang":"en","set":"cmm","collector_number":"410","prices":{"eur":"1.20"}}),
        );
        guardar(&db, &c).unwrap();
        guardar(&db, &c).unwrap();
        let (l, t) = leer(&db, "a").unwrap().unwrap();
        assert_eq!(l, c);
        assert!(t > 0);
        assert!(leer(&db, "b").unwrap().is_none());
    }

    #[test]
    fn leer_una_impresion_por_set_numero_e_idioma() {
        let db = Db::in_memory().unwrap();
        for (id, lang) in [("en", "en"), ("es", "es")] {
            guardar(
                &db,
                &carta(serde_json::json!({"id":id,"name":"Sol Ring","lang":lang,"set":"cmm","collector_number":"410"})),
            )
            .unwrap();
        }
        let (c, _) = leer_impresion(&db, "cmm", "410", "en").unwrap().unwrap();
        assert_eq!(c.id, "en");
        assert!(leer_impresion(&db, "cmm", "411", "en").unwrap().is_none());
    }
}
