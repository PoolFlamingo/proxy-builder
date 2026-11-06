//! **El almacén de imágenes por huella.**
//!
//! Cada imagen (de Scryfall o subida) se guarda UNA vez, con su SHA-256 como
//! nombre: `blobs/<2 primeros hex>/<sha256>.<ext>`. Los decks la referencian
//! por la huella, así que dos decks con la misma carta no duplican nada.
//!
//! 🛑 Vive en `app_data_dir`, no en la caché: los decks dependen de estas
//! imágenes y el sistema puede vaciar la caché cuando quiera.

use std::collections::BTreeSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use proxy_core::imaging::probe::{probe, Kind, ProbeError};
use proxy_core::model::{FaceImage, ImageOrigin};
use rusqlite::OptionalExtension;
use serde::Serialize;
use sha2::{Digest, Sha256};
use ts_rs::TS;

use super::db::Db;
use crate::error::{AppError, AppResult, ErrorCode};

/// Lado máximo que se acepta (una imagen más grande es casi seguro un error).
const LADO_MAXIMO: u32 = 12_000;
/// Las imágenes más recientes que esto no se borran al limpiar: pueden ser de
/// una descarga cuyo deck todavía no se ha guardado.
const GRACIA_MS: i64 = 24 * 60 * 60 * 1000;
/// Lo que se deja siempre libre en el disco al traer algo de fuera: un móvil sin
/// nada de espacio deja de funcionar, y el deck no vale tanto.
const RESERVA_DISCO: u64 = 200 * 1024 * 1024;
/// Clave de `app_settings` con la huella del reverso genérico.
const CLAVE_REVERSO: &str = "generic_back_blob";
/// El reverso genérico propio de la app (diseño original, `scripts/reverso.py`).
const REVERSO_GENERICO: &[u8] = include_bytes!("../../assets/reverso-generico.png");

pub struct BlobStore {
    root: PathBuf,
}

/// ¿Es una huella válida (64 caracteres hexadecimales en minúscula)?
pub fn es_huella(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn origen_texto(o: ImageOrigin) -> &'static str {
    match o {
        ImageOrigin::ScryfallPng => "scryfallPng",
        ImageOrigin::ScryfallLarge => "scryfallLarge",
        ImageOrigin::Upload => "upload",
        ImageOrigin::Generated => "generated",
        ImageOrigin::Designed => "designed",
    }
}

fn origen_de(s: &str) -> ImageOrigin {
    match s {
        "scryfallPng" => ImageOrigin::ScryfallPng,
        "scryfallLarge" => ImageOrigin::ScryfallLarge,
        "generated" => ImageOrigin::Generated,
        "designed" => ImageOrigin::Designed,
        _ => ImageOrigin::Upload,
    }
}

impl From<ProbeError> for AppError {
    fn from(e: ProbeError) -> Self {
        match e {
            ProbeError::Unsupported => AppError::new(ErrorCode::Unsupported),
            ProbeError::Corrupt => AppError::new(ErrorCode::Image).with("motivo", "corrupt"),
        }
    }
}

/// Lo que ha liberado una limpieza.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct GcReport {
    pub removed: u32,
    #[ts(type = "number")]
    pub freed_bytes: u64,
}

/// Quién usa una imagen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Usos {
    pub decks: BTreeSet<String>,
    /// La usa algo que no es un deck (una colección, la mesa, el reverso…).
    pub otros: bool,
}

/// Lo que apunta la base de datos de una imagen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FichaBlob {
    pub ext: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub origin: String,
}

/// Lo que queda libre en el disco de esta ruta (`None` si no se puede saber).
pub fn espacio_libre(ruta: &Path) -> Option<u64> {
    let real = ruta.canonicalize().ok()?;
    let discos = sysinfo::Disks::new_with_refreshed_list();
    discos
        .list()
        .iter()
        .filter(|d| real.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().as_os_str().len())
        .map(sysinfo::Disk::available_space)
}

/// ¿Caben estos bytes en el disco de esta ruta sin dejarlo tiritando? Si no se
/// puede saber, se intenta: quedarse sin disco a medias es un error más.
pub fn hay_espacio(ruta: &Path, bytes: u64) -> bool {
    espacio_libre(ruta).is_none_or(|libre| bytes.saturating_add(RESERVA_DISCO) <= libre)
}

#[cfg(test)]
#[test]
fn lo_que_no_cabe_en_el_disco_se_sabe() {
    let dir = tempfile::tempdir().unwrap();
    assert!(espacio_libre(dir.path()).is_some_and(|l| l > 0));
    assert!(!hay_espacio(dir.path(), u64::MAX));
    assert!(!hay_espacio(dir.path(), u64::MAX / 2));
    // Una ruta que no existe no bloquea nada: se intenta.
    assert!(hay_espacio(&dir.path().join("no-existe"), 1));
    // Un almacén que todavía no tiene carpeta mira la de arriba.
    assert!(!BlobStore::new(dir.path().join("blobs")).cabe(u64::MAX));
}

impl BlobStore {
    pub fn new(root: PathBuf) -> BlobStore {
        BlobStore { root }
    }

    /// ¿Caben estos bytes en el disco del almacén?
    pub fn cabe(&self, bytes: u64) -> bool {
        // La carpeta puede no existir aún: se mira la de arriba.
        let ruta = if self.root.exists() {
            self.root.as_path()
        } else {
            self.root.parent().unwrap_or(&self.root)
        };
        hay_espacio(ruta, bytes)
    }

    pub fn path_of(&self, hash: &str, ext: &str) -> PathBuf {
        self.root
            .join(&hash[..2.min(hash.len())])
            .join(format!("{hash}.{ext}"))
    }

    /// Ruta y tipo de un blob conocido.
    pub fn locate(&self, db: &Db, hash: &str) -> AppResult<Option<(PathBuf, Kind)>> {
        if !es_huella(hash) {
            return Ok(None);
        }
        let ext: Option<String> = db
            .conn()?
            .query_row("SELECT ext FROM blobs WHERE hash = ?1", [hash], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(ext.and_then(|e| Kind::from_ext(&e).map(|k| (self.path_of(hash, &e), k))))
    }

    /// Lo que se sabe de una imagen sin leerla.
    pub fn ficha(&self, db: &Db, hash: &str) -> AppResult<Option<FichaBlob>> {
        Ok(db
            .conn()?
            .query_row(
                "SELECT ext, width, height, bytes, origin FROM blobs WHERE hash = ?1",
                [hash],
                |r| {
                    Ok(FichaBlob {
                        ext: r.get(0)?,
                        width: r.get(1)?,
                        height: r.get(2)?,
                        bytes: r.get::<_, i64>(3)?.max(0) as u64,
                        origin: r.get(4)?,
                    })
                },
            )
            .optional()?)
    }

    /// Guarda unos bytes y devuelve su `FaceImage`. Si ya estaban, no escribe.
    pub fn put_bytes(&self, db: &Db, bytes: &[u8], origin: ImageOrigin) -> AppResult<FaceImage> {
        let info = probe(bytes)?;
        if info.width > LADO_MAXIMO || info.height > LADO_MAXIMO {
            return Err(AppError::new(ErrorCode::Image).with("motivo", "too_large"));
        }
        let hash = hex(&Sha256::digest(bytes));
        let destino = self.path_of(&hash, info.kind.ext());
        if !destino.exists() {
            escribir_atomico(&destino, bytes)?;
        }
        db.conn()?.execute(
            "INSERT OR IGNORE INTO blobs (hash, ext, mime, width, height, bytes, origin, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &hash,
                info.kind.ext(),
                info.kind.mime(),
                info.width,
                info.height,
                bytes.len() as i64,
                origen_texto(origin),
                super::ahora_ms(),
            ),
        )?;
        // Si ya existía, se devuelve con el origen con el que entró la primera vez.
        let origen: String =
            db.conn()?
                .query_row("SELECT origin FROM blobs WHERE hash = ?1", [&hash], |r| {
                    r.get(0)
                })?;
        Ok(FaceImage {
            blob: hash,
            width: info.width,
            height: info.height,
            origin: origen_de(&origen),
        })
    }

    /// Copia ficheros del usuario al almacén (nunca los enlaza: borrar el
    /// original no puede romper un deck).
    pub fn import_files(&self, db: &Db, rutas: &[PathBuf]) -> AppResult<Vec<FaceImage>> {
        rutas
            .iter()
            .map(|r| {
                let bytes = std::fs::read(r)?;
                self.put_bytes(db, &bytes, ImageOrigin::Upload)
            })
            .collect()
    }

    /// El reverso genérico de la app, metido en el almacén la primera vez.
    pub fn generic_back(&self, db: &Db) -> AppResult<FaceImage> {
        let img = self.put_bytes(db, REVERSO_GENERICO, ImageOrigin::Generated)?;
        super::settings::set(db, CLAVE_REVERSO, &img.blob)?;
        Ok(img)
    }

    /// Borra las imágenes que no usa nadie (ningún deck, ni el reverso
    /// genérico) y que tienen más de 24 h. `ocupado` = hay un trabajo de
    /// resolución en marcha: no se toca nada.
    pub fn gc(&self, db: &Db, ocupado: bool) -> AppResult<GcReport> {
        if ocupado {
            return Err(AppError::new(ErrorCode::Busy));
        }
        let limite = super::ahora_ms() - GRACIA_MS;
        let huerfanas: Vec<(String, String, i64)> = {
            let c = db.conn()?;
            let mut st = c.prepare(
                "SELECT hash, ext, bytes FROM blobs b
                 WHERE created_at < ?1
                   AND NOT EXISTS (SELECT 1 FROM deck_cards WHERE front_blob = b.hash OR back_blob = b.hash)
                   AND NOT EXISTS (SELECT 1 FROM decks WHERE default_back_blob = b.hash)
                   AND NOT EXISTS (SELECT 1 FROM blob_refs WHERE blob = b.hash)
                   AND NOT EXISTS (SELECT 1 FROM app_settings WHERE key = ?2 AND value = b.hash)",
            )?;
            let filas = st.query_map((limite, CLAVE_REVERSO), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
            filas.collect::<Result<_, _>>()?
        };
        let mut informe = GcReport {
            removed: 0,
            freed_bytes: 0,
        };
        for (hash, ext, bytes) in huerfanas {
            let ruta = self.path_of(&hash, &ext);
            match std::fs::remove_file(&ruta) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            db.conn()?
                .execute("DELETE FROM blobs WHERE hash = ?1", [&hash])?;
            informe.removed += 1;
            informe.freed_bytes += u64::try_from(bytes).unwrap_or(0);
        }
        Ok(informe)
    }
}

impl BlobStore {
    /// Quién usa esta imagen (lo mismo que mira `gc`).
    pub fn usos(&self, db: &Db, hash: &str) -> AppResult<Usos> {
        let c = db.conn()?;
        let mut u = Usos::default();
        let mut st = c.prepare_cached(
            "SELECT deck_id FROM deck_cards WHERE front_blob = ?1 OR back_blob = ?1
             UNION SELECT id FROM decks WHERE default_back_blob = ?1
             UNION SELECT owner_id FROM blob_refs WHERE blob = ?1 AND owner_kind = 'deck'",
        )?;
        for d in st.query_map([hash], |r| r.get::<_, String>(0))? {
            u.decks.insert(d?);
        }
        u.otros = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM blob_refs WHERE blob = ?1 AND owner_kind <> 'deck')
                 OR EXISTS(SELECT 1 FROM app_settings WHERE key = ?2 AND value = ?1)",
            (hash, CLAVE_REVERSO),
            |r| r.get(0),
        )?;
        Ok(u)
    }

    /// Borra estas imágenes si ya no las usa nadie, sin la espera de 24 h de
    /// `gc`: quien llama sabe que acaban de dejar de hacer falta.
    pub fn borrar_sin_usar(&self, db: &Db, hashes: &[&str]) -> AppResult<GcReport> {
        let mut informe = GcReport {
            removed: 0,
            freed_bytes: 0,
        };
        for &hash in hashes {
            let u = self.usos(db, hash)?;
            if u.otros || !u.decks.is_empty() {
                continue;
            }
            let Some(f) = self.ficha(db, hash)? else {
                continue;
            };
            match std::fs::remove_file(self.path_of(hash, &f.ext)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
            db.conn()?
                .execute("DELETE FROM blobs WHERE hash = ?1", [hash])?;
            informe.removed += 1;
            informe.freed_bytes += f.bytes;
        }
        Ok(informe)
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// El número de cada escritura de este proceso: nombra su temporal.
static ESCRITURAS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Escribe a un temporal en la misma carpeta y lo renombra: un corte a mitad
/// nunca deja un fichero a medias con el nombre bueno.
///
/// 🛑 El temporal es **propio de cada escritura** (proceso y número): la
/// preparación de la mesa escribe en paralelo, y dos copias de la misma carta
/// (las tierras básicas, las fichas repetidas) o dos decks que la comparten
/// escriben el mismo destino a la vez. Con un temporal fijo, uno lo renombraba
/// y al otro le faltaba el fichero («no se puede escribir en disco»).
pub fn escribir_atomico(destino: &Path, bytes: &[u8]) -> AppResult<()> {
    let dir = destino
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::Io).with("detalle", "ruta sin carpeta"))?;
    std::fs::create_dir_all(dir)?;
    let n = ESCRITURAS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = destino.with_extension(format!(
        "{}.{}-{n}.part",
        destino
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin"),
        std::process::id()
    ));
    let escrito = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, destino)
    })();
    if let Err(e) = escrito {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    #[test]
    fn escribir_a_la_vez_el_mismo_destino_no_falla() {
        let dir = std::env::temp_dir().join(format!("pb-escritura-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let destino = dir.join("carta.jpg");
        let hilos: Vec<_> = (0..8)
            .map(|_| {
                let d = destino.clone();
                std::thread::spawn(move || {
                    for _ in 0..60 {
                        escribir_atomico(&d, b"contenido de la carta").unwrap();
                    }
                })
            })
            .collect();
        for h in hilos {
            h.join().unwrap();
        }
        assert_eq!(std::fs::read(&destino).unwrap(), b"contenido de la carta");
        // No quedan temporales.
        let sobran = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|x| x == "part"))
            .count();
        assert_eq!(sobran, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Un PNG de verdad (decodificable) de w×h, para los tests.
    pub fn png(w: u32, h: u32, semilla: u8) -> Vec<u8> {
        // PNG sin comprimir: IHDR + IDAT con zlib «stored» + IEND.
        fn crc(datos: &[u8]) -> u32 {
            let mut c = 0xFFFF_FFFFu32;
            for &b in datos {
                c ^= u32::from(b);
                for _ in 0..8 {
                    c = if c & 1 != 0 {
                        0xEDB8_8320 ^ (c >> 1)
                    } else {
                        c >> 1
                    };
                }
            }
            !c
        }
        fn trozo(v: &mut Vec<u8>, tipo: &[u8], datos: &[u8]) {
            v.extend_from_slice(&(datos.len() as u32).to_be_bytes());
            let mut t = tipo.to_vec();
            t.extend_from_slice(datos);
            v.extend_from_slice(&t);
            v.extend_from_slice(&crc(&t).to_be_bytes());
        }
        let mut crudo = Vec::new();
        for y in 0..h {
            crudo.push(0);
            for x in 0..w {
                crudo.extend_from_slice(&[semilla, (x % 256) as u8, (y % 256) as u8]);
            }
        }
        let mut z = vec![0x78, 0x01];
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &x in &crudo {
            a = (a + u32::from(x)) % 65521;
            b = (b + a) % 65521;
        }
        for (i, bloque) in crudo.chunks(65535).enumerate() {
            let ultimo = (i + 1) * 65535 >= crudo.len();
            z.push(u8::from(ultimo));
            let n = bloque.len() as u16;
            z.extend_from_slice(&n.to_le_bytes());
            z.extend_from_slice(&(!n).to_le_bytes());
            z.extend_from_slice(bloque);
        }
        z.extend_from_slice(&((b << 16) | a).to_be_bytes());
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
        trozo(&mut v, b"IHDR", &ihdr);
        trozo(&mut v, b"IDAT", &z);
        trozo(&mut v, b"IEND", &[]);
        v
    }

    fn almacen() -> (tempfile::TempDir, Db, BlobStore) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::in_memory().unwrap();
        let s = BlobStore::new(dir.path().join("blobs"));
        (dir, db, s)
    }

    #[test]
    fn la_misma_imagen_dos_veces_es_un_fichero_y_una_fila() {
        let (_d, db, s) = almacen();
        let a = s
            .put_bytes(&db, &png(4, 6, 1), ImageOrigin::Upload)
            .unwrap();
        let b = s
            .put_bytes(&db, &png(4, 6, 1), ImageOrigin::Upload)
            .unwrap();
        assert_eq!(a, b);
        assert!(es_huella(&a.blob));
        assert_eq!((a.width, a.height), (4, 6));
        assert!(s.path_of(&a.blob, "png").exists());
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM blobs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn un_texto_no_entra() {
        let (_d, db, s) = almacen();
        let e = s.put_bytes(&db, b"no soy una imagen", ImageOrigin::Upload);
        assert_eq!(e.unwrap_err().code, ErrorCode::Unsupported);
    }

    #[test]
    fn la_limpieza_solo_borra_lo_huerfano_y_viejo() {
        let (_d, db, s) = almacen();
        let usada = s
            .put_bytes(&db, &png(2, 2, 1), ImageOrigin::Upload)
            .unwrap();
        let huerfana = s
            .put_bytes(&db, &png(2, 2, 2), ImageOrigin::Upload)
            .unwrap();
        let reciente = s
            .put_bytes(&db, &png(2, 2, 3), ImageOrigin::Upload)
            .unwrap();
        let generico = s.generic_back(&db).unwrap();
        {
            let c = db.conn().unwrap();
            c.execute(
                "UPDATE blobs SET created_at = 0 WHERE hash IN (?1, ?2, ?3)",
                (&usada.blob, &huerfana.blob, &generico.blob),
            )
            .unwrap();
            c.execute(
                "INSERT INTO decks (id, name, settings_json, schema_version, created_at, updated_at) VALUES ('d', 'd', '{}', 1, 0, 0)",
                [],
            )
            .unwrap();
            c.execute(
                "INSERT INTO deck_cards (id, deck_id, position, front_blob, card_json) VALUES ('c', 'd', 0, ?1, '{}')",
                [&usada.blob],
            )
            .unwrap();
        }
        assert_eq!(s.gc(&db, true).unwrap_err().code, ErrorCode::Busy);
        let r = s.gc(&db, false).unwrap();
        assert_eq!(r.removed, 1);
        assert!(!s.path_of(&huerfana.blob, "png").exists());
        assert!(s.path_of(&usada.blob, "png").exists());
        assert!(s.path_of(&reciente.blob, "png").exists());
        assert!(s.path_of(&generico.blob, "png").exists());
    }

    #[test]
    fn el_reverso_generico_es_un_png_de_745_por_1040() {
        let (_d, db, s) = almacen();
        let r = s.generic_back(&db).unwrap();
        assert_eq!((r.width, r.height), (745, 1040));
        assert_eq!(r.origin, ImageOrigin::Generated);
    }
}
