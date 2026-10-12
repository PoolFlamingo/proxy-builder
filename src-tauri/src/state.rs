//! El estado compartido de la app: base de datos, almacén de imágenes,
//! carpetas y trabajos en marcha.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::error::AppResult;
use crate::store::blobs::BlobStore;
use crate::store::db::Db;

/// Las carpetas de la app (las da Tauri según la plataforma).
#[derive(Debug, Clone)]
pub struct Paths {
    /// `app_data_dir`: la biblioteca y las imágenes de los decks.
    pub data: PathBuf,
    /// `app_cache_dir`: derivados que se pueden borrar (miniaturas).
    pub cache: PathBuf,
}

pub struct AppState {
    pub db: Db,
    pub blobs: BlobStore,
    pub paths: Paths,
    /// Trabajos de resolución/exportación en marcha (la limpieza de imágenes
    /// no corre mientras haya alguno).
    trabajos: AtomicUsize,
}

impl AppState {
    pub fn open(paths: Paths) -> AppResult<AppState> {
        std::fs::create_dir_all(&paths.data)?;
        std::fs::create_dir_all(&paths.cache)?;
        let db = Db::open(&paths.data.join("library.db"))?;
        let blobs = BlobStore::new(paths.data.join("blobs"));
        Ok(AppState {
            db,
            blobs,
            paths,
            trabajos: AtomicUsize::new(0),
        })
    }

    pub fn ocupado(&self) -> bool {
        self.trabajos.load(Ordering::SeqCst) > 0
    }

    /// Marca un trabajo en marcha hasta que se suelte el guardián.
    pub fn trabajo(&self) -> TrabajoEnMarcha<'_> {
        self.trabajos.fetch_add(1, Ordering::SeqCst);
        TrabajoEnMarcha { estado: self }
    }
}

pub struct TrabajoEnMarcha<'a> {
    estado: &'a AppState,
}

impl Drop for TrabajoEnMarcha<'_> {
    fn drop(&mut self) {
        self.estado.trabajos.fetch_sub(1, Ordering::SeqCst);
    }
}
