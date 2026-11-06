//! **La identidad de la instalación**: la clave con la que se conecta a los
//! demás (y, en la Fase F, al servidor). Es una clave ed25519 de iroh: su
//! parte pública es el `EndpointId` con el que los demás la reconocen, y
//! sirve también para firmar el reto del servidor (`SecretKey::sign`).
//!
//! Se crea la primera vez y se guarda en `app_data_dir/identidad/clave`
//! (32 bytes, permisos 0600 en Unix), nunca en la base de datos ni en el
//! registro. Borrar la carpeta crea una nueva.
//!
//! En Android va **sellada** con una clave del Android Keystore
//! (`identidad/clave.sellada`, [`Cofre`]): copiar el fichero fuera del móvil no
//! sirve de nada, y la carpeta no entra en la copia automática de Google. La
//! clave de quien ya tenía la app se sella al arrancar, y el fichero en claro solo
//! se borra tras comprobar que lo sellado se abre y da la misma clave.
//!
//! El apodo y el avatar van en
//! `app_settings` (`red.apodo`, `red.avatar`); el avatar, protegido del
//! borrado de imágenes en `blob_refs` (`owner_kind = 'red'`).

use std::path::Path;

use iroh::SecretKey;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::db::Db;
use crate::store::settings;

/// Donde el sistema guarda secretos que no salen del equipo (`android.rs`: el
/// Android Keystore). Detrás de un trait para poder probarlo sin Android.
pub trait Cofre {
    fn sellar(&self, claro: &[u8]) -> AppResult<Vec<u8>>;
    fn abrir(&self, sellado: &[u8]) -> AppResult<Vec<u8>>;
}

const EN_CLARO: &str = "clave";
const SELLADA: &str = "clave.sellada";
const ANTERIOR: &str = "clave.anterior";
const SELLADA_ANTERIOR: &str = "clave.sellada.anterior";

fn de_bytes(bytes: &[u8]) -> Option<SecretKey> {
    <[u8; 32]>::try_from(bytes)
        .ok()
        .map(|b| SecretKey::from_bytes(&b))
}

fn leer_en_claro(ruta: &Path) -> Option<SecretKey> {
    std::fs::read(ruta).ok().and_then(|b| de_bytes(&b))
}

/// Sella estos bytes y comprueba que se abren y dan lo mismo. 🛑 Sin esa
/// comprobación no se borra nunca lo que hay en claro: una identidad perdida no se
/// recupera (los playgroups van por ella).
fn sellar_comprobado(cofre: &dyn Cofre, claro: &[u8]) -> AppResult<Vec<u8>> {
    let sellado = cofre.sellar(claro)?;
    if cofre.abrir(&sellado)? != claro {
        return Err(AppError::new(ErrorCode::Internal).with("detalle", "cofre"));
    }
    Ok(sellado)
}

/// Guarda la clave donde toca. Si el cofre falla, en claro: mejor con los permisos
/// de siempre que sin identidad.
fn guardar_clave(carpeta: &Path, clave: &SecretKey, cofre: Option<&dyn Cofre>) -> AppResult<()> {
    if let Some(c) = cofre {
        match sellar_comprobado(c, &clave.to_bytes()) {
            Ok(sellado) => {
                escribir_privado(&carpeta.join(SELLADA), &sellado)?;
                let _ = std::fs::remove_file(carpeta.join(EN_CLARO));
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "identidad: no se pudo sellar, sigue en claro");
            }
        }
    }
    escribir_privado(&carpeta.join(EN_CLARO), &clave.to_bytes())
}

/// La identidad de antes de importar otra: tampoco se queda en claro.
fn sellar_la_anterior(carpeta: &Path, cofre: &dyn Cofre) {
    let ruta = carpeta.join(ANTERIOR);
    let Ok(bytes) = std::fs::read(&ruta) else {
        return;
    };
    let sellada = sellar_comprobado(cofre, &bytes)
        .and_then(|s| escribir_privado(&carpeta.join(SELLADA_ANTERIOR), &s));
    if sellada.is_ok() {
        let _ = std::fs::remove_file(ruta);
    }
}

/// La clave de la instalación: la guardada, o una nueva. Con `cofre`, la deja
/// sellada aunque la encuentre en claro.
pub fn cargar_o_crear(dir: &Path, cofre: Option<&dyn Cofre>) -> AppResult<SecretKey> {
    let carpeta = dir.join("identidad");
    let ruta = carpeta.join(EN_CLARO);
    let sellada = carpeta.join(SELLADA);
    let en_claro = leer_en_claro(&ruta);
    if let Some(c) = cofre {
        sellar_la_anterior(&carpeta, c);
        if let Ok(bytes) = std::fs::read(&sellada) {
            match c.abrir(&bytes).ok().as_deref().and_then(de_bytes) {
                Some(clave) => {
                    // Lo de en claro, si es otra clave, es lo último que se escribió (un
                    // importar que no pudo sellar): no se borra sin comprobarlo.
                    if let Some(c) = en_claro {
                        if c.to_bytes() != clave.to_bytes() {
                            tracing::warn!(
                                "identidad: lo de en claro no es lo sellado; se usa lo de en claro"
                            );
                            return Ok(c);
                        }
                    }
                    // Un sellado que se cortó antes de borrar lo de antes.
                    let _ = std::fs::remove_file(&ruta);
                    return Ok(clave);
                }
                None => {
                    tracing::warn!("identidad: lo sellado no se abre");
                    if en_claro.is_none() {
                        // No se borra: puede ser un fallo pasajero del Keystore.
                        let _ = std::fs::rename(&sellada, carpeta.join("clave.sellada.ilegible"));
                    }
                }
            }
        }
    }
    if let Some(clave) = en_claro {
        if cofre.is_some() {
            guardar_clave(&carpeta, &clave, cofre)?;
        }
        return Ok(clave);
    }
    if ruta.exists() {
        tracing::warn!("la clave de la identidad no se pudo leer: se crea otra");
    }
    let clave = SecretKey::generate();
    std::fs::create_dir_all(&carpeta)?;
    guardar_clave(&carpeta, &clave, cofre)?;
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
/// (o `clave.sellada.anterior`), por si fue un error. Devuelve la clave nueva.
pub fn importar(dir: &Path, ruta: &Path, cofre: Option<&dyn Cofre>) -> AppResult<SecretKey> {
    let clave = leer_fichero(ruta)?;
    let carpeta = dir.join("identidad");
    std::fs::create_dir_all(&carpeta)?;
    for (actual, anterior) in [(EN_CLARO, ANTERIOR), (SELLADA, SELLADA_ANTERIOR)] {
        if carpeta.join(actual).exists() {
            let _ = std::fs::copy(carpeta.join(actual), carpeta.join(anterior));
        }
    }
    guardar_clave(&carpeta, &clave, cofre)?;
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

/// Cambia solo el apodo: el nombre con el que este equipo se presenta al pasar
/// decks. Vacío, vuelve al que le da el sistema.
pub fn guardar_apodo(db: &Db, apodo: &str) -> AppResult<()> {
    let apodo = crate::pasar::protocolo::apodo_limpio(apodo);
    settings::set(db, APODO, &apodo)
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

    /// Un cofre de mentira, con sus maneras de fallar.
    #[derive(Default)]
    struct CofrePrueba {
        no_sella: bool,
        no_abre: bool,
        sella_mal: bool,
    }

    impl Cofre for CofrePrueba {
        fn sellar(&self, claro: &[u8]) -> AppResult<Vec<u8>> {
            if self.no_sella {
                return Err(AppError::new(ErrorCode::Internal));
            }
            let mut v: Vec<u8> = claro.iter().map(|b| b ^ 0x5a).collect();
            v.push(if self.sella_mal { 1 } else { 0 });
            Ok(v)
        }
        fn abrir(&self, sellado: &[u8]) -> AppResult<Vec<u8>> {
            if self.no_abre {
                return Err(AppError::new(ErrorCode::Internal));
            }
            let (ultimo, resto) = sellado
                .split_last()
                .ok_or(AppError::new(ErrorCode::Internal))?;
            let mut v: Vec<u8> = resto.iter().map(|b| b ^ 0x5a).collect();
            if *ultimo == 1 {
                v[0] ^= 1;
            }
            Ok(v)
        }
    }

    fn hay(dir: &Path, f: &str) -> bool {
        dir.join("identidad").join(f).exists()
    }

    #[test]
    fn la_clave_de_quien_ya_tenia_la_app_se_sella_y_sigue_siendo_la_misma() {
        let dir = tempfile::tempdir().unwrap();
        let antes = cargar_o_crear(dir.path(), None).unwrap();
        assert!(hay(dir.path(), EN_CLARO));
        let c = CofrePrueba::default();
        let despues = cargar_o_crear(dir.path(), Some(&c)).unwrap();
        assert_eq!(despues.public(), antes.public());
        assert!(!hay(dir.path(), EN_CLARO), "ya no queda en claro");
        assert!(hay(dir.path(), SELLADA));
        // Lo sellado no es la clave.
        let sellado = std::fs::read(dir.path().join("identidad").join(SELLADA)).unwrap();
        assert_ne!(&sellado[..32], &antes.to_bytes()[..]);
        // Y al volver a arrancar, la misma.
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&c)).unwrap().public(),
            antes.public()
        );
        // Una instalación nueva nace sellada.
        let nueva = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(nueva.path(), Some(&c)).unwrap();
        assert!(!hay(nueva.path(), EN_CLARO));
        assert_eq!(
            cargar_o_crear(nueva.path(), Some(&c)).unwrap().public(),
            k.public()
        );
    }

    #[test]
    fn si_el_cofre_falla_no_se_pierde_la_identidad() {
        let dir = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(dir.path(), None).unwrap();
        // No sella: sigue en claro.
        let c = CofrePrueba {
            no_sella: true,
            ..Default::default()
        };
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&c)).unwrap().public(),
            k.public()
        );
        assert!(hay(dir.path(), EN_CLARO));
        // Sella pero lo sellado no da lo mismo: no se borra lo que hay en claro.
        let c = CofrePrueba {
            sella_mal: true,
            ..Default::default()
        };
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&c)).unwrap().public(),
            k.public()
        );
        assert!(hay(dir.path(), EN_CLARO));
        assert!(!hay(dir.path(), SELLADA));
        // Corte entre escribir lo sellado y borrar lo de antes: se termina al volver.
        let bien = CofrePrueba::default();
        escribir_privado(
            &dir.path().join("identidad").join(SELLADA),
            &bien.sellar(&k.to_bytes()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&bien)).unwrap().public(),
            k.public()
        );
        assert!(!hay(dir.path(), EN_CLARO));
        // Lo sellado no se abre y no hay en claro: se guarda aparte, no se borra.
        let roto = CofrePrueba {
            no_abre: true,
            ..Default::default()
        };
        let otra = cargar_o_crear(dir.path(), Some(&roto)).unwrap();
        assert_ne!(otra.public(), k.public());
        assert!(hay(dir.path(), "clave.sellada.ilegible"));
    }

    #[test]
    fn lo_en_claro_distinto_de_lo_sellado_no_se_borra() {
        let dir = tempfile::tempdir().unwrap();
        let c = CofrePrueba::default();
        cargar_o_crear(dir.path(), Some(&c)).unwrap();
        let nueva = SecretKey::generate();
        escribir_privado(
            &dir.path().join("identidad").join(EN_CLARO),
            &nueva.to_bytes(),
        )
        .unwrap();
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&c)).unwrap().public(),
            nueva.public()
        );
        assert!(hay(dir.path(), EN_CLARO));
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&c)).unwrap().public(),
            nueva.public()
        );
    }

    #[test]
    fn importar_con_cofre_no_deja_nada_en_claro() {
        let dir = tempfile::tempdir().unwrap();
        let origen = tempfile::tempdir().unwrap();
        let de_fuera = cargar_o_crear(origen.path(), None).unwrap();
        let fichero = origen.path().join("id.json");
        exportar(&de_fuera, &fichero).unwrap();
        let c = CofrePrueba::default();
        let mia = cargar_o_crear(dir.path(), Some(&c)).unwrap();
        assert_eq!(
            importar(dir.path(), &fichero, Some(&c)).unwrap().public(),
            de_fuera.public()
        );
        assert!(!hay(dir.path(), EN_CLARO));
        assert_eq!(
            cargar_o_crear(dir.path(), Some(&c)).unwrap().public(),
            de_fuera.public()
        );
        // La de antes se conserva, también sellada.
        let vieja = std::fs::read(dir.path().join("identidad").join(SELLADA_ANTERIOR)).unwrap();
        assert_eq!(
            de_bytes(&c.abrir(&vieja).unwrap()).unwrap().public(),
            mia.public()
        );
        // Una `clave.anterior` en claro de antes de sellar, también entra al cofre.
        let otro = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(otro.path(), None).unwrap();
        importar(otro.path(), &fichero, None).unwrap();
        assert!(hay(otro.path(), ANTERIOR));
        cargar_o_crear(otro.path(), Some(&c)).unwrap();
        assert!(!hay(otro.path(), ANTERIOR));
        let s = std::fs::read(otro.path().join("identidad").join(SELLADA_ANTERIOR)).unwrap();
        assert_eq!(
            de_bytes(&c.abrir(&s).unwrap()).unwrap().public(),
            k.public()
        );
    }

    #[test]
    fn la_clave_se_crea_una_vez_y_se_queda() {
        let dir = tempfile::tempdir().unwrap();
        let a = cargar_o_crear(dir.path(), None).unwrap();
        let b = cargar_o_crear(dir.path(), None).unwrap();
        assert_eq!(a.public(), b.public());
        let otra = tempfile::tempdir().unwrap();
        assert_ne!(
            cargar_o_crear(otra.path(), None).unwrap().public(),
            a.public()
        );
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
        let a = cargar_o_crear(origen.path(), None).unwrap();
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
        let b = cargar_o_crear(destino.path(), None).unwrap();
        assert_ne!(a.public(), b.public());
        let nueva = importar(destino.path(), &fichero, None).unwrap();
        assert_eq!(nueva.public(), a.public());
        // Queda guardada: al volver a arrancar es la misma, y la vieja se conserva.
        assert_eq!(
            cargar_o_crear(destino.path(), None).unwrap().public(),
            a.public()
        );
        let vieja = std::fs::read(destino.path().join("identidad/clave.anterior")).unwrap();
        assert_eq!(
            SecretKey::from_bytes(&vieja.try_into().unwrap()).public(),
            b.public()
        );
    }

    #[test]
    fn un_fichero_que_no_es_una_identidad_no_cambia_nada() {
        let dir = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(dir.path(), None).unwrap();
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
            assert!(importar(dir.path(), &f, None).is_err(), "malo {i}");
        }
        assert_eq!(
            cargar_o_crear(dir.path(), None).unwrap().public(),
            k.public()
        );
        assert!(importar(dir.path(), &dir.path().join("no-existe.json"), None).is_err());
    }

    #[test]
    fn la_clave_firma_para_el_reto_del_servidor() {
        let dir = tempfile::tempdir().unwrap();
        let k = cargar_o_crear(dir.path(), None).unwrap();
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
