//! **Paquetes `.pbdeck`**: un deck con todas sus imágenes en un ZIP, para
//! llevárselo a otro equipo.
//!
//! - `manifest.json`: `{ format: "proxy-builder-deck", version: 2, deck,
//!   colecciones }`.
//! - `blobs/<sha256>.<ext>`: cada imagen que usa el deck (frentes, reversos,
//!   reverso por defecto y, desde la 2, arte, emblemas e imágenes de «antes»
//!   de los diseños), una vez.
//!
//! La versión 2 lleva diseños del editor de cartas y las colecciones que
//! usan; una app 1.x la rechaza (`version > VERSION`). Las letras y los packs
//! importados no viajan: en el otro equipo se usan los de la casa.
//!
//! Desde la 3.3, `biblioteca` lleva también el **linaje** del deck y la hora de
//! su versión (`store::linaje`) y la portada elegida, y un paquete se puede
//! escribir y leer **sin proxies** (`proxy_core::compartir`). Los campos son
//! opcionales: un paquete viejo se importa como un deck nuevo, y una app vieja
//! ignora los nuevos (`package::VERSION` sigue en 2).
//!
//! 🛑 Al importar no se confía en nada: se ignoran las entradas con rutas
//! raras (zip-slip), cada imagen tiene que dar la huella que dice su nombre y
//! el deck nace con ids nuevos.

use std::collections::BTreeSet;
use std::io::{Read, Seek, Write};
use std::path::Path;

use proxy_core::diseno::modelo::Collection;
use proxy_core::model::{BackSpec, Deck};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::blobs::{es_huella, escribir_atomico, BlobStore};
use crate::store::db::Db;
use crate::store::{colecciones, decks, etiquetas, linaje, listas};
use mesa::lista::ListaDeJuego;

pub const FORMATO: &str = "proxy-builder-deck";
pub const VERSION: u32 = 2;
/// Tope por entrada al descomprimir (una imagen de 12.000 px en PNG cabe de sobra).
const MAX_ENTRADA: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Manifiesto {
    format: String,
    version: u32,
    deck: Deck,
    /// Las colecciones que usan los diseños del deck.
    #[serde(default)]
    colecciones: Vec<Collection>,
    /// Desde la 2.2: etiquetas (nombre y color) y notas del deck. Una app
    /// anterior lo ignora.
    #[serde(default)]
    biblioteca: Option<Biblioteca>,
}

#[derive(Serialize, Deserialize, Default)]
struct Biblioteca {
    #[serde(default)]
    etiquetas: Vec<EtiquetaPaquete>,
    #[serde(default)]
    notas: String,
    /// El idioma del deck elegido a mano (desde la 2.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    idioma: Option<String>,
    /// Las listas para jugar (mesa de juego), con los ids de copias del deck.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    listas: Vec<ListaDeJuego>,
    /// El linaje del deck (`store::linaje`): el mismo en todas sus copias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    linaje: Option<String>,
    /// La hora (ms) en que se cambió por última vez el contenido de esta
    /// versión, en el equipo donde se cambió.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actualizado: Option<i64>,
    /// La carta elegida como portada (su id en el deck del paquete).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    portada: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct EtiquetaPaquete {
    nombre: String,
    color: String,
    /// Desde la 3.2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icono: Option<String>,
}

fn mal(motivo: &str) -> AppError {
    AppError::new(ErrorCode::InvalidInput).with("motivo", motivo)
}

fn de_zip(e: zip::result::ZipError) -> AppError {
    match e {
        zip::result::ZipError::Io(e) => e.into(),
        otro => mal("zip").with("detalle", otro),
    }
}

/// Las huellas de todas las imágenes que usa un deck.
pub fn blobs_del_deck(deck: &Deck) -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    if let Some(b) = &deck.default_back {
        s.insert(b.blob.clone());
    }
    for c in &deck.cards {
        if let Some(f) = &c.front {
            s.insert(f.blob.clone());
        }
        if let BackSpec::Dfc { image } | BackSpec::Custom { image } = &c.back {
            s.insert(image.blob.clone());
        }
        if let Some(d) = &c.design {
            s.extend(d.blobs().into_iter().map(str::to_string));
        }
    }
    s
}

/// Las colecciones que usan los diseños de un deck.
fn colecciones_del_deck(deck: &Deck, db: &Db) -> AppResult<Vec<Collection>> {
    let usadas: BTreeSet<&str> = deck
        .cards
        .iter()
        .filter_map(|c| c.design.as_ref()?.collection.as_deref())
        .collect();
    if usadas.is_empty() {
        return Ok(Vec::new());
    }
    Ok(colecciones::list(db)?
        .into_iter()
        .filter(|c| usadas.contains(c.id.as_str()))
        .collect())
}

fn escribir<W: Write + Seek>(w: W, deck: &Deck, blobs: &BlobStore, db: &Db) -> AppResult<W> {
    let mut zip = ZipWriter::new(w);
    let cols = colecciones_del_deck(deck, db)?;
    let mut todas = blobs_del_deck(deck);
    todas.extend(
        cols.iter()
            .filter_map(|c| c.emblem.as_ref().map(|e| e.blob.clone())),
    );
    let lin = linaje::de(db, &deck.id)?;
    let biblioteca = Biblioteca {
        etiquetas: etiquetas::de_deck(db, &deck.id)?
            .into_iter()
            .map(|t| EtiquetaPaquete {
                nombre: t.name,
                color: t.color,
                icono: t.icon,
            })
            .collect(),
        notas: decks::notas_de(db, &deck.id)?,
        idioma: decks::idioma_de(db, &deck.id)?,
        listas: listas::de_deck(db, &deck.id)?,
        linaje: Some(lin.linaje),
        actualizado: Some(lin.actualizado),
        portada: decks::portada_de(db, &deck.id)?,
    };
    let json = serde_json::to_vec_pretty(&Manifiesto {
        format: FORMATO.into(),
        version: VERSION,
        deck: deck.clone(),
        colecciones: cols,
        biblioteca: Some(biblioteca),
    })
    .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    // Fecha fija: el mismo deck da el mismo fichero.
    let fecha = zip::DateTime::default();
    let deflate = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(fecha);
    let guardado = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(fecha);
    zip.start_file("manifest.json", deflate).map_err(de_zip)?;
    zip.write_all(&json)?;
    for hash in todas {
        let (ruta, _) = blobs
            .locate(db, &hash)?
            .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("blob", &hash))?;
        let ext = ruta.extension().and_then(|e| e.to_str()).unwrap_or("bin");
        zip.start_file(format!("blobs/{hash}.{ext}"), guardado)
            .map_err(de_zip)?;
        zip.write_all(&std::fs::read(&ruta)?)?;
    }
    zip.finish().map_err(de_zip)
}

/// Escribe el paquete de un deck (de una vez: nunca queda uno a medias).
pub fn export(deck: &Deck, destino: &Path, blobs: &BlobStore, db: &Db) -> AppResult<u64> {
    export_con(deck, destino, blobs, db, false)
}

/// Lo mismo, con o sin proxies (`proxy_core::compartir::sin_proxies`): sin
/// ellos, el paquete lleva solo lo que se puede volver a bajar.
pub fn export_con(
    deck: &Deck,
    destino: &Path,
    blobs: &BlobStore,
    db: &Db,
    sin_proxies: bool,
) -> AppResult<u64> {
    let deck = segun(deck, sin_proxies);
    let buf = escribir(std::io::Cursor::new(Vec::new()), &deck, blobs, db)?.into_inner();
    escribir_atomico(destino, &buf)?;
    Ok(buf.len() as u64)
}

/// Escribe el paquete directamente en un fichero (para enviarlo: sin tenerlo
/// entero en memoria, una imagen cada vez). El que llama lo borra después.
pub fn export_en_fichero(
    deck: &Deck,
    destino: &Path,
    blobs: &BlobStore,
    db: &Db,
    sin_proxies: bool,
) -> AppResult<u64> {
    let deck = segun(deck, sin_proxies);
    let f = std::fs::File::create(destino)?;
    let mut w = escribir(std::io::BufWriter::new(f), &deck, blobs, db)?;
    w.flush()?;
    Ok(std::fs::metadata(destino)?.len())
}

fn segun(deck: &Deck, sin: bool) -> Deck {
    if sin {
        proxy_core::compartir::sin_proxies(deck)
    } else {
        deck.clone()
    }
}

/// Lo que pesaría el paquete del deck (la suma de sus imágenes, sin
/// comprimir: las imágenes ya vienen comprimidas), con o sin proxies. Para la
/// oferta, sin construir nada.
pub fn tamano_estimado(deck: &Deck, blobs: &BlobStore, db: &Db, sin_proxies: bool) -> u64 {
    let deck = segun(deck, sin_proxies);
    let mut todas = blobs_del_deck(&deck);
    if let Ok(cols) = colecciones_del_deck(&deck, db) {
        todas.extend(
            cols.iter()
                .filter_map(|c| c.emblem.as_ref().map(|e| e.blob.clone())),
        );
    }
    let mut total = 16 * 1024;
    for h in todas {
        if let Ok(Some((ruta, _))) = blobs.locate(db, &h) {
            total += std::fs::metadata(ruta).map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

fn leer_entrada<R: Read + Seek>(zip: &mut ZipArchive<R>, i: usize) -> AppResult<(String, Vec<u8>)> {
    let mut f = zip.by_index(i).map_err(de_zip)?;
    // `enclosed_name` rechaza `..` y rutas absolutas.
    let nombre = f
        .enclosed_name()
        .and_then(|p| p.to_str().map(|s| s.replace('\\', "/")))
        .ok_or_else(|| mal("ruta"))?;
    if f.size() > MAX_ENTRADA {
        return Err(mal("tamano"));
    }
    let mut v = Vec::with_capacity(f.size() as usize);
    (&mut f).take(MAX_ENTRADA + 1).read_to_end(&mut v)?;
    if v.len() as u64 > MAX_ENTRADA {
        return Err(mal("tamano"));
    }
    Ok((nombre, v))
}

/// Qué se hace con el deck del paquete en esta biblioteca.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Destino {
    /// Un deck nuevo que conserva el linaje del que vino (si traía uno).
    #[default]
    Nuevo,
    /// «Guardar los dos»: un deck nuevo y aparte (otro linaje, como un
    /// duplicado): ya no es el mismo deck que el de allí.
    GuardarLosDos,
    /// «Reemplazar el de aquí»: este deck (su id).
    Reemplazar(String),
}

/// Cómo importar un paquete.
#[derive(Debug, Clone, Default)]
pub struct Opciones {
    /// Quedarse solo con las cartas originales (`compartir::sin_proxies`):
    /// no se guardan ni las imágenes propias ni las colecciones.
    pub solo_originales: bool,
    pub destino: Destino,
}

/// Importa un paquete y devuelve el deck nuevo (ya guardado).
pub fn import<R: Read + Seek>(r: R, blobs: &BlobStore, db: &Db) -> AppResult<Deck> {
    import_con(r, blobs, db, &Opciones::default())
}

/// Un linaje de fuera solo vale si es corto y de caracteres de un id.
fn linaje_sano(l: &str) -> bool {
    !l.is_empty()
        && l.len() <= 64
        && l.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Lo que el paquete dice de sí mismo, sin tocar la biblioteca.
struct Leido {
    manifiesto: Manifiesto,
    imagenes: Vec<(String, Vec<u8>)>,
}

fn leer_paquete<R: Read + Seek>(r: R) -> AppResult<Leido> {
    let mut zip = ZipArchive::new(r).map_err(de_zip)?;
    let mut manifiesto: Option<Manifiesto> = None;
    let mut imagenes: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..zip.len() {
        let (nombre, bytes) = leer_entrada(&mut zip, i)?;
        if nombre == "manifest.json" {
            manifiesto = Some(serde_json::from_slice(&bytes).map_err(|_| mal("manifiesto"))?);
        } else if let Some(resto) = nombre.strip_prefix("blobs/") {
            let hash = resto.split('.').next().unwrap_or_default();
            if !es_huella(hash) || resto.contains('/') {
                return Err(mal("ruta"));
            }
            let real: String = Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            if real != hash {
                return Err(mal("huella").with("blob", hash));
            }
            imagenes.push((hash.to_string(), bytes));
        }
        // Cualquier otra entrada se ignora.
    }
    let m = manifiesto.ok_or_else(|| mal("manifiesto"))?;
    if m.format != FORMATO {
        return Err(mal("formato"));
    }
    if m.version > VERSION {
        return Err(AppError::new(ErrorCode::Unsupported).with("version", m.version));
    }
    Ok(Leido {
        manifiesto: m,
        imagenes,
    })
}

/// Importa un paquete con estas opciones y devuelve el deck guardado.
pub fn import_con<R: Read + Seek>(
    r: R,
    blobs: &BlobStore,
    db: &Db,
    op: &Opciones,
) -> AppResult<Deck> {
    let Leido {
        manifiesto: m,
        imagenes,
    } = leer_paquete(r)?;
    // El deck que se va a reemplazar tiene que existir antes de tocar nada.
    let local = match &op.destino {
        Destino::Reemplazar(id) => Some(decks::open(db, id)?),
        _ => None,
    };
    let mut deck = m.deck;
    let mut cols = m.colecciones;
    if op.solo_originales {
        deck = proxy_core::compartir::sin_proxies(&deck);
        cols.clear();
        if deck.default_back.is_none() {
            deck.default_back = Some(blobs.generic_back(db)?);
        }
    }
    // Solo se guardan las imágenes que el deck usa (y los emblemas de sus
    // colecciones).
    let mut necesarias = blobs_del_deck(&deck);
    necesarias.extend(
        cols.iter()
            .filter_map(|c| c.emblem.as_ref().map(|e| e.blob.clone())),
    );
    let tenemos: BTreeSet<&str> = imagenes.iter().map(|(h, _)| h.as_str()).collect();
    if let Some(falta) = necesarias
        .iter()
        .find(|h| !tenemos.contains(h.as_str()) && blobs.locate(db, h).ok().flatten().is_none())
    {
        return Err(mal("falta_imagen").with("blob", falta));
    }

    for (h, bytes) in &imagenes {
        if necesarias.contains(h) {
            let origen = deck_origen(&deck, bytes);
            blobs.put_bytes(db, bytes, origen)?;
        }
    }

    // Las colecciones: si ya hay una con el mismo id, se queda la de este
    // equipo.
    let ya: BTreeSet<String> = colecciones::list(db)?.into_iter().map(|c| c.id).collect();
    for col in cols {
        if !ya.contains(&col.id) {
            colecciones::save(db, &col)?;
        }
    }

    // Ids nuevos para las copias; las listas para jugar los siguen.
    let mut nuevos = std::collections::BTreeMap::new();
    for c in &mut deck.cards {
        let id = uuid::Uuid::new_v4().to_string();
        nuevos.insert(std::mem::replace(&mut c.id, id.clone()), id);
    }
    match &local {
        // Reemplazar: el id y el nombre son de este equipo.
        Some(l) => {
            deck.id = l.id.clone();
            deck.name = l.name.clone();
        }
        None => {
            deck.id = uuid::Uuid::new_v4().to_string();
            let ya = decks::list(db)?;
            if ya.iter().any(|d| d.name == deck.name) {
                deck.name = format!("{} (importado)", deck.name);
            }
        }
    }
    // Al reemplazar, lo de aquí antes de tocarlo: si algo de lo que sigue
    // falla, se vuelve a poner tal cual (cada paso pide la conexión por su
    // cuenta, así que no cabe en una sola transacción).
    let copia = match &local {
        Some(l) => Some(Copia::de(db, l)?),
        None => None,
    };
    decks::save(db, &deck)?;
    let nuevo = local.is_none();
    // Lo que sigue, si falla, no deja un deck a medias: el nuevo se borra y
    // el reemplazado vuelve a como estaba.
    let resto = (|| -> AppResult<()> {
        let b = m.biblioteca.unwrap_or_default();
        let cambiar = |ids: &[String]| -> Vec<String> {
            ids.iter().filter_map(|i| nuevos.get(i).cloned()).collect()
        };
        // El linaje.
        let propio = linaje::de(db, &deck.id)?;
        let lin = match (&op.destino, b.linaje.as_deref().filter(|l| linaje_sano(l))) {
            (Destino::Nuevo, Some(l)) => l.to_string(),
            // El mismo deck de aquí, o uno aparte: el suyo.
            _ => propio.linaje,
        };
        let hora = b.actualizado.unwrap_or(propio.actualizado);
        linaje::poner(db, &deck.id, &lin, hora)?;
        // Las listas para jugar: las de aquí apuntaban a ids de carta que ya
        // no existen, así que se toman las del paquete.
        if !nuevo {
            for l in listas::de_deck(db, &deck.id)? {
                listas::borrar(db, &deck.id, &l.id)?;
            }
        }
        for l in &b.listas {
            // Una lista que no se puede guardar (sin nombre, hecha a mano)
            // no deja el deck a medias: se salta.
            let _ = listas::guardar(
                db,
                &deck.id,
                &ListaDeJuego {
                    id: String::new(),
                    nombre: l.nombre.clone(),
                    comandantes: cambiar(&l.comandantes),
                    cartas: cambiar(&l.cartas),
                },
            );
        }
        // La portada: la que trae (con el id nuevo) o ninguna.
        let portada = b.portada.as_deref().and_then(|p| nuevos.get(p));
        etiquetas::portada(db, &deck.id, portada.map(String::as_str))?;
        if !nuevo {
            // Etiquetas, favorito, notas, idioma e historial son de este
            // equipo: no se tocan.
            return Ok(());
        }
        // Las etiquetas: las de este equipo con el mismo nombre, o nuevas.
        let mut ids = Vec::new();
        for e in &b.etiquetas {
            if let Ok(t) = etiquetas::create(db, &e.nombre, &e.color) {
                // La de este equipo manda; solo se le pone si no tenía.
                if t.icon.is_none() {
                    if let Some(i) = &e.icono {
                        let _ = etiquetas::icono(db, &t.id, Some(i));
                    }
                }
                ids.push(t.id);
            }
        }
        etiquetas::etiquetar(db, std::slice::from_ref(&deck.id), &ids, &[])?;
        if !b.notas.is_empty() {
            etiquetas::notas(db, &deck.id, &b.notas)?;
        }
        // Un idioma que este equipo no conoce se ignora.
        if let Some(l) = b.idioma.as_deref() {
            let _ = etiquetas::idioma(db, std::slice::from_ref(&deck.id), Some(l));
        }
        Ok(())
    })();
    if let Err(e) = resto {
        if nuevo {
            let _ = decks::delete(db, &deck.id);
        }
        if let Some(c) = copia {
            if let Err(otro) = c.restaurar(db) {
                tracing::error!(codigo = ?otro.code, "importar: no se pudo restaurar el deck reemplazado");
            }
        }
        return Err(e);
    }
    Ok(deck)
}

/// Lo que «Reemplazar» cambia de un deck de aquí, para volver atrás.
struct Copia {
    deck: Deck,
    listas: Vec<ListaDeJuego>,
    portada: Option<String>,
    linaje: linaje::Linaje,
}

impl Copia {
    fn de(db: &Db, deck: &Deck) -> AppResult<Copia> {
        Ok(Copia {
            deck: deck.clone(),
            listas: listas::de_deck(db, &deck.id)?,
            portada: decks::portada_de(db, &deck.id)?,
            linaje: linaje::de(db, &deck.id)?,
        })
    }

    fn restaurar(self, db: &Db) -> AppResult<()> {
        let id = &self.deck.id;
        decks::save(db, &self.deck)?;
        for l in listas::de_deck(db, id)? {
            listas::borrar(db, id, &l.id)?;
        }
        for l in &self.listas {
            listas::guardar(db, id, l)?;
        }
        etiquetas::portada(db, id, self.portada.as_deref())?;
        linaje::poner(db, id, &self.linaje.linaje, self.linaje.actualizado)?;
        Ok(())
    }
}

/// Lo que dice un paquete de sí mismo, para enseñarlo antes de importarlo.
#[derive(Debug, Clone)]
pub struct ResumenPaquete {
    pub nombre: String,
    pub cartas: u32,
    /// Lleva imágenes propias, diseños o reversos propios.
    pub proxies: bool,
    /// Lo que pesaría el paquete con solo las cartas originales (la suma de
    /// las imágenes que seguirían en él).
    pub tamano_originales: u64,
    pub linaje: Option<String>,
    pub actualizado: Option<i64>,
    /// Una miniatura JPEG de la portada.
    pub portada: Option<Vec<u8>>,
}

/// Lee solo el manifiesto (y la imagen de la portada) de un paquete.
pub fn resumen<R: Read + Seek>(r: R) -> AppResult<ResumenPaquete> {
    let mut zip = ZipArchive::new(r).map_err(de_zip)?;
    let idx = zip
        .index_for_name("manifest.json")
        .ok_or_else(|| mal("manifiesto"))?;
    let (_, bytes) = leer_entrada(&mut zip, idx)?;
    let m: Manifiesto = serde_json::from_slice(&bytes).map_err(|_| mal("manifiesto"))?;
    if m.format != FORMATO {
        return Err(mal("formato"));
    }
    if m.version > VERSION {
        return Err(AppError::new(ErrorCode::Unsupported).with("version", m.version));
    }
    let b = m.biblioteca.unwrap_or_default();
    // La portada: la elegida o la primera carta con imagen.
    let carta = b
        .portada
        .as_deref()
        .and_then(|p| m.deck.cards.iter().find(|c| c.id == p && c.front.is_some()))
        .or_else(|| m.deck.cards.iter().find(|c| c.front.is_some()));
    let mut portada = None;
    if let Some(f) = carta.and_then(|c| c.front.as_ref()) {
        if es_huella(&f.blob) {
            let prefijo = format!("blobs/{}.", f.blob);
            let idx = (0..zip.len())
                .find(|&i| matches!(zip.name_for_index(i), Some(Ok(n)) if n.starts_with(&prefijo)));
            if let Some(i) = idx {
                if let Ok((_, img)) = leer_entrada(&mut zip, i) {
                    portada = miniatura(&img);
                }
            }
        }
    }
    // Lo que quedaría sin proxies: las imágenes que sigue usando el deck.
    let quedan = blobs_del_deck(&proxy_core::compartir::sin_proxies(&m.deck));
    let mut tamano_originales = 16 * 1024u64;
    for i in 0..zip.len() {
        let Some(Ok(n)) = zip.name_for_index(i) else {
            continue;
        };
        let hash = n
            .strip_prefix("blobs/")
            .and_then(|r| r.split('.').next())
            .unwrap_or_default()
            .to_string();
        if quedan.contains(&hash) {
            if let Ok(f) = zip.by_index_raw(i) {
                tamano_originales += f.compressed_size();
            }
        }
    }
    Ok(ResumenPaquete {
        nombre: m.deck.name.clone(),
        cartas: m.deck.cards.len() as u32,
        proxies: proxy_core::compartir::resumen_proxies(&m.deck).hay(),
        tamano_originales,
        linaje: b.linaje.filter(|l| linaje_sano(l)),
        actualizado: b.actualizado,
        portada,
    })
}

/// Una miniatura JPEG pequeña (36 px de ancho) de una imagen de carta.
pub fn miniatura(bytes: &[u8]) -> Option<Vec<u8>> {
    let img = proxy_core::imaging::decode::decode(bytes).ok()?;
    let (w, h) = (img.width().max(1), img.height().max(1));
    let ancho = 36u32;
    let alto = (u64::from(h) * u64::from(ancho) / u64::from(w)).clamp(1, 80) as u32;
    let peq = image::imageops::thumbnail(&img, ancho, alto);
    proxy_core::imaging::encode::jpeg(&peq, 55).ok()
}

/// El origen con el que entró la imagen en el equipo de origen (lo dice el deck).
fn deck_origen(deck: &Deck, bytes: &[u8]) -> proxy_core::model::ImageOrigin {
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let todas = deck
        .default_back
        .iter()
        .chain(deck.cards.iter().filter_map(|c| c.front.as_ref()))
        .chain(deck.cards.iter().filter_map(|c| match &c.back {
            BackSpec::Dfc { image } | BackSpec::Custom { image } => Some(image),
            BackSpec::DeckDefault => None,
        }));
    for img in todas {
        if img.blob == hash {
            return img.origin;
        }
    }
    proxy_core::model::ImageOrigin::Upload
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::blobs::tests::png;
    use proxy_core::model::*;

    fn entorno() -> (tempfile::TempDir, Db, BlobStore) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::in_memory().unwrap();
        let s = BlobStore::new(dir.path().join("blobs"));
        (dir, db, s)
    }

    fn deck_con_imagenes(db: &Db, s: &BlobStore) -> Deck {
        let a = s
            .put_bytes(db, &png(20, 28, 1), ImageOrigin::Upload)
            .unwrap();
        let b = s
            .put_bytes(db, &png(20, 28, 2), ImageOrigin::Upload)
            .unwrap();
        let mut d = decks::create(db, "Mi deck", Some(b.clone())).unwrap();
        d.cards.push(DeckCard {
            id: "c1".into(),
            source: CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            front: Some(a),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
            design: None,
            marca: None,
        });
        decks::save(db, &d).unwrap();
        d
    }

    #[test]
    fn las_listas_para_jugar_viajan_con_los_ids_nuevos() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        listas::guardar(
            &db,
            &d.id,
            &ListaDeJuego {
                id: String::new(),
                nombre: "Torneo".into(),
                comandantes: vec!["c1".into()],
                cartas: vec!["c1".into(), "ya-no-esta".into()],
            },
        )
        .unwrap();
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let (_dir2, db2, s2) = entorno();
        let nuevo = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        let l = listas::de_deck(&db2, &nuevo.id).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].nombre, "Torneo");
        let id = nuevo.cards[0].id.clone();
        assert_ne!(id, "c1");
        assert_eq!(l[0].comandantes, vec![id.clone()]);
        assert_eq!(l[0].cartas, vec![id], "la que no estaba en el deck, fuera");
    }

    #[test]
    fn las_etiquetas_y_las_notas_viajan() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let t = etiquetas::create(&db, "Torneo", "red").unwrap();
        etiquetas::etiquetar(&db, std::slice::from_ref(&d.id), &[t.id], &[]).unwrap();
        etiquetas::notas(&db, &d.id, "Mazo del viernes").unwrap();
        etiquetas::idioma(&db, std::slice::from_ref(&d.id), Some("es")).unwrap();
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();

        // En el otro equipo ya hay una «torneo» (azul): se usa esa.
        let (_dir2, db2, s2) = entorno();
        let ya = etiquetas::create(&db2, "torneo", "blue").unwrap();
        let nuevo = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        let r = decks::list(&db2).unwrap();
        let r = r.iter().find(|x| x.id == nuevo.id).unwrap();
        assert_eq!(r.tags, vec![ya.id]);
        assert_eq!(r.notes, "Mazo del viernes");
        assert_eq!(r.lang_override.as_deref(), Some("es"));
        assert_eq!(etiquetas::list(&db2).unwrap().len(), 1);
    }

    #[test]
    fn ida_y_vuelta_en_un_equipo_limpio() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();

        let (_dir2, db2, s2) = entorno();
        let nuevo = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        assert_ne!(nuevo.id, d.id);
        assert_eq!(nuevo.name, "Mi deck");
        assert_eq!(nuevo.cards.len(), 1);
        assert_ne!(nuevo.cards[0].id, "c1");
        assert_eq!(blobs_del_deck(&nuevo), blobs_del_deck(&d));
        for h in blobs_del_deck(&nuevo) {
            assert!(s2.locate(&db2, &h).unwrap().is_some());
        }
        // Importar otra vez en el mismo sitio le cambia el nombre.
        let otro = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        assert_eq!(otro.name, "Mi deck (importado)");
    }

    #[test]
    fn un_deck_disenado_viaja_con_su_arte_y_su_coleccion() {
        use proxy_core::diseno::modelo::*;
        let (dir, db, s) = entorno();
        let mut d = deck_con_imagenes(&db, &s);
        let arte = s
            .put_bytes(&db, &png(30, 20, 7), ImageOrigin::Upload)
            .unwrap();
        let slime = s
            .put_bytes(&db, &png(8, 6, 8), ImageOrigin::Upload)
            .unwrap();
        let mut col = colecciones::tests::nueva("Tensura 2");
        col.emblem = Some(slime.clone());
        let col = colecciones::save(&db, &col).unwrap();
        let c = d.cards[0].clone();
        let datos = CardData {
            scryfall_id: "x".into(),
            layout: "normal".into(),
            faces: vec![FaceData::default()],
            ..CardData::default()
        };
        let mut diseno = crate::diseno::borrador::montar(&c, datos, Some(&col), &[]).design;
        diseno.faces[0].art = Some(ArtRef {
            image: arte.clone(),
            source: ArtSource::Upload,
            framing: ArtFraming::default(),
        });
        d.cards[0].design = Some(diseno);
        decks::save(&db, &d).unwrap();
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();

        let (_dir2, db2, s2) = entorno();
        let nuevo = import(std::fs::File::open(&ruta).unwrap(), &s2, &db2).unwrap();
        assert!(nuevo.cards[0].design.is_some());
        assert!(s2.locate(&db2, &arte.blob).unwrap().is_some());
        assert!(s2.locate(&db2, &slime.blob).unwrap().is_some());
        let cols = colecciones::list(&db2).unwrap();
        assert_eq!(cols.len(), 1);
        assert_eq!(cols[0].id, col.id);
        // El arte queda protegido de la limpieza en el equipo nuevo.
        db2.conn()
            .unwrap()
            .execute("UPDATE blobs SET created_at = 0", [])
            .unwrap();
        s2.gc(&db2, false).unwrap();
        assert!(s2.locate(&db2, &arte.blob).unwrap().is_some());
        assert!(s2.locate(&db2, &slime.blob).unwrap().is_some());
    }

    #[test]
    fn el_mismo_deck_da_el_mismo_fichero() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let (a, b) = (dir.path().join("a.pbdeck"), dir.path().join("b.pbdeck"));
        export(&d, &a, &s, &db).unwrap();
        export(&d, &b, &s, &db).unwrap();
        assert_eq!(std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
    }

    fn zip_a_mano(entradas: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut z = ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (n, b) in entradas {
            z.start_file(*n, SimpleFileOptions::default()).unwrap();
            z.write_all(b).unwrap();
        }
        z.finish().unwrap().into_inner()
    }

    #[test]
    fn una_imagen_alterada_se_rechaza() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("x.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let mut zip = ZipArchive::new(std::fs::File::open(&ruta).unwrap()).unwrap();
        let mut entradas = Vec::new();
        for i in 0..zip.len() {
            let (n, mut b) = leer_entrada(&mut zip, i).unwrap();
            if n.starts_with("blobs/") {
                let ultimo = b.len() - 1;
                b[ultimo] ^= 0xff;
            }
            entradas.push((n, b));
        }
        let refs: Vec<(&str, Vec<u8>)> = entradas
            .iter()
            .map(|(n, b)| (n.as_str(), b.clone()))
            .collect();
        let (_d2, db2, s2) = entorno();
        let e = import(std::io::Cursor::new(zip_a_mano(&refs)), &s2, &db2).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidInput);
    }

    #[test]
    fn una_ruta_con_puntos_se_rechaza() {
        let (_d, db, s) = entorno();
        let z = zip_a_mano(&[("../fuera.txt", b"x".to_vec())]);
        assert!(import(std::io::Cursor::new(z), &s, &db).is_err());
        let z = zip_a_mano(&[("/etc/passwd", b"x".to_vec())]);
        assert!(import(std::io::Cursor::new(z), &s, &db).is_err());
    }

    fn importar_de(ruta: &Path, s: &BlobStore, db: &Db, op: &Opciones) -> Deck {
        import_con(std::fs::File::open(ruta).unwrap(), s, db, op).unwrap()
    }

    #[test]
    fn el_linaje_hace_ida_y_vuelta_entre_equipos() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let origen = linaje::de(&db, &d.id).unwrap();
        assert_eq!(origen.linaje, d.id);
        let a = dir.path().join("a.pbdeck");
        export(&d, &a, &s, &db).unwrap();

        // En el segundo equipo conserva el linaje y la hora de la versión.
        let (dir2, db2, s2) = entorno();
        let en_2 = importar_de(&a, &s2, &db2, &Opciones::default());
        assert_ne!(en_2.id, d.id);
        let l2 = linaje::de(&db2, &en_2.id).unwrap();
        assert_eq!(l2, origen);

        // De vuelta al primero, sigue siendo el mismo.
        let b = dir2.path().join("b.pbdeck");
        export(&en_2, &b, &s2, &db2).unwrap();
        let de_vuelta = importar_de(&b, &s, &db, &Opciones::default());
        assert_eq!(linaje::de(&db, &de_vuelta.id).unwrap().linaje, d.id);
        assert_eq!(linaje::decks_de(&db, &d.id).unwrap().len(), 2);
    }

    #[test]
    fn guardar_los_dos_es_otro_linaje_y_un_duplicado_tambien() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let aparte = importar_de(
            &ruta,
            &s,
            &db,
            &Opciones {
                destino: Destino::GuardarLosDos,
                ..Opciones::default()
            },
        );
        assert_eq!(aparte.name, "Mi deck (importado)");
        assert_eq!(linaje::de(&db, &aparte.id).unwrap().linaje, aparte.id);
        let copia = decks::duplicate(&db, &d.id, "copia").unwrap();
        assert_eq!(linaje::de(&db, &copia.id).unwrap().linaje, copia.id);
        assert_eq!(linaje::decks_de(&db, &d.id).unwrap().len(), 1);
    }

    /// Reescribe el manifiesto de un paquete (para imitar los de antes).
    fn reescribir_manifiesto(ruta: &Path, f: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut zip = ZipArchive::new(std::fs::File::open(ruta).unwrap()).unwrap();
        let mut entradas = Vec::new();
        for i in 0..zip.len() {
            let (n, b) = leer_entrada(&mut zip, i).unwrap();
            entradas.push((n, b));
        }
        let mut f = Some(f);
        let refs: Vec<(String, Vec<u8>)> = entradas
            .into_iter()
            .map(|(n, b)| {
                if n == "manifest.json" {
                    let mut v: serde_json::Value = serde_json::from_slice(&b).unwrap();
                    (f.take().unwrap())(&mut v);
                    (n, serde_json::to_vec(&v).unwrap())
                } else {
                    (n, b)
                }
            })
            .collect();
        let refs: Vec<(&str, Vec<u8>)> =
            refs.iter().map(|(n, b)| (n.as_str(), b.clone())).collect();
        zip_a_mano(&refs)
    }

    #[test]
    fn los_paquetes_de_antes_se_siguen_importando_como_decks_nuevos() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let t = etiquetas::create(&db, "Torneo", "red").unwrap();
        etiquetas::etiquetar(&db, std::slice::from_ref(&d.id), &[t.id], &[]).unwrap();
        listas::guardar(
            &db,
            &d.id,
            &ListaDeJuego {
                id: String::new(),
                nombre: "L".into(),
                comandantes: vec!["c1".into()],
                cartas: vec!["c1".into()],
            },
        )
        .unwrap();
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();

        // La 3.2.3: la biblioteca sin linaje, hora ni portada.
        let v3 = reescribir_manifiesto(&ruta, |m| {
            let b = m["biblioteca"].as_object_mut().unwrap();
            b.remove("linaje");
            b.remove("actualizado");
            b.remove("portada");
        });
        // La 2.7.2: ni listas ni iconos (ni la `biblioteca` de antes de la 2.2).
        let v2 = reescribir_manifiesto(&ruta, |m| {
            m.as_object_mut().unwrap().remove("biblioteca");
        });
        for (nombre, bytes) in [("3.2.3", v3), ("2.7.2", v2)] {
            let (_d2, db2, s2) = entorno();
            let nuevo = import(std::io::Cursor::new(bytes), &s2, &db2).unwrap_or_else(|e| {
                panic!("el paquete de la {nombre} no se importa: {e:?}");
            });
            assert_eq!(nuevo.cards.len(), 1, "{nombre}");
            // Sin linaje: cuenta como un deck nuevo (el suyo propio).
            assert_eq!(
                linaje::de(&db2, &nuevo.id).unwrap().linaje,
                nuevo.id,
                "{nombre}"
            );
        }
    }

    #[test]
    fn un_linaje_raro_de_fuera_se_ignora() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let malo = reescribir_manifiesto(&ruta, |m| {
            m["biblioteca"]["linaje"] = serde_json::json!("x'; DROP TABLE decks; --");
        });
        let (_d2, db2, s2) = entorno();
        let n = import(std::io::Cursor::new(malo), &s2, &db2).unwrap();
        assert_eq!(linaje::de(&db2, &n.id).unwrap().linaje, n.id);
    }

    #[test]
    fn reemplazar_conserva_lo_de_aqui_y_toma_las_cartas_las_listas_y_la_portada() {
        // El equipo de origen: un deck con dos cartas, una lista y portada.
        let (dir, db, s) = entorno();
        let mut d = deck_con_imagenes(&db, &s);
        let extra = s
            .put_bytes(&db, &png(20, 28, 9), ImageOrigin::Upload)
            .unwrap();
        d.cards.push(DeckCard {
            id: "c2".into(),
            front: Some(extra),
            ..d.cards[0].clone()
        });
        decks::save(&db, &d).unwrap();
        listas::guardar(
            &db,
            &d.id,
            &ListaDeJuego {
                id: String::new(),
                nombre: "Nueva".into(),
                comandantes: vec!["c2".into()],
                cartas: vec!["c1".into(), "c2".into()],
            },
        )
        .unwrap();
        etiquetas::portada(&db, &d.id, Some("c2")).unwrap();
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();

        // Este equipo: el mismo deck (primera versión), con lo suyo.
        let (_dir2, db2, s2) = entorno();
        let primera = {
            let mut vieja = d.clone();
            vieja.cards.truncate(1);
            let r = dir.path().join("vieja.pbdeck");
            // La primera versión viaja primero.
            let (_d, dbv, sv) = entorno();
            for c in &vieja.cards {
                let f = c.front.as_ref().unwrap();
                let (ruta_img, _) = s.locate(&db, &f.blob).unwrap().unwrap();
                sv.put_bytes(&dbv, &std::fs::read(ruta_img).unwrap(), ImageOrigin::Upload)
                    .unwrap();
            }
            if let Some(b) = &vieja.default_back {
                let (ruta_img, _) = s.locate(&db, &b.blob).unwrap().unwrap();
                sv.put_bytes(&dbv, &std::fs::read(ruta_img).unwrap(), ImageOrigin::Upload)
                    .unwrap();
            }
            decks::save(&dbv, &vieja).unwrap();
            linaje::poner(&dbv, &vieja.id, &d.id, 1).unwrap();
            export(&vieja, &r, &sv, &dbv).unwrap();
            importar_de(&r, &s2, &db2, &Opciones::default())
        };
        let t = etiquetas::create(&db2, "Mía", "blue").unwrap();
        etiquetas::etiquetar(
            &db2,
            std::slice::from_ref(&primera.id),
            std::slice::from_ref(&t.id),
            &[],
        )
        .unwrap();
        etiquetas::notas(&db2, &primera.id, "mis notas").unwrap();
        etiquetas::favorito(&db2, std::slice::from_ref(&primera.id), true).unwrap();
        etiquetas::idioma(&db2, std::slice::from_ref(&primera.id), Some("fr")).unwrap();
        decks::rename(&db2, &primera.id, "Mi nombre").unwrap();
        listas::guardar(
            &db2,
            &primera.id,
            &ListaDeJuego {
                id: String::new(),
                nombre: "Vieja".into(),
                comandantes: vec![primera.cards[0].id.clone()],
                cartas: vec![primera.cards[0].id.clone()],
            },
        )
        .unwrap();
        // Una partida del historial apunta al deck.
        db2.conn()
            .unwrap()
            .execute_batch(&format!(
                "INSERT INTO partidas (id, empezada, terminada, formato, modo, jugadores, ganadores, como, rondas, duracion_ms, version_catalogo)
                   VALUES ('p1', 1, 2, '\"commander\"', '\"solitario\"', '[]', '[]', 'x', 1, 1, 1);
                 INSERT INTO partida_decks (partida_id, jugador, deck_id, puesto) VALUES ('p1', 0, '{}', 1);",
                primera.id
            ))
            .unwrap();

        // Reemplazar.
        let hecho = importar_de(
            &ruta,
            &s2,
            &db2,
            &Opciones {
                destino: Destino::Reemplazar(primera.id.clone()),
                ..Opciones::default()
            },
        );
        assert_eq!(hecho.id, primera.id, "conserva el id");
        let ahora = decks::open(&db2, &primera.id).unwrap();
        assert_eq!(ahora.cards.len(), 2, "toma las cartas");
        assert_eq!(ahora.name, "Mi nombre", "conserva el nombre de aquí");
        let r = decks::list(&db2).unwrap();
        let r = r.iter().find(|x| x.id == primera.id).unwrap();
        assert_eq!(r.tags, vec![t.id], "conserva las etiquetas");
        assert_eq!(r.notes, "mis notas");
        assert!(r.pinned);
        assert_eq!(r.lang_override.as_deref(), Some("fr"));
        // Las listas son las del paquete, con los ids nuevos.
        let l = listas::de_deck(&db2, &primera.id).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].nombre, "Nueva");
        assert_eq!(l[0].comandantes, vec![ahora.cards[1].id.clone()]);
        // La portada: la del paquete, con el id nuevo.
        assert_eq!(r.cover_card_id.as_deref(), Some(ahora.cards[1].id.as_str()));
        // Sigue habiendo un solo deck y su linaje.
        assert_eq!(decks::list(&db2).unwrap().len(), 1);
        assert_eq!(linaje::de(&db2, &primera.id).unwrap().linaje, d.id);
        // Y el historial sigue colgando del mismo id.
        let n: i64 = db2
            .conn()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM partida_decks WHERE deck_id = ?1",
                [&primera.id],
                |r| r.get(0),
            )
            .unwrap_or(1);
        assert_eq!(n, 1);
    }

    #[test]
    fn si_reemplazar_falla_a_medias_el_deck_de_aqui_vuelve_a_como_estaba() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        listas::guardar(
            &db,
            &d.id,
            &ListaDeJuego {
                id: "l1".into(),
                nombre: "Mía".into(),
                comandantes: vec!["c1".into()],
                cartas: vec!["c1".into()],
            },
        )
        .unwrap();
        etiquetas::portada(&db, &d.id, Some("c1")).unwrap();
        let antes = linaje::de(&db, &d.id).unwrap();
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        // Un paquete con una hora que la base rechaza al poner el linaje
        // (después de guardar las cartas nuevas).
        let bytes = reescribir_manifiesto(&ruta, |v| {
            v["biblioteca"]["actualizado"] = serde_json::json!(4242);
        });
        db.conn()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER rompe BEFORE UPDATE OF linaje ON deck_linaje
                 WHEN NEW.actualizado = 4242 BEGIN SELECT RAISE(ABORT, 'rompe'); END;",
            )
            .unwrap();
        let e = import_con(
            std::io::Cursor::new(bytes),
            &s,
            &db,
            &Opciones {
                destino: Destino::Reemplazar(d.id.clone()),
                ..Opciones::default()
            },
        );
        assert!(e.is_err());
        let ahora = decks::open(&db, &d.id).unwrap();
        assert_eq!(
            ahora
                .cards
                .iter()
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c1"],
            "las cartas de antes, con sus ids"
        );
        let l = listas::de_deck(&db, &d.id).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].id, "l1");
        assert_eq!(
            decks::portada_de(&db, &d.id).unwrap().as_deref(),
            Some("c1")
        );
        assert_eq!(linaje::de(&db, &d.id).unwrap(), antes);
        assert_eq!(decks::list(&db).unwrap().len(), 1);
    }

    #[test]
    fn reemplazar_un_deck_que_no_existe_no_hace_nada() {
        let (dir, db, s) = entorno();
        let d = deck_con_imagenes(&db, &s);
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let (_d2, db2, s2) = entorno();
        let e = import_con(
            std::fs::File::open(&ruta).unwrap(),
            &s2,
            &db2,
            &Opciones {
                destino: Destino::Reemplazar("no-existe".into()),
                ..Opciones::default()
            },
        )
        .unwrap_err();
        assert_eq!(e.code, ErrorCode::NotFound);
        assert!(decks::list(&db2).unwrap().is_empty());
    }

    /// Un deck con una propia vinculada, una de Scryfall y su reverso propio.
    fn deck_mixto(db: &Db, s: &BlobStore) -> Deck {
        let mut d = deck_con_imagenes(db, s);
        d.cards[0].source = CardSource::Custom {
            scryfall_id: Some("bolt".into()),
            externa: None,
        };
        d.cards[0].meta.name = "Lightning Bolt".into();
        let scry = s
            .put_bytes(db, &png(20, 28, 5), ImageOrigin::ScryfallPng)
            .unwrap();
        d.cards.push(DeckCard {
            id: "c2".into(),
            source: CardSource::Scryfall {
                scryfall_id: "sol".into(),
            },
            front: Some(scry),
            ..d.cards[0].clone()
        });
        d.cards[1].meta.name = "Sol Ring".into();
        decks::save(db, &d).unwrap();
        d
    }

    #[test]
    fn solo_originales_no_guarda_las_imagenes_propias_y_pesa_menos() {
        let (dir, db, s) = entorno();
        let d = deck_mixto(&db, &s);
        let con = dir.path().join("con.pbdeck");
        let sin = dir.path().join("sin.pbdeck");
        let b_con = export_con(&d, &con, &s, &db, false).unwrap();
        let b_sin = export_con(&d, &sin, &s, &db, true).unwrap();
        assert!(b_sin < b_con, "{b_sin} < {b_con}");
        assert!(tamano_estimado(&d, &s, &db, true) < tamano_estimado(&d, &s, &db, false));

        // Recibir «solo originales» un paquete con proxies: la propia pasa a
        // su carta de Scryfall, sin imagen, y su imagen no se guarda.
        let (_d2, db2, s2) = entorno();
        let n = importar_de(
            &con,
            &s2,
            &db2,
            &Opciones {
                solo_originales: true,
                ..Opciones::default()
            },
        );
        assert_eq!(
            n.cards[0].source,
            CardSource::Scryfall {
                scryfall_id: "bolt".into()
            }
        );
        assert!(n.cards[0].front.is_none());
        assert!(
            n.cards[1].front.is_some(),
            "la de Scryfall conserva su imagen"
        );
        let propia = d.cards[0].front.as_ref().unwrap();
        assert!(s2.locate(&db2, &propia.blob).unwrap().is_none());
        assert_eq!(
            proxy_core::compartir::pendientes(&n).len(),
            1,
            "falta bajar una imagen"
        );
        // El reverso propio del deck se cambia por el de la app.
        assert_eq!(
            n.default_back.as_ref().unwrap().origin,
            ImageOrigin::Generated
        );

        // El paquete ya escrito sin proxies da lo mismo al importarlo normal.
        let (_d3, db3, s3) = entorno();
        let m = importar_de(&sin, &s3, &db3, &Opciones::default());
        assert_eq!(m.cards[0].source, n.cards[0].source);
    }

    #[test]
    fn el_resumen_lee_el_manifiesto_sin_importar() {
        let (dir, db, s) = entorno();
        let d = deck_mixto(&db, &s);
        let ruta = dir.path().join("a.pbdeck");
        export(&d, &ruta, &s, &db).unwrap();
        let r = resumen(std::fs::File::open(&ruta).unwrap()).unwrap();
        assert_eq!(r.nombre, "Mi deck");
        assert_eq!(r.cartas, 2);
        assert!(r.proxies);
        assert_eq!(r.linaje.as_deref(), Some(d.id.as_str()));
        assert!(r.actualizado.is_some());
        assert!(r
            .portada
            .as_ref()
            .is_some_and(|p| p.starts_with(&[0xff, 0xd8])));
        // Sin proxies, ya no.
        let sin = dir.path().join("sin.pbdeck");
        export_con(&d, &sin, &s, &db, true).unwrap();
        assert!(!resumen(std::fs::File::open(&sin).unwrap()).unwrap().proxies);
        assert!(resumen(std::io::Cursor::new(zip_a_mano(&[]))).is_err());
    }

    #[test]
    fn sin_manifiesto_o_de_otro_formato_no_entra() {
        let (_d, db, s) = entorno();
        assert!(import(std::io::Cursor::new(zip_a_mano(&[])), &s, &db).is_err());
        let m = br#"{"format":"otra-cosa","version":1,"deck":null}"#.to_vec();
        assert!(import(
            std::io::Cursor::new(zip_a_mano(&[("manifest.json", m)])),
            &s,
            &db
        )
        .is_err());
    }
}
