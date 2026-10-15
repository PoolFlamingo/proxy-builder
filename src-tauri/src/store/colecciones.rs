//! **Colecciones** del editor de cartas: lo compartido por un grupo de
//! diseños (marco, emblema, pie…). Viven en `presets` con
//! `kind = 'collection'` y se guardan **por id**: renombrar una colección no
//! crea otra (el `presets::save` de siempre va por nombre).
//!
//! El emblema es una imagen del almacén; se apunta en `blob_refs` para que la
//! limpieza no lo borre.

use proxy_core::diseno::modelo::Collection;

use super::db::Db;
use crate::error::{AppError, AppResult, ErrorCode};

const KIND: &str = "collection";

pub fn list(db: &Db) -> AppResult<Vec<Collection>> {
    let c = db.conn()?;
    let mut st = c.prepare("SELECT json FROM presets WHERE kind = ?1 ORDER BY name")?;
    let filas = st.query_map([KIND], |r| r.get::<_, String>(0))?;
    let mut v = Vec::new();
    for f in filas {
        let json = f?;
        // Una colección que no se entiende (de una versión futura) se salta.
        if let Ok(col) = serde_json::from_str::<Collection>(&json) {
            v.push(col);
        }
    }
    Ok(v)
}

pub fn get(db: &Db, id: &str) -> AppResult<Option<Collection>> {
    let json: Option<String> = rusqlite::OptionalExtension::optional(db.conn()?.query_row(
        "SELECT json FROM presets WHERE kind = ?1 AND id = ?2",
        (KIND, id),
        |r| r.get(0),
    ))?;
    json.map(|j| {
        serde_json::from_str(&j).map_err(|e| AppError::new(ErrorCode::Db).with("detalle", e))
    })
    .transpose()
}

/// Guarda por id (uno nuevo si viene vacío) y devuelve la colección guardada.
pub fn save(db: &Db, col: &Collection) -> AppResult<Collection> {
    let mut col = col.clone();
    col.name = col.name.trim().to_string();
    if col.name.is_empty() {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "nombre_vacio"));
    }
    if col.id.is_empty() {
        col.id = uuid::Uuid::new_v4().to_string();
    }
    let json = serde_json::to_string(&col)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    if let Some(e) = &col.emblem {
        let existe: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM blobs WHERE hash = ?1)",
            [&e.blob],
            |r| r.get(0),
        )?;
        if !existe {
            return Err(AppError::new(ErrorCode::InvalidInput)
                .with("motivo", "blob_missing")
                .with("blob", &e.blob));
        }
    }
    tx.execute(
        "INSERT INTO presets (id, kind, name, json, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, json = excluded.json, updated_at = excluded.updated_at",
        (&col.id, KIND, &col.name, &json, super::ahora_ms()),
    )?;
    tx.execute(
        "DELETE FROM blob_refs WHERE owner_kind = 'preset' AND owner_id = ?1",
        [&col.id],
    )?;
    if let Some(e) = &col.emblem {
        tx.execute(
            "INSERT OR IGNORE INTO blob_refs (owner_kind, owner_id, blob) VALUES ('preset', ?1, ?2)",
            (&col.id, &e.blob),
        )?;
    }
    tx.commit()?;
    Ok(col)
}

pub fn delete(db: &Db, id: &str) -> AppResult<()> {
    let mut c = db.conn()?;
    let tx = c.transaction()?;
    tx.execute(
        "DELETE FROM presets WHERE kind = ?1 AND id = ?2",
        (KIND, id),
    )?;
    tx.execute(
        "DELETE FROM blob_refs WHERE owner_kind = 'preset' AND owner_id = ?1",
        [id],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::store::blobs::{tests::png, BlobStore};
    use proxy_core::diseno::modelo::*;
    use proxy_core::model::ImageOrigin;

    pub(crate) fn nueva(nombre: &str) -> Collection {
        Collection {
            id: String::new(),
            name: nombre.into(),
            default_style: DesignStyle::Borderless,
            frame: FrameOptions::default(),
            emblem: None,
            set_code: "RTS".into(),
            rarity_letter: Some("S".into()),
            year: None,
            artist: Some("S.J.".into()),
            credit: None,
            universe: None,
            original_name: OriginalName::Footer,
            footer_template: ByLang::default(),
            font_title: None,
            font_rules: None,
            title_squeeze_min: ESTRECHADO_DEFECTO,
            title_squeeze_base: ESTRECHADO_BASE_DEFECTO,
            weight_title: None,
            weight_type: None,
            weight_rules: None,
        }
    }

    #[test]
    fn renombrar_conserva_el_id_y_el_emblema_no_se_limpia() {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::in_memory().unwrap();
        let s = BlobStore::new(dir.path().join("blobs"));
        let slime = s
            .put_bytes(&db, &png(4, 4, 3), ImageOrigin::Upload)
            .unwrap();
        let mut c = nueva("Tensura 2");
        c.emblem = Some(slime.clone());
        let c = save(&db, &c).unwrap();
        assert!(!c.id.is_empty());
        let mut r = c.clone();
        r.name = "Tensura II".into();
        let r = save(&db, &r).unwrap();
        assert_eq!(r.id, c.id);
        assert_eq!(list(&db).unwrap().len(), 1);
        assert_eq!(get(&db, &c.id).unwrap().unwrap().name, "Tensura II");

        db.conn()
            .unwrap()
            .execute("UPDATE blobs SET created_at = 0", [])
            .unwrap();
        assert_eq!(s.gc(&db, false).unwrap().removed, 0);
        delete(&db, &c.id).unwrap();
        assert!(get(&db, &c.id).unwrap().is_none());
        assert_eq!(s.gc(&db, false).unwrap().removed, 1);
    }

    #[test]
    fn sin_nombre_o_con_un_emblema_que_no_existe_no_se_guarda() {
        let db = Db::in_memory().unwrap();
        assert_eq!(
            save(&db, &nueva("  ")).unwrap_err().params["motivo"],
            "nombre_vacio"
        );
        let mut c = nueva("X");
        c.emblem = Some(proxy_core::model::FaceImage {
            blob: "f".repeat(64),
            width: 1,
            height: 1,
            origin: ImageOrigin::Upload,
        });
        assert_eq!(save(&db, &c).unwrap_err().params["motivo"], "blob_missing");
    }
}
