//! **Bajar las imágenes que faltan** tras recibir «solo cartas originales»:
//! las propias vinculadas pasaron a su carta de Scryfall sin imagen
//! (`proxy_core::compartir`), y las fichas de otra fuente conservan su
//! referencia. Un trabajo con progreso y cancelable (`lanzarTrabajo` en la
//! pantalla) las baja con `traer_carta` y `card_fetch_externa`.
//!
//! La carta del deck se completa con lo que llega (`compartir::completar`) y
//! **conserva su id**, así las listas para jugar, la portada y las marcas
//! siguen valiendo. Lo que ya se bajó se guarda cada pocas cartas: si se
//! cancela, no se pierde.

use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::compartir::{self, Origen, Pendiente};

use crate::commands::cards::traer_carta;
use crate::error::{AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::state::AppState;
use crate::store::decks;

/// Cada cuántas cartas se guarda el deck.
const GUARDAR_CADA: usize = 10;

/// Lo que se hizo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Bajadas {
    pub bajadas: u32,
    pub fallidas: u32,
    pub cancelado: bool,
}

/// De dónde sacar una carta.
async fn traer<A: CardApi>(
    api: &A,
    st: &AppState,
    origen: &Origen,
) -> AppResult<proxy_core::model::DeckCard> {
    match origen {
        Origen::Scryfall(id) | Origen::Ficha(id) => traer_carta(api, st, id).await,
        Origen::Externa(r) => match r.fuente {
            proxy_core::model::FuenteCarta::Cockatrice => {
                crate::fuentes::carta_cockatrice(st, &r.id).await
            }
        },
    }
}

/// Guarda las cartas traídas en el deck tal como está ahora en la base.
fn guardar(
    st: &AppState,
    deck_id: &str,
    traidas: &mut Vec<(String, proxy_core::model::DeckCard)>,
) -> AppResult<()> {
    if traidas.is_empty() {
        return Ok(());
    }
    let mut deck = decks::open(&st.db, deck_id)?;
    for (id, carta) in traidas.drain(..) {
        if let Some(c) = deck.cards.iter_mut().find(|c| c.id == id) {
            compartir::completar(c, carta);
        }
    }
    decks::save(&st.db, &deck)?;
    Ok(())
}

/// Baja las imágenes que le faltan al deck. `avance(hechas, total)` se llama
/// tras cada carta.
pub async fn completar<A: CardApi>(
    api: &A,
    st: &AppState,
    deck_id: &str,
    cancelado: &AtomicBool,
    avance: impl Fn(u32, u32),
) -> AppResult<Bajadas> {
    let deck = decks::open(&st.db, deck_id)?;
    let pendientes: Vec<Pendiente> = compartir::pendientes(&deck);
    let total = pendientes.len() as u32;
    let mut r = Bajadas::default();
    let mut traidas = Vec::new();
    avance(0, total);
    for (i, p) in pendientes.iter().enumerate() {
        if cancelado.load(Ordering::SeqCst) {
            r.cancelado = true;
            break;
        }
        match traer(api, st, &p.origen).await {
            Ok(c) if c.front.is_some() => {
                traidas.push((p.carta.clone(), c));
                r.bajadas += 1;
            }
            Ok(_) => r.fallidas += 1,
            Err(e) if e.code == ErrorCode::Cancelled => {
                r.cancelado = true;
                break;
            }
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "pasar: no se pudo bajar una imagen");
                r.fallidas += 1;
            }
        }
        if traidas.len() >= GUARDAR_CADA {
            guardar(st, deck_id, &mut traidas)?;
        }
        avance(i as u32 + 1, total);
    }
    guardar(st, deck_id, &mut traidas)?;
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::client::CardApi;
    use crate::scryfall::models::{Collection, ScryCard};
    use crate::state::{AppState, Paths};
    use crate::store::blobs::tests::png;
    use proxy_core::model::{
        BackSpec, CardEdits, CardMeta, CardSource, Deck, DeckCard, ImageOrigin, MarcaCarta,
    };
    use serde_json::json;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;

    /// Un Scryfall de mentira: da las cartas que tiene y descarga PNG de verdad.
    struct Api {
        cartas: Vec<ScryCard>,
        descargas: Mutex<Vec<String>>,
    }

    impl CardApi for Api {
        async fn collection(
            &self,
            _: &[crate::scryfall::models::Identifier],
        ) -> AppResult<Collection> {
            Ok(Collection::default())
        }
        async fn card(&self, id: &str) -> AppResult<Option<ScryCard>> {
            Ok(self.cartas.iter().find(|c| c.id == id).cloned())
        }
        async fn card_lang(&self, _: &str, _: &str, _: &str) -> AppResult<Option<ScryCard>> {
            Ok(None)
        }
        async fn search(&self, _: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            Ok(Vec::new())
        }
        async fn download(&self, url: &str) -> AppResult<Vec<u8>> {
            self.descargas.lock().unwrap().push(url.to_string());
            let semilla = url.bytes().fold(0u8, |a, b| a.wrapping_add(b));
            Ok(png(6, 8, semilla))
        }
    }

    fn scry(id: &str) -> ScryCard {
        serde_json::from_value(json!({
            "id": id, "name": format!("Carta {id}"), "lang": "en", "set": "cmm",
            "collector_number": "1", "layout": "normal", "highres_image": true,
            "image_status": "highres_scan",
            "image_uris": {"png": format!("https://img/{id}.png")}
        }))
        .unwrap()
    }

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    /// Un deck recién recibido «solo originales» con `n` propias vinculadas
    /// (más una sin vínculo, que queda como hueco).
    fn deck_recibido(st: &AppState, n: usize) -> Deck {
        let origen = {
            let dir = tempfile::tempdir().unwrap();
            let s = AppState::open(Paths {
                data: dir.path().join("d"),
                cache: dir.path().join("c"),
            })
            .unwrap();
            let reverso = s.blobs.generic_back(&s.db).unwrap();
            let mut d = crate::store::decks::create(&s.db, "Con propias", Some(reverso)).unwrap();
            for i in 0..n {
                let img = s
                    .blobs
                    .put_bytes(&s.db, &png(20, 28, i as u8 + 1), ImageOrigin::Upload)
                    .unwrap();
                d.cards.push(DeckCard {
                    id: format!("c{i}"),
                    source: CardSource::Custom {
                        scryfall_id: Some(format!("s{i}")),
                        externa: None,
                    },
                    front: Some(img),
                    back: BackSpec::DeckDefault,
                    edits: CardEdits::default(),
                    locked: false,
                    meta: CardMeta {
                        name: format!("Carta s{i}"),
                        ..CardMeta::default()
                    },
                    design: None,
                    marca: (i == 0).then_some(MarcaCarta::Banquillo),
                });
            }
            crate::store::decks::save(&s.db, &d).unwrap();
            let ruta = dir.path().join("x.pbdeck");
            crate::package::export(&d, &ruta, &s.blobs, &s.db).unwrap();
            let f = std::fs::File::open(&ruta).unwrap();
            crate::package::import_con(
                std::io::BufReader::new(f),
                &st.blobs,
                &st.db,
                &crate::package::Opciones {
                    solo_originales: true,
                    ..Default::default()
                },
            )
            .unwrap()
        };
        origen
    }

    #[tokio::test]
    async fn diez_propias_vinculadas_dan_diez_cartas_de_scryfall_con_su_imagen() {
        let (_dir, st) = estado();
        let deck = deck_recibido(&st, 10);
        assert_eq!(compartir::pendientes(&deck).len(), 10);
        assert!(deck.cards.iter().all(|c| c.front.is_none()));
        let api = Api {
            cartas: (0..10).map(|i| scry(&format!("s{i}"))).collect(),
            descargas: Mutex::new(Vec::new()),
        };
        let avances = Mutex::new(Vec::new());
        let r = completar(&api, &st, &deck.id, &AtomicBool::new(false), |h, t| {
            avances.lock().unwrap().push((h, t));
        })
        .await
        .unwrap();
        assert_eq!(
            r,
            Bajadas {
                bajadas: 10,
                fallidas: 0,
                cancelado: false
            }
        );
        // El progreso va de 0 a 10.
        let a = avances.lock().unwrap();
        assert_eq!(a.first(), Some(&(0, 10)));
        assert_eq!(a.last(), Some(&(10, 10)));
        // El deck de la base: 10 cartas de Scryfall con su imagen, con sus ids.
        let ahora = crate::store::decks::open(&st.db, &deck.id).unwrap();
        for (antes, c) in deck.cards.iter().zip(&ahora.cards) {
            assert_eq!(c.id, antes.id, "la carta conserva su id");
            assert!(matches!(c.source, CardSource::Scryfall { .. }));
            let f = c.front.as_ref().expect("con su imagen");
            assert!(st.blobs.locate(&st.db, &f.blob).unwrap().is_some());
            assert_eq!(f.origin, ImageOrigin::ScryfallPng);
            assert_eq!(c.marca, antes.marca, "y su marca");
        }
        assert!(compartir::pendientes(&ahora).is_empty());
    }

    #[tokio::test]
    async fn lo_que_no_se_puede_bajar_se_cuenta_y_se_puede_cancelar() {
        let (_dir, st) = estado();
        let deck = deck_recibido(&st, 5);
        // Scryfall solo conoce tres.
        let api = Api {
            cartas: (0..3).map(|i| scry(&format!("s{i}"))).collect(),
            descargas: Mutex::new(Vec::new()),
        };
        let r = completar(&api, &st, &deck.id, &AtomicBool::new(false), |_, _| {})
            .await
            .unwrap();
        assert_eq!((r.bajadas, r.fallidas, r.cancelado), (3, 2, false));
        let ahora = crate::store::decks::open(&st.db, &deck.id).unwrap();
        assert_eq!(compartir::pendientes(&ahora).len(), 2);

        // Cancelar: se para y lo ya bajado se queda.
        let (_d2, st2) = estado();
        let deck2 = deck_recibido(&st2, 6);
        let api2 = Api {
            cartas: (0..6).map(|i| scry(&format!("s{i}"))).collect(),
            descargas: Mutex::new(Vec::new()),
        };
        let cancelado = AtomicBool::new(false);
        let r = completar(&api2, &st2, &deck2.id, &cancelado, |hechas, _| {
            if hechas == 2 {
                cancelado.store(true, Ordering::SeqCst);
            }
        })
        .await
        .unwrap();
        assert!(r.cancelado);
        assert_eq!(r.bajadas, 2);
        let ahora = crate::store::decks::open(&st2.db, &deck2.id).unwrap();
        assert_eq!(compartir::pendientes(&ahora).len(), 4);
        // Un deck que no existe.
        assert!(
            completar(&api2, &st2, "no-existe", &AtomicBool::new(false), |_, _| {})
                .await
                .is_err()
        );
    }
}
