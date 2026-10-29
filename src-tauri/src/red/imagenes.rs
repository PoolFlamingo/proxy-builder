//! **Las imágenes de la partida en red** (E8, E9).
//!
//! - **Propias** (cartas subidas, retocadas o diseñadas, tapetes y reversos
//!   propios): viajan por la red, pedidas por su huella SHA-256. Cada
//!   invitado se las pide al anfitrión por su conexión (un stream por
//!   imagen); el anfitrión, si no la tiene, se la pide a su dueño y se la
//!   queda. Así nadie necesita la dirección de nadie más que la del
//!   anfitrión (la malla entre todos llega con la Fase F).
//! - **De Scryfall**: no viajan; cada uno las baja de `cards.scryfall.io`
//!   (la `url` de la carta, comprobada por `fuentes::Externo`).
//!
//! Se piden **cuando la pantalla las pide** (`pbimg://…/m/`, por el
//! manejador del protocolo): lo que está a la vista llega primero.
//!
//! Al llegar, se comprueba el tope, la huella y se sanea (se decodifica y se
//! vuelve a codificar con `limpiar_imagen_con`). La saneada se guarda donde
//! la propia (`mesa/<2 hex>/<sha256>.jpg`, la que ve la pantalla) y los
//! bytes recibidos en `mesa/r/` (los que dan la huella, para pasárselos a
//! otro). Todo es caché: si se borra, se vuelve a pedir.
//!
//! Solo se sirven imágenes de la partida en curso: quien conoce el nodo no
//! puede pedir otra cosa.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use iroh::endpoint::{Connection, RecvStream, SendStream};
use mesa::modelo::{OrigenCarta, Partida, RefImagen};
use mesa::vista::VistaMesa;
use mesa::IdJugador;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::anfitrion::Anfitrion;
use super::mesa::Mesa;
use super::nodo::de_red;
use super::protocolo::{escribir_trama, leer_trama};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::mesa::imagenes::{propia_en, scryfall_en};
use crate::store::blobs::{es_huella, escribir_atomico};

/// Lo más que puede pesar una carta y un tapete al llegar.
pub const TOPE_CARTA: usize = 4 * 1024 * 1024;
pub const TOPE_TAPETE: usize = 8 * 1024 * 1024;
/// La medida máxima tras sanear.
const CARTA_MAX: (u32, u32) = (745, 1040);
const TAPETE_MAX: (u32, u32) = (2560, 1600);
/// Lo que se espera a una imagen antes de rendirse.
const ESPERA: Duration = Duration::from_secs(30);
/// Descargas de Scryfall a la vez.
const A_LA_VEZ: usize = 4;

/// La petición de una imagen (una trama con su JSON).
#[derive(Debug, Serialize, Deserialize)]
struct Pide {
    sha256: String,
}

/// Lo que se sabe de una imagen propia en la partida: de quién es y si es
/// un tapete (otro tope y otra medida).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeQuien {
    pub dueno: IdJugador,
    pub tapete: bool,
}

fn caras(o: &OrigenCarta) -> impl Iterator<Item = &RefImagen> {
    o.caras.iter().chain(std::iter::once(&o.dorso))
}

fn es_propia(r: &RefImagen, sha: &str) -> bool {
    matches!(r, RefImagen::Propia { sha256, .. } if sha256 == sha)
}

fn url_scryfall<'a>(r: &'a RefImagen, id: &str, cara: u8) -> Option<&'a str> {
    match r {
        RefImagen::Scryfall {
            id: i,
            cara: c,
            url,
        } if i == id && *c == cara => Some(url),
        _ => None,
    }
}

/// De quién es la imagen propia `sha` en la partida (o `None`: no está).
pub fn en_partida(p: &Partida, sha: &str) -> Option<DeQuien> {
    for j in &p.jugadores {
        if j.tapete.as_ref().is_some_and(|t| es_propia(t, sha)) {
            return Some(DeQuien {
                dueno: j.id,
                tapete: true,
            });
        }
        if es_propia(&j.dorso, sha) || j.fichas.iter().flat_map(caras).any(|r| es_propia(r, sha)) {
            return Some(DeQuien {
                dueno: j.id,
                tapete: false,
            });
        }
    }
    p.cartas
        .values()
        .find(|c| caras(&c.origen).any(|r| es_propia(r, sha)))
        .map(|c| DeQuien {
            dueno: c.dueno,
            tapete: false,
        })
}

/// Lo mismo en la vista de un espectador (solo lo que ve).
pub fn en_vista(v: &VistaMesa, sha: &str) -> Option<DeQuien> {
    for j in &v.jugadores {
        if j.tapete.as_ref().is_some_and(|t| es_propia(t, sha)) {
            return Some(DeQuien {
                dueno: j.id,
                tapete: true,
            });
        }
        if es_propia(&j.dorso, sha) {
            return Some(DeQuien {
                dueno: j.id,
                tapete: false,
            });
        }
    }
    v.cartas
        .iter()
        .find(|c| {
            es_propia(&c.dorso, sha)
                || c.origen
                    .as_ref()
                    .is_some_and(|o| caras(o).any(|r| es_propia(r, sha)))
        })
        .map(|c| DeQuien {
            dueno: c.dueno,
            tapete: false,
        })
}

/// La `url` de una imagen de Scryfall de la partida.
pub fn url_en_partida(p: &Partida, id: &str, cara: u8) -> Option<String> {
    p.jugadores
        .iter()
        .flat_map(|j| {
            j.fichas
                .iter()
                .flat_map(caras)
                .chain(std::iter::once(&j.dorso))
        })
        .chain(p.cartas.values().flat_map(|c| caras(&c.origen)))
        .find_map(|r| url_scryfall(r, id, cara))
        .map(str::to_string)
}

pub fn url_en_vista(v: &VistaMesa, id: &str, cara: u8) -> Option<String> {
    v.cartas
        .iter()
        .flat_map(|c| {
            c.origen
                .iter()
                .flat_map(caras)
                .chain(std::iter::once(&c.dorso))
        })
        .chain(v.jugadores.iter().map(|j| &j.dorso))
        .find_map(|r| url_scryfall(r, id, cara))
        .map(str::to_string)
}

/// De quién es una imagen propia en lo que esta app tiene abierto.
pub fn buscar<M: Mesa>(mesa: &M, sha: &str) -> Option<DeQuien> {
    // La de ahora y, en una repetición, la del punto que se mira (una ficha que
    // ya no está).
    mesa.con(|s| {
        s.as_ref()
            .and_then(|s| en_partida(s.partida(), sha).or_else(|| en_partida(s.en_pantalla(), sha)))
    })
    .or_else(|| mesa.mirada(|m| m.as_ref().and_then(|m| en_vista(&m.vista, sha))))
}

fn buscar_url<M: Mesa>(mesa: &M, id: &str, cara: u8) -> Option<String> {
    mesa.con(|s| {
        s.as_ref().and_then(|s| {
            url_en_partida(s.partida(), id, cara)
                .or_else(|| url_en_partida(s.en_pantalla(), id, cara))
        })
    })
    .or_else(|| mesa.mirada(|m| m.as_ref().and_then(|m| url_en_vista(&m.vista, id, cara))))
}

/// Los bytes recibidos (los que dan la huella) de una imagen ajena.
pub fn ruta_cruda(carpeta: &Path, sha: &str) -> PathBuf {
    let dos = sha.get(..2).unwrap_or("00");
    carpeta.join("r").join(dos).join(format!("{sha}.jpg"))
}

fn huella(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// Los bytes con la huella `sha`, si esta app los tiene: los recibidos o,
/// si es suya, la que preparó.
fn crudos(carpeta: &Path, sha: &str) -> Option<Vec<u8>> {
    if let Ok(b) = std::fs::read(ruta_cruda(carpeta, sha)) {
        return Some(b);
    }
    std::fs::read(propia_en(carpeta, sha))
        .ok()
        .filter(|b| huella(b) == sha)
}

/// Si la imagen ya se puede enseñar.
pub fn la_tengo(carpeta: &Path, sha: &str) -> bool {
    propia_en(carpeta, sha).exists()
}

/// Un cerrojo por imagen y carpeta: dos que piden la misma esperan a la
/// primera (con la carpeta, varias mesas en un proceso no se esperan entre
/// ellas: el anfitrión trae la que le pide un invitado).
fn cerrojo_de(carpeta: &Path, clave: &str) -> Arc<tokio::sync::Mutex<()>> {
    static EN_CAMINO: OnceLock<Mutex<BTreeMap<String, Arc<tokio::sync::Mutex<()>>>>> =
        OnceLock::new();
    let m = EN_CAMINO.get_or_init(Mutex::default);
    let mut g = match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    // Los que nadie espera ya, fuera.
    g.retain(|_, c| Arc::strong_count(c) > 1);
    g.entry(format!("{}|{clave}", carpeta.display()))
        .or_default()
        .clone()
}

/// Sanea y guarda una imagen recibida con la huella comprobada.
async fn guardar(carpeta: &Path, sha: &str, bytes: Vec<u8>, tapete: bool) -> AppResult<()> {
    let (w, h) = if tapete { TAPETE_MAX } else { CARTA_MAX };
    let carpeta = carpeta.to_path_buf();
    let sha = sha.to_string();
    tokio::task::spawn_blocking(move || {
        let limpia = crate::fuentes::limpiar_imagen_con(&bytes, w, h)?;
        escribir_atomico(&ruta_cruda(&carpeta, &sha), &bytes)?;
        escribir_atomico(&propia_en(&carpeta, &sha), &limpia)
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Internal))?
}

/// Pide la imagen `sha` por la conexión `conn` y la guarda.
pub async fn traer(conn: &Connection, carpeta: &Path, sha: &str, tapete: bool) -> AppResult<()> {
    if !es_huella(sha) {
        return Err(AppError::new(ErrorCode::InvalidInput));
    }
    let cerrojo = cerrojo_de(carpeta, sha);
    let _g = cerrojo.lock().await;
    if la_tengo(carpeta, sha) {
        return Ok(());
    }
    let tope = if tapete { TOPE_TAPETE } else { TOPE_CARTA };
    let bytes = tokio::time::timeout(ESPERA, async {
        let (mut tx, mut rx) = conn.open_bi().await.map_err(de_red)?;
        let pide = serde_json::to_vec(&Pide {
            sha256: sha.to_string(),
        })
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
        escribir_trama(&mut tx, &pide, 256).await.map_err(de_red)?;
        let _ = tx.finish();
        leer_trama(&mut rx, tope).await.map_err(de_red)
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Network).with("motivo", "sin_respuesta"))??;
    if bytes.is_empty() {
        return Err(AppError::new(ErrorCode::NotFound).with("imagen", sha));
    }
    if huella(&bytes) != sha {
        tracing::warn!(sha, "una imagen llegó con otra huella: se tira");
        return Err(AppError::new(ErrorCode::Image).with("motivo", "huella"));
    }
    guardar(carpeta, sha, bytes, tapete).await
}

/// Atiende un stream de imagen: lee qué se pide y lo manda (o un vacío si
/// no la tiene o no es de la partida). En el anfitrión, lo que no tiene se
/// lo pide antes a su dueño.
async fn servir_una<M: Mesa>(
    mut tx: SendStream,
    mut rx: RecvStream,
    mesa: M,
    anf: Option<Arc<Anfitrion>>,
) -> AppResult<()> {
    let pide = tokio::time::timeout(Duration::from_secs(5), leer_trama(&mut rx, 256))
        .await
        .map_err(|_| AppError::new(ErrorCode::Network))?
        .map_err(de_red)?;
    let Pide { sha256: sha } =
        serde_json::from_slice(&pide).map_err(|_| AppError::new(ErrorCode::InvalidInput))?;
    let carpeta = mesa.carpeta();
    let mut bytes = Vec::new();
    if es_huella(&sha) {
        if let Some(de) = buscar(&mesa, &sha) {
            if crudos(&carpeta, &sha).is_none() {
                if let Some(conn) = anf.as_ref().and_then(|a| a.conexion_de(de.dueno)) {
                    if let Err(e) = traer(&conn, &carpeta, &sha, de.tapete).await {
                        tracing::info!(codigo = ?e.code, "no se pudo traer una imagen de su dueño");
                    }
                }
            }
            bytes = crudos(&carpeta, &sha).unwrap_or_default();
        }
    }
    escribir_trama(&mut tx, &bytes, TOPE_TAPETE)
        .await
        .map_err(de_red)?;
    let _ = tx.finish();
    // Que lleguen antes de soltar el stream.
    let _ = tx.stopped().await;
    Ok(())
}

/// Atiende los streams de imágenes que abra el otro lado de `conn`.
pub async fn servir_conexion<M: Mesa>(conn: Connection, mesa: M, anf: Option<Arc<Anfitrion>>) {
    while let Ok((tx, rx)) = conn.accept_bi().await {
        let mesa = mesa.clone();
        let anf = anf.clone();
        tokio::spawn(async move {
            if let Err(e) = servir_una(tx, rx, mesa, anf).await {
                tracing::info!(codigo = ?e.code, "no se pudo servir una imagen");
            }
        });
    }
}

/// Quien conecta directamente por `proxy-builder/img/1` (Fase F: la malla).
pub async fn servir<M: Mesa>(conn: Connection, mesa: M) {
    servir_conexion(conn, mesa, None).await;
}

/// Una imagen de Scryfall de la partida: se baja de su `url`.
pub async fn bajar_scryfall<M: Mesa>(mesa: &M, id: &str, cara: u8) -> AppResult<()> {
    static PERMISOS: OnceLock<tokio::sync::Semaphore> = OnceLock::new();
    let carpeta = mesa.carpeta();
    let ruta = scryfall_en(&carpeta, id, cara);
    let cerrojo = cerrojo_de(&carpeta, &format!("s-{id}-{cara}"));
    let _g = cerrojo.lock().await;
    if ruta.exists() {
        return Ok(());
    }
    let url = buscar_url(mesa, id, cara).ok_or_else(|| AppError::new(ErrorCode::NotFound))?;
    let _permiso = PERMISOS
        .get_or_init(|| tokio::sync::Semaphore::new(A_LA_VEZ))
        .acquire()
        .await
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
    let bytes = crate::fuentes::Externo::nuevo()?
        .bajar(&url, TOPE_CARTA)
        .await?;
    tokio::task::spawn_blocking(move || {
        let limpia = crate::fuentes::limpiar_imagen_con(&bytes, CARTA_MAX.0, CARTA_MAX.1)?;
        escribir_atomico(&ruta, &limpia)
    })
    .await
    .map_err(|_| AppError::new(ErrorCode::Internal))?
}

/// Las imágenes propias de una partida (de todos los jugadores): caras,
/// fichas, reversos y tapetes, cada una con si es un tapete.
pub fn propias_de(p: &Partida) -> Vec<(String, bool)> {
    let mut out: BTreeMap<String, bool> = BTreeMap::new();
    let mut meter = |r: &RefImagen, tapete: bool| {
        if let RefImagen::Propia { sha256, .. } = r {
            out.entry(sha256.clone()).or_insert(tapete);
        }
    };
    for j in &p.jugadores {
        if let Some(t) = &j.tapete {
            meter(t, true);
        }
        meter(&j.dorso, false);
        for f in &j.fichas {
            caras(f).for_each(|r| meter(r, false));
        }
    }
    for c in p.cartas.values() {
        caras(&c.origen).for_each(|r| meter(r, false));
    }
    out.into_iter().collect()
}

/// En el invitado: trae de antemano, sin prisa y de pocas en pocas, todas las
/// imágenes propias de la partida que aún no tiene. Así las cartas de los
/// demás ya están cuando las juegan (antes llegaban al pedirlas la pantalla,
/// con la carta vacía un rato). Espera a tener la partida; lo que falle se
/// vuelve a pedir cuando la pantalla lo pida.
pub async fn precargar_invitado<M: Mesa>(mesa: M, conn: Connection) {
    let mut propias = Vec::new();
    for _ in 0..60 {
        propias = mesa.con(|s| {
            s.as_ref()
                .map(|s| propias_de(s.partida()))
                .unwrap_or_default()
        });
        if !propias.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let carpeta = mesa.carpeta();
    let faltan: Vec<(String, bool)> = propias
        .into_iter()
        .filter(|(sha, _)| !la_tengo(&carpeta, sha))
        .collect();
    use futures::StreamExt;
    futures::stream::iter(faltan)
        .for_each_concurrent(A_LA_VEZ, |(sha, tapete)| {
            let conn = conn.clone();
            let carpeta = carpeta.clone();
            async move {
                let _ = traer(&conn, &carpeta, &sha, tapete).await;
            }
        })
        .await;
}

/// Lo que hace falta para traer una imagen propia: la conexión por la que
/// pedirla.
pub enum Por {
    /// Esta app es el anfitrión: a su dueño.
    Anfitrion(Arc<Anfitrion>),
    /// Esta app es invitada: al anfitrión.
    Invitado(Connection),
}

/// Trae una imagen propia de la partida que la pantalla quiere ver.
pub async fn traer_propia<M: Mesa>(mesa: &M, por: Por, sha: &str) -> AppResult<()> {
    let de = buscar(mesa, sha).ok_or_else(|| AppError::new(ErrorCode::NotFound))?;
    let conn = match por {
        Por::Anfitrion(a) => a
            .conexion_de(de.dueno)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound))?,
        Por::Invitado(c) => c,
    };
    traer(&conn, &mesa.carpeta(), sha, de.tapete).await
}

/// La pantalla pide una imagen de la mesa que no está: la de Scryfall se
/// baja de su `url` siempre; la propia, por la red si hay partida en red.
pub async fn asegurar<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    img: &crate::mesa::imagenes::ImagenMesa,
) -> AppResult<()> {
    use tauri::Manager;
    let mesa = super::mesa::MesaDeApp(app.clone());
    // Las de Scryfall no pasan por nadie: también sin sala (una repetición
    // del historial, una caché borrada).
    if let crate::mesa::imagenes::ImagenMesa::Scryfall(id, cara) = img {
        return bajar_scryfall(&mesa, id, *cara).await;
    }
    let estado = app.state::<crate::state::AppState>();
    let por = estado.red.con_sala(|s| match s {
        Some(super::Sala::Anfitrion { anf, .. }) => Some(Por::Anfitrion(anf.clone())),
        Some(super::Sala::Invitado { conexion, .. }) => conexion.conexion().map(Por::Invitado),
        None => None,
    });
    let Some(por) = por else {
        return Err(AppError::new(ErrorCode::NotFound));
    };
    match img {
        crate::mesa::imagenes::ImagenMesa::Propia(sha) => traer_propia(&mesa, por, sha).await,
        crate::mesa::imagenes::ImagenMesa::Scryfall(..) => Ok(()),
    }
}
