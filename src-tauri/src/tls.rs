//! **El TLS de cada plataforma**, en un solo sitio. Hay tres consumidores: los
//! clientes HTTP (`reqwest`, el actualizador incluido), el WebSocket del
//! servidor de salas (`rustls`) y el nodo de iroh.
//!
//! En escritorio, el verificador del sistema (`rustls-platform-verifier`: los
//! certificados que el sistema ya tiene, con su revocación). En Android (y iOS,
//! hasta que se pruebe allí) ese verificador entra en pánico si no se inicializa
//! con su parte de Kotlin, así que en móvil se usan los certificados raíz de
//! Mozilla incrustados en la app: sin Kotlin ni JNI en el arranque. Se pierden
//! las autoridades que alguien instale a mano en el teléfono, que esta app no
//! necesita.
//!
//! 🛑 Nadie más crea un cliente: ningún `reqwest::Client::builder()`,
//! `with_platform_verifier` ni `CaTlsConfig::` fuera de este fichero (lo
//! vigila un test).

use crate::error::{AppError, AppResult, ErrorCode};

/// Un `reqwest::ClientBuilder` con el TLS de esta plataforma. Se usa así:
/// `tls::reqwest(reqwest::Client::builder()).user_agent(…).build()`.
pub fn reqwest(b: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
    #[cfg(mobile)]
    {
        b.tls_certs_only(raices_reqwest().iter().cloned())
    }
    #[cfg(not(mobile))]
    {
        b
    }
}

/// Los certificados raíz de Mozilla como los quiere `reqwest`, hechos una vez.
#[cfg(mobile)]
fn raices_reqwest() -> &'static [reqwest::Certificate] {
    use std::sync::OnceLock;
    static RAICES: OnceLock<Vec<reqwest::Certificate>> = OnceLock::new();
    RAICES.get_or_init(|| {
        webpki_root_certs::TLS_SERVER_ROOT_CERTS
            .iter()
            .filter_map(|c| reqwest::Certificate::from_der(c.as_ref()).ok())
            .collect()
    })
}

/// La configuración de `rustls` para el WebSocket del servidor de salas.
pub fn rustls() -> AppResult<rustls::ClientConfig> {
    use std::sync::Arc;
    let proveedor = Arc::new(rustls::crypto::ring::default_provider());
    let red = |e: rustls::Error| AppError::new(ErrorCode::Network).with("detalle", e);
    let b = rustls::ClientConfig::builder_with_provider(proveedor)
        .with_safe_default_protocol_versions()
        .map_err(red)?;
    #[cfg(not(mobile))]
    let b = {
        use rustls_platform_verifier::BuilderVerifierExt;
        b.with_platform_verifier().map_err(red)?
    };
    #[cfg(mobile)]
    let b = b.with_root_certificates(rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    });
    Ok(b.with_no_client_auth())
}

/// Los certificados con los que el nodo de iroh comprueba los relés.
pub fn iroh() -> iroh::tls::CaTlsConfig {
    #[cfg(mobile)]
    {
        iroh::tls::CaTlsConfig::embedded()
    }
    #[cfg(not(mobile))]
    {
        iroh::tls::CaTlsConfig::system()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    fn ficheros(dir: &Path, fuera: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                ficheros(&p, fuera);
            } else if p.extension().is_some_and(|x| x == "rs") {
                fuera.push(p);
            }
        }
    }

    #[test]
    fn nadie_crea_un_cliente_ni_un_verificador_fuera_de_tls_rs() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut todos = Vec::new();
        ficheros(&src, &mut todos);
        let mut mal = Vec::new();
        for f in todos {
            if f.file_name().is_some_and(|n| n == "tls.rs") {
                continue;
            }
            let texto = std::fs::read_to_string(&f).unwrap();
            for (i, l) in texto.lines().enumerate() {
                let l = l.trim();
                if l.starts_with("//") {
                    continue;
                }
                let cliente = l.contains("Client::builder()") && !l.contains("tls::reqwest(");
                if cliente || l.contains("with_platform_verifier") || l.contains("CaTlsConfig::") {
                    mal.push(format!("{}:{}", f.display(), i + 1));
                }
            }
        }
        assert!(mal.is_empty(), "TLS fuera de tls.rs: {mal:?}");
    }
}
