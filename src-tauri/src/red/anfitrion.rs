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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointId};
use mesa::accion::{Cambio, Evento};
use mesa::preparar::DeckDeMesa;
use mesa::registro::Hecho;
use mesa::vista::{parche, vista};
use mesa::{IdJugador, Mirador, Partida};
use tokio::sync::mpsc;

use super::codigo::mismo_secreto;
use super::mesa::{AvisoRed, Mesa};
use super::nodo::{ALPN_IMG, ALPN_MESA};
use super::protocolo::{
    escribir_del_host, leer_al_host, leer_hola, version_app, AlHost, DelHost, EnSala, NodoJugador,
    Rol, PROTOCOLO,
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
}

struct Sala {
    empezada: bool,
    pares: Vec<Par>,
}

pub struct Anfitrion {
    pub epoch: u32,
    secreto: String,
    /// El anfitrión en la sala: su apodo y su deck.
    pub apodo: String,
    pub deck: Mutex<Option<String>>,
    sala: Mutex<Sala>,
    siguiente: AtomicU64,
    /// Con el servidor de salas no hay secreto: entra quien está en la sala
    /// (su nodo de iroh es su clave en el servidor).
    permitidos: Mutex<Option<std::collections::BTreeSet<EndpointId>>>,
}

/// Cómo queda quien entra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admision {
    pub conexion: u64,
    pub jugador: Option<IdJugador>,
    pub empezada: bool,
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
            apodo,
            deck: Mutex::new(None),
            sala: Mutex::new(Sala {
                empezada: false,
                pares: Vec::new(),
            }),
            siguiente: AtomicU64::new(1),
            permitidos: Mutex::new(None),
        })
    }

    /// Un anfitrión que sigue una partida ya empezada (al migrar: el host
    /// se fue y otro jugador sigue). Conoce a los jugadores por su nodo.
    pub fn desde(epoch: u32, secreto: String, apodo: String, nodos: &[NodoJugador]) -> Arc<Self> {
        let pares = nodos
            .iter()
            .filter_map(|n| {
                let nodo = n.nodo.parse::<EndpointId>().ok()?;
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
                })
            })
            .collect();
        Arc::new(Anfitrion {
            epoch,
            secreto,
            apodo,
            deck: Mutex::new(None),
            sala: Mutex::new(Sala {
                empezada: true,
                pares,
            }),
            siguiente: AtomicU64::new(1),
            permitidos: Mutex::new(None),
        })
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
            apodo: self.apodo.clone(),
            rol: Rol::Jugador,
            deck: cerrojo(&self.deck).clone(),
        }];
        v.extend(s.pares.iter().filter(|p| p.tx.is_some()).map(|p| EnSala {
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
    ) -> Result<Admision, &'static str> {
        let mut s = cerrojo(&self.sala);
        let conexion = self.siguiente.fetch_add(1, Ordering::SeqCst);
        let apodo: String = apodo.chars().take(32).collect();
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

    /// Cierra la partida para todos.
    pub fn cerrar(&self) {
        let s = cerrojo(&self.sala);
        for p in s.pares.iter() {
            if let Some(tx) = &p.tx {
                let _ = tx.send(DelHost::Cerrada);
            }
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
        let s = cerrojo(&self.sala);
        let mut v = vec![NodoJugador {
            jugador: IdJugador(0),
            nodo: yo.to_string(),
        }];
        v.extend(s.pares.iter().filter_map(|p| {
            Some(NodoJugador {
                jugador: p.jugador?,
                nodo: p.nodo.to_string(),
            })
        }));
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
            },
        };
        let _ = tx.send(m);
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
/// trae solo a él, y su deck) y los de la sala con deck, los siguientes, por
/// orden de llegada. Abre la sesión en `mesa` y manda las bienvenidas.
pub fn empezar_partida<M: Mesa>(
    anf: &Arc<Anfitrion>,
    mesa: &M,
    yo: EndpointId,
    mut config: crate::mesa::sesion::ConfigMesa,
    deck: DeckDeMesa,
) -> crate::error::AppResult<()> {
    let invitados = anf.para_empezar();
    let mut mazos = vec![deck];
    let mut asignados = Vec::new();
    for (i, (conexion, apodo, d)) in invitados.into_iter().enumerate() {
        let j = IdJugador(u8::try_from(i + 1).unwrap_or(u8::MAX));
        config.jugadores.push(crate::mesa::sesion::JugadorMesa {
            deck_id: String::new(),
            nombre: if apodo.trim().is_empty() {
                d.nombre.clone()
            } else {
                apodo
            },
            color: super::colores::de_asiento(i + 1),
            lista: None,
        });
        mazos.push(d);
        asignados.push((conexion, j));
    }
    let sesion = crate::mesa::sesion::Sesion::en_red(config, mazos, anf.clone())?;
    let partida = sesion.partida().clone();
    anf.empezar(&asignados);
    mesa.con(|m| *m = Some(sesion));
    anf.bienvenidas(&partida, yo);
    Ok(())
}

fn para_espectador(epoch: u32, partida: &Partida, hecho: &Hecho, restaura: bool) -> DelHost {
    if restaura {
        DelHost::VistaEspectador {
            epoch,
            vista: Box::new(vista(partida, Mirador::Espectador)),
        }
    } else {
        DelHost::ParcheEspectador {
            epoch,
            parche: Box::new(parche(partida, &hecho.tocado, Mirador::Espectador)),
            evento: Some(Box::new(hecho.evento.clone())),
        }
    }
}

/// Atiende las conexiones que llegan al nodo mientras dure.
pub async fn atender<M: Mesa>(ep: Endpoint, anf: Arc<Anfitrion>, mesa: M) {
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
        ..
    } = hola
    else {
        return;
    };
    let rechazo = if protocolo != PROTOCOLO
        || v != version_app()
        || version_catalogo != mesa::VERSION_CATALOGO
    {
        Some("version")
    } else if !anf.deja_entrar(nodo, secreto.as_deref()) {
        Some("codigo")
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
    let adm = match anf.admitir(nodo, apodo, rol, salida.clone()) {
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
    // Lo que sale hacia él, en orden, por su propia tarea.
    let escritor = tokio::spawn(async move {
        while let Some(m) = entrada.recv().await {
            if escribir_del_host(&mut tx, &m).await.is_err() {
                break;
            }
            if matches!(m, DelHost::Cerrada) {
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
}
