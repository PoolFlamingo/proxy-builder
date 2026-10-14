//! **Las últimas carpetas**: dónde se importó algo por última vez (imágenes,
//! decks) y dónde se exportó (PDF, decks), por separado, para que los
//! diálogos de ficheros empiecen ahí.

use std::path::Path;

use serde::Deserialize;
use tauri::State;
use ts_rs::TS;

use crate::error::AppResult;
use crate::state::AppState;
use crate::store::settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum TipoCarpeta {
    Importar,
    Exportar,
}

fn clave(tipo: TipoCarpeta) -> &'static str {
    match tipo {
        TipoCarpeta::Importar => "carpeta_importar",
        TipoCarpeta::Exportar => "carpeta_exportar",
    }
}

/// La carpeta de una ruta elegida: la propia si es una carpeta; si no (un
/// fichero, exista o no todavía), la que lo contiene.
fn carpeta_de(ruta: &Path) -> Option<&Path> {
    if ruta.is_dir() {
        Some(ruta)
    } else {
        ruta.parent().filter(|p| !p.as_os_str().is_empty())
    }
}

/// Recuerda la carpeta de `ruta` (un fichero o una carpeta).
#[tauri::command]
pub fn carpeta_recordar(
    state: State<'_, AppState>,
    tipo: TipoCarpeta,
    ruta: String,
) -> AppResult<()> {
    if let Some(dir) = carpeta_de(Path::new(&ruta)) {
        settings::set(&state.db, clave(tipo), &dir.to_string_lossy())?;
    }
    Ok(())
}

/// La ruta con la que abrir el diálogo: la última carpeta de ese tipo (si
/// sigue existiendo) y, si se da, el nombre del fichero dentro de ella.
#[tauri::command]
pub fn carpeta_inicial(
    state: State<'_, AppState>,
    tipo: TipoCarpeta,
    nombre: Option<String>,
) -> AppResult<Option<String>> {
    let Some(dir) = settings::get(&state.db, clave(tipo))? else {
        return Ok(nombre);
    };
    let dir = Path::new(&dir);
    if !dir.is_dir() {
        return Ok(nombre);
    }
    Ok(Some(match nombre {
        Some(n) => dir.join(n).to_string_lossy().into_owned(),
        None => dir.to_string_lossy().into_owned(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_carpeta_de_una_ruta() {
        let tmp = std::env::temp_dir();
        assert_eq!(carpeta_de(&tmp), Some(tmp.as_path()));
        let f = tmp.join("no-existe-todavia.pdf");
        assert_eq!(carpeta_de(&f), Some(tmp.as_path()));
        assert_eq!(carpeta_de(Path::new("suelto.pdf")), None);
    }
}
