//! **Optimizar las imágenes de la biblioteca** («Liberar espacio»): los PNG sin
//! transparencia de los decks elegidos pasan a JPEG con las mismas medidas y
//! los originales se borran de este equipo. Con las mismas reglas y calidades
//! que el `.pbdeck` optimizado (`package::calidades_de`).
//!
//! 🛑 No se puede deshacer, así que el orden importa:
//!
//! 1. Se hacen todos los JPEG y se guardan en el almacén. Los decks aún no han
//!    cambiado: cancelar o un fallo aquí no toca nada (lo que sobre lo recoge
//!    el `gc` de siempre).
//! 2. Cada deck se reescribe en una transacción. No cambia su fecha ni la de su
//!    versión (`save_imagenes`): el contenido es el mismo.
//! 3. Se borra un original solo si ya no lo usa nadie: una imagen que comparte
//!    un deck no elegido, una colección o la mesa se queda.
//!
//! Lo que no compensa (transparencia, colores planos) se apunta y no se vuelve
//! a ofrecer: saberlo pide leer la imagen.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::imaging::compactar;
use proxy_core::model::{Deck, ImageOrigin};
use rayon::prelude::*;
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult};
use crate::package;
use crate::state::AppState;
use crate::store::{decks, settings};

/// Las imágenes en las que ya se vio que no compensa.
const DESCARTADAS: &str = "optimizar.descartadas";
/// Cuántas imágenes se tienen en memoria a la vez, como mucho.
const LOTE_MAXIMO: usize = 16;

/// Un deck con imágenes que se pueden optimizar.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DeckOptimizable {
    pub id: String,
    pub nombre: String,
    pub imagenes: u32,
    /// Lo que se ahorraría en este deck, como si no compartiera nada.
    #[ts(type = "number")]
    pub ahorro: u64,
}

/// Lo que liberaría optimizar estos decks, contando lo compartido.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstimacionOptimizar {
    pub imagenes: u32,
    #[ts(type = "number")]
    pub libera: u64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OptimizarProgreso {
    pub job_id: u32,
    pub hechas: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OptimizarFin {
    pub job_id: u32,
    /// Imágenes que han pasado a JPEG.
    pub imagenes: u32,
    #[ts(type = "number")]
    pub libera: u64,
    pub cancelado: bool,
    pub error: Option<AppError>,
}

/// Lo que se hizo.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hecho {
    pub imagenes: u32,
    pub libera: u64,
    pub cancelado: bool,
}

fn descartadas(st: &AppState) -> AppResult<BTreeSet<String>> {
    Ok(settings::get(&st.db, DESCARTADAS)?
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default())
}

/// Lo que se puede optimizar de un deck, con su calidad.
fn pendientes(
    st: &AppState,
    deck: &Deck,
    no: &BTreeSet<String>,
) -> AppResult<BTreeMap<String, u8>> {
    let mut c = package::calidades_de(deck, &st.blobs, &st.db)?;
    c.retain(|h, _| !no.contains(h));
    Ok(c)
}

/// Los decks que tienen algo que optimizar.
pub fn optimizables(st: &AppState) -> AppResult<Vec<DeckOptimizable>> {
    let no = descartadas(st)?;
    let mut out = Vec::new();
    for r in decks::list(&st.db)? {
        let deck = decks::open(&st.db, &r.id)?;
        let p = pendientes(st, &deck, &no)?;
        let mut ahorro = 0u64;
        for (h, q) in &p {
            if let Some(f) = st.blobs.ficha(&st.db, h)? {
                ahorro += f.bytes - package::peso_jpeg(&f, *q);
            }
        }
        if ahorro > 0 {
            out.push(DeckOptimizable {
                id: deck.id,
                nombre: deck.name,
                imagenes: p.len() as u32,
                ahorro,
            });
        }
    }
    Ok(out)
}

/// Lo que se liberaría de verdad: un original que usa otro deck no se borra y
/// su JPEG ocupa además.
pub fn estimar(st: &AppState, ids: &[String]) -> AppResult<EstimacionOptimizar> {
    let no = descartadas(st)?;
    let elegidos: BTreeSet<&str> = ids.iter().map(String::as_str).collect();
    let mut c: BTreeMap<String, u8> = BTreeMap::new();
    for id in ids {
        mezclar(&mut c, pendientes(st, &decks::open(&st.db, id)?, &no)?);
    }
    let mut libera = 0i64;
    for (h, q) in &c {
        let Some(f) = st.blobs.ficha(&st.db, h)? else {
            continue;
        };
        libera -= package::peso_jpeg(&f, *q) as i64;
        let usos = st.blobs.usos(&st.db, h)?;
        if !usos.otros && usos.decks.iter().all(|d| elegidos.contains(d.as_str())) {
            libera += f.bytes as i64;
        }
    }
    Ok(EstimacionOptimizar {
        imagenes: c.len() as u32,
        libera: libera.max(0) as u64,
    })
}

/// Una imagen en varios decks se queda con la mejor calidad que le pidan.
fn mezclar(c: &mut BTreeMap<String, u8>, otra: BTreeMap<String, u8>) {
    for (h, q) in otra {
        let v = c.entry(h).or_insert(q);
        *v = (*v).max(q);
    }
}

/// Cambia en el deck cada huella por la nueva.
fn con_huellas(deck: &Deck, cambios: &BTreeMap<String, String>) -> AppResult<Deck> {
    let interno =
        |e: serde_json::Error| AppError::new(crate::error::ErrorCode::Internal).with("detalle", e);
    let mut v = serde_json::to_value(deck).map_err(interno)?;
    package::cambiar_huellas(&mut v, cambios);
    serde_json::from_value(v).map_err(interno)
}

/// Optimiza los decks `ids`. `avance(hechas, total)` se llama tras cada lote.
pub fn optimizar(
    st: &AppState,
    ids: &[String],
    cancelado: &AtomicBool,
    avance: impl Fn(u32, u32),
) -> AppResult<Hecho> {
    let mut no = descartadas(st)?;
    let mut mazos = Vec::with_capacity(ids.len());
    let mut c: BTreeMap<String, u8> = BTreeMap::new();
    for id in ids {
        let deck = decks::open(&st.db, id)?;
        mezclar(&mut c, pendientes(st, &deck, &no)?);
        mazos.push(deck);
    }
    let mut rutas = Vec::with_capacity(c.len());
    for (h, q) in &c {
        if let Some((ruta, _)) = st.blobs.locate(&st.db, h)? {
            rutas.push((h.clone(), ruta, *q));
        }
    }

    // 1. Los JPEG. Hasta que acabe esto, los decks no cambian.
    let total = rutas.len() as u32;
    let lote = rayon::current_num_threads().clamp(1, LOTE_MAXIMO);
    let mut cambios = BTreeMap::new();
    let mut hechas = 0u32;
    let mut peso_nuevo = 0u64;
    avance(0, total);
    for trozo in rutas.chunks(lote) {
        if cancelado.load(Ordering::SeqCst) {
            return Ok(Hecho {
                cancelado: true,
                ..Hecho::default()
            });
        }
        let hechos = trozo
            .par_iter()
            .map(|(h, ruta, q)| {
                let bytes = std::fs::read(ruta)?;
                Ok((h, compactar::png_a_jpeg(&bytes, *q)))
            })
            .collect::<AppResult<Vec<_>>>()?;
        for (h, jpeg) in hechos {
            match jpeg {
                Some(j) => {
                    let img = st.blobs.put_bytes(&st.db, &j, ImageOrigin::Upload)?;
                    peso_nuevo += j.len() as u64;
                    cambios.insert(h.clone(), img.blob);
                }
                None => {
                    no.insert(h.clone());
                }
            }
        }
        hechas += trozo.len() as u32;
        avance(hechas, total);
    }
    settings::set(
        &st.db,
        DESCARTADAS,
        &serde_json::to_string(&no)
            .map_err(|e| AppError::new(crate::error::ErrorCode::Internal).with("detalle", e))?,
    )?;

    // 2. Los decks, cada uno de una vez.
    for deck in &mazos {
        let nuevo = con_huellas(deck, &cambios)?;
        if nuevo != *deck {
            decks::save_imagenes(&st.db, &nuevo)?;
        }
    }

    // 3. Los originales que ya no usa nadie.
    let viejas: Vec<&str> = cambios.keys().map(String::as_str).collect();
    let borrado = st.blobs.borrar_sin_usar(&st.db, &viejas)?;
    Ok(Hecho {
        imagenes: cambios.len() as u32,
        libera: borrado.freed_bytes.saturating_sub(peso_nuevo),
        cancelado: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;
    use proxy_core::model::*;

    fn foto(semilla: u32, alfa: u8) -> Vec<u8> {
        let mut x = semilla;
        let img = image::RgbaImage::from_fn(400, 300, |i, j| {
            x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let r = (x >> 28) as u8;
            image::Rgba([
                (i / 2) as u8 + r,
                (j / 2) as u8 + r,
                ((i + j) / 4) as u8 + r,
                alfa,
            ])
        });
        proxy_core::imaging::encode::png(&img).unwrap()
    }

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let st = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, st)
    }

    fn carta(id: &str, front: FaceImage) -> DeckCard {
        DeckCard {
            id: id.into(),
            source: CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            front: Some(front),
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
            design: None,
            marca: None,
        }
    }

    fn poner(st: &AppState, bytes: &[u8]) -> FaceImage {
        st.blobs
            .put_bytes(&st.db, bytes, ImageOrigin::Upload)
            .unwrap()
    }

    fn deck(st: &AppState, nombre: &str, cartas: Vec<DeckCard>) -> Deck {
        let mut d = decks::create(&st.db, nombre, None).unwrap();
        d.cards = cartas;
        decks::save(&st.db, &d).unwrap();
        decks::open(&st.db, &d.id).unwrap()
    }

    fn fechas(st: &AppState, id: &str) -> (i64, i64) {
        let c = st.db.conn().unwrap();
        let u = c
            .query_row("SELECT updated_at FROM decks WHERE id = ?1", [id], |r| {
                r.get(0)
            })
            .unwrap();
        let l = c
            .query_row(
                "SELECT actualizado FROM deck_linaje WHERE deck_id = ?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        (u, l)
    }

    #[test]
    fn optimiza_borra_el_original_y_no_cambia_las_fechas() {
        let (_d, st) = estado();
        let opaca = poner(&st, &foto(1, 255));
        let d = deck(&st, "A", vec![carta("c", opaca.clone())]);
        // Una fecha reconocible: si se tocara, se notaría aunque vaya rápido.
        {
            let c = st.db.conn().unwrap();
            c.execute("UPDATE decks SET updated_at = 123 WHERE id = ?1", [&d.id])
                .unwrap();
            c.execute(
                "UPDATE deck_linaje SET actualizado = 456 WHERE deck_id = ?1",
                [&d.id],
            )
            .unwrap();
        }
        let est = estimar(&st, std::slice::from_ref(&d.id)).unwrap();
        assert_eq!(est.imagenes, 1);
        assert!(est.libera > 0);

        let r = optimizar(
            &st,
            std::slice::from_ref(&d.id),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(r.imagenes, 1);
        assert!(r.libera > 0);

        let n = decks::open(&st.db, &d.id).unwrap();
        let f = n.cards[0].front.as_ref().unwrap();
        assert_ne!(f.blob, opaca.blob);
        assert_eq!((f.width, f.height), (400, 300));
        assert_eq!(st.blobs.ficha(&st.db, &f.blob).unwrap().unwrap().ext, "jpg");
        // El original ya no está, ni en la base ni en el disco.
        assert!(st.blobs.locate(&st.db, &opaca.blob).unwrap().is_none());
        assert_eq!(fechas(&st, &d.id), (123, 456));
        // Ya no queda nada que ofrecer.
        assert!(optimizables(&st).unwrap().is_empty());
    }

    #[test]
    fn una_imagen_de_otro_deck_se_queda_hasta_que_nadie_la_use() {
        let (_d, st) = estado();
        let compartida = poner(&st, &foto(1, 255));
        let a = deck(&st, "A", vec![carta("c", compartida.clone())]);
        let b = deck(&st, "B", vec![carta("c", compartida.clone())]);
        // Si solo se elige uno, no se libera nada: el original sigue haciendo falta.
        assert_eq!(estimar(&st, std::slice::from_ref(&a.id)).unwrap().libera, 0);

        optimizar(
            &st,
            std::slice::from_ref(&a.id),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert!(st.blobs.locate(&st.db, &compartida.blob).unwrap().is_some());
        assert_eq!(
            decks::open(&st.db, &b.id).unwrap().cards[0],
            b.cards[0],
            "el otro deck no cambia"
        );
        // El segundo apunta al mismo JPEG y entonces sí se borra.
        optimizar(
            &st,
            std::slice::from_ref(&b.id),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert!(st.blobs.locate(&st.db, &compartida.blob).unwrap().is_none());
        assert_eq!(
            decks::open(&st.db, &a.id).unwrap().cards[0].front,
            decks::open(&st.db, &b.id).unwrap().cards[0].front
        );
    }

    #[test]
    fn lo_que_usa_otra_cosa_no_se_borra() {
        let (_d, st) = estado();
        let img = poner(&st, &foto(1, 255));
        let a = deck(&st, "A", vec![carta("c", img.clone())]);
        st.db
            .conn()
            .unwrap()
            .execute(
                "INSERT INTO blob_refs (owner_kind, owner_id, blob) VALUES ('mesa', 'tapete', ?1)",
                [&img.blob],
            )
            .unwrap();
        optimizar(
            &st,
            std::slice::from_ref(&a.id),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert!(st.blobs.locate(&st.db, &img.blob).unwrap().is_some());
    }

    #[test]
    fn cancelar_no_cambia_nada() {
        let (_d, st) = estado();
        let img = poner(&st, &foto(1, 255));
        let a = deck(&st, "A", vec![carta("c", img.clone())]);
        let r = optimizar(
            &st,
            std::slice::from_ref(&a.id),
            &AtomicBool::new(true),
            |_, _| {},
        )
        .unwrap();
        assert!(r.cancelado);
        assert_eq!(decks::open(&st.db, &a.id).unwrap(), a);
        assert!(st.blobs.locate(&st.db, &img.blob).unwrap().is_some());
    }

    #[test]
    fn lo_que_no_compensa_no_se_vuelve_a_ofrecer() {
        let (_d, st) = estado();
        let con_alfa = poner(&st, &foto(1, 200));
        let a = deck(&st, "A", vec![carta("c", con_alfa.clone())]);
        assert_eq!(optimizables(&st).unwrap().len(), 1);
        let r = optimizar(
            &st,
            std::slice::from_ref(&a.id),
            &AtomicBool::new(false),
            |_, _| {},
        )
        .unwrap();
        assert_eq!(r.imagenes, 0);
        assert_eq!(decks::open(&st.db, &a.id).unwrap(), a);
        assert!(optimizables(&st).unwrap().is_empty());
    }
}
