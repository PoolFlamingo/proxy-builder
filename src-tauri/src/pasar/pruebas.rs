//! Pruebas de pasar decks en el mismo proceso: dos nodos de iroh sin relé
//! (direcciones locales), cada uno con su biblioteca (`AppState`) y un
//! entorno que apunta lo que la pantalla vería.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::{Endpoint, SecretKey};
use proxy_core::model::{
    BackSpec, CardEdits, CardMeta, CardSource, Deck, DeckCard, ImageOrigin, MarcaCarta,
};

use super::tipos::{
    Decision, Destino, Eleccion, EstadoEmisor, EstadoReceptor, EstadoResultado, FaseEmisor,
    FaseReceptor, Resolucion,
};
use super::{encontrar, enviar, recibir, Entorno};
use crate::red::nodo::{crear, Relay};
use crate::state::{AppState, Paths};
use crate::store::blobs::tests::png;
use crate::store::{decks, linaje};

type GanchoEmisor = Arc<dyn Fn(&EstadoEmisor) + Send + Sync>;

/// Un equipo de prueba: su biblioteca y lo que la pantalla ha visto.
#[derive(Clone)]
pub(crate) struct Prueba {
    pub st: Arc<AppState>,
    pub receptor: Arc<Mutex<Vec<EstadoReceptor>>>,
    pub emisor: Arc<Mutex<Vec<EstadoEmisor>>>,
    pub al_emitir_emisor: Arc<Mutex<Option<GanchoEmisor>>>,
    /// Hace como si hubiera una partida en red.
    pub partida: Arc<std::sync::atomic::AtomicBool>,
    _dir: Arc<tempfile::TempDir>,
}

impl Entorno for Prueba {
    fn estado(&self) -> &AppState {
        &self.st
    }

    fn emitir_receptor(&self, e: &EstadoReceptor) {
        if let Ok(mut v) = self.receptor.lock() {
            v.push(e.clone());
        }
    }

    fn emitir_emisor(&self, e: &EstadoEmisor) {
        if let Ok(mut v) = self.emisor.lock() {
            v.push(e.clone());
        }
        let g = self.al_emitir_emisor.lock().ok().and_then(|g| g.clone());
        if let Some(f) = g {
            f(e);
        }
    }

    fn emitir_cerca(&self, _c: &[super::tipos::Cerca]) {}

    fn partida_en_red(&self) -> bool {
        self.partida.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub(crate) fn equipo() -> Prueba {
    let dir = tempfile::tempdir().unwrap();
    let st = AppState::open(Paths {
        data: dir.path().join("data"),
        cache: dir.path().join("cache"),
    })
    .unwrap();
    Prueba {
        st: Arc::new(st),
        receptor: Arc::default(),
        emisor: Arc::default(),
        al_emitir_emisor: Arc::default(),
        partida: Arc::default(),
        _dir: Arc::new(dir),
    }
}

pub(crate) async fn nodo() -> Endpoint {
    crear(SecretKey::generate(), Relay::Ninguno).await.unwrap()
}

fn carta(
    id: &str,
    nombre: &str,
    source: CardSource,
    semilla: u8,
    e: &Prueba,
    origen: ImageOrigin,
) -> DeckCard {
    let img =
        e.st.blobs
            .put_bytes(&e.st.db, &png(120, 168, semilla), origen)
            .unwrap();
    DeckCard {
        id: id.into(),
        source,
        front: Some(img),
        back: BackSpec::DeckDefault,
        edits: CardEdits::default(),
        locked: false,
        meta: CardMeta {
            name: nombre.into(),
            ..CardMeta::default()
        },
        design: None,
        marca: None,
    }
}

/// Un deck guardado: una carta de Scryfall y, si `con_proxies`, una propia
/// vinculada y otra sin vínculo.
pub(crate) fn deck(e: &Prueba, nombre: &str, con_proxies: bool, semilla: u8) -> Deck {
    let reverso = e.st.blobs.generic_back(&e.st.db).unwrap();
    let mut d = decks::create(&e.st.db, nombre, Some(reverso)).unwrap();
    d.cards.push(carta(
        &format!("{nombre}-s"),
        "Sol Ring",
        CardSource::Scryfall {
            scryfall_id: "sol".into(),
        },
        semilla,
        e,
        ImageOrigin::ScryfallPng,
    ));
    if con_proxies {
        let mut p = carta(
            &format!("{nombre}-p"),
            "Lightning Bolt",
            CardSource::Custom {
                scryfall_id: Some("bolt".into()),
                externa: None,
            },
            semilla + 1,
            e,
            ImageOrigin::Upload,
        );
        p.marca = Some(MarcaCarta::Opcional);
        d.cards.push(p);
        d.cards.push(carta(
            &format!("{nombre}-r"),
            "Mi carta",
            CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            semilla + 2,
            e,
            ImageOrigin::Upload,
        ));
    }
    decks::save(&e.st.db, &d).unwrap();
    d
}

pub(crate) async fn esperar<T>(que: &str, f: impl Fn() -> Option<T>) -> T {
    for _ in 0..600 {
        if let Some(x) = f() {
            return x;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no llegó: {que}");
}

async fn esperar_estado<T: std::fmt::Debug>(
    que: &str,
    actual: impl Fn() -> Option<T>,
    vale: impl Fn(&T) -> bool,
) -> T {
    for _ in 0..600 {
        if let Some(x) = actual().filter(&vale) {
            return x;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no llegó: {que}; ahora: {:?}", actual());
}

pub(crate) async fn fase_receptor(e: &Prueba, f: FaseReceptor) -> EstadoReceptor {
    esperar_estado(
        &format!("el receptor en {f:?}"),
        || recibir::estado(e),
        |s| s.fase == f,
    )
    .await
}

pub(crate) async fn fase_emisor(e: &Prueba, f: FaseEmisor) -> EstadoEmisor {
    esperar_estado(
        &format!("el emisor en {f:?}"),
        || enviar::estado(e),
        |s| s.fase == f,
    )
    .await
}

fn nombres(e: &Prueba) -> Vec<String> {
    let mut v: Vec<String> = decks::list(&e.st.db)
        .unwrap()
        .into_iter()
        .map(|d| d.name)
        .collect();
    v.sort();
    v
}

fn carpeta_vacia(e: &Prueba) -> bool {
    std::fs::read_dir(super::carpeta_temporal(&e.st))
        .map(|mut d| d.next().is_none())
        .unwrap_or(true)
}

fn dec(i: u32, eleccion: Eleccion, resolucion: Option<Resolucion>) -> Decision {
    Decision {
        indice: i,
        eleccion,
        resolucion,
    }
}

/// B abre «Recibir» y devuelve su código.
async fn abrir(b: &Prueba, ep: &Endpoint) -> String {
    recibir::abrir(b, ep.clone(), "Beto", false)
        .await
        .unwrap()
        .codigo
}

#[tokio::test]
async fn tres_decks_uno_con_proxies_uno_solo_originales_y_uno_rechazado() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let d1 = deck(&a, "Con proxies", true, 10);
    let d2 = deck(&a, "Solo originales", true, 20);
    let d3 = deck(&a, "Rechazado", false, 30);
    let codigo = abrir(&b, &ep_b).await;

    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto: codigo },
        vec![d1.id.clone(), d2.id.clone(), d3.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();

    // La oferta, con lo que cuenta cada deck.
    let s = fase_receptor(&b, FaseReceptor::Oferta).await;
    assert!(s.remoto.as_ref().unwrap().huella.len() > 3);
    assert_eq!(s.oferta.len(), 3);
    assert_eq!(s.oferta[0].deck.nombre, "Con proxies");
    assert!(s.oferta[0].deck.proxies);
    assert!(!s.oferta[2].deck.proxies);
    assert!(s.oferta[0].deck.tamano_originales < s.oferta[0].deck.tamano);
    assert!(s.oferta[0].deck.portada.is_some());
    assert!(s.oferta.iter().all(|d| d.conflicto.is_none()));
    // Y el emisor ya espera la respuesta.
    let e = fase_emisor(&a, FaseEmisor::EsperandoRespuesta).await;
    assert_eq!(e.decks.len(), 3);
    assert_eq!(e.remoto.unwrap().apodo, "");

    recibir::responder(
        &b,
        vec![
            dec(0, Eleccion::ConProxies, None),
            dec(1, Eleccion::SoloOriginales, None),
            dec(2, Eleccion::No, None),
        ],
    )
    .unwrap();
    let fin = fase_receptor(&b, FaseReceptor::Terminado).await;
    fase_emisor(&a, FaseEmisor::Terminado).await;

    // En B: dos decks; el rechazado no está.
    assert_eq!(nombres(&b), vec!["Con proxies", "Solo originales"]);
    let lista = decks::list(&b.st.db).unwrap();
    let id_de = |n: &str| lista.iter().find(|d| d.name == n).unwrap().id.clone();
    let con = decks::open(&b.st.db, &id_de("Con proxies")).unwrap();
    assert_eq!(con.cards.len(), 3);
    assert!(con.cards.iter().all(|c| c.front.is_some()));
    assert_eq!(con.cards[1].marca, Some(MarcaCarta::Opcional));
    let sin = decks::open(&b.st.db, &id_de("Solo originales")).unwrap();
    assert_eq!(sin.cards.len(), 3);
    // La de Scryfall conserva su imagen; la vinculada espera la suya; la
    // sin vínculo es un hueco con su nombre.
    assert!(sin.cards[0].front.is_some());
    assert_eq!(
        sin.cards[1].source,
        CardSource::Scryfall {
            scryfall_id: "bolt".into()
        }
    );
    assert!(sin.cards[1].front.is_none());
    assert_eq!(sin.cards[2].source, CardSource::Blank);
    assert_eq!(sin.cards[2].meta.name, "Mi carta");
    // Y el linaje es el de A.
    assert_eq!(
        linaje::de(&b.st.db, &con.id).unwrap().linaje,
        linaje::de(&a.st.db, &d1.id).unwrap().linaje
    );
    // Los resultados, en las dos pantallas.
    let estados: Vec<_> = fin
        .resultados
        .iter()
        .map(|r| (r.nombre.as_str(), r.estado))
        .collect();
    assert!(estados.contains(&("Rechazado", EstadoResultado::Rechazado)));
    assert!(estados.contains(&("Con proxies", EstadoResultado::Hecho)));
    assert!(fin
        .resultados
        .iter()
        .any(|r| r.nombre == "Solo originales" && r.faltan_imagenes == 1));
    let e = enviar::estado(&a).unwrap();
    assert_eq!(
        e.resultados
            .iter()
            .filter(|r| r.estado == EstadoResultado::Hecho)
            .count(),
        2
    );
    // En A no ha cambiado nada, y no queda nada en la caché de ninguno.
    assert_eq!(decks::list(&a.st.db).unwrap().len(), 3);
    assert!(carpeta_vacia(&a));
    assert!(carpeta_vacia(&b));
    // El código era de un solo uso y «Recibir» ya acabó: otro intento falla
    // al momento, no tras un plazo.
    let _ = (d1, d2, d3);
}

#[tokio::test]
async fn un_deck_optimizado_llega_en_jpeg_y_el_de_aqui_no_cambia() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let mut x = 9u32;
    let foto = image::RgbaImage::from_fn(400, 300, |i, j| {
        x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let r = (x >> 28) as u8;
        image::Rgba([(i / 2) as u8 + r, (j / 2) as u8 + r, r, 255])
    });
    let png = proxy_core::imaging::encode::png(&foto).unwrap();
    let propia =
        a.st.blobs
            .put_bytes(&a.st.db, &png, ImageOrigin::Upload)
            .unwrap();
    let mut d = deck(&a, "Ligero", true, 10);
    d.cards[1].front = Some(propia.clone());
    decks::save(&a.st.db, &d).unwrap();
    let codigo = abrir(&b, &ep_b).await;

    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto: codigo },
        vec![d.id.clone()],
        crate::package::Modo::Optimizado,
    )
    .await
    .unwrap();
    let s = fase_receptor(&b, FaseReceptor::Oferta).await;
    assert!(s.oferta[0].deck.optimizado);
    recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
    fase_receptor(&b, FaseReceptor::Terminado).await;
    fase_emisor(&a, FaseEmisor::Terminado).await;

    let lista = decks::list(&b.st.db).unwrap();
    let n = decks::open(&b.st.db, &lista[0].id).unwrap();
    let f = n.cards[1].front.as_ref().unwrap();
    assert_ne!(f.blob, propia.blob);
    assert_eq!(
        b.st.blobs.ficha(&b.st.db, &f.blob).unwrap().unwrap().ext,
        "jpg"
    );
    assert_eq!((f.width, f.height), (400, 300));
    // El que envía conserva su PNG.
    assert_eq!(decks::open(&a.st.db, &d.id).unwrap(), d);
    assert!(a.st.blobs.locate(&a.st.db, &propia.blob).unwrap().is_some());
    ep_a.close().await;
    ep_b.close().await;
}

#[tokio::test]
async fn un_codigo_de_un_solo_uso_y_uno_malo_no_dejan_pasar() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let d = deck(&a, "D", false, 1);
    let texto = abrir(&b, &ep_b).await;

    // Un código de partida pegado aquí da su error, sin llamar a nadie.
    let c = crate::red::codigo::Codigo::de_texto_deck(&texto).unwrap();
    let de_partida = c.a_texto().unwrap();
    let e = enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto: de_partida },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap_err();
    assert_eq!(e.params["motivo"], "codigo_de_partida");

    // Un secreto que no es: rechazado, y «Recibir» sigue esperando.
    let malo = crate::red::codigo::Codigo {
        addr: c.addr.clone(),
        secreto: crate::red::codigo::secreto_nuevo(),
    }
    .a_texto_deck()
    .unwrap();
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto: malo },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    let s = fase_emisor(&a, FaseEmisor::Error).await;
    assert_eq!(s.error.unwrap().params["motivo"], "codigo");
    assert_eq!(recibir::estado(&b).unwrap().fase, FaseReceptor::Esperando);
    enviar::olvidar(&a);

    // El bueno entra una vez...
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo {
            texto: texto.clone(),
        },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(&b, vec![dec(0, Eleccion::No, None)]).unwrap();
    fase_receptor(&b, FaseReceptor::Terminado).await;
    fase_emisor(&a, FaseEmisor::Terminado).await;
    assert!(decks::list(&b.st.db).unwrap().is_empty());
    enviar::olvidar(&a);

    // ...y a la segunda, «Recibir» ya no espera a nadie y se entera al momento.
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    let s = fase_emisor(&a, FaseEmisor::Error).await;
    assert_eq!(s.error.unwrap().params["motivo"], "no_espera");
}

#[tokio::test]
async fn un_nodo_que_no_espera_un_deck_cierra_la_conexion_sin_leer() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    // B atiende `deck/1` pero no ha abierto «Recibir».
    super::registrar(&b, ep_b.clone());
    let conn = ep_a
        .connect(ep_b.addr(), crate::red::nodo::ALPN_DECK)
        .await
        .unwrap();
    let razon = tokio::time::timeout(Duration::from_secs(5), conn.closed())
        .await
        .expect("la conexión se cierra");
    match razon {
        iroh::endpoint::ConnectionError::ApplicationClosed(c) => {
            assert_eq!(c.error_code, super::CIERRE_NO_ESPERADO.into());
        }
        otro => panic!("se esperaba un cierre de la app: {otro:?}"),
    }
    assert!(recibir::estado(&b).is_none());
    let _ = a;
}

/// Cerrar «Recibir» retira el bucle propio, y con él vivo una conexión de la
/// mesa (la sucesión del anfitrión) no se corta si hay una partida en red.
#[tokio::test]
async fn cerrar_recibir_suelta_el_nodo_y_la_mesa_no_se_corta_con_partida() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let mesa = crate::red::nodo::ALPN_MESA;

    // Sin partida, lo de la mesa se corta al momento.
    abrir(&b, &ep_b).await;
    let conn = ep_a.connect(ep_b.addr(), mesa).await.unwrap();
    let razon = tokio::time::timeout(Duration::from_secs(5), conn.closed())
        .await
        .expect("se cierra");
    assert!(matches!(
        razon,
        iroh::endpoint::ConnectionError::ApplicationClosed(c)
            if c.error_code == super::CIERRE_NO_ESPERADO.into()
    ));

    // Con una partida en red, se retiene sin cerrarla.
    b.partida.store(true, std::sync::atomic::Ordering::SeqCst);
    let conn = ep_a.connect(ep_b.addr(), mesa).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(800), conn.closed())
            .await
            .is_err(),
        "no se cierra"
    );
    drop(conn);

    // Al cerrar «Recibir», el bucle se retira: nadie atiende el nodo.
    recibir::cerrar(&b);
    assert!(super::cerrojo(&super::REGISTRO).get(&ep_b.id()).is_none());
    // Y al cancelar sin intercambio, igual.
    abrir(&b, &ep_b).await;
    assert!(super::cerrojo(&super::REGISTRO).get(&ep_b.id()).is_some());
    recibir::cancelar(&b);
    assert!(super::cerrojo(&super::REGISTRO).get(&ep_b.id()).is_none());
    let _ = a;
}

#[tokio::test]
async fn un_deck_que_ya_esta_pregunta_y_reemplazar_conserva_lo_de_aqui() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let mut d = deck(&a, "Mi deck", true, 10);

    // La primera vez, tal cual.
    let texto = abrir(&b, &ep_b).await;
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
    fase_receptor(&b, FaseReceptor::Terminado).await;
    fase_emisor(&a, FaseEmisor::Terminado).await;
    let en_b = decks::list(&b.st.db).unwrap();
    assert_eq!(en_b.len(), 1);
    let id_b = en_b[0].id.clone();
    // B lo hace suyo: etiqueta, notas y favorito.
    let t = crate::store::etiquetas::create(&b.st.db, "Mía", "blue").unwrap();
    crate::store::etiquetas::etiquetar(
        &b.st.db,
        std::slice::from_ref(&id_b),
        std::slice::from_ref(&t.id),
        &[],
    )
    .unwrap();
    crate::store::etiquetas::notas(&b.st.db, &id_b, "mis notas").unwrap();
    crate::store::etiquetas::favorito(&b.st.db, std::slice::from_ref(&id_b), true).unwrap();
    enviar::olvidar(&a);

    // A cambia el deck (una carta más) y lo manda otra vez.
    std::thread::sleep(Duration::from_millis(5));
    let extra = carta(
        "extra",
        "Nueva",
        CardSource::Scryfall {
            scryfall_id: "n".into(),
        },
        99,
        &a,
        ImageOrigin::ScryfallPng,
    );
    d.cards.push(extra);
    decks::save(&a.st.db, &d).unwrap();
    let texto = abrir(&b, &ep_b).await;
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    let s = fase_receptor(&b, FaseReceptor::Oferta).await;
    let c = s.oferta[0].conflicto.clone().expect("ya está aquí");
    assert_eq!(c.deck_id, id_b);
    assert_eq!(c.mas_reciente, super::tipos::MasReciente::Llega);
    // Sin decir qué hacer, no vale.
    assert!(recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).is_ok());
    let s = fase_receptor(&b, FaseReceptor::Error).await;
    assert_eq!(s.error.unwrap().params["motivo"], "falta_resolucion");
    fase_emisor(&a, FaseEmisor::Cancelado).await;
    enviar::olvidar(&a);
    assert_eq!(decks::open(&b.st.db, &id_b).unwrap().cards.len(), 3);

    // Ahora sí: reemplazar.
    let texto = abrir(&b, &ep_b).await;
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(
        &b,
        vec![dec(0, Eleccion::ConProxies, Some(Resolucion::Reemplazar))],
    )
    .unwrap();
    let fin = fase_receptor(&b, FaseReceptor::Terminado).await;
    assert!(fin.resultados[0].reemplazado);
    let lista = decks::list(&b.st.db).unwrap();
    assert_eq!(lista.len(), 1, "sigue habiendo uno");
    assert_eq!(lista[0].id, id_b, "con su id");
    assert_eq!(lista[0].tags, vec![t.id]);
    assert_eq!(lista[0].notes, "mis notas");
    assert!(lista[0].pinned);
    assert_eq!(decks::open(&b.st.db, &id_b).unwrap().cards.len(), 4);
    enviar::olvidar(&a);

    // Y «Guardar los dos» deja los dos, el nuevo con otro linaje.
    let texto = abrir(&b, &ep_b).await;
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(
        &b,
        vec![dec(
            0,
            Eleccion::ConProxies,
            Some(Resolucion::GuardarLosDos),
        )],
    )
    .unwrap();
    fase_receptor(&b, FaseReceptor::Terminado).await;
    let lista = decks::list(&b.st.db).unwrap();
    assert_eq!(lista.len(), 2);
    let nuevo = lista.iter().find(|x| x.id != id_b).unwrap();
    assert_eq!(nuevo.name, "Mi deck (importado)");
    assert_eq!(linaje::de(&b.st.db, &nuevo.id).unwrap().linaje, nuevo.id);
}

#[tokio::test]
async fn cortar_a_la_mitad_no_deja_nada_a_medias() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    // Un deck de varios cientos de KB (los PNG de prueba van sin comprimir).
    let reverso = a.st.blobs.generic_back(&a.st.db).unwrap();
    let mut d = decks::create(&a.st.db, "Grande", Some(reverso)).unwrap();
    for i in 0..4u8 {
        let img =
            a.st.blobs
                .put_bytes(&a.st.db, &png(400, 560, 40 + i), ImageOrigin::Upload)
                .unwrap();
        d.cards.push(DeckCard {
            front: Some(img),
            ..carta(
                &format!("g{i}"),
                "x",
                CardSource::Blank,
                0,
                &a,
                ImageOrigin::Upload,
            )
        });
    }
    decks::save(&a.st.db, &d).unwrap();
    // El emisor corta en cuanto ve avanzar el primer paquete.
    let cortado = Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let (a2, c2) = (a.clone(), cortado.clone());
        *a.al_emitir_emisor.lock().unwrap() = Some(Arc::new(move |s: &EstadoEmisor| {
            if let Some(p) = &s.progreso {
                if p.hechos > 0
                    && p.hechos < p.total
                    && !c2.swap(true, std::sync::atomic::Ordering::SeqCst)
                {
                    enviar::cancelar(&a2);
                }
            }
        }));
    }
    let texto = abrir(&b, &ep_b).await;
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
    fase_emisor(&a, FaseEmisor::Cancelado).await;
    fase_receptor(&b, FaseReceptor::Cancelado).await;
    assert!(cortado.load(std::sync::atomic::Ordering::SeqCst));
    // Nada en la biblioteca de B, ni en la caché de los dos.
    assert!(decks::list(&b.st.db).unwrap().is_empty());
    esperar("la caché limpia", || {
        (carpeta_vacia(&a) && carpeta_vacia(&b)).then_some(())
    })
    .await;
}

#[tokio::test]
async fn cancelar_desde_quien_recibe_corta_el_envio() {
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let d = deck(&a, "D", true, 5);
    let texto = abrir(&b, &ep_b).await;
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::cancelar(&b);
    fase_receptor(&b, FaseReceptor::Cancelado).await;
    fase_emisor(&a, FaseEmisor::Cancelado).await;
    assert!(decks::list(&b.st.db).unwrap().is_empty());
}

/// Un emisor tramposo: habla el protocolo a mano.
mod tramposo {
    use super::*;
    use crate::pasar::protocolo::{escribir, leer, AlEmisor, AlReceptor, VERSION};

    async fn hasta_la_oferta(
        ep: &Endpoint,
        destino: iroh::EndpointAddr,
        secreto: String,
    ) -> (
        iroh::endpoint::Connection,
        iroh::endpoint::SendStream,
        iroh::endpoint::RecvStream,
    ) {
        let conn = ep
            .connect(destino, crate::red::nodo::ALPN_DECK)
            .await
            .unwrap();
        let (mut tx, mut rx) = conn.open_bi().await.unwrap();
        escribir(
            &mut tx,
            &AlReceptor::Hola {
                version: VERSION,
                secreto: Some(secreto),
                apodo: "Mal\nicioso\u{1b}[31m".into(),
            },
        )
        .await
        .unwrap();
        let r: AlEmisor = leer(&mut rx).await.unwrap();
        assert!(matches!(r, AlEmisor::Listo { .. }));
        (conn, tx, rx)
    }

    #[tokio::test]
    async fn una_oferta_con_mil_decks_corta_y_el_apodo_llega_limpio() {
        let b = equipo();
        let (ep_a, ep_b) = (nodo().await, nodo().await);
        let texto = abrir(&b, &ep_b).await;
        let c = crate::red::codigo::Codigo::de_texto_deck(&texto).unwrap();
        let (conn, mut tx, _rx) = hasta_la_oferta(&ep_a, c.addr, c.secreto).await;
        // El apodo de fuera, limpio.
        let s = recibir::estado(&b).unwrap();
        assert_eq!(s.remoto.unwrap().apodo, "Malicioso[31m");
        // Una trama mayor que el tope ni se lee.
        tx.write_all(&(70_000u32).to_be_bytes()).await.unwrap();
        let s = fase_receptor(&b, FaseReceptor::Error).await;
        assert_eq!(s.error.unwrap().params["motivo"], "protocolo");
        drop(conn);
    }

    #[tokio::test]
    async fn un_paquete_mas_grande_que_el_tope_o_con_otra_huella_se_corta() {
        let b = equipo();
        let (ep_a, ep_b) = (nodo().await, nodo().await);
        let texto = abrir(&b, &ep_b).await;
        let c = crate::red::codigo::Codigo::de_texto_deck(&texto).unwrap();
        let (_conn, mut tx, mut rx) = hasta_la_oferta(&ep_a, c.addr, c.secreto).await;
        let oferta = super::super::tipos::DeckOfrecido {
            nombre: "X".into(),
            cartas: 1,
            proxies: false,
            tamano: 10,
            tamano_originales: 10,
            linaje: None,
            actualizado: None,
            portada: None,
            optimizado: false,
        };
        escribir(
            &mut tx,
            &AlReceptor::Oferta {
                decks: vec![oferta],
            },
        )
        .await
        .unwrap();
        fase_receptor(&b, FaseReceptor::Oferta).await;
        recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
        let _: AlEmisor = leer(&mut rx).await.unwrap();
        escribir(
            &mut tx,
            &AlReceptor::Paquete {
                indice: 0,
                tamano: crate::pasar::protocolo::MAX_PAQUETE + 1,
                huella: "0".repeat(64),
            },
        )
        .await
        .unwrap();
        let s = fase_receptor(&b, FaseReceptor::Error).await;
        assert_eq!(s.error.unwrap().params["motivo"], "paquete");
        assert!(decks::list(&b.st.db).unwrap().is_empty());
    }

    #[tokio::test]
    async fn un_paquete_con_la_huella_cambiada_no_se_importa() {
        let b = equipo();
        let (ep_a, ep_b) = (nodo().await, nodo().await);
        let texto = abrir(&b, &ep_b).await;
        let c = crate::red::codigo::Codigo::de_texto_deck(&texto).unwrap();
        let (_conn, mut tx, mut rx) = hasta_la_oferta(&ep_a, c.addr, c.secreto).await;
        let oferta = super::super::tipos::DeckOfrecido {
            nombre: "X".into(),
            cartas: 1,
            proxies: false,
            tamano: 10,
            tamano_originales: 10,
            linaje: None,
            actualizado: None,
            portada: None,
            optimizado: false,
        };
        escribir(
            &mut tx,
            &AlReceptor::Oferta {
                decks: vec![oferta],
            },
        )
        .await
        .unwrap();
        fase_receptor(&b, FaseReceptor::Oferta).await;
        recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
        let _: AlEmisor = leer(&mut rx).await.unwrap();
        escribir(
            &mut tx,
            &AlReceptor::Paquete {
                indice: 0,
                tamano: 5,
                huella: "ab".repeat(32),
            },
        )
        .await
        .unwrap();
        tx.write_all(b"hola!").await.unwrap();
        let s = fase_receptor(&b, FaseReceptor::Error).await;
        assert_eq!(s.error.unwrap().params["motivo"], "huella");
        assert!(decks::list(&b.st.db).unwrap().is_empty());
    }
}

#[tokio::test]
async fn el_codigo_de_6_cifras_acepta_a_alguien_de_la_red_local() {
    // Sin secreto, solo vale si «Recibir» se anuncia en la red local; aquí se
    // fuerza el anuncio (sin mDNS de verdad) para probar la aceptación.
    let (a, b) = (equipo(), equipo());
    let (ep_a, ep_b) = (nodo().await, nodo().await);
    let d = deck(&a, "D", false, 1);
    abrir(&b, &ep_b).await;
    {
        let mut g = super::cerrojo(&b.st.pasar.receptor);
        g.as_mut().unwrap().estado.cerca = true;
    }
    // A conoce la dirección de B (en la app la da la lista de «cerca»): la de
    // la propia máquina, que es de la red local aunque esta tenga IP pública.
    let mut dir = iroh::EndpointAddr::new(ep_b.id());
    dir.addrs
        .extend(ep_b.addr().addrs.iter().filter_map(|a| match a {
            iroh::TransportAddr::Ip(sa) if sa.is_ipv4() => Some(iroh::TransportAddr::Ip(
                std::net::SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), sa.port()),
            )),
            _ => None,
        }));
    *super::cerrojo(&a.st.pasar.busqueda) = Some(super::encontrar::Busqueda::de_prueba(
        ep_b.id(),
        "Beto",
        dir,
    ));
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Cerca {
            id: ep_b.id().to_string(),
        },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    let s = fase_receptor(&b, FaseReceptor::Peticion).await;
    let codigo = s.codigo6.unwrap();
    assert_eq!(codigo, encontrar::codigo6(ep_a.id(), ep_b.id()));
    let e = fase_emisor(&a, FaseEmisor::EsperandoAceptacion).await;
    assert_eq!(e.codigo6.unwrap(), codigo);
    super::aceptar_peticion(&b, true);
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
    fase_receptor(&b, FaseReceptor::Terminado).await;
    assert_eq!(decks::list(&b.st.db).unwrap().len(), 1);

    // Si no se acepta, no entra.
    enviar::olvidar(&a);
    abrir(&b, &ep_b).await;
    {
        let mut g = super::cerrojo(&b.st.pasar.receptor);
        g.as_mut().unwrap().estado.cerca = true;
    }
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Cerca {
            id: ep_b.id().to_string(),
        },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Peticion).await;
    super::aceptar_peticion(&b, false);
    let s = fase_emisor(&a, FaseEmisor::Error).await;
    assert_eq!(s.error.unwrap().params["motivo"], "rechazado");
    fase_receptor(&b, FaseReceptor::Esperando).await;
}

/// El código de pasar un deck vale a distancia: por el relé de n0 y con las
/// direcciones directas quitadas. Necesita internet: sin él, se salta.
#[tokio::test]
async fn el_codigo_vale_por_el_rele_sin_direcciones_directas() {
    let (a, b) = (equipo(), equipo());
    let ep_a = crear(SecretKey::generate(), Relay::N0).await.unwrap();
    let ep_b = crear(SecretKey::generate(), Relay::N0).await.unwrap();
    if tokio::time::timeout(Duration::from_secs(15), async {
        ep_a.online().await;
        ep_b.online().await;
    })
    .await
    .is_err()
    {
        eprintln!("sin relé de n0 desde esta máquina: se salta");
        return;
    }
    let d = deck(&a, "Lejos", true, 70);
    let abierto = recibir::abrir(&b, ep_b.clone(), "Beto", true)
        .await
        .unwrap();
    // El código lleva el relé.
    let c = crate::red::codigo::Codigo::de_texto_deck(&abierto.codigo).unwrap();
    assert!(c.addr.addrs.iter().any(|x| x.is_relay()));
    // Sin direcciones directas: solo se llega por el relé.
    let solo_rele = crate::red::codigo::Codigo {
        addr: encontrar::sin_directas(&c.addr),
        secreto: c.secreto.clone(),
    }
    .a_texto_deck()
    .unwrap();
    enviar::enviar(
        &a,
        ep_a.clone(),
        Destino::Codigo { texto: solo_rele },
        vec![d.id.clone()],
        crate::package::Modo::Original,
    )
    .await
    .unwrap();
    fase_receptor(&b, FaseReceptor::Oferta).await;
    recibir::responder(&b, vec![dec(0, Eleccion::ConProxies, None)]).unwrap();
    fase_receptor(&b, FaseReceptor::Terminado).await;
    fase_emisor(&a, FaseEmisor::Terminado).await;
    assert_eq!(nombres(&b), vec!["Lejos"]);
}
