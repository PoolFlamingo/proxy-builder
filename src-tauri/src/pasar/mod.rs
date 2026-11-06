//! **Pasar decks entre equipos** (plan del móvil, Fase M5): enviar un deck a
//! otro equipo —del ordenador al móvil y al revés, o a un amigo— por iroh,
//! sin cuenta y sin servidor.
//!
//! - `protocolo.rs`: el ALPN `proxy-builder/deck/1`, sus tramas y topes.
//! - `encontrar.rs`: equipos cerca (mDNS, con su pase), el código de 8 cifras,
//!   el código en texto `PBD1-…` y su QR.
//! - `recibir.rs` / `enviar.rs`: los dos lados de un intercambio.
//! - `imagenes.rs`: bajar las imágenes que faltan tras recibir «solo
//!   originales».
//! - `tipos.rs`: lo que ve la pantalla (`ts-rs`).
//!
//! 🛑 **Un solo nodo por instalación** (`red::Red::nodo`): el mismo `Endpoint`
//! que la mesa, con el ALPN `deck/1` entre los suyos. Quien atiende lo que
//! entra se reparte así: mientras hay una partida (anfitrión de código
//! directo o sala del servidor) lo hacen sus bucles de `accept`, que mandan
//! `deck/1` a [`atender`]; el resto del tiempo, el bucle propio de aquí
//! (`Registro`). Nunca dos a la vez: dos bucles de `accept` se repartirían las
//! conexiones al azar. [`ceder`] los coordina.
//!
//! 🛑 Todo lo que llega del otro equipo es entrada no confiable: tramas con
//! tope, oferta validada, paquete con tamaño y huella, y `package::import`,
//! que no se fía de nada. Nada llega a la biblioteca a medias.
//!
//! Vale en cualquier sitio: con el código, dos equipos se encuentran por internet
//! (también con datos móviles) y, si no consiguen hablarse directamente, los datos
//! pasan por un relé, cifrados de punta a punta (el relé no puede leerlos).

pub mod encontrar;
pub mod enviar;
pub mod imagenes;
pub mod protocolo;
#[cfg(test)]
pub(crate) mod pruebas;
pub mod recibir;
pub mod tipos;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointId};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::oneshot;
use tokio::task::AbortHandle;

use crate::red::nodo::ALPN_DECK;
use crate::state::AppState;
use encontrar::Busqueda;
use tipos::{Cerca, Decision, EstadoEmisor, EstadoReceptor};

/// Con qué código se cierra una conexión `deck/1` que no se espera.
pub const CIERRE_NO_ESPERADO: u32 = 40;
/// Con qué código se cancela un intercambio.
pub const CIERRE_CANCELADO: u32 = 41;
/// Con qué código se cierra uno que se acaba bien.
pub const CIERRE_FIN: u32 = 0;
/// Con qué código corta quien recibe si el deck no le cabe en el disco.
pub const CIERRE_SIN_ESPACIO: u32 = 42;

/// Lo que necesita un intercambio de la app, detrás de un trait para poder
/// probarlo sin Tauri (como `red::mesa::Mesa`).
pub trait Entorno: Clone + Send + Sync + 'static {
    fn estado(&self) -> &AppState;
    /// Lo que cambia en «Recibir».
    fn emitir_receptor(&self, e: &EstadoReceptor);
    /// Lo que cambia en «Enviar».
    fn emitir_emisor(&self, e: &EstadoEmisor);
    /// La lista de equipos cerca.
    fn emitir_cerca(&self, c: &[Cerca]);
    /// El `MulticastLock` de la Wi-Fi (Android): sin él se tira el mDNS.
    /// Cada uso lo pide y lo suelta por su cuenta (`android::multicast` cuenta).
    fn multicast(&self, _uso: crate::android::UsoMulticast, _si: bool) {}
    /// Que el móvil no apague la pantalla durante un intercambio: con la
    /// pantalla apagada, Android acaba cortando la red de la app.
    fn pantalla_encendida(&self, _si: bool) {}
    /// El nombre del equipo según el sistema, para quien no ha elegido uno.
    fn nombre_del_sistema(&self) -> Option<String> {
        None
    }
    /// Si hay una partida en red (o una sala) que pueda esperar conexiones
    /// de la mesa en este nodo (la sucesión del anfitrión, Fase K).
    fn partida_en_red(&self) -> bool {
        self.estado().red.ocupada()
    }
}

impl<Rt: Runtime> Entorno for AppHandle<Rt> {
    fn estado(&self) -> &AppState {
        self.state::<AppState>().inner()
    }

    fn emitir_receptor(&self, e: &EstadoReceptor) {
        if let Err(err) = self.emit("pasar://receptor", e) {
            tracing::warn!(error = %err, "pasar: no se pudo avisar a la pantalla");
        }
    }

    fn emitir_emisor(&self, e: &EstadoEmisor) {
        if let Err(err) = self.emit("pasar://emisor", e) {
            tracing::warn!(error = %err, "pasar: no se pudo avisar a la pantalla");
        }
    }

    fn emitir_cerca(&self, c: &[Cerca]) {
        if let Err(err) = self.emit("pasar://cerca", c) {
            tracing::warn!(error = %err, "pasar: no se pudo avisar a la pantalla");
        }
    }

    fn nombre_del_sistema(&self) -> Option<String> {
        // `android::nombre_equipo` es de la app con el runtime de siempre.
        let cualquiera: &dyn std::any::Any = self;
        cualquiera
            .downcast_ref::<AppHandle>()
            .and_then(crate::android::nombre_equipo)
    }

    fn multicast(&self, uso: crate::android::UsoMulticast, si: bool) {
        // `android::multicast` es de la app con el runtime de siempre.
        let cualquiera: &dyn std::any::Any = self;
        if let Some(app) = cualquiera.downcast_ref::<AppHandle>() {
            if let Err(e) = crate::android::multicast(app, uso, si) {
                tracing::warn!(codigo = ?e.code, "no se pudo cambiar el MulticastLock");
            }
        }
    }

    fn pantalla_encendida(&self, si: bool) {
        let cualquiera: &dyn std::any::Any = self;
        if let Some(app) = cualquiera.downcast_ref::<AppHandle>() {
            let uso = crate::android::UsoPantalla::Pasar;
            if let Err(e) = crate::android::pantalla_encendida(app, uso, si) {
                tracing::warn!(codigo = ?e.code, "no se pudo cambiar la pantalla encendida");
            }
        }
    }
}

/// El nombre con el que este equipo se presenta a los demás: el que eligió la
/// persona («Recibir») o, si no, el que le da el sistema.
pub fn nombre_equipo<E: Entorno>(e: &E) -> String {
    let st = e.estado();
    crate::red::identidad::leer(&st.db, &st.red.clave())
        .map(|i| i.apodo)
        .ok()
        .filter(|a| !a.is_empty())
        .or_else(|| e.nombre_del_sistema())
        .map(|n| protocolo::apodo_limpio(&n))
        .unwrap_or_default()
}

// ── El estado de la app ─────────────────────────────────────────────────

/// Lo que la app tiene en marcha de pasar decks.
#[derive(Default)]
pub struct Pasar {
    pub(crate) receptor: Mutex<Option<recibir::Receptor>>,
    pub(crate) emisor: Mutex<Option<enviar::Emisor>>,
    pub(crate) busqueda: Mutex<Option<Busqueda>>,
    generacion: AtomicU64,
    /// Sube cada vez que se pide abrir o cerrar «Recibir»: un `abrir` que aún
    /// espera a tener relé sabe así que ya no lo quiere nadie.
    pedido_recibir: AtomicU64,
}

impl Pasar {
    pub(crate) fn nueva_generacion(&self) -> u64 {
        self.generacion.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub(crate) fn nuevo_pedido_recibir(&self) -> u64 {
        self.pedido_recibir.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub(crate) fn pedido_recibir(&self) -> u64 {
        self.pedido_recibir.load(Ordering::SeqCst)
    }
}

/// Lo que ocupan estos bytes mientras se reciben: el paquete en la caché y sus
/// imágenes en el almacén, a la vez hasta que se borra el paquete.
pub(crate) fn cabe_al_recibir(estado: &AppState, bytes: u64) -> bool {
    crate::store::blobs::hay_espacio(&estado.paths.cache, bytes.saturating_mul(2))
}

/// Por qué se ha caído de verdad una conexión, si no ha sido porque alguien
/// cancelase: `cortado` si se ha ido la red, `otro_sin_espacio` si al otro no le
/// cabe. Sin esto, un corte de cobertura se vería como «Se ha cancelado».
pub(crate) fn motivo_del_corte(conn: &Connection) -> Option<&'static str> {
    use iroh::endpoint::ConnectionError as C;
    match conn.close_reason()? {
        C::TimedOut | C::Reset | C::TransportError(_) => Some("cortado"),
        C::ApplicationClosed(c) if c.error_code == CIERRE_SIN_ESPACIO.into() => {
            Some("otro_sin_espacio")
        }
        _ => None,
    }
}

pub(crate) fn cerrojo<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// La carpeta de los paquetes en tránsito (caché: se puede borrar).
pub(crate) fn carpeta_temporal(estado: &AppState) -> PathBuf {
    estado.paths.cache.join("pasar")
}

/// Una carpeta nueva dentro de la de paquetes en tránsito, que se borra al
/// soltarla (también si se cancela el intercambio y la tarea se aborta).
pub(crate) struct Temporal(pub PathBuf);

impl Temporal {
    pub fn nueva(estado: &AppState) -> std::io::Result<Temporal> {
        let d = carpeta_temporal(estado).join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&d)?;
        Ok(Temporal(d))
    }
}

impl Drop for Temporal {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Ejecuta algo que bloquea (disco, SQLite) sin parar al resto de tareas del
/// runtime de varios hilos; en el de un hilo (los tests) lo hace sin más.
pub(crate) fn bloqueante<R>(f: impl FnOnce() -> R) -> R {
    use tokio::runtime::{Handle, RuntimeFlavor};
    match Handle::try_current() {
        Ok(h) if h.runtime_flavor() == RuntimeFlavor::MultiThread => tokio::task::block_in_place(f),
        _ => f(),
    }
}

// ── Quién atiende lo que entra ──────────────────────────────────────────

type Arrancar = Arc<dyn Fn() -> AbortHandle + Send + Sync>;

#[derive(Default)]
struct Entrada {
    /// Cómo arrancar el bucle de aquí; `None` si no se está recibiendo.
    arrancar: Option<Arrancar>,
    /// El bucle de aquí, si está corriendo.
    propio: Option<AbortHandle>,
    /// Cuántos bucles de la partida tienen el nodo ahora.
    cedido: usize,
}

impl Entrada {
    fn vacia(&self) -> bool {
        self.arrancar.is_none() && self.cedido == 0
    }

    /// Arranca el bucle de aquí si nadie más atiende el nodo.
    fn reanudar(&mut self) {
        if self.cedido == 0 && self.propio.is_none() {
            self.propio = self.arrancar.as_ref().map(|a| a());
        }
    }
}

static REGISTRO: Mutex<BTreeMap<EndpointId, Entrada>> = Mutex::new(BTreeMap::new());

/// Empieza a atender `deck/1` en este nodo (si no lo atiende ya una
/// partida). Se llama al abrir «Recibir».
pub(crate) fn registrar<E: Entorno>(e: &E, ep: Endpoint) {
    let id = ep.id();
    let e = e.clone();
    let h = tokio::runtime::Handle::current();
    let arrancar: Arrancar = Arc::new(move || {
        let (e, ep) = (e.clone(), ep.clone());
        h.spawn(bucle(e, ep)).abort_handle()
    });
    let mut r = cerrojo(&REGISTRO);
    let en = r.entry(id).or_default();
    // Ya se atiende (o lo atiende una partida): no se reinicia el bucle.
    if en.arrancar.is_some() {
        return;
    }
    en.arrancar = Some(arrancar);
    en.reanudar();
}

/// Deja de atender `deck/1` aquí (el nodo se cerró o se cerró «Recibir»).
pub(crate) fn retirar(id: EndpointId) {
    let mut r = cerrojo(&REGISTRO);
    if let Some(en) = r.get_mut(&id) {
        en.arrancar = None;
        if let Some(a) = en.propio.take() {
            a.abort();
        }
        if en.vacia() {
            r.remove(&id);
        }
    }
}

/// El nodo `id` pasa a ser atendido por un bucle de la partida: el de aquí se
/// para mientras la guardia viva (y, si no se está recibiendo nada, solo se
/// apunta, por si se abre «Recibir» con la partida en marcha).
pub fn ceder(id: EndpointId) -> Guardia {
    let mut r = cerrojo(&REGISTRO);
    let en = r.entry(id).or_default();
    en.cedido += 1;
    if let Some(a) = en.propio.take() {
        a.abort();
    }
    Guardia(id)
}

/// Mientras vive, el bucle de la partida atiende `deck/1`; al soltarla, si
/// se sigue esperando un deck, vuelve el de aquí.
pub struct Guardia(EndpointId);

impl Drop for Guardia {
    fn drop(&mut self) {
        let mut r = cerrojo(&REGISTRO);
        if let Some(en) = r.get_mut(&self.0) {
            en.cedido = en.cedido.saturating_sub(1);
            // Fuera de un runtime (al cerrar la app) no hay nada que reanudar.
            if tokio::runtime::Handle::try_current().is_ok() {
                en.reanudar();
            }
            if en.vacia() {
                r.remove(&self.0);
            }
        }
    }
}

/// Lo que se retiene una conexión de otro ALPN con una partida en red.
const RETENER: std::time::Duration = std::time::Duration::from_secs(10);

/// El bucle de `accept` de aquí: solo `deck/1`. Vive mientras «Recibir» está
/// abierto (`recibir::cerrar` y `cancelar` lo retiran): con él vivo, nadie
/// más atiende el nodo. Lo de otro ALPN se cierra al momento, salvo con una
/// partida en red en marcha: puede ser otro jugador que busca anfitrión
/// (`red/sucesion.rs`) y cerrarla le haría creer que aquí no hay nadie; se
/// retiene sin contestar, como si nadie atendiera todavía, hasta que el otro
/// desista (su intento dura `ESPERA_INTENTO`) y vuelva a probar, ya con el
/// bucle del anfitrión si este nodo lo es (`ceder` para el de aquí).
async fn bucle<E: Entorno>(e: E, ep: Endpoint) {
    let id = ep.id();
    bucle_interior(e, ep).await;
    retirar(id);
}

async fn bucle_interior<E: Entorno>(e: E, ep: Endpoint) {
    while let Some(entrante) = ep.accept().await {
        let e = e.clone();
        tokio::spawn(async move {
            let Ok(conn) = entrante.await else { return };
            if conn.alpn() == ALPN_DECK {
                atender(e, conn).await;
            } else if e.partida_en_red() {
                let _ = tokio::time::timeout(RETENER, conn.closed()).await;
            } else {
                conn.close(CIERRE_NO_ESPERADO.into(), b"no se espera");
            }
        });
    }
}

/// Atiende una conexión `deck/1` que llega. Sin «Recibir» abierto, se cierra
/// al momento sin leer nada.
pub async fn atender<E: Entorno>(e: E, conn: Connection) {
    if !recibir::esperando(&e) {
        conn.close(CIERRE_NO_ESPERADO.into(), b"no espero un deck");
        return;
    }
    recibir::sesion(e, conn).await;
}

// ── Lo que hacen los comandos ───────────────────────────────────────────

/// Acepta o rechaza al equipo que quiere enviar (el código de 8 cifras).
pub fn aceptar_peticion<E: Entorno>(e: &E, si: bool) {
    let tx = cerrojo(&e.estado().pasar.receptor)
        .as_mut()
        .and_then(|r| r.peticion.take());
    if let Some(tx) = tx {
        let _ = tx.send(si);
    }
}

/// Lo que dice quien recibe de la oferta.
pub fn responder<E: Entorno>(e: &E, decisiones: Vec<Decision>) -> crate::error::AppResult<()> {
    recibir::responder(e, decisiones)
}

pub(crate) type Respuesta<T> = oneshot::Sender<T>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    /// Un bucle de mentira: cuenta cuántas veces se arranca y cuántos viven.
    fn falso(arrancadas: Arc<AtomicUsize>, vivos: Arc<AtomicUsize>) -> Arrancar {
        Arc::new(move || {
            arrancadas.fetch_add(1, Ordering::SeqCst);
            let v = vivos.clone();
            tokio::spawn(async move {
                struct Baja(Arc<AtomicUsize>);
                impl Drop for Baja {
                    fn drop(&mut self) {
                        self.0.fetch_sub(1, Ordering::SeqCst);
                    }
                }
                v.fetch_add(1, Ordering::SeqCst);
                let _b = Baja(v);
                std::future::pending::<()>().await;
            })
            .abort_handle()
        })
    }

    async fn dejar_correr() {
        for _ in 0..5 {
            tokio::task::yield_now().await;
        }
    }

    /// Nunca dos bucles de `accept` a la vez en el mismo nodo: mientras una
    /// partida lo tiene, el de `pasar/` está parado, y cuando la partida lo
    /// suelta, vuelve (si se sigue esperando un deck).
    #[tokio::test]
    async fn el_bucle_de_pasar_cede_el_nodo_a_la_partida_y_lo_recupera() {
        let id = iroh::SecretKey::generate().public();
        let (arrancadas, vivos) = (
            Arc::new(AtomicUsize::default()),
            Arc::new(AtomicUsize::default()),
        );
        {
            let mut r = cerrojo(&REGISTRO);
            let en = r.entry(id).or_default();
            en.arrancar = Some(falso(arrancadas.clone(), vivos.clone()));
            en.reanudar();
        }
        dejar_correr().await;
        assert_eq!(vivos.load(Ordering::SeqCst), 1);

        // Llega una partida: el de aquí se para.
        let g1 = ceder(id);
        dejar_correr().await;
        assert_eq!(vivos.load(Ordering::SeqCst), 0);
        // Otra a la vez (la sala y el anfitrión): sigue parado al soltar una.
        let g2 = ceder(id);
        drop(g1);
        dejar_correr().await;
        assert_eq!(vivos.load(Ordering::SeqCst), 0);
        // Al soltar la última, vuelve (uno solo).
        drop(g2);
        dejar_correr().await;
        assert_eq!(vivos.load(Ordering::SeqCst), 1);
        assert_eq!(arrancadas.load(Ordering::SeqCst), 2);

        // Sin «Recibir» (nadie pidió el bucle), una partida solo se apunta y
        // al acabar no queda nada que arrancar.
        retirar(id);
        dejar_correr().await;
        assert_eq!(vivos.load(Ordering::SeqCst), 0);
        let g = ceder(id);
        drop(g);
        dejar_correr().await;
        assert_eq!(vivos.load(Ordering::SeqCst), 0);
        assert_eq!(arrancadas.load(Ordering::SeqCst), 2);
        assert!(cerrojo(&REGISTRO).get(&id).is_none(), "no queda la entrada");
    }

    #[test]
    fn la_carpeta_de_paso_se_borra_al_soltarla() {
        let dir = tempfile::tempdir().unwrap();
        let t = Temporal(dir.path().join("x"));
        std::fs::create_dir_all(&t.0).unwrap();
        std::fs::write(t.0.join("a"), b"1").unwrap();
        let ruta = t.0.clone();
        drop(t);
        assert!(!ruta.exists());
    }
}
