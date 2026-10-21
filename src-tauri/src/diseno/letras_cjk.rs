//! **Letras de japonés, coreano y chino** para las cartas diseñadas en esos
//! idiomas: Noto Sans JP, KR, SC y TC (OFL) de Google Fonts, de una versión
//! fija de su repositorio y con la huella SHA-256 de cada fichero
//! (`assets/letras-cjk.json`). Pesan de 10 a 18 MB cada una: no van en el
//! instalador; se bajan la primera vez que un deck elige uno de esos idiomas
//! de diseño (o desde el aviso «faltan letras») y se guardan en
//! `<datos de la app>/letras-cjk/<código>.ttf`. Solo la Sans: con la Serif
//! serían el doble.

use proxy_core::diseno::modelo::DesignLang;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;
use crate::store::blobs::escribir_atomico;

const CATALOGO: &str = include_str!("../../assets/letras-cjk.json");
const REPO: &str = "https://raw.githubusercontent.com/google/fonts";
/// Tope de lo que se lee de una respuesta (la mayor pesa 17,8 MB).
const TOPE: usize = 32 << 20;

#[derive(Debug, Clone, Deserialize)]
struct Letra {
    lang: DesignLang,
    // `familia` y `bytes` del catálogo son para quien lo lee: la familia la
    // da la letra y el tamaño se comprueba con el tope al bajarla.
    ruta: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Licencia {
    ruta: String,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalogo {
    commit: String,
    base: String,
    letras: Vec<Letra>,
    licencia: Licencia,
}

fn catalogo() -> AppResult<Catalogo> {
    serde_json::from_str(CATALOGO)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
}

fn huella(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Si `lang` necesita una letra que se baja aparte.
pub fn necesita(lang: DesignLang) -> bool {
    catalogo().is_ok_and(|c| c.letras.iter().any(|l| l.lang == lang))
}

async fn bajar(http: &reqwest::Client, url: &str, sha: &str) -> AppResult<Vec<u8>> {
    let mut r = http
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
    tracing::info!(
        host = "raw.githubusercontent.com",
        estado = r.status().as_u16(),
        "red"
    );
    if !r.status().is_success() {
        return Err(AppError::new(ErrorCode::Network).with("estado", r.status().as_u16()));
    }
    let mut b = Vec::new();
    while let Some(trozo) = r
        .chunk()
        .await
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?
    {
        b.extend_from_slice(&trozo);
        if b.len() > TOPE {
            return Err(AppError::new(ErrorCode::Network).with("motivo", "tamano"));
        }
    }
    // Lo que llega tiene que ser justo lo esperado.
    if huella(&b) != sha {
        return Err(AppError::new(ErrorCode::Network).with("motivo", "huella"));
    }
    Ok(b)
}

/// Baja (si falta) la letra de `lang` y la carga en los motores. Un idioma
/// que no la necesita no hace nada.
pub async fn descargar(state: &AppState, lang: DesignLang) -> AppResult<()> {
    let c = catalogo()?;
    let Some(l) = c.letras.iter().find(|l| l.lang == lang) else {
        return Ok(());
    };
    if state.diseno.tiene_cjk(lang)? {
        return Ok(());
    }
    let http = reqwest::Client::builder()
        .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
    let url = |ruta: &str| {
        // Los corchetes de `[wght]` van escapados en la URL.
        let ruta = ruta.replace('[', "%5B").replace(']', "%5D");
        format!("{REPO}/{}/{}{ruta}", c.commit, c.base)
    };
    let datos = bajar(&http, &url(&l.ruta), &l.sha256).await?;
    let licencia = bajar(&http, &url(&c.licencia.ruta), &c.licencia.sha256).await?;
    escribir_atomico(
        &state
            .diseno
            .dir()
            .join("letras-cjk")
            .join("Noto-CJK-OFL.txt"),
        &licencia,
    )?;
    state.diseno.guardar_cjk(lang, datos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_catalogo_trae_los_cuatro_idiomas_de_asia() {
        let c = catalogo().unwrap();
        assert_eq!(c.commit.len(), 40);
        let langs: Vec<DesignLang> = c.letras.iter().map(|l| l.lang).collect();
        assert_eq!(
            langs,
            [
                DesignLang::Ja,
                DesignLang::Ko,
                DesignLang::Zhs,
                DesignLang::Zht
            ]
        );
        for l in &c.letras {
            assert_eq!(l.sha256.len(), 64);
            assert!(l.ruta.ends_with(".ttf") && !l.ruta.contains(".."));
        }
        assert!(necesita(DesignLang::Ja) && !necesita(DesignLang::Ru));
    }

    /// Baja de verdad la letra japonesa: `cargar_cjk` la acepta y tiene los
    /// kana. `cargo test --lib letras_cjk -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn baja_la_japonesa() {
        let c = catalogo().unwrap();
        let l = &c.letras[0];
        let http = reqwest::Client::builder()
            .user_agent("ProxyBuilder-test")
            .build()
            .unwrap();
        let url = format!(
            "{REPO}/{}/{}{}",
            c.commit,
            c.base,
            l.ruta.replace('[', "%5B").replace(']', "%5D")
        );
        let b = bajar(&http, &url, &l.sha256).await.unwrap();
        let mut letras = proxy_core::diseno::letras::Letras::nuevas();
        letras
            .cargar_cjk(b, proxy_core::diseno::modelo::Escritura::Japonesa)
            .unwrap();
        assert!(letras.tiene_escritura(proxy_core::diseno::modelo::Escritura::Japonesa));
    }
}
