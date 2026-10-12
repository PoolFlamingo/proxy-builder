//! **Los decks guardados**, con todas sus cartas.

use proxy_core::model::{BackSpec, Deck, DeckCard, FaceImage, PrintSettings, DECK_SCHEMA_VERSION};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::db::Db;
use crate::error::{AppError, AppResult, ErrorCode};

/// Lo que se enseña de un deck en la Biblioteca.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckSummary {
    pub id: String,
    pub name: String,
    pub card_count: u32,
    #[ts(type = "number")]
    pub updated_at: i64,
    /// Huella de la imagen de la primera carta, para la miniatura.
    pub thumbnail: Option<String>,
}

fn json<T: Serialize>(v: &T) -> AppResult<String> {
    serde_json::to_string(v).map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

fn de_json<T: serde::de::DeserializeOwned>(s: &str) -> AppResult<T> {
    serde_json::from_str(s).map_err(|e| AppError::new(ErrorCode::Db).with("detalle", e))
}

fn back_blob(b: &BackSpec) -> Option<&str> {
    match b {
        BackSpec::Dfc { image } | BackSpec::Custom { image } => Some(&image.blob),
        BackSpec::DeckDefault => None,
    }
}

/// Crea un deck vacío con los ajustes de fábrica y el reverso genérico.
pub fn create(db: &Db, name: &str, default_back: Option<FaceImage>) -> AppResult<Deck> {
    let deck = Deck {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.trim().to_string(),
        decklist_text: String::new(),
        cards: Vec::new(),
        settings: PrintSettings::default(),
        default_back,
        schema_version: DECK_SCHEMA_VERSION,
    };
    save(db, &deck)?;
    Ok(deck)
}

pub fn open(db: &Db, id: &str) -> AppResult<Deck> {
    let c = db.conn()?;
    let fila: Option<(String, String, String, Option<String>, u32)> = c
        .query_row(
            "SELECT name, decklist_text, settings_json, default_back_json, schema_version FROM decks WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()?;
    let (name, decklist_text, settings, back, schema_version) =
        fila.ok_or_else(|| AppError::new(ErrorCode::NotFound).with("deck", id))?;
    let mut st =
        c.prepare("SELECT card_json FROM deck_cards WHERE deck_id = ?1 ORDER BY position")?;
    let cards = st
        .query_map([id], |r| r.get::<_, String>(0))?
        .map(|r| {
            r.map_err(AppError::from)
                .and_then(|s| de_json::<DeckCard>(&s))
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Deck {
        id: id.to_string(),
        name,
        decklist_text,
        cards,
        settings: de_json(&settings)?,
        default_back: back.as_deref().map(de_json).transpose()?,
        schema_version,
    })
}

/// Guarda el deck entero en una transacción: datos del deck y todas sus
/// cartas en orden. Devuelve la hora de guardado (ms).
pub fn save(db: &Db, deck: &Deck) -> AppResult<i64> {
    let ahora = super::ahora_ms();
    let mut c = db.conn()?;
    let tx = c.transaction()?;

    // Toda imagen referenciada tiene que estar en el almacén.
    {
        let mut existe = tx.prepare_cached("SELECT EXISTS(SELECT 1 FROM blobs WHERE hash = ?1)")?;
        let mut huellas: Vec<&str> = Vec::new();
        for card in &deck.cards {
            if let Some(f) = &card.front {
                huellas.push(&f.blob);
            }
            if let Some(b) = back_blob(&card.back) {
                huellas.push(b);
            }
        }
        if let Some(b) = &deck.default_back {
            huellas.push(&b.blob);
        }
        for h in huellas {
            let ok: bool = existe.query_row([h], |r| r.get(0))?;
            if !ok {
                return Err(AppError::new(ErrorCode::InvalidInput)
                    .with("motivo", "blob_missing")
                    .with("blob", h));
            }
        }
    }

    tx.execute(
        "INSERT INTO decks (id, name, decklist_text, settings_json, default_back_json, default_back_blob, schema_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, decklist_text = excluded.decklist_text,
           settings_json = excluded.settings_json, default_back_json = excluded.default_back_json,
           default_back_blob = excluded.default_back_blob, schema_version = excluded.schema_version,
           updated_at = excluded.updated_at",
        params![
            deck.id,
            deck.name,
            deck.decklist_text,
            json(&deck.settings)?,
            deck.default_back.as_ref().map(json).transpose()?,
            deck.default_back.as_ref().map(|b| b.blob.as_str()),
            DECK_SCHEMA_VERSION,
            ahora,
        ],
    )?;
    tx.execute("DELETE FROM deck_cards WHERE deck_id = ?1", [&deck.id])?;
    {
        let mut ins = tx.prepare_cached(
            "INSERT INTO deck_cards (id, deck_id, position, front_blob, back_blob, card_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )?;
        for (i, card) in deck.cards.iter().enumerate() {
            ins.execute(params![
                card.id,
                deck.id,
                i as i64,
                card.front.as_ref().map(|f| f.blob.as_str()),
                back_blob(&card.back),
                json(card)?,
            ])
            .map_err(|e| match e {
                rusqlite::Error::SqliteFailure(f, _)
                    if f.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    AppError::new(ErrorCode::InvalidInput)
                        .with("motivo", "duplicate_card_id")
                        .with("card", &card.id)
                }
                otro => otro.into(),
            })?;
        }
    }
    tx.commit()?;
    Ok(ahora)
}

pub fn list(db: &Db) -> AppResult<Vec<DeckSummary>> {
    let c = db.conn()?;
    let mut st = c.prepare(
        "SELECT d.id, d.name, d.updated_at,
                (SELECT COUNT(*) FROM deck_cards WHERE deck_id = d.id),
                (SELECT front_blob FROM deck_cards WHERE deck_id = d.id AND front_blob IS NOT NULL ORDER BY position LIMIT 1)
         FROM decks d ORDER BY d.updated_at DESC",
    )?;
    let filas = st.query_map([], |r| {
        Ok(DeckSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            updated_at: r.get(2)?,
            card_count: r.get(3)?,
            thumbnail: r.get(4)?,
        })
    })?;
    Ok(filas.collect::<Result<_, _>>()?)
}

pub fn rename(db: &Db, id: &str, name: &str) -> AppResult<()> {
    let n = db.conn()?.execute(
        "UPDATE decks SET name = ?2, updated_at = ?3 WHERE id = ?1",
        (id, name.trim(), super::ahora_ms()),
    )?;
    if n == 0 {
        return Err(AppError::new(ErrorCode::NotFound).with("deck", id));
    }
    Ok(())
}

/// Copia un deck con ids nuevos (las imágenes se comparten, no se copian).
pub fn duplicate(db: &Db, id: &str, name: &str) -> AppResult<Deck> {
    let mut d = open(db, id)?;
    d.id = uuid::Uuid::new_v4().to_string();
    d.name = name.trim().to_string();
    for c in &mut d.cards {
        c.id = uuid::Uuid::new_v4().to_string();
    }
    save(db, &d)?;
    Ok(d)
}

pub fn delete(db: &Db, id: &str) -> AppResult<()> {
    db.conn()?
        .execute("DELETE FROM decks WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::blobs::{tests::png, BlobStore};
    use proxy_core::model::*;

    fn entorno() -> (tempfile::TempDir, Db, BlobStore) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::in_memory().unwrap();
        let s = BlobStore::new(dir.path().join("blobs"));
        (dir, db, s)
    }

    fn carta(i: usize, img: &FaceImage) -> DeckCard {
        DeckCard {
            id: format!("c{i}"),
            source: CardSource::Custom,
            front: Some(img.clone()),
            back: BackSpec::DeckDefault,
            edits: CardEdits {
                rotation: Rotation::R90,
                crop: Crop {
                    zoom: 1.2,
                    offset_x: 0.1,
                    offset_y: -0.3,
                },
                adjust: Adjustments {
                    brightness: i as f64,
                    ..Default::default()
                },
            },
            locked: i.is_multiple_of(2),
            meta: CardMeta {
                name: format!("Carta {i}"),
                ..Default::default()
            },
        }
    }

    #[test]
    fn guardar_y_reabrir_135_cartas_da_lo_mismo() {
        let (_d, db, s) = entorno();
        let img = s
            .put_bytes(&db, &png(3, 4, 7), ImageOrigin::Upload)
            .unwrap();
        let reverso = s.generic_back(&db).unwrap();
        let mut deck = create(&db, "Prueba", Some(reverso)).unwrap();
        deck.cards = (0..135).map(|i| carta(i, &img)).collect();
        deck.settings.gap_mm = 3.0;
        deck.decklist_text = "1 Sol Ring".into();
        let t = std::time::Instant::now();
        save(&db, &deck).unwrap();
        assert!(t.elapsed().as_millis() < 500, "{:?}", t.elapsed());
        assert_eq!(open(&db, &deck.id).unwrap(), deck);
        let l = list(&db).unwrap();
        assert_eq!(l[0].card_count, 135);
        assert_eq!(l[0].thumbnail.as_deref(), Some(img.blob.as_str()));
    }

    #[test]
    fn no_se_guarda_una_carta_con_una_imagen_que_no_existe() {
        let (_d, db, _s) = entorno();
        let mut deck = create(&db, "x", None).unwrap();
        let falsa = FaceImage {
            blob: "f".repeat(64),
            width: 1,
            height: 1,
            origin: ImageOrigin::Upload,
        };
        deck.cards.push(carta(0, &falsa));
        let e = save(&db, &deck).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
        assert_eq!(e.params["motivo"], "blob_missing");
        assert!(open(&db, &deck.id).unwrap().cards.is_empty());
    }

    #[test]
    fn duplicar_renombrar_y_borrar() {
        let (_d, db, s) = entorno();
        let img = s
            .put_bytes(&db, &png(3, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let mut deck = create(&db, "Uno", None).unwrap();
        deck.cards = (0..3).map(|i| carta(i, &img)).collect();
        save(&db, &deck).unwrap();
        let copia = duplicate(&db, &deck.id, "Dos").unwrap();
        assert_ne!(copia.id, deck.id);
        assert_ne!(copia.cards[0].id, deck.cards[0].id);
        assert_eq!(copia.cards[0].front, deck.cards[0].front);
        rename(&db, &deck.id, "Uno bis").unwrap();
        assert_eq!(open(&db, &deck.id).unwrap().name, "Uno bis");
        delete(&db, &deck.id).unwrap();
        assert_eq!(open(&db, &deck.id).unwrap_err().code, ErrorCode::NotFound);
        assert_eq!(list(&db).unwrap().len(), 1);
    }

    #[test]
    fn dos_cartas_con_el_mismo_id_es_un_error_claro() {
        let (_d, db, s) = entorno();
        let img = s
            .put_bytes(&db, &png(3, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let mut deck = create(&db, "x", None).unwrap();
        deck.cards = vec![carta(0, &img), carta(0, &img)];
        let e = save(&db, &deck).unwrap_err();
        assert_eq!(e.params["motivo"], "duplicate_card_id");
    }
}
