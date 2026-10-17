//! **La biblioteca local**: SQLite (`library.db`) y el almacén de imágenes por
//! huella (`blobs/`), los dos en `app_data_dir`.

pub mod blobs;
pub mod colecciones;
pub mod db;
pub mod decks;
pub mod etiquetas;
pub mod presets;
pub mod settings;

/// Milisegundos desde 1970: así se guardan todas las fechas.
pub fn ahora_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
