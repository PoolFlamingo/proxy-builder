//! **El anfitrión de una partida en red** (topología en estrella: cada
//! jugador, conectado solo con él). Ordena los eventos: recibe las
//! intenciones de todos, las resuelve con la sesión de la app (la misma de
//! solitario, `Sesion::accion_remota`) y reparte cada evento a los demás
//! (`difundir`, que llama la sesión al publicarlo).
//!
//! - **Sala de espera**: quien entra con el código (el secreto del `Hola`)
//!   manda su deck (`Preparacion`); el anfitrión ve quién hay y empieza.
//! - **Jugadores**: reciben la partida entera (`Bienvenida`) y luego cada
//!   evento, con la huella de la partida cada 50 (si no coincide con la
//!   suya, piden la partida otra vez). Cada uno guarda la partida entera
//!   (decisión D-E1: entre amigos, sin miedo a trampas), así cualquiera puede
//!   seguir de anfitrión (`Anfitrion::desde`, Fase F).
//! - **Espectadores**: nunca reciben la partida ni eventos de lo oculto: solo
//!   su vista (`Mirador::Espectador`) y sus parches.
//! - Topes: como mucho 30 intenciones por segundo por jugador; las
//!   intenciones repetidas (al reconectar) se tiran por su `id`.
//!
//! 🛑 Orden de los cerrojos: la sesión (`Mesa::con`) antes que la sala; nunca
//! se pide la sesión con la sala cogida.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use mesa::accion::{Cambio, Evento};
use mesa::encuentro::Encuentro;
use mesa::preparar::DeckDeMesa;
use mesa::registro::Hecho;
use mesa::vista::{parche, vista};
use mesa::{IdJugador, Mirador, Partida};
use tokio::sync::mpsc;

use super::codigo::mismo_secreto;
use super::mesa::{AvisoRed, Mesa};
use super::nodo::{ALPN_IMG, ALPN_MESA};
use super::protocolo::{
    datagrama_a_bytes, escribir_del_host, leer_al_host, leer_hola, version_app, AlHost, Ausente,
    Datagrama, DelHost, EnSala, Enlace, NodoJugador, Rol, PROTOCOLO,
};

/// Cada cuántos eventos va la huella de la partida.
pub const HUELLA_CADA: u64 = 50;
/// Intenciones por segundo que se le consienten a un jugador.
const RITMO: usize = 30;
/// Jugadores como mucho (con el anfitrión).
pub const MAX_JUGADORES: usize = 10;
/// Espectadores como mucho.
const MAX_ESPECTADORES: usize = 20;
/// Lo que se espera el `Hola` de quien conecta.
const ESPERA_HOLA: Duration = Duration::from_secs(5);

struct Par {
    conexion: u64,
    nodo: EndpointId,
    apodo: String,
    rol: Rol,
    deck: Option<DeckDeMesa>,
    jugador: Option<IdJugador>,
    tx: Option<mpsc::UnboundedSender<DelHost>>,
    conn: Option<Connection>,
    /// La última intención aplicada (las de antes se tiran).
    ultimo_id: u64,
    ritmo: VecDeque<Instant>,
    /// Lo último que se midió de su conexión: RTT (ms) y si va directa.
    enlace: Option<(u32, bool)>,
    /// Desde cuándo falta, si ya empezó la partida y se fue (G8).
    ausente_desde: Option<Instant>,
    /// Su `EndpointAddr` en JSON, comprobado con su nodo (Fase K).
    direccion: Option<String>,
    /// Se fue a propósito (era el anfitrión y cedió la partida): no cuenta
    /// para la mayoría, porque no va a volver a votar.
    ido: bool,
}

struct Sala {
    empezada: bool,
    pares: Vec<Par>,
}

pub struct Anfitrion {
    pub epoch: u32,
    secreto: String,
    /// El anfitrión en la sala: su apodo (que puede cambiar mientras se espera)
    /// y su deck.
    apodo: Mutex<String>,
    pub deck: Mutex<Option<String>>,
    sala: Mutex<Sala>,
    siguiente: AtomicU64,
    /// Con el servidor de salas no hay secreto: entra quien está en la sala
    /// (su nodo de iroh es su clave en el servidor).
    permitidos: Mutex<Option<std::collections::BTreeSet<EndpointId>>>,
    /// Los expulsados de esta partida: no vuelven a entrar (G8).
    expulsados: Mutex<std::collections::BTreeSet<EndpointId>>,
    /// El asiento de esta app: el 0 al abrir la partida; al heredarla, el
    /// suyo.
    pub mi_jugador: IdJugador,
    /// El nodo de esta app, si heredó la partida (no se cuenta entre los
    /// demás).
    mi_nodo: Option<EndpointId>,
    /// El nodo que atiende la partida (para dar su dirección).
    endpoint: Mutex<Option<Endpoint>>,
    /// Si deja jugar. Con código directo, solo con la mayoría de los
    /// jugadores conectados (Fase K): sin ella se para, para que no haya dos
    /// partidas a la vez. Con el servidor de salas, siempre.
    activo: AtomicBool,
    /// Cerrado o cedido a otro anfitrión: el vigía se para.
    acabado: AtomicBool,
    /// Cómo va el encuentro (al mejor de varias) y quién se eligió para
    /// empezar: se les manda a los jugadores al empezar, al volver y con
    /// cada partida nueva del encuentro.
    encuentro: Mutex<(Option<Encuentro>, Option<IdJugador>)>,
}

/// Cómo queda quien entra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admision {
    pub conexion: u64,
    pub jugador: Option<IdJugador>,
    pub empezada: bool,
}

/// Lo más largo que puede ser un nombre en la sala, en caracteres.
const MAX_APODO: usize = 32;

/// Un nombre de la sala: sin espacios de más y hasta `MAX_APODO` caracteres.
pub fn limpiar_apodo(apodo: &str) -> String {
    let junto = apodo.split_whitespace().collect::<Vec<_>>().join(" ");
    junto.chars().take(MAX_APODO).collect()
}

fn cerrojo<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

impl Anfitrion {
    pub fn nuevo(apodo: String, secreto: String) -> Arc<Self> {
        Arc::new(Anfitrion {
            epoch: 1,
            secreto,
            apodo: Mutex::new(limpiar_apodo(&apodo)),
            deck: Mutex::new(None),
            sala: Mutex::new(Sala {
                empezada: false,
                pares: Vec::new(),
            }),
            siguiente: AtomicU64::new(1),
            permitidos: Mutex::new(None),
            expulsados: Mutex::new(Default::default()),
            mi_jugador: IdJugador(0),
            mi_nodo: None,
            endpoint: Mutex::new(None),
            activo: AtomicBool::new(true),
            acabado: AtomicBool::new(false),
            encuentro: Mutex::new((None, None)),
        })
    }

    /// Un anfitrión que sigue una partida ya empezada (al migrar: el host
    /// se fue y otro jugador sigue). Conoce a los jugadores por su nodo; él
    /// es `mi_jugador` (`mi_nodo`). Con código directo (`secreto`) empieza
    /// parado hasta tener la mayoría (Fase K); con el servidor, no.
    pub fn desde(
        epoch: u32,
        secreto: String,
        apodo: String,
        mi_nodo: EndpointId,
        mi_jugador: IdJugador,
        nodos: &[NodoJugador],
    ) -> Arc<Self> {
        let pares = nodos
            .iter()
            .filter_map(|n| {
                let nodo = n.nodo.parse::<EndpointId>().ok()?;
                if nodo == mi_nodo {
                    return None;
                }
                Some(Par {
                    conexion: 0,
                    nodo,
                    apodo: String::new(),
                    rol: Rol::Jugador,
                    deck: None,
                    jugador: Some(n.jugador),
                    tx: None,
                    conn: None,
                    ultimo_id: 0,
                    ritmo: VecDeque::new(),
                    enlace: None,
                    // Al migrar nadie está conectado todavía: cuentan desde ahora.
                    ausente_desde: Some(Instant::now()),
                    direccion: n.direccion.clone(),
                    ido: false,
                })
            })
            .collect();
        let activo = secreto.is_empty();
        Arc::new(Anfitrion {
            epoch,
            secreto,
            apodo: Mutex::new(limpiar_apodo(&apodo)),
            deck: Mutex::new(None),
            sala: Mutex::new(Sala {
                empezada: true,
                pares,
            }),
            siguiente: AtomicU64::new(1),
            permitidos: Mutex::new(None),
            expulsados: Mutex::new(Default::default()),
            mi_jugador,
            mi_nodo: Some(mi_nodo),
            endpoint: Mutex::new(None),
            activo: AtomicBool::new(activo),
            acabado: AtomicBool::new(false),
            encuentro: Mutex::new((None, None)),
        })
    }

    /// Si la partida es de código directo (sin servidor de salas).
    pub fn con_codigo(&self) -> bool {
        !self.secreto.is_empty()
    }

    /// El nodo que atiende la partida (para dar su dirección a los demás).
    pub fn poner_endpoint(&self, ep: Endpoint) {
        *cerrojo(&self.endpoint) = Some(ep);
    }

    /// Si deja jugar (Fase K).
    pub fn activo(&self) -> bool {
        self.activo.load(Ordering::SeqCst)
    }

    pub fn activar(&self, si: bool) {
        self.activo.store(si, Ordering::SeqCst);
    }

    /// Cerrado o cedido a otro.
    pub fn acabado(&self) -> bool {
        self.acabado.load(Ordering::SeqCst)
    }

    /// Cuántos jugadores hay en la partida (sin los expulsados, con esta
    /// app) y cuántos están conectados con ella (con esta app).
    pub fn recuento(&self) -> (usize, usize) {
        let expulsados = cerrojo(&self.expulsados).clone();
        let s = cerrojo(&self.sala);
        let otros: Vec<&Par> = s
            .pares
            .iter()
            .filter(|p| {
                p.rol == Rol::Jugador
                    && p.jugador.is_some()
                    && Some(p.nodo) != self.mi_nodo
                    && !p.ido
                    && !expulsados.contains(&p.nodo)
            })
            .collect();
        (
            otros.len() + 1,
            otros.iter().filter(|p| p.tx.is_some()).count() + 1,
        )
    }

    /// Más de la mitad de los jugadores, con esta app, están conectados.
    pub fn mayoria(&self) -> bool {
        let (total, conectados) = self.recuento();
        2 * conectados > total
    }

    /// Menos de la mitad: los que faltan podrían tener la mayoría sin esta
    /// app (con justo la mitad, nadie la tiene).
    pub fn minoria(&self) -> bool {
        let (total, conectados) = self.recuento();
        2 * conectados < total
    }

    /// Apunta cómo va el encuentro, sin mandarlo (se manda con la bienvenida).
    pub fn poner_encuentro(&self, encuentro: Option<Encuentro>, primero: Option<IdJugador>) {
        *cerrojo(&self.encuentro) = (encuentro, primero);
    }

    /// Apunta cómo va el encuentro y se lo manda a los jugadores conectados.
    pub fn repartir_encuentro(&self, encuentro: Option<Encuentro>, primero: Option<IdJugador>) {
        self.poner_encuentro(encuentro, primero);
        let s = cerrojo(&self.sala);
        for p in s.pares.iter().filter(|p| p.rol == Rol::Jugador) {
            self.mandar_encuentro(p);
        }
    }

    fn mandar_encuentro(&self, p: &Par) {
        let (encuentro, primero) = cerrojo(&self.encuentro).clone();
        if encuentro.is_none() {
            return;
        }
        if let Some(tx) = &p.tx {
            let _ = tx.send(DelHost::Encuentro { encuentro, primero });
        }
    }

    /// Apunta que este jugador se fue a propósito (el anfitrión de antes, que
    /// cedió la partida): deja de contar para la mayoría.
    pub fn marcar_ido(&self, jugador: IdJugador) {
        let mut s = cerrojo(&self.sala);
        for p in s.pares.iter_mut().filter(|p| p.jugador == Some(jugador)) {
            p.ido = true;
        }
    }

    /// A quien dejar la partida si este anfitrión se va: el conectado de
    /// asiento más bajo (como el que buscarían los demás si se cayera).
    fn sucesor(&self) -> Option<IdJugador> {
        let expulsados = cerrojo(&self.expulsados).clone();
        cerrojo(&self.sala)
            .pares
            .iter()
            .filter(|p| p.rol == Rol::Jugador && p.tx.is_some() && !expulsados.contains(&p.nodo))
            .filter_map(|p| p.jugador)
            .min()
    }

    /// Quién seguiría si el anfitrión se fuera ahora: solo con la partida
    /// empezada, de código directo y algún jugador conectado.
    fn sucesor_si_cede(&self) -> Option<IdJugador> {
        (self.empezada() && self.con_codigo() && !self.acabado())
            .then(|| self.sucesor())
            .flatten()
    }

    /// Si al irse el anfitrión la partida seguiría con otro jugador.
    pub fn puede_ceder(&self) -> bool {
        self.sucesor_si_cede().is_some()
    }

    /// El anfitrión se va a propósito. Con la partida empezada, de código
    /// directo y algún jugador conectado, la cede: les dice a todos quién
    /// sigue (así no esperan a notar que falta) y corta sin cerrarla. Si no,
    /// la cierra para todos. Devuelve si la cedió.
    pub async fn irse(&self) -> bool {
        let Some(sucesor) = self.sucesor_si_cede() else {
            self.cerrar();
            return false;
        };
        {
            let s = cerrojo(&self.sala);
            for p in s.pares.iter() {
                if let Some(tx) = &p.tx {
                    let _ = tx.send(DelHost::Traspaso {
                        jugador: self.mi_jugador,
                        sucesor,
                    });
                }
            }
        }
        // Un momento para que el aviso salga antes de cortar.
        tokio::time::sleep(PAUSA_TRASPASO).await;
        self.soltar();
        true
    }

    /// Deja de ser el anfitrión (cede a otro): corta a todos sin cerrar la
    /// partida, para que busquen al anfitrión que sigue.
    pub fn soltar(&self) {
        self.acabado.store(true, Ordering::SeqCst);
        self.activar(false);
        let mut s = cerrojo(&self.sala);
        for p in s.pares.iter_mut() {
            p.tx = None;
            if let Some(c) = p.conn.take() {
                c.close(6u32.into(), b"otro_anfitrion");
            }
        }
    }

    pub fn secreto(&self) -> &str {
        &self.secreto
    }

    /// Solo entran estos nodos (los de la sala del servidor), sin secreto.
    pub fn permitir_solo(&self, nodos: impl IntoIterator<Item = EndpointId>) {
        *cerrojo(&self.permitidos) = Some(nodos.into_iter().collect());
    }

    /// Si este nodo con este secreto puede entrar.
    fn deja_entrar(&self, nodo: EndpointId, secreto: Option<&str>) -> bool {
        if cerrojo(&self.expulsados).contains(&nodo) {
            return false;
        }
        match cerrojo(&self.permitidos).as_ref() {
            Some(p) => p.contains(&nodo),
            None => secreto.is_some_and(|s| mismo_secreto(s, &self.secreto)),
        }
    }

    pub fn empezada(&self) -> bool {
        cerrojo(&self.sala).empezada
    }

    /// Quién hay en la sala (el anfitrión primero).
    pub fn vista_sala(&self) -> Vec<EnSala> {
        let s = cerrojo(&self.sala);
        let mut v = vec![EnSala {
            id: 0,
            apodo: self.apodo(),
            rol: Rol::Jugador,
            deck: cerrojo(&self.deck).clone(),
        }];
        v.extend(s.pares.iter().filter(|p| p.tx.is_some()).map(|p| EnSala {
            id: p.conexion,
            apodo: p.apodo.clone(),
            rol: p.rol,
            deck: p.deck.as_ref().map(|d| d.nombre.clone()),
        }));
        v
    }

    /// Manda a todos los de la sala cómo está (y se lo cuenta a la pantalla).
    pub fn avisar_sala<M: Mesa>(&self, mesa: &M) {
        let jugadores = self.vista_sala();
        {
            let s = cerrojo(&self.sala);
            for p in s.pares.iter() {
                if let Some(tx) = &p.tx {
                    let _ = tx.send(DelHost::Sala {
                        jugadores: jugadores.clone(),
                    });
                }
            }
        }
        mesa.avisar(AvisoRed::Sala { jugadores });
    }

    /// Admite a quien trae el secreto: en la sala, como nuevo; con la
    /// partida empezada, a quien vuelve (por su nodo) o a un espectador.
    pub fn admitir(
        &self,
        nodo: EndpointId,
        apodo: String,
        rol: Rol,
        tx: mpsc::UnboundedSender<DelHost>,
        direccion: Option<String>,
    ) -> Result<Admision, &'static str> {
        let mut s = cerrojo(&self.sala);
        let conexion = self.siguiente.fetch_add(1, Ordering::SeqCst);
        let apodo = limpiar_apodo(&apodo);
        if s.empezada && rol == Rol::Jugador {
            let Some(p) = s
                .pares
                .iter_mut()
                .find(|p| p.nodo == nodo && p.jugador.is_some())
            else {
                return Err("empezada");
            };
            p.conexion = conexion;
            p.tx = Some(tx);
            p.ausente_desde = None;
            p.ido = false;
            if direccion.is_some() {
                p.direccion = direccion;
            }
            if !apodo.is_empty() {
                p.apodo = apodo;
            }
            return Ok(Admision {
                conexion,
                jugador: p.jugador,
                empezada: true,
            });
        }
        let cuantos = s
            .pares
            .iter()
            .filter(|p| p.rol == rol && p.tx.is_some())
            .count();
        let max = match rol {
            Rol::Jugador => MAX_JUGADORES - 1,
            Rol::Espectador => MAX_ESPECTADORES,
        };
        if cuantos >= max {
            return Err("llena");
        }
        // Si ya estaba (vuelve a la sala), se cambia su conexión.
        s.pares.retain(|p| !(p.nodo == nodo && p.jugador.is_none()));
        s.pares.push(Par {
            conexion,
            nodo,
            apodo,
            rol,
            deck: None,
            jugador: None,
            tx: Some(tx),
            conn: None,
            ultimo_id: 0,
            ritmo: VecDeque::new(),
            enlace: None,
            ausente_desde: None,
            direccion,
            ido: false,
        });
        Ok(Admision {
            conexion,
            jugador: None,
            empezada: s.empezada,
        })
    }

    /// La conexión de quien acaba de entrar (para poder cortarla).
    fn poner_conexion(&self, conexion: u64, conn: Connection) {
        let mut s = cerrojo(&self.sala);
        if let Some(p) = s.pares.iter_mut().find(|p| p.conexion == conexion) {
            p.conn = Some(conn);
        }
    }

    /// La conexión de un jugador conectado (para pedirle sus imágenes).
    pub fn conexion_de(&self, jugador: IdJugador) -> Option<Connection> {
        let s = cerrojo(&self.sala);
        s.pares
            .iter()
            .find(|p| p.jugador == Some(jugador) && p.tx.is_some())
            .and_then(|p| p.conn.clone())
    }

    /// El jugador de esta conexión, si juega y la partida ya empezó.
    pub fn jugador_de(&self, conexion: u64) -> Option<IdJugador> {
        let s = cerrojo(&self.sala);
        if !s.empezada {
            return None;
        }
        s.pares
            .iter()
            .find(|p| p.conexion == conexion && p.rol == Rol::Jugador)
            .and_then(|p| p.jugador)
    }

    /// Apunta lo que se midió de la conexión de alguien.
    pub fn poner_enlace(&self, conexion: u64, rtt_ms: u32, directo: bool) {
        let mut s = cerrojo(&self.sala);
        if let Some(p) = s.pares.iter_mut().find(|p| p.conexion == conexion) {
            p.enlace = Some((rtt_ms, directo));
        }
    }

    /// El enlace de cada jugador conectado con el anfitrión (el suyo, el 0,
    /// va primero con 0 ms).
    pub fn enlaces(&self) -> Vec<Enlace> {
        let s = cerrojo(&self.sala);
        let mut v = vec![Enlace {
            jugador: self.mi_jugador,
            rtt_ms: 0,
            directo: true,
        }];
        v.extend(s.pares.iter().filter(|p| p.tx.is_some()).filter_map(|p| {
            let (rtt_ms, directo) = p.enlace?;
            Some(Enlace {
                jugador: p.jugador.filter(|_| p.rol == Rol::Jugador)?,
                rtt_ms,
                directo,
            })
        }));
        v
    }

    /// Los jugadores de la partida que faltan y desde hace cuánto (los
    /// expulsados no cuentan: ya están eliminados).
    pub fn ausentes(&self) -> Vec<Ausente> {
        let expulsados = cerrojo(&self.expulsados).clone();
        let s = cerrojo(&self.sala);
        if !s.empezada {
            return Vec::new();
        }
        s.pares
            .iter()
            .filter(|p| p.rol == Rol::Jugador && p.tx.is_none() && !expulsados.contains(&p.nodo))
            .filter_map(|p| {
                Some(Ausente {
                    jugador: p.jugador?,
                    hace_ms: p.ausente_desde.map_or(0, |t| {
                        u32::try_from(t.elapsed().as_millis()).unwrap_or(u32::MAX)
                    }),
                })
            })
            .collect()
    }

    /// Reparte un datagrama a los conectados, salvo a `excepto`. A los
    /// espectadores solo si `a_espectadores` (lo que no ven, no se les cuenta).
    pub fn repartir(&self, d: &Datagrama, excepto: Option<IdJugador>, a_espectadores: bool) {
        let Some(bytes) = datagrama_a_bytes(d) else {
            return;
        };
        let bytes = bytes::Bytes::from(bytes);
        let s = cerrojo(&self.sala);
        for p in s.pares.iter() {
            let Some(conn) = &p.conn else { continue };
            if p.tx.is_none() || (p.jugador.is_some() && p.jugador == excepto) {
                continue;
            }
            if p.rol == Rol::Espectador && !a_espectadores {
                continue;
            }
            let _ = conn.send_datagram(bytes.clone());
        }
    }

    /// Corta la conexión de un jugador (expulsarlo, o en las pruebas).
    pub fn cortar(&self, jugador: IdJugador) {
        let s = cerrojo(&self.sala);
        if let Some(c) = s
            .pares
            .iter()
            .find(|p| p.jugador == Some(jugador))
            .and_then(|p| p.conn.as_ref())
        {
            c.close(3u32.into(), b"cortada");
        }
    }

    /// Expulsa a un nodo (G8): se corta su conexión y ya no entra más a esta
    /// partida. Con la partida empezada devuelve su jugador, para darlo por
    /// eliminado; en la sala de espera simplemente deja de estar.
    pub fn expulsar(&self, nodo: EndpointId) -> Option<IdJugador> {
        cerrojo(&self.expulsados).insert(nodo);
        let mut s = cerrojo(&self.sala);
        let empezada = s.empezada;
        let mut jugador = None;
        for p in s.pares.iter_mut().filter(|p| p.nodo == nodo) {
            if empezada && p.rol == Rol::Jugador {
                jugador = p.jugador;
            }
            if let Some(tx) = p.tx.take() {
                let _ = tx.send(DelHost::Rechazo {
                    id: None,
                    motivo: "expulsado".into(),
                });
            }
            if let Some(c) = p.conn.take() {
                c.close(4u32.into(), b"expulsado");
            }
        }
        if !empezada {
            s.pares.retain(|p| p.nodo != nodo);
        }
        jugador
    }

    /// Cierra la partida para todos.
    pub fn cerrar(&self) {
        self.acabado.store(true, Ordering::SeqCst);
        let s = cerrojo(&self.sala);
        for p in s.pares.iter() {
            if let Some(tx) = &p.tx {
                let _ = tx.send(DelHost::Cerrada);
            }
        }
    }

    /// Cómo se llama el anfitrión.
    pub fn apodo(&self) -> String {
        cerrojo(&self.apodo).clone()
    }

    /// El anfitrión cambia su nombre en la sala de espera; `false` si no
    /// cambia nada (vacío, igual o ya empezada).
    pub fn poner_apodo(&self, apodo: &str) -> bool {
        let nuevo = limpiar_apodo(apodo);
        if nuevo.is_empty() || self.empezada() {
            return false;
        }
        let mut a = cerrojo(&self.apodo);
        if *a == nuevo {
            return false;
        }
        *a = nuevo;
        true
    }

    /// Alguien de la sala cambia su nombre; `false` si no cambia nada.
    pub fn apodo_de(&self, conexion: u64, apodo: &str) -> bool {
        let nuevo = limpiar_apodo(apodo);
        let mut s = cerrojo(&self.sala);
        if nuevo.is_empty() || s.empezada {
            return false;
        }
        match s.pares.iter_mut().find(|p| p.conexion == conexion) {
            Some(p) if p.apodo != nuevo => {
                p.apodo = nuevo;
                true
            }
            _ => false,
        }
    }

    /// El deck con el que entra alguien de la sala.
    pub fn preparacion(&self, conexion: u64, deck: DeckDeMesa) -> bool {
        let mut s = cerrojo(&self.sala);
        if s.empezada {
            return false;
        }
        match s
            .pares
            .iter_mut()
            .find(|p| p.conexion == conexion && p.rol == Rol::Jugador)
        {
            Some(p) => {
                p.deck = Some(deck);
                true
            }
            None => false,
        }
    }

    /// Se fue alguien: de la sala, fuera; de la partida, queda su sitio.
    pub fn desconectar(&self, conexion: u64) {
        let mut s = cerrojo(&self.sala);
        if s.empezada {
            if let Some(p) = s.pares.iter_mut().find(|p| p.conexion == conexion) {
                if p.tx.is_some() {
                    p.ausente_desde = Some(Instant::now());
                }
                p.tx = None;
                p.conn = None;
            }
            s.pares.retain(|p| p.jugador.is_some() || p.tx.is_some());
        } else {
            s.pares.retain(|p| p.conexion != conexion);
        }
    }

    /// Para empezar: los jugadores de la sala con deck, en el orden en que
    /// entraron (los que no lo mandaron, se quedan fuera).
    pub fn para_empezar(&self) -> Vec<(u64, String, DeckDeMesa)> {
        let s = cerrojo(&self.sala);
        s.pares
            .iter()
            .filter(|p| p.rol == Rol::Jugador && p.tx.is_some())
            .filter_map(|p| Some((p.conexion, p.apodo.clone(), p.deck.clone()?)))
            .collect()
    }

    /// Empieza: cada conexión con su jugador. Los de la sala sin deck se van.
    pub fn empezar(&self, asignados: &[(u64, IdJugador)]) {
        let mut s = cerrojo(&self.sala);
        s.empezada = true;
        for p in s.pares.iter_mut() {
            p.jugador = asignados
                .iter()
                .find(|(c, _)| *c == p.conexion)
                .map(|(_, j)| *j);
            if p.rol == Rol::Jugador && p.jugador.is_none() {
                if let Some(tx) = p.tx.take() {
                    let _ = tx.send(DelHost::Rechazo {
                        id: None,
                        motivo: "sin_deck".into(),
                    });
                }
            }
        }
        s.pares
            .retain(|p| p.rol == Rol::Espectador || p.jugador.is_some());
    }

    /// Quién es quién (para que cualquiera pueda seguir de anfitrión).
    pub fn nodos(&self, yo: EndpointId) -> Vec<NodoJugador> {
        let expulsados = cerrojo(&self.expulsados).clone();
        let mia = cerrojo(&self.endpoint)
            .as_ref()
            .and_then(|e| serde_json::to_string(&e.addr()).ok());
        let s = cerrojo(&self.sala);
        let mut v = vec![NodoJugador {
            jugador: self.mi_jugador,
            nodo: yo.to_string(),
            direccion: mia,
        }];
        v.extend(
            s.pares
                .iter()
                // Quien se fue a propósito no es candidato a seguir de anfitrión.
                .filter(|p| p.nodo != yo && !p.ido && !expulsados.contains(&p.nodo))
                .filter_map(|p| {
                    Some(NodoJugador {
                        jugador: p.jugador.filter(|_| p.rol == Rol::Jugador)?,
                        nodo: p.nodo.to_string(),
                        direccion: p.direccion.clone(),
                    })
                }),
        );
        v.sort_by_key(|n| n.jugador);
        v
    }

    /// La partida entera a un jugador, o su vista a un espectador.
    pub fn bienvenida(&self, conexion: u64, partida: &Partida, nodos: Vec<NodoJugador>) {
        let s = cerrojo(&self.sala);
        let Some(p) = s.pares.iter().find(|p| p.conexion == conexion) else {
            return;
        };
        let Some(tx) = &p.tx else { return };
        let m = match (p.rol, p.jugador) {
            (Rol::Jugador, Some(yo)) => DelHost::Bienvenida {
                epoch: self.epoch,
                yo,
                partida: Box::new(partida.clone()),
                nodos,
            },
            _ => DelHost::VistaEspectador {
                epoch: self.epoch,
                vista: Box::new(vista(partida, Mirador::Espectador)),
                nodos,
            },
        };
        let _ = tx.send(m);
        if p.rol == Rol::Jugador {
            self.mandar_encuentro(p);
        }
    }

    /// Las bienvenidas de todos (al empezar).
    pub fn bienvenidas(&self, partida: &Partida, yo: EndpointId) {
        let nodos = self.nodos(yo);
        let conexiones: Vec<u64> = cerrojo(&self.sala)
            .pares
            .iter()
            .map(|p| p.conexion)
            .collect();
        for c in conexiones {
            self.bienvenida(c, partida, nodos.clone());
        }
    }

    /// Reparte un evento recién hecho: a los jugadores, el evento (con la
    /// confirmación de su intención y, cada 50, la huella); a los
    /// espectadores, su parche (o su vista entera, si era un deshacer).
    pub fn difundir(&self, partida: &Partida, hecho: &Hecho, origen: Option<(IdJugador, u64)>) {
        let s = cerrojo(&self.sala);
        let seq = hecho.evento.seq;
        let huella = seq
            .is_multiple_of(HUELLA_CADA)
            .then(|| mesa::huella(partida));
        let restaura = matches!(hecho.evento.cambio, Cambio::Restaurar { .. });
        let mut espectador = None;
        for p in s.pares.iter() {
            let Some(tx) = &p.tx else { continue };
            let m = match (p.rol, p.jugador) {
                (Rol::Jugador, Some(j)) => DelHost::Evento {
                    epoch: self.epoch,
                    evento: Box::new(hecho.evento.clone()),
                    confirma: origen.filter(|(o, _)| *o == j).map(|(_, id)| id),
                    huella: huella.clone(),
                },
                (Rol::Espectador, _) => espectador
                    .get_or_insert_with(|| para_espectador(self.epoch, partida, hecho, restaura))
                    .clone(),
                _ => continue,
            };
            let _ = tx.send(m);
        }
    }

    /// Si la intención `id` de esta conexión hay que hacerla: no repetida y
    /// sin pasarse del ritmo. Devuelve el jugador, o el motivo del rechazo
    /// (`None` = repetida, se tira sin más).
    fn puede(&self, conexion: u64, id: u64) -> Result<IdJugador, Option<&'static str>> {
        let mut s = cerrojo(&self.sala);
        let empezada = s.empezada;
        let Some(p) = s.pares.iter_mut().find(|p| p.conexion == conexion) else {
            return Err(Some("sin_partida"));
        };
        let Some(j) = p.jugador.filter(|_| empezada) else {
            return Err(Some(if p.rol == Rol::Espectador {
                "espectador"
            } else {
                "sin_partida"
            }));
        };
        if id <= p.ultimo_id {
            return Err(None);
        }
        let ahora = Instant::now();
        while p
            .ritmo
            .front()
            .is_some_and(|t| ahora.duration_since(*t) > Duration::from_secs(1))
        {
            p.ritmo.pop_front();
        }
        if p.ritmo.len() >= RITMO {
            return Err(Some("ritmo"));
        }
        p.ritmo.push_back(ahora);
        p.ultimo_id = id;
        Ok(j)
    }

    fn enviar(&self, conexion: u64, m: DelHost) {
        let s = cerrojo(&self.sala);
        if let Some(tx) = s
            .pares
            .iter()
            .find(|p| p.conexion == conexion)
            .and_then(|p| p.tx.as_ref())
        {
            let _ = tx.send(m);
        }
    }
}

/// Empieza la partida en red: el anfitrión es el jugador 0 (con `config`, que
/// trae al anfitrión y, tras él, lo que el anfitrión eligió de cada invitado:
/// color y equipo, por el mismo orden que `orden`) y los de la sala con deck,
/// los siguientes. `orden` son las conexiones de los invitados por asiento;
/// vacío, por orden de llegada. Si no son exactamente los que hay en la sala
/// (alguien entró o se fue entre medias), se rechaza con `sala_cambio` y no
/// empieza nada. Abre la sesión en `mesa` y manda las bienvenidas.
pub fn empezar_partida<M: Mesa>(
    anf: &Arc<Anfitrion>,
    mesa: &M,
    yo: EndpointId,
    mut config: crate::mesa::sesion::ConfigMesa,
    deck: DeckDeMesa,
    orden: &[u64],
) -> crate::error::AppResult<()> {
    let mut invitados = anf.para_empezar();
    if !orden.is_empty() {
        let mut en_sala: Vec<u64> = invitados.iter().map(|(c, ..)| *c).collect();
        let mut pedidos = orden.to_vec();
        en_sala.sort_unstable();
        pedidos.sort_unstable();
        if en_sala != pedidos {
            return Err(
                crate::error::AppError::new(crate::error::ErrorCode::Rejected)
                    .with("motivo", "sala_cambio"),
            );
        }
        invitados = orden
            .iter()
            .filter_map(|c| invitados.iter().find(|(x, ..)| x == c).cloned())
            .collect();
    }
    // Lo que el anfitrión eligió de cada invitado, por posición.
    let elegidos = config.jugadores.split_off(1.min(config.jugadores.len()));
    let mut mazos = vec![deck];
    let mut asignados = Vec::new();
    for (i, (conexion, apodo, d)) in invitados.into_iter().enumerate() {
        let j = IdJugador(u8::try_from(i + 1).unwrap_or(u8::MAX));
        let elegido = elegidos.get(i);
        config.jugadores.push(crate::mesa::sesion::JugadorMesa {
            deck_id: String::new(),
            nombre: if apodo.trim().is_empty() {
                d.nombre.clone()
            } else {
                apodo
            },
            color: elegido
                .map(|e| e.color.clone())
                .filter(|c| !c.is_empty())
                .unwrap_or_else(|| super::colores::de_asiento(i + 1)),
            lista: None,
            equipo: elegido.and_then(|e| e.equipo),
        });
        mazos.push(d);
        asignados.push((conexion, j));
    }
    let primero = config.primero;
    let sesion = crate::mesa::sesion::Sesion::en_red(config, mazos, anf.clone())?;
    let partida = sesion.partida().clone();
    anf.poner_encuentro(sesion.encuentro.clone(), primero);
    anf.empezar(&asignados);
    mesa.con(|m| *m = Some(sesion));
    anf.bienvenidas(&partida, yo);
    vigilar_ausentes(anf, mesa.clone(), ESPERA_MULLIGAN);
    Ok(())
}

/// Lo del banquillo de un jugador entre dos partidas de un encuentro: el
/// cambio, si trae uno, y siempre su mazo y su banquillo como quedan. Solo
/// toca el de quien lo pide.
fn banquillo_de<M: Mesa>(
    anf: &Arc<Anfitrion>,
    mesa: &M,
    conexion: u64,
    cambio: Option<(Vec<String>, Vec<String>)>,
) {
    let Some(j) = anf.jugador_de(conexion) else {
        anf.enviar(
            conexion,
            DelHost::BanquilloNo {
                motivo: "sin_partida".into(),
            },
        );
        return;
    };
    let r = mesa.con(|m| match m.as_mut() {
        Some(s) => {
            if let Some((sale, entra)) = cambio {
                s.cambiar_banquillo_de(j, sale, entra)?;
            }
            s.banquillo_de(j)
        }
        None => Err(crate::error::AppError::new(
            crate::error::ErrorCode::NotFound,
        )),
    });
    anf.enviar(
        conexion,
        match r {
            Ok(vista) => DelHost::Banquillo { vista },
            Err(e) => DelHost::BanquilloNo {
                motivo: e
                    .params
                    .get("motivo")
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", e.code)),
            },
        },
    );
}

/// Lo que se espera a que salga el aviso de traspaso antes de cortar.
const PAUSA_TRASPASO: Duration = Duration::from_millis(250);

/// Lo que se espera a un jugador que se fue en el mulligan antes de quedarse
/// por él con la mano que tiene.
pub const ESPERA_MULLIGAN: Duration = Duration::from_secs(60);

/// Vigila a los que faltan de una partida ya abierta en `mesa` (G8): le dice
/// a la pantalla del anfitrión quién falta cuando cambia (a los demás se lo
/// cuenta el latido) y, pasado un minuto en el mulligan, se queda por ellos
/// con su mano. Acaba sola cuando se cierra la partida.
pub fn vigilar_ausentes<M: Mesa>(anf: &Arc<Anfitrion>, mesa: M, espera_mulligan: Duration) {
    let debil = Arc::downgrade(anf);
    tauri::async_runtime::spawn(async move {
        let mut ultimos: Option<Vec<IdJugador>> = None;
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let Some(anf) = debil.upgrade() else { return };
            if mesa.con(|m| m.is_none()) {
                return;
            }
            let ausentes = anf.ausentes();
            let quienes: Vec<IdJugador> = ausentes.iter().map(|a| a.jugador).collect();
            if ultimos.as_ref() != Some(&quienes) {
                ultimos = Some(quienes);
                mesa.avisar(AvisoRed::Ausentes {
                    filas: ausentes.clone(),
                });
            }
            for a in ausentes
                .iter()
                .filter(|a| u128::from(a.hace_ms) >= espera_mulligan.as_millis())
            {
                mesa.con(|m| {
                    let Some(s) = m.as_mut() else { return };
                    if let Some(accion) =
                        mesa::ausencia::quedarse_por_defecto(s.partida(), a.jugador)
                    {
                        if let Err(e) = s.suplir(a.jugador, accion) {
                            tracing::warn!(error = ?e.code, "no se pudo quedar por el ausente");
                        }
                    }
                });
            }
        }
    });
}

fn para_espectador(epoch: u32, partida: &Partida, hecho: &Hecho, restaura: bool) -> DelHost {
    if restaura {
        DelHost::VistaEspectador {
            epoch,
            vista: Box::new(vista(partida, Mirador::Espectador)),
            // Los nodos ya los tiene de su primera vista.
            nodos: Vec::new(),
        }
    } else {
        let parche = parche(partida, &hecho.tocado, Mirador::Espectador);
        let evento = evento_visible(partida, &hecho.evento.cambio, &parche.quitadas)
            .then(|| Box::new(hecho.evento.clone()));
        DelHost::ParcheEspectador {
            epoch,
            parche: Box::new(parche),
            evento,
        }
    }
}

/// Los campos de `Accion` (y de lo que lleva dentro) que nombran cartas, como
/// salen en JSON.
const CAMPOS_DE_CARTA: &[&str] = &[
    "carta",
    "cartas",
    "a",
    "abajo",
    "alFondo",
    "arriba",
    "bajo",
    "cementerio",
    "comandante",
    "eleccion",
    "elegidas",
    "exilio",
    "monton",
    "resto",
];

/// Las cartas que nombra un cambio. Se leen del JSON por el nombre del campo
/// para no tener que tocar este código con cada acción nueva; si sobra algún
/// número, el espectador se queda sin esa línea del registro, nunca al revés.
fn cartas_nombradas(cambio: &Cambio) -> Vec<u32> {
    fn ids(v: &serde_json::Value, out: &mut Vec<u32>) {
        match v {
            serde_json::Value::Number(n) => {
                out.extend(n.as_u64().and_then(|x| u32::try_from(x).ok()))
            }
            serde_json::Value::Array(a) => {
                for x in a {
                    match x {
                        serde_json::Value::Number(_) => ids(x, out),
                        // `(IdCarta, Destino)`: la carta va primero.
                        serde_json::Value::Array(par) => {
                            if let Some(primero) = par.first() {
                                ids(primero, out);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    fn recorrer(v: &serde_json::Value, out: &mut Vec<u32>) {
        match v {
            serde_json::Value::Object(m) => {
                // `ModoDescarte::Elegidas` va como `{tipo: "elegidas", valor: [...]}`.
                if m.get("tipo").and_then(|t| t.as_str()) == Some("elegidas") {
                    if let Some(valor) = m.get("valor") {
                        ids(valor, out);
                    }
                }
                for (k, x) in m {
                    if CAMPOS_DE_CARTA.contains(&k.as_str()) {
                        ids(x, out);
                    }
                    recorrer(x, out);
                }
            }
            serde_json::Value::Array(a) => a.iter().for_each(|x| recorrer(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    if let Ok(v) = serde_json::to_value(cambio) {
        recorrer(&v, &mut out);
    }
    out
}

/// Si el espectador puede recibir el evento tal cual: todas las cartas que
/// nombra las ve ahora (en una zona pública o enseñadas) o las veía antes de
/// este evento (`quitadas` del parche). Si no, solo le llega el parche: el
/// id de una carta oculta (de la mano a la biblioteca, por ejemplo) diría
/// algo que su vista no dice. Deshacer nunca le llega como evento (lleva la
/// partida entera).
fn evento_visible(partida: &Partida, cambio: &Cambio, quitadas: &[mesa::IdCarta]) -> bool {
    if matches!(cambio, Cambio::Restaurar { .. }) {
        return false;
    }
    cartas_nombradas(cambio).into_iter().all(|n| {
        let id = mesa::IdCarta(n);
        if quitadas.contains(&id) {
            return true;
        }
        match (partida.donde(id), partida.cartas.get(&id)) {
            (Some((zona, i)), Some(c)) => {
                zona.es_publica()
                    || mesa::visibilidad::ve(partida, c, &zona, i, Mirador::Espectador)
            }
            // Ya no existe (una ficha que se fue): estaba a la vista.
            _ => true,
        }
    })
}

/// Atiende las conexiones que llegan al nodo mientras dure.
pub async fn atender<M: Mesa>(ep: Endpoint, anf: Arc<Anfitrion>, mesa: M) {
    anf.poner_endpoint(ep.clone());
    if anf.con_codigo() {
        // Sin servidor, el anfitrión vigila su mayoría (Fase K).
        tokio::spawn(super::sucesion::vigilar(
            Arc::downgrade(&anf),
            ep.clone(),
            mesa.clone(),
        ));
    }
    while let Some(entrante) = ep.accept().await {
        let anf = anf.clone();
        let mesa = mesa.clone();
        let yo = ep.id();
        tokio::spawn(async move {
            let Ok(conn) = entrante.await else { return };
            let alpn = conn.alpn().to_vec();
            if alpn == ALPN_MESA {
                atender_conexion(conn, anf, mesa, yo).await;
            } else if alpn == ALPN_IMG {
                // Las imágenes propias (E8).
                super::imagenes::servir(conn, mesa).await;
            }
        });
    }
}

pub(crate) async fn atender_conexion<M: Mesa>(
    conn: Connection,
    anf: Arc<Anfitrion>,
    mesa: M,
    yo: EndpointId,
) {
    let nodo = conn.remote_id();
    let Ok(Ok((mut tx, mut rx))) = tokio::time::timeout(ESPERA_HOLA, conn.accept_bi()).await else {
        conn.close(1u32.into(), b"sin stream");
        return;
    };
    // Lo primero, el Hola (con tope y plazo): sin él no se lee nada más.
    let hola = match tokio::time::timeout(ESPERA_HOLA, leer_hola(&mut rx)).await {
        Ok(Ok(h)) => h,
        _ => {
            conn.close(1u32.into(), b"hola");
            return;
        }
    };
    let AlHost::Hola {
        protocolo,
        version_app: v,
        version_catalogo,
        rol,
        apodo,
        secreto,
        direccion,
        solo_si_activo,
        ..
    } = hola
    else {
        return;
    };
    // Su dirección solo vale si es la de su nodo.
    let direccion =
        direccion.filter(|d| serde_json::from_str::<EndpointAddr>(d).is_ok_and(|a| a.id == nodo));
    let rechazo = if protocolo != PROTOCOLO
        || v != version_app()
        || version_catalogo != mesa::VERSION_CATALOGO
    {
        Some("version")
    } else if !anf.deja_entrar(nodo, secreto.as_deref()) {
        Some("codigo")
    } else if solo_si_activo && !anf.activo() {
        Some("no_activo")
    } else {
        None
    };
    if let Some(motivo) = rechazo {
        let _ = escribir_del_host(
            &mut tx,
            &DelHost::Rechazo {
                id: None,
                motivo: motivo.into(),
            },
        )
        .await;
        let _ = tx.finish();
        tokio::time::sleep(Duration::from_millis(200)).await;
        conn.close(2u32.into(), motivo.as_bytes());
        return;
    }
    let (salida, mut entrada) = mpsc::unbounded_channel::<DelHost>();
    let adm = match anf.admitir(nodo, apodo, rol, salida.clone(), direccion) {
        Ok(a) => a,
        Err(motivo) => {
            let _ = escribir_del_host(
                &mut tx,
                &DelHost::Rechazo {
                    id: None,
                    motivo: motivo.into(),
                },
            )
            .await;
            let _ = tx.finish();
            tokio::time::sleep(Duration::from_millis(200)).await;
            conn.close(2u32.into(), motivo.as_bytes());
            return;
        }
    };
    anf.poner_conexion(adm.conexion, conn.clone());
    // Los demás streams que abra son peticiones de imágenes.
    let imagenes = tokio::spawn(super::imagenes::servir_conexion(
        conn.clone(),
        mesa.clone(),
        Some(anf.clone()),
    ));
    // Latidos, enlaces y movimiento en vivo, por datagramas (E7, E10).
    let latencia = tokio::spawn(super::latencia::atender_host(
        conn.clone(),
        anf.clone(),
        mesa.clone(),
        adm.conexion,
        super::latencia::CADA_LATIDO,
    ));
    // Lo que sale hacia él, en orden, por su propia tarea.
    let escritor = tokio::spawn(async move {
        while let Some(m) = entrada.recv().await {
            if escribir_del_host(&mut tx, &m).await.is_err() {
                break;
            }
            if matches!(m, DelHost::Cerrada | DelHost::Traspaso { .. }) {
                let _ = tx.finish();
                break;
            }
        }
    });
    if adm.empezada {
        // Vuelve (o entra a mirar): la partida entera o su vista.
        let partida = mesa.con(|m| m.as_ref().map(|s| s.partida().clone()));
        if let Some(p) = partida {
            anf.bienvenida(adm.conexion, &p, anf.nodos(yo));
        }
    } else {
        anf.avisar_sala(&mesa);
    }
    loop {
        let m = match leer_al_host(&mut rx).await {
            Ok(m) => m,
            Err(e) => {
                tracing::info!(error = %e, "se cerró la conexión de un jugador");
                break;
            }
        };
        match m {
            AlHost::Hola { .. } => break,
            AlHost::Preparacion { deck } => {
                if anf.preparacion(adm.conexion, deck) {
                    anf.avisar_sala(&mesa);
                }
            }
            AlHost::Apodo { apodo } => {
                if anf.apodo_de(adm.conexion, &apodo) {
                    anf.avisar_sala(&mesa);
                }
            }
            AlHost::Intencion { id, accion } => match anf.puede(adm.conexion, id) {
                Ok(j) => {
                    let r = mesa.con(|m| match m.as_mut() {
                        Some(s) => s.accion_remota(j, id, accion),
                        None => Err(crate::error::AppError::new(
                            crate::error::ErrorCode::NotFound,
                        )),
                    });
                    if let Err(e) = r {
                        let motivo = e
                            .params
                            .get("motivo")
                            .cloned()
                            .unwrap_or_else(|| format!("{:?}", e.code));
                        anf.enviar(
                            adm.conexion,
                            DelHost::Rechazo {
                                id: Some(id),
                                motivo,
                            },
                        );
                    }
                }
                Err(Some(motivo)) => anf.enviar(
                    adm.conexion,
                    DelHost::Rechazo {
                        id: Some(id),
                        motivo: motivo.into(),
                    },
                ),
                Err(None) => {}
            },
            AlHost::PideBanquillo => banquillo_de(&anf, &mesa, adm.conexion, None),
            AlHost::CambiaBanquillo { sale, entra } => {
                banquillo_de(&anf, &mesa, adm.conexion, Some((sale, entra)));
            }
            AlHost::Pide { desde_seq } => {
                // Los eventos que le faltan; si no los hay, la partida entera.
                let r = mesa.con(|m| {
                    m.as_ref().map(|s| {
                        let eventos: Vec<Evento> = s
                            .registro
                            .eventos
                            .iter()
                            .filter(|e| e.seq >= desde_seq)
                            .cloned()
                            .collect();
                        let completos = desde_seq > s.registro.inicial.seq
                            && eventos.first().is_some_and(|e| e.seq == desde_seq);
                        (completos.then_some(eventos), s.partida().clone())
                    })
                });
                match r {
                    Some((Some(eventos), _)) => {
                        for e in eventos {
                            anf.enviar(
                                adm.conexion,
                                DelHost::Evento {
                                    epoch: anf.epoch,
                                    evento: Box::new(e),
                                    confirma: None,
                                    huella: None,
                                },
                            );
                        }
                    }
                    Some((None, p)) => anf.bienvenida(adm.conexion, &p, anf.nodos(yo)),
                    None => {}
                }
            }
            AlHost::Adios => break,
        }
    }
    anf.desconectar(adm.conexion);
    if !anf.empezada() {
        anf.avisar_sala(&mesa);
    }
    drop(salida);
    escritor.abort();
    imagenes.abort();
    latencia.abort();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesa::sesion::tests::{config, mazo};
    use crate::mesa::sesion::Sesion;
    use mesa::accion::{Destino, Sitio, ZonaDestino};
    use mesa::modelo::{RefZona, Zona};
    use mesa::Accion;

    fn mover(cartas: Vec<mesa::IdCarta>, zona: Zona) -> Cambio {
        Cambio::Accion {
            accion: Accion::Mover {
                cartas,
                a: Destino {
                    zona: ZonaDestino::De(zona),
                    jugador: None,
                    sitio: Sitio::Arriba,
                },
                como: None,
            },
            azar: None,
        }
    }

    #[test]
    fn al_espectador_no_le_llega_el_id_de_una_carta_oculta() {
        let s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        let p = s.partida();
        let yo = IdJugador(0);
        let oculta = p.zona(&RefZona::de(yo, Zona::Biblioteca))[5];
        // De la biblioteca a la mano: no la ve antes ni después.
        assert!(!evento_visible(p, &mover(vec![oculta], Zona::Mano), &[]));
        // Si el parche la quita de su vista, es que la veía.
        assert!(evento_visible(
            p,
            &mover(vec![oculta], Zona::Mano),
            &[oculta]
        ));
        // Acciones sin cartas (robar) y cartas que ya no existen, sí.
        let robar = Cambio::Accion {
            accion: Accion::Robar { n: 1 },
            azar: None,
        };
        assert!(evento_visible(p, &robar, &[]));
        assert!(evento_visible(
            p,
            &mover(vec![mesa::IdCarta(9_999)], Zona::Cementerio),
            &[]
        ));
    }

    #[test]
    fn las_cartas_se_leen_de_todos_los_campos_que_las_nombran() {
        let c = mover(vec![mesa::IdCarta(3), mesa::IdCarta(4)], Zona::Mano);
        assert_eq!(cartas_nombradas(&c), vec![3, 4]);
        let d = Cambio::Dados {
            tiradas: vec![],
            semilla_animacion: 7,
        };
        assert!(cartas_nombradas(&d).is_empty());
    }
}
