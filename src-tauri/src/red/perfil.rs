//! **Perfiles de prueba** (`PB_PERFIL=<nombre>`, solo en debug): abrir dos o
//! más instancias de la app en el mismo equipo con identidades distintas,
//! para probar la partida en red. Cada perfil tiene sus propias carpetas de
//! datos, caché y registro (`…/perfiles/<nombre>`), su propio directorio de
//! la vista web (así no comparten `localStorage`: idioma, preferencias,
//! tutoriales vistos) y sin el estado guardado de la ventana. En release no
//! hace nada.

use std::path::{Path, PathBuf};

/// El perfil de esta instancia, si hay uno.
pub fn actual() -> Option<String> {
    de(
        std::env::var("PB_PERFIL").ok().as_deref(),
        cfg!(debug_assertions),
    )
}

/// El perfil para un valor de `PB_PERFIL`: solo en debug y con un nombre
/// sencillo (letras, números, `-` y `_`, hasta 32), para usarlo de carpeta.
pub fn de(valor: Option<&str>, debug: bool) -> Option<String> {
    let v = valor?.trim();
    let valido = !v.is_empty()
        && v.len() <= 32
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    (debug && valido).then(|| v.to_ascii_lowercase())
}

/// La carpeta de un perfil dentro de una de la app.
pub fn carpeta(base: &Path, perfil: Option<&str>) -> PathBuf {
    match perfil {
        Some(p) => base.join("perfiles").join(p),
        None => base.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_en_debug_y_con_un_nombre_sencillo() {
        assert_eq!(de(Some("a"), true).as_deref(), Some("a"));
        assert_eq!(de(Some("Jugador_2"), true).as_deref(), Some("jugador_2"));
        // En release, nada.
        assert_eq!(de(Some("a"), false), None);
        for malo in ["", "  ", "../x", "a/b", "con espacio", &"x".repeat(33)] {
            assert_eq!(de(Some(malo), true), None, "{malo:?}");
        }
        assert_eq!(de(None, true), None);
    }

    #[test]
    fn las_carpetas_van_aparte() {
        let base = Path::new("/datos");
        assert_eq!(carpeta(base, None), PathBuf::from("/datos"));
        assert_eq!(carpeta(base, Some("b")), PathBuf::from("/datos/perfiles/b"));
    }
}
