//! **Organizar la biblioteca**: las etiquetas del usuario (nombre y color) y
//! lo que se apunta de cada deck sin tocar sus cartas (favorito, notas,
//! portada). Nada de esto cambia la fecha de modificación del deck.

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::db::Db;
use crate::error::{AppError, AppResult, ErrorCode};

/// Colores de etiqueta que entiende la pantalla.
pub const COLORES: [&str; 10] = [
    "gray", "red", "orange", "amber", "green", "teal", "sky", "blue", "violet", "pink",
];

/// Una etiqueta del usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Tag {
    pub id: String,
    pub name: String,
    /// Uno de [`COLORES`].
    pub color: String,
}

fn mal(motivo: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput).with("motivo", motivo)
}

/// Nombre limpio (sin espacios de más, hasta 40 caracteres) y color válido.
fn limpiar(nombre: &str, color: &str) -> AppResult<(String, String)> {
    let nombre: String = nombre.split_whitespace().collect::<Vec<_>>().join(" ");
    let nombre: String = nombre.chars().take(40).collect();
    if nombre.is_empty() {
        return Err(mal("tag_vacio"));
    }
    let color = if COLORES.contains(&color) {
        color.to_string()
    } else {
        COLORES[0].to_string()
    };
    Ok((nombre, color))
}

/// Todas, por nombre.
pub fn list(db: &Db) -> AppResult<Vec<Tag>> {
    let c = db.conn()?;
    let mut st = c.prepare("SELECT id, name, color FROM tags ORDER BY name COLLATE NOCASE")?;
    let v = st
        .query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

fn por_nombre(db: &Db, nombre: &str) -> AppResult<Option<Tag>> {
    Ok(db
        .conn()?
        .query_row(
            "SELECT id, name, color FROM tags WHERE name = ?1 COLLATE NOCASE",
            [nombre],
            |r| {
                Ok(Tag {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    color: r.get(2)?,
                })
            },
        )
        .optional()?)
}

/// Crea una etiqueta. Si ya hay una con ese nombre (sin mirar mayúsculas),
/// devuelve esa.
pub fn create(db: &Db, nombre: &str, color: &str) -> AppResult<Tag> {
    let (nombre, color) = limpiar(nombre, color)?;
    if let Some(t) = por_nombre(db, &nombre)? {
        return Ok(t);
    }
    let t = Tag {
        id: uuid::Uuid::new_v4().to_string(),
        name: nombre,
        color,
    };
    db.conn()?.execute(
        "INSERT INTO tags (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![t.id, t.name, t.color, super::ahora_ms()],
    )?;
    Ok(t)
}

/// Cambia nombre y color. Un nombre que ya tiene otra etiqueta no vale.
pub fn update(db: &Db, id: &str, nombre: &str, color: &str) -> AppResult<Tag> {
    let (nombre, color) = limpiar(nombre, color)?;
    if por_nombre(db, &nombre)?.is_some_and(|t| t.id != id) {
        return Err(mal("tag_repetido").with("nombre", &nombre));
    }
    let n = db.conn()?.execute(
        "UPDATE tags SET name = ?2, color = ?3 WHERE id = ?1",
        (id, &nombre, &color),
    )?;
    if n == 0 {
        return Err(AppError::new(ErrorCode::NotFound).with("tag", id));
    }
    Ok(Tag {
        id: id.to_string(),
        name: nombre,
        color,
    })
}

/// Borra la etiqueta (y se la quita a sus decks).
pub fn delete(db: &Db, id: &str) -> AppResult<()> {
    db.conn()?.execute("DELETE FROM tags WHERE id = ?1", [id])?;
    Ok(())
}

/// Pone las etiquetas `poner` y quita `quitar` a todos los `decks`.
pub fn etiquetar(db: &Db, decks: &[String], poner: &[String], quitar: &[String]) -> AppResult<()> {
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    {
        let mut ins = tx.prepare_cached(
            "INSERT OR IGNORE INTO deck_tags (deck_id, tag_id)
             SELECT ?1, ?2 WHERE EXISTS(SELECT 1 FROM decks WHERE id = ?1)
                            AND EXISTS(SELECT 1 FROM tags WHERE id = ?2)",
        )?;
        let mut del =
            tx.prepare_cached("DELETE FROM deck_tags WHERE deck_id = ?1 AND tag_id = ?2")?;
        for d in decks {
            for t in poner {
                ins.execute([d, t])?;
            }
            for t in quitar {
                del.execute([d, t])?;
            }
        }
    }
    tx.commit()?;
    Ok(())
}

/// Las etiquetas de un deck (con nombre y color, para llevarlas en un `.pbdeck`).
pub fn de_deck(db: &Db, deck: &str) -> AppResult<Vec<Tag>> {
    let c = db.conn()?;
    let mut st = c.prepare(
        "SELECT t.id, t.name, t.color FROM tags t JOIN deck_tags dt ON dt.tag_id = t.id
         WHERE dt.deck_id = ?1 ORDER BY t.name COLLATE NOCASE",
    )?;
    let v = st
        .query_map([deck], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
                color: r.get(2)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

/// Marca o desmarca como favoritos.
pub fn favorito(db: &Db, decks: &[String], si: bool) -> AppResult<()> {
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    {
        let mut st = tx.prepare_cached("UPDATE decks SET pinned = ?2 WHERE id = ?1")?;
        for d in decks {
            st.execute(params![d, i64::from(si)])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn existe(n: usize, id: &str) -> AppResult<()> {
    if n == 0 {
        return Err(AppError::new(ErrorCode::NotFound).with("deck", id));
    }
    Ok(())
}

/// Las notas del deck (texto libre, hasta 4000 caracteres).
pub fn notas(db: &Db, deck: &str, texto: &str) -> AppResult<()> {
    let texto: String = texto.trim_end().chars().take(4000).collect();
    let n = db
        .conn()?
        .execute("UPDATE decks SET notes = ?2 WHERE id = ?1", (deck, texto))?;
    existe(n, deck)
}

/// La carta de la portada (`None`: la primera).
pub fn portada(db: &Db, deck: &str, carta: Option<&str>) -> AppResult<()> {
    let n = db.conn()?.execute(
        "UPDATE decks SET cover_card_id = ?2 WHERE id = ?1",
        (deck, carta),
    )?;
    existe(n, deck)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::decks;

    fn db_con_decks(n: usize) -> (Db, Vec<String>) {
        let db = Db::in_memory().unwrap();
        let ids = (0..n)
            .map(|i| decks::create(&db, &format!("Deck {i}"), None).unwrap().id)
            .collect();
        (db, ids)
    }

    #[test]
    fn crear_con_el_mismo_nombre_devuelve_la_que_hay() {
        let (db, _) = db_con_decks(0);
        let a = create(&db, "  Commander  ", "red").unwrap();
        assert_eq!(a.name, "Commander");
        let b = create(&db, "commander", "blue").unwrap();
        assert_eq!(a, b);
        assert_eq!(list(&db).unwrap().len(), 1);
        // Color desconocido: gris. Nombre vacío: no.
        assert_eq!(create(&db, "Otra", "fucsia").unwrap().color, "gray");
        assert_eq!(
            create(&db, "   ", "red").unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn renombrar_no_pisa_otra_etiqueta() {
        let (db, _) = db_con_decks(0);
        let a = create(&db, "Español", "red").unwrap();
        create(&db, "Inglés", "blue").unwrap();
        let e = update(&db, &a.id, "inglés", "red").unwrap_err();
        assert_eq!(e.params["motivo"], "tag_repetido");
        // El mismo nombre con otras mayúsculas, en la misma etiqueta, sí.
        let a2 = update(&db, &a.id, "ESPAÑOL", "green").unwrap();
        assert_eq!((a2.name.as_str(), a2.color.as_str()), ("ESPAÑOL", "green"));
        assert_eq!(
            update(&db, "x", "Z", "red").unwrap_err().code,
            ErrorCode::NotFound
        );
    }

    #[test]
    fn etiquetar_en_bloque_y_borrar_limpia() {
        let (db, ids) = db_con_decks(3);
        let a = create(&db, "A", "red").unwrap();
        let b = create(&db, "B", "blue").unwrap();
        etiquetar(&db, &ids, &[a.id.clone(), b.id.clone()], &[]).unwrap();
        // Repetir no duplica; un deck o etiqueta que no existe se ignora.
        etiquetar(
            &db,
            &[ids[0].clone(), "no".into()],
            &[a.id.clone(), "x".into()],
            &[],
        )
        .unwrap();
        etiquetar(&db, &ids[1..2], &[], std::slice::from_ref(&a.id)).unwrap();
        let l = decks::list(&db).unwrap();
        let de = |id: &str| {
            let mut t = l.iter().find(|d| d.id == id).unwrap().tags.clone();
            t.sort();
            t
        };
        let mut ab = vec![a.id.clone(), b.id.clone()];
        ab.sort();
        assert_eq!(de(&ids[0]), ab);
        assert_eq!(de(&ids[1]), vec![b.id.clone()]);
        assert_eq!(de_deck(&db, &ids[0]).unwrap().len(), 2);
        // Borrar la etiqueta se la quita a todos; borrar un deck, sus filas.
        delete(&db, &b.id).unwrap();
        decks::delete(&db, &ids[0]).unwrap();
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM deck_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1); // el deck 2 con «A»
    }

    #[test]
    fn favorito_notas_y_portada_no_cambian_la_fecha() {
        let (db, ids) = db_con_decks(1);
        let antes = decks::list(&db).unwrap()[0].updated_at;
        favorito(&db, &ids, true).unwrap();
        notas(&db, &ids[0], "Para el viernes  \n").unwrap();
        portada(&db, &ids[0], Some("c3")).unwrap();
        let d = &decks::list(&db).unwrap()[0];
        assert!(d.pinned);
        assert_eq!(d.notes, "Para el viernes");
        assert_eq!(d.cover_card_id.as_deref(), Some("c3"));
        assert_eq!(d.updated_at, antes);
        assert_eq!(notas(&db, "x", "a").unwrap_err().code, ErrorCode::NotFound);
    }
}
