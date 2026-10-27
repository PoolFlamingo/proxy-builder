//! **Las listas para jugar de cada deck** (`deck_listas`, migración 0005):
//! qué copias van a la partida cuando el deck tiene más de las que caben.
//! Aparte del deck, como las etiquetas: guardar el deck no las toca y no
//! cambian su fecha. Se borran con el deck.

use mesa::lista::ListaDeJuego;
use rusqlite::params;

use super::db::Db;
use crate::error::{AppError, AppResult, ErrorCode};

fn json(v: &[String]) -> AppResult<String> {
    serde_json::to_string(v).map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

fn de_json(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

/// Las listas de un deck, por nombre.
pub fn de_deck(db: &Db, deck_id: &str) -> AppResult<Vec<ListaDeJuego>> {
    let c = db.conn()?;
    let mut st = c.prepare(
        "SELECT id, nombre, comandantes, cartas FROM deck_listas
         WHERE deck_id = ?1 ORDER BY nombre COLLATE NOCASE, id",
    )?;
    let v = st
        .query_map([deck_id], |r| {
            Ok(ListaDeJuego {
                id: r.get(0)?,
                nombre: r.get(1)?,
                comandantes: de_json(&r.get::<_, String>(2)?),
                cartas: de_json(&r.get::<_, String>(3)?),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

/// Una lista de un deck, si existe.
pub fn una(db: &Db, deck_id: &str, id: &str) -> AppResult<Option<ListaDeJuego>> {
    Ok(de_deck(db, deck_id)?.into_iter().find(|l| l.id == id))
}

/// Guarda (crea o reemplaza) una lista. Sin id, le pone uno nuevo; el nombre
/// se limpia (sin espacios de más, hasta 60 caracteres).
pub fn guardar(db: &Db, deck_id: &str, lista: &ListaDeJuego) -> AppResult<ListaDeJuego> {
    let mut l = lista.clone();
    if l.id.trim().is_empty() {
        l.id = uuid::Uuid::new_v4().to_string();
    }
    l.nombre = l
        .nombre
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(60)
        .collect();
    if l.nombre.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "lista_sin_nombre"));
    }
    db.conn()?.execute(
        "INSERT INTO deck_listas (deck_id, id, nombre, comandantes, cartas, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(deck_id, id) DO UPDATE SET
           nombre = excluded.nombre, comandantes = excluded.comandantes,
           cartas = excluded.cartas, updated_at = excluded.updated_at",
        params![
            deck_id,
            l.id,
            l.nombre,
            json(&l.comandantes)?,
            json(&l.cartas)?,
            super::ahora_ms()
        ],
    )?;
    Ok(l)
}

pub fn borrar(db: &Db, deck_id: &str, id: &str) -> AppResult<()> {
    db.conn()?.execute(
        "DELETE FROM deck_listas WHERE deck_id = ?1 AND id = ?2",
        params![deck_id, id],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::decks;

    fn lista(nombre: &str) -> ListaDeJuego {
        ListaDeJuego {
            id: String::new(),
            nombre: nombre.into(),
            comandantes: vec!["a".into()],
            cartas: vec!["b".into(), "c".into()],
        }
    }

    #[test]
    fn guardar_leer_reemplazar_y_borrar() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "Grande", None).unwrap();
        let l = guardar(&db, &d.id, &lista("  Torneo   de  viernes ")).unwrap();
        assert!(!l.id.is_empty());
        assert_eq!(l.nombre, "Torneo de viernes");
        guardar(&db, &d.id, &lista("Casual")).unwrap();
        let todas = de_deck(&db, &d.id).unwrap();
        assert_eq!(
            todas.iter().map(|l| l.nombre.as_str()).collect::<Vec<_>>(),
            vec!["Casual", "Torneo de viernes"]
        );
        let mut otra = l.clone();
        otra.cartas = vec!["z".into()];
        guardar(&db, &d.id, &otra).unwrap();
        assert_eq!(una(&db, &d.id, &l.id).unwrap().unwrap().cartas, vec!["z"]);
        borrar(&db, &d.id, &l.id).unwrap();
        assert_eq!(de_deck(&db, &d.id).unwrap().len(), 1);
        assert!(guardar(&db, &d.id, &lista("   ")).is_err());
    }

    #[test]
    fn se_van_con_el_deck() {
        let db = Db::in_memory().unwrap();
        let d = decks::create(&db, "Grande", None).unwrap();
        guardar(&db, &d.id, &lista("Torneo")).unwrap();
        decks::delete(&db, &d.id).unwrap();
        assert!(de_deck(&db, &d.id).unwrap().is_empty());
    }
}
