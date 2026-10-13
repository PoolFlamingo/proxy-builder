//! **Paquetes `.pbdeck`**: un deck con todas sus imágenes en un ZIP, para
//! llevárselo a otro equipo.
//!
//! - `manifest.json`: `{ format: "proxy-builder-deck", version: 1, deck }`.
//! - `blobs/<sha256>.<ext>`: cada imagen que usa el deck (frentes, reversos y
//!   reverso por defecto), una vez.
//!
//! 🛑 Al importar no se confía en nada: se ignoran las entradas con rutas
//! raras (zip-slip), cada imagen tiene que dar la huella que dice su nombre y
//! el deck nace con ids nuevos.

use std::collections::BTreeSet;
use std::io::{Read, Seek, Write};
use std::path::Path;

use proxy_core::model::{BackSpec, Deck};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::blobs::{es_huella, escribir_atomico, BlobStore};
use crate::store::db::Db;
use crate::store::decks;

pub const FORMATO: &str = "proxy-builder-deck";
pub const VERSION: u32 = 1;
/// Tope por entrada al descomprimir (una imagen de 12.000 px en PNG cabe de sobra).
const MAX_ENTRADA: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Manifiesto {
    format: String,
    version: u32,
    deck: Deck,
}

fn mal(motivo: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput).with("motivo", motivo)
}

fn de_zip(e: zip::result::ZipError) -> AppError {
    match e {
        zip::result::ZipError::Io(e) => e.into(),
        otro => mal("zip").with("detalle", otro),
    }
}

/// Las huellas de todas las imágenes que usa un deck.
pub fn blobs_del_deck(deck: &Deck) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    if let Some(b) = &deck.default_back {
        s.insert(b.blob.clone());
    }
    for c in &deck.cards {
        if let Some(f) = &c.front {
            s.insert(f.blob.clone());
        }
        if let BackSpec::Dfc { image } | BackSpec::Custom { image } = &c.back {
            s.insert(image.blob.clone());
        }
    }
    s
}

fn escribir<W: Write + Seek>(w: W, deck: &Deck, blobs: &BlobStore, db: &Db) -> AppResult<W> {
    let mut zip = ZipWriter::new(w);
    let json = serde_json::to_vec_pretty(&Manifiesto {
        format: FORMATO.into(),
        version: VERSION,
        deck: deck.clone(),
    })
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    // Fecha fija: el mismo deck da el mismo fichero.
    let fecha = zip::DateTime::default();
    let deflate = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(fecha);
    let guardado = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(fecha);
    zip.start_file("manifest.json", deflate).map_err(de_zip)?;
    zip.write_all(&json)?;
    for hash in blobs_del_deck(deck) {
        let (ruta, _) = blobs
            .locate(db, &hash)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("blob", &hash))?;
        let ext = ruta.extension().and_then(|e| e.to_str()).unwrap_or("bin");
        zip.start_file(format!("blobs/{hash}.{ext}"), guardado)
            .map_err(de_zip)?;
        zip.write_all(&std::fs::read(&ruta)?)?;
    }
    zip.finish().map_err(de_zip)
}

/// Escribe el paquete de un deck (de una vez: nunca queda uno a medias).
pub fn export(deck: &Deck, destino: &Path, blobs: &BlobStore, db: &Db) -> AppResult<u64> {
    let buf = escribir(std::io::Cursor::new(Vec::new()), deck, blobs, db)?.into_inner();
    escribir_atomico(destino, &buf)?;
    Ok(buf.len() as u64)
}

fn leer_entrada<R: Read + Seek>(zip: &mut ZipArchive<R>, i: usize) -> AppResult<(String, Vec<u8>)> {
    let mut f = zip.by_index(i).map_err(de_zip)?;
    // `enclosed_name` rechaza `..` y rutas absolutas.
    let nombre = f
        .enclosed_name()
        .and_then(|p| p.to_str().map(|s| s.replace('\\', "/")))
        .ok_or_else(|| mal("ruta"))?;
    if f.size() > MAX_ENTRADA {
        return Err(mal("tamano"));
    }
    let mut v = Vec::with_capacity(f.size() as usize);
    (&mut f).take(MAX_ENTRADA + 1).read_to_end(&mut v)?;
    if v.len() as u64 > MAX_ENTRADA {
        return Err(mal("tamano"));
    }
    Ok((nombre, v))
}

/// Importa un paquete y devuelve el deck nuevo (ya guardado).
pub fn import<R: Read + Seek>(r: R, blobs: &BlobStore, db: &Db) -> AppResult<Deck> {
    let mut zip = ZipArchive::new(r).map_err(de_zip)?;
    let mut manifiesto: Option<Manifiesto> = None;
    let mut imagenes: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let (nombre, bytes) = leer_entrada(&mut zip, i)?;
        if nombre == "manifest.json" {
            manifiesto = Some(serde_json::from_slice(&bytes).map_err(|_| mal("manifiesto"))?);
        } else if let Some(resto) = nombre.strip_prefix("blobs/") {
            let hash = resto.split('.').next().unwrap_or_default();
            if !es_huella(hash) || resto.contains('/') {
                return Err(mal("ruta"));
            }
            let real: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if real != hash {
                return Err(mal("huella").with("blob", hash));
            }
            imagenes.push((hash.to_string(), bytes));
        }
        // Cualquier otra entrada se ignora.
    }
    let m = manifiesto.ok_or_else(|| mal("manifiesto"))?;
    if m.format != FORMATO {
        return Err(mal("formato"));
    }
    if m.version > VERSION {
        return Err(AppError::new(ErrorCode::Unsupported).with("version", m.version));
    }
    let mut deck = m.deck;
    let tenemos: BTreeSet<&str> = imagenes.iter().map(|(h, _)| h.as_str()).collect();
    if let Some(falta) = blobs_del_deck(&deck)
        .into_iter()
        .find(|h| !tenemos.contains(h.as_str()) && blobs.locate(db, h).ok().flatten().is_none())
    {
        return Err(mal("falta_imagen").with("blob", falta));
    }

    for (_, bytes) in &imagenes {
        let origen = deck_origen(&deck, bytes);
        blobs.put_bytes(db, bytes, origen)?;
    }

    deck.id = uuid::Uuid::new_v4().to_string();
    for c in &mut deck.cards {
        c.id = uuid::Uuid::new_v4().to_string();
    }
    let ya = decks::list(db)?;
    if ya.iter().any(|d| d.name == deck.name) {
        deck.name = format!("{} (importado)", deck.name);
    }
    decks::save(db, &deck)?;
    Ok(deck)
}

/// El origen con el que entró la imagen en el equipo de origen (lo dice el deck).
fn deck_origen(deck: &Deck, bytes: &[u8]) -> proxy_core::model::ImageOrigin {
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let todas = deck
        .default_back
        .iter()
        .chain(deck.cards.iter().filter_map(|c| c.front.as_ref()))
        .chain(deck.cards.iter().filter_map(|c| match &c.back {
            BackSpec::Dfc { image } | BackSpec::Custom { image } => Some(image),
            BackSpec::DeckDefault => None,
        }));
    for img in todas {
        if img.blob == hash {
            return img.origin;
        }
    }
    proxy_core::model::ImageOrigin::Upload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::blobs::tests::png;
    use proxy_core::model::*;

    fn entorno() -> (tempfile::TempDir, Db, BlobStore) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::in_memory().unwrap();
        let s = BlobStore::new(dir.path().join("blobs"));
        (dir, db, s)
    }

    fn deck_con_imagenes(db: &Db, s: &BlobStore) -> Deck {
        let a = s
            .put_bytes(db, &png(20, 28, 1), ImageOrigin::Upload)
            .unwrap();
        let b = s
            .put_bytes(db, &png(20, 28, 2), ImageOrigin::Upload)
            .unwrap();
        let mut d = decks::create(db, "Mi deck", Some(b.clone())).unwrap();
        d.cards.push(DeckCard {
            id: "c1".into(),
            source: CardSource::Custom { scryfall_id: None },
            front: Some(a),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
        });
        decks::save(db, &d).unwrap();
        d
    }

    #[test]
    fn ida_y_vuelta_en_un_equipo_limpio() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();

        let (_dir2, db2, s2) = entorno();
        let nuevo = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        assert_ne!(nuevo.id, d.id);
        assert_eq!(nuevo.name, "Mi deck");
        assert_eq!(nuevo.cards.len(), 1);
        assert_ne!(nuevo.cards[0].id, "c1");
        assert_eq!(blobs_del_deck(&nuevo), blobs_del_deck(&d));
        for h in blobs_del_deck(&nuevo) {
            assert!(s2.locate(&db2, &h).unwrap().is_some());
        }
        // Importar otra vez en el mismo sitio le cambia el nombre.
        let otro = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        assert_eq!(otro.name, "Mi deck (importado)");
    }

    #[test]
    fn el_mismo_deck_da_el_mismo_fichero() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let (a, b) = (dir.path().join("a.pbdeck"), dir.path().join("b.pbdeck"));
        export(&d, &a, &s, &db).unwrap();
        export(&d, &b, &s, &db).unwrap();
        assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
    }

    fn zip_a_mano(entradas: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut z = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (n, b) in entradas {
            z.start_file(*n, SimpleFileOptions::default()).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn una_imagen_alterada_se_rechaza() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let mut zip = ZipArchive::new(std::fs::File::open(&ruta).unwrap()).unwrap();
        let mut entradas = Vec::new();
        for i in 0..zip.len() {
            let (n, mut b) = leer_entrada(&mut zip, i).unwrap();
            if n.starts_with("blobs/") {
                let ultimo = b.len() - 1;
                b[ultimo] ^= 0xff;
            }
            entradas.push((n, b));
        }
        let refs: Vec<(&str, Vec<u8>)> = entradas
            .iter()
            .map(|(n, b)| (n.as_str(), b.clone()))
            .collect();
        let (_d2, db2, s2) = entorno();
        let e = import(std::io::Cursor::new(zip_a_mano(&refs)), &s2, &db2).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn una_ruta_con_puntos_se_rechaza() {
        let (_d, db, s) = entorno();
        let z = zip_a_mano(&[("../fuera.txt", b"x".to_vec())]);
        assert!(import(std::io::Cursor::new(z), &s, &db).is_err());
        let z = zip_a_mano(&[("/etc/passwd", b"x".to_vec())]);
        assert!(import(std::io::Cursor::new(z), &s, &db).is_err());
    }

    #[test]
    fn sin_manifiesto_o_de_otro_formato_no_entra() {
        let (_d, db, s) = entorno();
        assert!(import(std::io::Cursor::new(zip_a_mano(&[])), &s, &db).is_err());
        let m = br#"{"format":"otra-cosa","version":1,"deck":null}"#.to_vec();
        assert!(import(
            std::io::Cursor::new(zip_a_mano(&[("manifest.json", m)])),
            &s,
            &db
        )
        .is_err());
    }
}
