//! **La identidad de la instalación**: la clave con la que se conecta a los
//! demás (y, en la Fase F, al servidor). Es una clave ed25519 de iroh: su
//! parte pública es el `EndpointId` con el que los demás la reconocen, y
//! sirve también para firmar el reto del servidor (`SecretKey::sign`).
//!
//! Se crea la primera vez y se guarda en `app_data_dir/identidad/clave`
//! (32 bytes, permisos 0600 en Unix), nunca en la base de datos ni en el
//! registro. Borrar la carpeta crea una nueva. El apodo y el avatar van en
//! `app_settings` (`red.apodo`, `red.avatar`); el avatar, protegido del
//! borrado de imágenes en `blob_refs` (`owner_kind = 'red'`).

use std::path::Path;

use iroh::SecretKey;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::db::Db;
use crate::store::settings;

/// La clave de la instalación: la de `dir/identidad/clave`, o una nueva.
pub fn cargar_o_crear(dir: &Path) -> AppResult<SecretKey> {
    let carpeta = dir.join("identidad");
    let ruta = carpeta.join("clave");
    if let Ok(bytes) = std::fs::read(&ruta) {
        if let Ok(b) = <[u8; 32]>::try_from(bytes.as_slice()) {
            return Ok(SecretKey::from_bytes(&b));
        }
        tracing::warn!("la clave de la identidad no se pudo leer: se crea otra");
    }
    let clave = SecretKey::generate();
    std::fs::create_dir_all(&carpeta)?;
    escribir_privado(&ruta, &clave.to_bytes())?;
    Ok(clave)
}

/// Lo que lleva el fichero de la identidad exportada. Quien lo tenga puede
/// jugar como esta instalación: se guarda con permiso solo del usuario.
#[derive(Serialize, Deserialize)]
struct FicheroIdentidad {
    tipo: String,
    version: u32,
    /// La clave secreta, en hexadecimal (64 caracteres).
    clave: String,
    /// La clave pública, para comprobar que es la que se quería.
    id: String,
}

const TIPO_FICHERO: &str = "proxy-builder-identidad";
/// Un fichero de identidad de verdad mide unos 200 bytes.
const MAX_FICHERO: u64 = 4096;

fn a_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn de_hex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// Guarda la identidad en `ruta` (para llevarla a otro equipo o tener copia).
pub fn exportar(clave: &SecretKey, ruta: &Path) -> AppResult<()> {
    let f = FicheroIdentidad {
        tipo: TIPO_FICHERO.into(),
        version: 1,
        clave: a_hex(&clave.to_bytes()),
        id: clave.public().to_string(),
    };
    let json = serde_json::to_vec_pretty(&f)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    escribir_privado(ruta, &json)
}

/// Lee un fichero de identidad exportado y comprueba que es válido.
pub fn leer_fichero(ruta: &Path) -> AppResult<SecretKey> {
    let malo = || AppError::new(ErrorCode::InvalidInput).with("motivo", "identidad");
    let meta = std::fs::metadata(ruta)?;
    if !meta.is_file() || meta.len() > MAX_FICHERO {
        return Err(malo());
    }
    let f: FicheroIdentidad = serde_json::from_slice(&std::fs::read(ruta)?).map_err(|_| malo())?;
    if f.tipo != TIPO_FICHERO || f.version != 1 {
        return Err(malo());
    }
    let clave = SecretKey::from_bytes(&de_hex(&f.clave).ok_or_else(malo)?);
    if clave.public().to_string() != f.id {
        return Err(malo());
    }
    Ok(clave)
}

/// Pone la identidad del fichero `ruta` como la de la instalación (`dir` es
/// la carpeta de datos). La anterior se queda en `identidad/clave.anterior`,
/// por si fue un error. Devuelve la clave nueva.
pub fn importar(dir: &Path, ruta: &Path) -> AppResult<SecretKey> {
    let clave = leer_fichero(ruta)?;
    let carpeta = dir.join("identidad");
    let actual = carpeta.join("clave");
    std::fs::create_dir_all(&carpeta)?;
    if actual.exists() {
        let _ = std::fs::copy(&actual, carpeta.join("clave.anterior"));
    }
    escribir_privado(&actual, &clave.to_bytes())?;
    Ok(clave)
}

/// Escribe un fichero que solo puede leer el usuario (0600 en Unix), de una vez.
fn escribir_privado(ruta: &Path, bytes: &[u8]) -> AppResult<()> {
    use std::io::Write;
    let tmp = ruta.with_extension("tmp");
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    let mut f = o.open(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, ruta)?;
    Ok(())
}

/// Lo que la pantalla sabe de la identidad.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Identidad {
    /// La clave pública (el `EndpointId`), en texto.
    pub id: String,
    pub apodo: String,
    /// Una huella de blob (la imagen elegida), si hay.
    pub avatar: Option<String>,
}

const APODO: &str = "red.apodo";
const AVATAR: &str = "red.avatar";
/// Lo más largo que puede ser un apodo.
pub const MAX_APODO: usize = 32;

pub fn leer(db: &Db, clave: &SecretKey) -> AppResult<Identidad> {
    Ok(Identidad {
        id: clave.public().to_string(),
        apodo: settings::get(db, APODO)?.unwrap_or_default(),
        avatar: settings::get(db, AVATAR)?.filter(|a| crate::store::blobs::es_huella(a)),
    })
}

/// Cambia el apodo (sin espacios de más y hasta `MAX_APODO` caracteres) y el avatar.
pub fn guardar(db: &Db, apodo: &str, avatar: Option<&str>) -> AppResult<()> {
    let apodo = apodo.split_whitespace().collect::<Vec<_>>().join(" ");
    if apodo.chars().count() > MAX_APODO {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "apodo"));
    }
    if avatar.is_some_and(|a| !crate::store::blobs::es_huella(a)) {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "avatar"));
    }
    {
        // El avatar se protege del borrado de imágenes como el tapete. La
        // conexión se suelta antes de `settings::set`, que la vuelve a pedir.
        let c = db.conn()?;
        c.execute(
            "DELETE FROM blob_refs WHERE owner_kind = 'red' AND owner_id = 'avatar'",
            [],
        )?;
        match avatar {
            Some(a) => {
                c.execute(
                    "INSERT OR IGNORE INTO blob_refs (owner_kind, owner_id, blob) VALUES ('red', 'avatar', ?1)",
                    [a],
                )?;
            }
            None => {
                c.execute("DELETE FROM app_settings WHERE key = ?1", [AVATAR])?;
            }
        }
    }
    settings::set(db, APODO, &apodo)?;
    if let Some(a) = avatar {
        settings::set(db, AVATAR, a)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_clave_se_crea_una_vez_y_se_queda() {
        let dir = tempfile::tempdir().unwrap();
        let a = cargar_o_crear(dir.path()).unwrap();
        let b = cargar_o_crear(dir.path()).unwrap();
        assert_eq!(a.public(), b.public());
        let otra = tempfile::tempdir().unwrap();
        assert_ne!(cargar_o_crear(otra.path()).unwrap().public(), a.public());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = std::fs::metadata(dir.path().join("identidad/clave")).unwrap();
            assert_eq!(m.permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn exportada_e_importada_en_otro_equipo_es_la_misma_identidad() {
        let origen = tempfile::tempdir().unwrap();
        let a = cargar_o_crear(origen.path()).unwrap();
        let fichero = origen.path().join("mi-identidad.json");
        exportar(&a, &fichero).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = std::fs::metadata(&fichero).unwrap();
            assert_eq!(m.permissions().mode() & 0o777, 0o600);
        }
        // Otro equipo, con su propia clave.
        let destino = tempfile::tempdir().unwrap();
        let b = cargar_o_crear(destino.path()).unwrap();
        assert_ne!(a.public(), b.public());
        let nueva = importar(destino.path(), &fichero).unwrap();
        assert_eq!(nueva.public(), a.public());
        // Queda guardada: al volver a arrancar es la misma, y la vieja se conserva.
        assert_eq!(cargar_o_crear(destino.path()).unwrap().public(), a.public());
        let vieja = std::fs::read(destino.path().join("identidad/clave.anterior")).unwrap();
        assert_eq!(
            SecretKey::from_bytes(&vieja.try_into().unwrap()).public(),
            b.public()
        );
    }

    #[test]
    fn un_fichero_que_no_es_una_identidad_no_cambia_nada() {
        let dir = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(dir.path()).unwrap();
        let malos: Vec<Vec<u8>> = vec![
            b"hola".to_vec(),
            br#"{"tipo":"otra-cosa","version":1,"clave":"","id":""}"#.to_vec(),
            // La clave no corresponde al id.
            format!(
                r#"{{"tipo":"{TIPO_FICHERO}","version":1,"clave":"{}","id":"{}"}}"#,
                "00".repeat(32),
                k.public()
            )
            .into_bytes(),
            // Demasiado grande.
            vec![b' '; 10_000],
        ];
        for (i, m) in malos.iter().enumerate() {
            let f = dir.path().join(format!("malo{i}.json"));
            std::fs::write(&f, m).unwrap();
            assert!(importar(dir.path(), &f).is_err(), "malo {i}");
        }
        assert_eq!(cargar_o_crear(dir.path()).unwrap().public(), k.public());
        assert!(importar(dir.path(), &dir.path().join("no-existe.json")).is_err());
    }

    #[test]
    fn la_clave_firma_para_el_reto_del_servidor() {
        let dir = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(dir.path()).unwrap();
        let firma = k.sign(b"reto");
        assert!(k.public().verify(b"reto", &firma).is_ok());
        assert!(k.public().verify(b"otro", &firma).is_err());
    }

    #[test]
    fn apodo_y_avatar() {
        let db = Db::in_memory().unwrap();
        let k = SecretKey::generate();
        assert_eq!(leer(&db, &k).unwrap().apodo, "");
        guardar(&db, "  Pablo   el  de  Zeus ", Some(&"a".repeat(64))).unwrap();
        let i = leer(&db, &k).unwrap();
        assert_eq!(i.apodo, "Pablo el de Zeus");
        assert_eq!(i.avatar, Some("a".repeat(64)));
        assert_eq!(i.id, k.public().to_string());
        assert!(guardar(&db, &"x".repeat(33), None).is_err());
        assert!(guardar(&db, "ok", Some("no-es-huella")).is_err());
        guardar(&db, "ok", None).unwrap();
        assert_eq!(leer(&db, &k).unwrap().avatar, None);
    }
}
