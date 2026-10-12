//! El registro de la app (`app_log_dir/proxy-builder.log`): cada petición de
//! red queda apuntada, para poder comprobar que un deck guardado no descarga
//! nada (DoD-5).

use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Mutex;

pub fn iniciar(dir: &Path) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(fichero) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("proxy-builder.log"))
    else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_writer(Mutex::new(fichero))
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .try_init();
}
