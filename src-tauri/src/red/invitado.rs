//! **Un jugador invitado**: se conecta al anfitrión con el código, manda su
//! deck y, cuando empieza la partida, guarda una copia entera (la
//! `Bienvenida`) que pone al día con cada evento, en orden de `seq`. La
//! pantalla recibe lo mismo que en solitario (vista y parches de la sesión).
//!
//! - Sus intenciones llevan un `id` creciente y se guardan hasta que llega el
//!   evento que las confirma (o su rechazo); al reconectar se reenvían y el
//!   anfitrión tira las que ya aplicó.
//! - Un hueco en los `seq` → pide desde el que falta. Un evento que no
//!   encaja o una huella distinta → pide la partida entera.
//! - Si se corta: hasta 5 intentos con espera creciente (1, 2, 4, 8, 16 s);
//!   mientras, la pantalla sabe que se está reconectando.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use iroh::endpoint::{Connection, RecvStream, SendStream};
use iroh::Endpoint;
use mesa::accion::Accion;
use mesa::preparar::DeckDeMesa;
use tokio::sync::{mpsc, Mutex};

use super::codigo::Codigo;
use super::mesa::{AvisoRed, Mesa};
use super::mirada::{Mirada, Visto};
use super::nodo::{de_red, ALPN_MESA};
use super::protocolo::{
    escribir_al_host, leer_del_host, version_app, AlHost, DelHost, NodoJugador, Rol, PROTOCOLO,
};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::mesa::sesion::{Recibido, Sesion};

const INTENTOS: u32 = 5;
const ESPERA_RESPUESTA: Duration = Duration::from_secs(10);

/// Lo que guarda la sesión del invitado para hablar con el anfitrión.
pub struct Invitado {
    pub epoch: u32,
    /// De qué unión es (un bucle viejo no toca la sesión de otra).
    pub union: u64,
    pub(crate) tx: mpsc::UnboundedSender<AlHost>,
    siguiente: u64,
    /// Intenciones mandadas y aún sin confirmar.
    pub pendientes: BTreeMap<u64, Accion>,
    /// Quién es quién (para seguir de anfitrión si el de ahora se va).
    pub nodos: Vec<NodoJugador>,
}

impl Invitado {
    pub fn nuevo(epoch: u32, union: u64, tx: mpsc::UnboundedSender<AlHost>) -> Self {
        Invitado {
            epoch,
            union,
            tx,
            siguiente: 0,
            pendientes: BTreeMap::new(),
            nodos: Vec::new(),
        }
    }

    /// Manda una intención al anfitrión.
    pub fn enviar(&mut self, accion: Accion) -> AppResult<()> {
        self.siguiente += 1;
        let id = self.siguiente;
        self.pendientes.insert(id, accion.clone());
        self.tx
            .send(AlHost::Intencion { id, accion })
            .map_err(|_| AppError::new(ErrorCode::Network).with("motivo", "desconectado"))
    }

    pub fn confirmada(&mut self, id: u64) {
        self.pendientes.remove(&id);
    }

    /// Tras volver a conectar: las que no llegaron a confirmarse.
    pub fn reenviar(&self) {
        for (id, accion) in &self.pendientes {
            let _ = self.tx.send(AlHost::Intencion {
                id: *id,
                accion: accion.clone(),
            });
        }
    }
}

/// Con qué se entra a una partida.
#[derive(Clone)]
pub struct Union {
    pub codigo: Codigo,
    pub apodo: String,
    pub rol: Rol,
    /// El deck (los espectadores no traen).
    pub deck: Option<DeckDeMesa>,
}

static UNIONES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Si la sesión de la mesa es la de esta unión (o aún no hay ninguna).
fn es_mia<M: Mesa>(mesa: &M, union: u64) -> bool {
    mesa.con(|m| match m.as_ref().map(|s| &s.red) {
        None => true,
        Some(Some(crate::mesa::sesion::RedSesion::Invitado(i))) => i.union == union,
        Some(_) => false,
    })
}

/// Cómo acabó una conexión con el anfitrión.
enum Fin {
    /// Se cortó: se intenta volver.
    Cortada,
    Cerrada,
    Rechazada(String),
}

/// Conecta, manda el `Hola` y espera la primera respuesta (si es un
/// rechazo, error con su motivo).
async fn conectar(
    ep: &Endpoint,
    u: &Union,
    ultimo_seq: Option<u64>,
    epoch: u32,
) -> AppResult<(Connection, SendStream, RecvStream, DelHost)> {
    let conn = ep
        .connect(u.codigo.addr.clone(), ALPN_MESA)
        .await
        .map_err(de_red)?;
    let (mut tx, mut rx) = conn.open_bi().await.map_err(de_red)?;
    escribir_al_host(
        &mut tx,
        &AlHost::Hola {
            protocolo: PROTOCOLO,
            version_app: version_app(),
            version_catalogo: mesa::VERSION_CATALOGO,
            rol: u.rol,
            apodo: u.apodo.clone(),
            secreto: Some(u.codigo.secreto.clone()),
            ultimo_seq,
            epoch,
        },
    )
    .await
    .map_err(de_red)?;
    let primera = tokio::time::timeout(ESPERA_RESPUESTA, leer_del_host(&mut rx))
        .await
        .map_err(|_| AppError::new(ErrorCode::Network).with("motivo", "sin_respuesta"))?
        .map_err(de_red)?;
    if let DelHost::Rechazo { id: None, motivo } = &primera {
        return Err(AppError::new(ErrorCode::Rejected).with("motivo", motivo.clone()));
    }
    Ok((conn, tx, rx, primera))
}

/// Una unión en marcha: por dónde mandar y la tarea que la atiende.
#[derive(Debug)]
pub struct Conexion {
    pub tx: mpsc::UnboundedSender<AlHost>,
    pub tarea: tokio::task::AbortHandle,
    /// La conexión de ahora con el anfitrión (cambia al reconectar): por
    /// ella se le piden las imágenes.
    pub actual: Arc<std::sync::Mutex<Option<Connection>>>,
}

impl Conexion {
    pub fn conexion(&self) -> Option<Connection> {
        match self.actual.lock() {
            Ok(g) => g.clone(),
            Err(e) => e.into_inner().clone(),
        }
    }

    /// Se despide del anfitrión y deja de atenderlo.
    pub async fn salir(self) {
        let _ = self.tx.send(AlHost::Adios);
        // Un momento para que el adiós salga antes de cortar.
        tokio::time::sleep(Duration::from_millis(150)).await;
        self.tarea.abort();
    }
}

/// Se une a una partida: conecta (o devuelve por qué no) y deja una tarea
/// atendiendo al anfitrión hasta que acabe.
pub async fn unirse<M: Mesa>(ep: Endpoint, u: Union, mesa: M) -> AppResult<Conexion> {
    let (conn, mut tx, rx, primera) = conectar(&ep, &u, None, 1).await?;
    if let Some(deck) = &u.deck {
        escribir_al_host(&mut tx, &AlHost::Preparacion { deck: deck.clone() })
            .await
            .map_err(de_red)?;
    }
    let (salida, entrada) = mpsc::unbounded_channel::<AlHost>();
    let entrada = Arc::new(Mutex::new(entrada));
    let union = UNIONES.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let para_salir = salida.clone();
    let actual = Arc::new(std::sync::Mutex::new(Some(conn.clone())));
    let tarea = tokio::spawn(bucle(
        ep,
        u,
        mesa,
        union,
        Canales {
            salida,
            entrada,
            conexion: actual.clone(),
        },
        (conn, tx, rx, primera),
    ));
    Ok(Conexion {
        tx: para_salir,
        tarea: tarea.abort_handle(),
        actual,
    })
}

/// Lo que el bucle de una unión comparte con quien lo lanzó.
struct Canales {
    salida: mpsc::UnboundedSender<AlHost>,
    entrada: Arc<Mutex<mpsc::UnboundedReceiver<AlHost>>>,
    conexion: Arc<std::sync::Mutex<Option<Connection>>>,
}

async fn bucle<M: Mesa>(
    ep: Endpoint,
    u: Union,
    mesa: M,
    union: u64,
    canales: Canales,
    primera: (Connection, SendStream, RecvStream, DelHost),
) {
    let Canales {
        salida,
        entrada,
        conexion,
    } = canales;
    let mut actual = Some(primera);
    loop {
        let Some((conn, tx, rx, m)) = actual.take() else {
            return;
        };
        if let Ok(mut c) = conexion.lock() {
            *c = Some(conn.clone());
        }
        let fin = atender(&conn, tx, rx, m, &mesa, union, &salida, &entrada).await;
        // Sin conexión hasta que vuelva (la sala del servidor lo mira para
        // avisar de que el host se cayó).
        if let Ok(mut c) = conexion.lock() {
            *c = None;
        }
        // Otra unión (o esta app pasó a anfitrión): esta ya no manda.
        if !es_mia(&mesa, union) {
            return;
        }
        match fin {
            Fin::Cerrada => {
                cerrar(&mesa);
                mesa.avisar(AvisoRed::Cerrada);
                return;
            }
            Fin::Rechazada(motivo) => {
                mesa.avisar(AvisoRed::Rechazada { motivo });
                return;
            }
            Fin::Cortada => {}
        }
        // Volver a conectar, con el último evento que se tiene.
        for intento in 1..=INTENTOS {
            if !es_mia(&mesa, union) {
                return;
            }
            mesa.avisar(AvisoRed::Reconectando { intento });
            tokio::time::sleep(Duration::from_secs(1 << (intento - 1))).await;
            let (ultimo, epoch) = mesa
                .con(|m| {
                    m.as_ref().map(|s| {
                        let epoch = match &s.red {
                            Some(crate::mesa::sesion::RedSesion::Invitado(i)) => i.epoch,
                            _ => 1,
                        };
                        (Some(s.partida().seq), epoch)
                    })
                })
                .or_else(|| mesa.mirada(|m| m.as_ref().map(|v| (Some(v.vista.version), v.epoch))))
                .unwrap_or((None, 1));
            match conectar(&ep, &u, ultimo, epoch).await {
                Ok(c) => {
                    mesa.avisar(AvisoRed::Conectada);
                    actual = Some(c);
                    break;
                }
                Err(e) if e.code == ErrorCode::Rejected => {
                    let motivo = e.params.get("motivo").cloned().unwrap_or_default();
                    mesa.avisar(AvisoRed::Rechazada { motivo });
                    return;
                }
                Err(_) => {}
            }
        }
        if actual.is_none() {
            if es_mia(&mesa, union) {
                mesa.avisar(AvisoRed::Perdida);
            }
            return;
        }
    }
}

/// El anfitrión cerró la partida: la pantalla se entera y se olvida.
fn cerrar<M: Mesa>(mesa: &M) {
    mesa.con(|m| {
        if let Some(s) = m.as_mut() {
            s.cerrar();
        }
        *m = None;
    });
    mesa.mirada(|m| {
        if let Some(v) = m.as_mut() {
            v.cerrar();
        }
        *m = None;
    });
}

/// Atiende una conexión con el anfitrión hasta que se corta o se cierra.
#[allow(clippy::too_many_arguments)]
async fn atender<M: Mesa>(
    conn: &Connection,
    mut tx: SendStream,
    mut rx: RecvStream,
    primera: DelHost,
    mesa: &M,
    union: u64,
    salida: &mpsc::UnboundedSender<AlHost>,
    entrada: &Arc<Mutex<mpsc::UnboundedReceiver<AlHost>>>,
) -> Fin {
    // Lo que sale hacia el anfitrión, por su propia tarea.
    let entrada2 = entrada.clone();
    let escritor = tokio::spawn(async move {
        let mut e = entrada2.lock().await;
        while let Some(m) = e.recv().await {
            if escribir_al_host(&mut tx, &m).await.is_err() {
                break;
            }
        }
    });
    // Los streams que abra el anfitrión piden imágenes de esta app.
    let imagenes = tokio::spawn(super::imagenes::servir_conexion(
        conn.clone(),
        mesa.clone(),
        None,
    ));
    // Tras volver, las intenciones que no se llegaron a confirmar.
    mesa.con(|m| {
        if let Some(Some(crate::mesa::sesion::RedSesion::Invitado(i))) = m.as_ref().map(|s| &s.red)
        {
            i.reenviar();
        }
    });
    let mut siguiente = Some(primera);
    let fin = loop {
        let m = match siguiente.take() {
            Some(m) => m,
            None => match leer_del_host(&mut rx).await {
                Ok(m) => m,
                Err(e) => {
                    tracing::info!(error = %e, "se cortó la conexión con el anfitrión");
                    break Fin::Cortada;
                }
            },
        };
        if let Some(fin) = recibir(m, mesa, union, salida) {
            break fin;
        }
    };
    escritor.abort();
    imagenes.abort();
    if !matches!(fin, Fin::Cortada) {
        conn.close(0u32.into(), b"fin");
    }
    fin
}

/// Lo que hace el invitado con un mensaje del anfitrión.
fn recibir<M: Mesa>(
    m: DelHost,
    mesa: &M,
    union: u64,
    salida: &mpsc::UnboundedSender<AlHost>,
) -> Option<Fin> {
    match m {
        DelHost::Sala { jugadores } => mesa.avisar(AvisoRed::Sala { jugadores }),
        DelHost::Bienvenida {
            epoch,
            yo,
            partida,
            nodos,
        } => {
            let nueva = mesa.con(|m| match m.as_mut() {
                Some(s) if matches!(s.red, Some(crate::mesa::sesion::RedSesion::Invitado(_))) => {
                    if let Some(crate::mesa::sesion::RedSesion::Invitado(i)) = &mut s.red {
                        // Puede ser otra unión (el anfitrión cambió): desde
                        // ahora, esta.
                        i.epoch = epoch;
                        i.nodos = nodos;
                        i.union = union;
                        i.tx = salida.clone();
                    }
                    s.recibir_partida(*partida, yo);
                    false
                }
                _ => {
                    let mut inv = Invitado::nuevo(epoch, union, salida.clone());
                    inv.nodos = nodos;
                    *m = Some(Sesion::invitada(*partida, yo, inv));
                    true
                }
            });
            if nueva {
                mesa.avisar(AvisoRed::Empezada);
            }
        }
        DelHost::Evento {
            epoch,
            evento,
            confirma,
            huella,
        } => {
            let r = mesa.con(|m| {
                let s = m.as_mut()?;
                if let Some(crate::mesa::sesion::RedSesion::Invitado(i)) = &s.red {
                    if epoch < i.epoch {
                        return Some(Recibido::Tirado);
                    }
                }
                let r = s.recibir(*evento, huella.as_deref());
                if let (Some(id), Recibido::Aplicado) = (confirma, r) {
                    s.confirmada(id);
                }
                Some(r)
            });
            match r {
                Some(Recibido::Hueco(desde)) => {
                    let _ = salida.send(AlHost::Pide { desde_seq: desde });
                }
                // Que mande la partida entera.
                Some(Recibido::Desfase) => {
                    let _ = salida.send(AlHost::Pide { desde_seq: 0 });
                }
                _ => {}
            }
        }
        DelHost::Rechazo { id: None, motivo } => return Some(Fin::Rechazada(motivo)),
        DelHost::Rechazo { id, motivo } => mesa.con(|m| {
            if let Some(s) = m.as_mut() {
                s.rechazada(id, motivo);
            }
        }),
        DelHost::VistaEspectador { epoch, vista } => {
            let nueva = mesa.mirada(|m| match m.as_mut() {
                Some(v) => {
                    v.vista_entera(epoch, *vista);
                    false
                }
                None => {
                    *m = Some(Mirada::nueva(epoch, *vista));
                    true
                }
            });
            if nueva {
                mesa.avisar(AvisoRed::Empezada);
            }
        }
        DelHost::ParcheEspectador {
            epoch,
            parche,
            evento,
        } => {
            let r = mesa.mirada(|m| {
                m.as_mut()
                    .map(|v| v.parche(epoch, *parche, evento.map(|e| *e)))
            });
            if r == Some(Visto::Falta) {
                let _ = salida.send(AlHost::Pide { desde_seq: 0 });
            }
        }
        DelHost::Cerrada => return Some(Fin::Cerrada),
    }
    None
}
