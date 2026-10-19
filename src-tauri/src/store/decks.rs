//! **Los decks guardados**, con todas sus cartas.

use proxy_core::model::{BackSpec, Deck, DeckCard, FaceImage, PrintSettings, DECK_SCHEMA_VERSION};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use ts_rs::TS;

use super::db::Db;
use crate::error::{AppError, AppResult, ErrorCode};

/// Lo que se enseña de un deck en la Biblioteca (lo barato: sale de la base
/// sin leer las cartas; lo que hay que contar va en `DeckDetails`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckSummary {
    pub id: String,
    pub name: String,
    pub card_count: u32,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    /// Cuándo se abrió por última vez (`None`: nunca desde la 2.2).
    #[ts(type = "number | null")]
    pub opened_at: Option<i64>,
    /// Huella de la imagen de la portada (la elegida o la primera carta).
    pub thumbnail: Option<String>,
    /// El nombre de la carta de la portada.
    pub cover_name: Option<String>,
    /// La carta elegida como portada, si se eligió.
    pub cover_card_id: Option<String>,
    pub pinned: bool,
    pub notes: String,
    /// El idioma elegido a mano (código de Scryfall); manda sobre el que se
    /// calcula de las cartas.
    pub lang_override: Option<String>,
    /// Las etiquetas del deck (ids de `tags`).
    pub tags: Vec<String>,
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
        // Las del editor de cartas: arte, emblemas, «antes».
        let refs = refs_del_deck(deck);
        huellas.extend(refs.iter().copied());
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
    escribir_refs(&tx, &deck.id, &refs_del_deck(deck))?;
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
    // La portada: la carta elegida si sigue en el deck y tiene imagen; si
    // no, la primera con imagen.
    let mut st = c.prepare(
        "SELECT d.id, d.name, d.created_at, d.updated_at, d.opened_at, d.pinned, d.notes, d.cover_card_id,
                (SELECT COUNT(*) FROM deck_cards WHERE deck_id = d.id),
                p.front_blob, json_extract(p.card_json, '$.meta.name'), d.lang_override
         FROM decks d
         LEFT JOIN deck_cards p ON p.deck_id = d.id AND p.id = COALESCE(
             (SELECT x.id FROM deck_cards x WHERE x.deck_id = d.id AND x.id = d.cover_card_id AND x.front_blob IS NOT NULL),
             (SELECT x.id FROM deck_cards x WHERE x.deck_id = d.id AND x.front_blob IS NOT NULL ORDER BY x.position LIMIT 1))
         ORDER BY d.updated_at DESC",
    )?;
    let filas = st.query_map([], |r| {
        Ok(DeckSummary {
            id: r.get(0)?,
            name: r.get(1)?,
            created_at: r.get(2)?,
            updated_at: r.get(3)?,
            opened_at: r.get(4)?,
            pinned: r.get::<_, i64>(5)? != 0,
            notes: r.get(6)?,
            cover_card_id: r.get(7)?,
            card_count: r.get(8)?,
            thumbnail: r.get(9)?,
            cover_name: r.get(10)?,
            lang_override: r.get(11)?,
            tags: Vec::new(),
        })
    })?;
    let mut v: Vec<DeckSummary> = filas.collect::<Result<_, _>>()?;
    let mut st = c.prepare("SELECT deck_id, tag_id FROM deck_tags")?;
    let mut por_deck: std::collections::HashMap<String, Vec<String>> = Default::default();
    for f in st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (d, t) = f?;
        por_deck.entry(d).or_default().push(t);
    }
    for d in &mut v {
        if let Some(t) = por_deck.remove(&d.id) {
            d.tags = t;
        }
    }
    Ok(v)
}

/// Las notas del deck.
pub fn notas_de(db: &Db, id: &str) -> AppResult<String> {
    Ok(db
        .conn()?
        .query_row("SELECT notes FROM decks WHERE id = ?1", [id], |r| r.get(0))
        .optional()?
        .unwrap_or_default())
}

/// El idioma elegido a mano.
pub fn idioma_de(db: &Db, id: &str) -> AppResult<Option<String>> {
    Ok(db
        .conn()?
        .query_row("SELECT lang_override FROM decks WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?
        .flatten())
}

/// Apunta que el deck se acaba de abrir.
pub fn marcar_abierto(db: &Db, id: &str) -> AppResult<()> {
    db.conn()?.execute(
        "UPDATE decks SET opened_at = ?2 WHERE id = ?1",
        (id, super::ahora_ms()),
    )?;
    Ok(())
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
/// La copia se lleva también las etiquetas, las notas y la portada (no el
/// favorito).
pub fn duplicate(db: &Db, id: &str, name: &str) -> AppResult<Deck> {
    let mut d = open(db, id)?;
    d.id = uuid::Uuid::new_v4().to_string();
    d.name = name.trim().to_string();
    let mut ids = std::collections::HashMap::new();
    for c in &mut d.cards {
        let nuevo = uuid::Uuid::new_v4().to_string();
        ids.insert(std::mem::replace(&mut c.id, nuevo.clone()), nuevo);
    }
    save(db, &d)?;
    let c = db.conn()?;
    let (notas, portada, idioma): (String, Option<String>, Option<String>) = c.query_row(
        "SELECT notes, cover_card_id, lang_override FROM decks WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    c.execute(
        "UPDATE decks SET notes = ?2, cover_card_id = ?3, lang_override = ?4 WHERE id = ?1",
        (
            &d.id,
            notas,
            portada.and_then(|p| ids.get(&p).cloned()),
            idioma,
        ),
    )?;
    c.execute(
        "INSERT INTO deck_tags (deck_id, tag_id) SELECT ?2, tag_id FROM deck_tags WHERE deck_id = ?1",
        (id, &d.id),
    )?;
    Ok(d)
}

pub fn delete(db: &Db, id: &str) -> AppResult<()> {
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    tx.execute("DELETE FROM decks WHERE id = ?1", [id])?;
    tx.execute(
        "DELETE FROM blob_refs WHERE owner_kind = 'deck' AND owner_id = ?1",
        [id],
    )?;
    tx.commit()?;
    Ok(())
}

/// Las imágenes del deck que no son ni frente ni dorso (las del editor de
/// cartas), sin repetir.
fn refs_del_deck(deck: &Deck) -> Vec<&str> {
    let mut v: Vec<&str> = deck
        .cards
        .iter()
        .filter_map(|c| c.design.as_ref())
        .flat_map(|d| d.blobs())
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn escribir_refs(tx: &rusqlite::Transaction<'_>, deck: &str, blobs: &[&str]) -> AppResult<()> {
    tx.execute(
        "DELETE FROM blob_refs WHERE owner_kind = 'deck' AND owner_id = ?1",
        [deck],
    )?;
    let mut ins = tx.prepare_cached(
        "INSERT OR IGNORE INTO blob_refs (owner_kind, owner_id, blob) VALUES ('deck', ?1, ?2)",
    )?;
    for b in blobs {
        ins.execute([deck, b])?;
    }
    Ok(())
}

/// Clave en `app_settings` que marca que `blob_refs` ya se rellenó desde
/// los decks que había (una sola vez; «si está vacía» no vale, porque un
/// deck sin diseños deja la tabla vacía).
const MARCA_REFS: &str = "blob_refs_v1";

/// Rellena `blob_refs` desde todos los decks guardados, una vez.
pub fn rellenar_refs(db: &Db) -> AppResult<()> {
    let hecha: bool = db.conn()?.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_settings WHERE key = ?1)",
        [MARCA_REFS],
        |r| r.get(0),
    )?;
    if hecha {
        return Ok(());
    }
    let ids: Vec<String> = {
        let c = db.conn()?;
        let mut st = c.prepare("SELECT id FROM decks")?;
        let v = st.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
        v
    };
    for id in ids {
        let deck = open(db, &id)?;
        let mut c = db.conn()?;
        let tx = c.transaction()?;
        escribir_refs(&tx, &id, &refs_del_deck(&deck))?;
        tx.commit()?;
    }
    db.conn()?.execute(
        "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, '1')",
        [MARCA_REFS],
    )?;
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
            source: CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
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
            design: None,
            marca: None,
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
    fn la_portada_elegida_y_lo_que_se_lleva_la_copia() {
        use crate::store::etiquetas;
        let (_d, db, s) = entorno();
        let imgs: Vec<FaceImage> = (0..3)
            .map(|i| {
                s.put_bytes(&db, &png(3, 4, i), ImageOrigin::Upload)
                    .unwrap()
            })
            .collect();
        let mut deck = create(&db, "Uno", None).unwrap();
        deck.cards = (0..3).map(|i| carta(i, &imgs[i])).collect();
        save(&db, &deck).unwrap();
        let resumen = |id: &str| list(&db).unwrap().into_iter().find(|d| d.id == id).unwrap();
        // Sin elegir: la primera.
        assert_eq!(
            resumen(&deck.id).thumbnail.as_deref(),
            Some(imgs[0].blob.as_str())
        );
        assert_eq!(resumen(&deck.id).cover_name.as_deref(), Some("Carta 0"));
        etiquetas::portada(&db, &deck.id, Some("c2")).unwrap();
        assert_eq!(
            resumen(&deck.id).thumbnail.as_deref(),
            Some(imgs[2].blob.as_str())
        );
        assert_eq!(resumen(&deck.id).cover_name.as_deref(), Some("Carta 2"));

        // La copia: etiquetas, notas y portada (con el id nuevo de la carta).
        let t = etiquetas::create(&db, "Modern", "red").unwrap();
        etiquetas::etiquetar(
            &db,
            std::slice::from_ref(&deck.id),
            std::slice::from_ref(&t.id),
            &[],
        )
        .unwrap();
        etiquetas::notas(&db, &deck.id, "Notas").unwrap();
        etiquetas::favorito(&db, std::slice::from_ref(&deck.id), true).unwrap();
        let copia = duplicate(&db, &deck.id, "Dos").unwrap();
        let r = resumen(&copia.id);
        assert_eq!(r.tags, vec![t.id.clone()]);
        assert_eq!(r.notes, "Notas");
        assert!(!r.pinned);
        assert_eq!(r.cover_card_id.as_deref(), Some(copia.cards[2].id.as_str()));
        assert_eq!(r.thumbnail.as_deref(), Some(imgs[2].blob.as_str()));

        // Si la carta de la portada se va del deck, vuelve la primera.
        deck.cards.pop();
        save(&db, &deck).unwrap();
        assert_eq!(
            resumen(&deck.id).thumbnail.as_deref(),
            Some(imgs[0].blob.as_str())
        );
        // Abrir apunta la fecha.
        assert_eq!(resumen(&deck.id).opened_at, None);
        marcar_abierto(&db, &deck.id).unwrap();
        assert!(resumen(&deck.id).opened_at.is_some());
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

    fn con_diseno(card: &mut DeckCard, arte: &FaceImage) {
        use proxy_core::diseno::modelo::*;
        card.design = Some(CardDesign {
            version: VERSION_DISENO,
            status: DesignStatus::Draft,
            collection: None,
            style: DesignStyle::Borderless,
            estilo_pack: None,
            variante_pack: None,
            frame: None,
            faces: vec![FaceDesign {
                art: Some(ArtRef {
                    image: arte.clone(),
                    source: ArtSource::Upload,
                    framing: ArtFraming::default(),
                }),
                texts: ByLang::default(),
            }],
            before: BeforeDesign {
                source: card.source.clone(),
                front: card.front.clone(),
                back: card.back.clone(),
                edits: card.edits,
            },
            card: CardData::default(),
            footer: FooterOptions::default(),
            options: DesignOptions::default(),
            emblem: EmblemChoice::Inherit,
            thumb_key: None,
        });
    }

    fn envejecer(db: &Db) {
        db.conn()
            .unwrap()
            .execute("UPDATE blobs SET created_at = 0", [])
            .unwrap();
    }

    #[test]
    fn la_limpieza_respeta_el_arte_de_los_disenos() {
        let (_d, db, s) = entorno();
        let img = s
            .put_bytes(&db, &png(3, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let arte = s
            .put_bytes(&db, &png(5, 4, 2), ImageOrigin::Upload)
            .unwrap();
        let mut deck = create(&db, "Uno", None).unwrap();
        deck.cards = vec![carta(0, &img)];
        con_diseno(&mut deck.cards[0], &arte);
        save(&db, &deck).unwrap();
        envejecer(&db);
        // El arte no es ni frente ni dorso, pero el diseño lo usa.
        assert_eq!(s.gc(&db, false).unwrap().removed, 0);

        // Duplicar copia las referencias; borrar el original no las suelta.
        let copia = duplicate(&db, &deck.id, "Dos").unwrap();
        delete(&db, &deck.id).unwrap();
        envejecer(&db);
        assert_eq!(s.gc(&db, false).unwrap().removed, 0);

        // Sin diseño, el arte queda huérfano y se limpia.
        let mut copia = open(&db, &copia.id).unwrap();
        copia.cards[0].design = None;
        save(&db, &copia).unwrap();
        let r = s.gc(&db, false).unwrap();
        assert_eq!(r.removed, 1);
        assert!(s.locate(&db, &arte.blob).unwrap().is_none());
        assert!(s.locate(&db, &img.blob).unwrap().is_some());
    }

    #[test]
    fn no_se_guarda_un_diseno_con_un_arte_que_no_existe() {
        let (_d, db, s) = entorno();
        let img = s
            .put_bytes(&db, &png(3, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let mut deck = create(&db, "x", None).unwrap();
        deck.cards = vec![carta(0, &img)];
        let falsa = FaceImage {
            blob: "f".repeat(64),
            ..img.clone()
        };
        con_diseno(&mut deck.cards[0], &falsa);
        let e = save(&db, &deck).unwrap_err();
        assert_eq!(e.params["motivo"], "blob_missing");
    }

    #[test]
    fn el_relleno_de_referencias_se_hace_una_vez() {
        let (_d, db, s) = entorno();
        let img = s
            .put_bytes(&db, &png(3, 4, 1), ImageOrigin::Upload)
            .unwrap();
        let arte = s
            .put_bytes(&db, &png(5, 4, 2), ImageOrigin::Upload)
            .unwrap();
        let mut deck = create(&db, "Uno", None).unwrap();
        deck.cards = vec![carta(0, &img)];
        con_diseno(&mut deck.cards[0], &arte);
        save(&db, &deck).unwrap();
        // Como si el deck viniera de antes de la tabla.
        db.conn()
            .unwrap()
            .execute("DELETE FROM blob_refs", [])
            .unwrap();
        rellenar_refs(&db).unwrap();
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM blob_refs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2); // el arte y la imagen de «antes»
        db.conn()
            .unwrap()
            .execute("DELETE FROM blob_refs", [])
            .unwrap();
        rellenar_refs(&db).unwrap(); // ya hecho: no vuelve a rellenar
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM blob_refs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
