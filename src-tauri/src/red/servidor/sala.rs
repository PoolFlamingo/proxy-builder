//! **La sala del servidor en la app** (F6, F11, F12): lo que pasa entre que
//! se entra en una sala y la partida P2P está en marcha, y cuando el host
//! cambia.
//!
//! - **Aceptador:** en una sala, el nodo atiende la malla (`ALPN_PING`), las
//!   imágenes y, si esta app es el host, la partida. Con el servidor no hay
//!   secreto: el host solo deja entrar a los nodos de la sala (su clave en
//!   el servidor es su nodo de iroh).
//! - **Malla:** cada 10 s, una conexión ligera con cada otro miembro para
//!   medir latencia y camino; la fila va al servidor (`sala_latencias`), que
//!   elige host, y a la pantalla (`salas://enlaces`).
//! - **Host:** con `sala_host` el elegido abre el anfitrión y, cuando están
//!   todos (o a los 20 s), empieza la partida; los demás se conectan a él con
//!   su deck. Si llega otro `epoch` con la partida empezada, se migra: el
//!   nuevo host sigue con su copia (`Anfitrion::desde`) y los demás se
//!   conectan a él con su último `seq`.
//! - **Vigía:** si el invitado pierde al host, avisa al servidor cada 2 s
//!   (`sala_host_caido`) hasta que llegue el siguiente.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iroh::endpoint::Connection;
use iroh::{Endpoint, EndpointAddr, EndpointId};
use mesa::formato::{Formato, Variantes};
use mesa::preparar::DeckDeMesa;
use mesa::sparring::Intensidad;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use ts_rs::TS;

use super::cliente::Cliente;
use super::mensajes::{AlServidor, Como, EstadoSala, Fase, Medida, RegistroSala};
use crate::mesa::sesion::{ConfigMesa, JugadorMesa, RedSesion};
use crate::red::anfitrion::{atender_conexion, empezar_partida, Anfitrion};
use crate::red::codigo::Codigo;
use crate::red::invitado::{unirse, Union};
use crate::red::latencia::medir;
use crate::red::mesa::{AvisoRed, Mesa, MesaDeApp};
use crate::red::nodo::{ALPN_IMG, ALPN_MESA, ALPN_PING};
use crate::red::protocolo::Rol;
use crate::red::{colores, imagenes, Sala};
use crate::state::AppState;

const CADA_MEDIDA: Duration = Duration::from_secs(10);
const CADA_VIGIA: Duration = Duration::from_secs(2);
/// Lo que el host espera a que lleguen todos antes de empezar.
const ESPERA_TODOS: Duration = Duration::from_secs(20);

/// Lo que esta app mide hacia otro de la sala.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EnlaceSala {
    pub clave: String,
    pub rtt_ms: u32,
    pub directo: bool,
}

/// El deck con el que se juega en la sala, ya preparado.
pub struct DeckListo {
    pub deck_id: String,
    pub lista: Option<String>,
    pub nombre: String,
    pub mazo: DeckDeMesa,
}

fn cerrojo<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// La sala del servidor en la que está esta app.
pub struct Salon {
    pub id: String,
    pub yo: EndpointId,
    pub como: Como,
    pub apodo: String,
    pub cliente: Arc<Cliente>,
    ep: Endpoint,
    estado: Mutex<Option<EstadoSala>>,
    pub deck: Mutex<Option<DeckListo>>,
    /// El `epoch` del host con el que ya se hizo lo que tocaba.
    epoch_hecho: AtomicU32,
    tareas: Mutex<Vec<tokio::task::AbortHandle>>,
}

impl Drop for Salon {
    fn drop(&mut self) {
        for t in cerrojo(&self.tareas).drain(..) {
            t.abort();
        }
    }
}

impl Salon {
    pub fn estado(&self) -> Option<EstadoSala> {
        cerrojo(&self.estado).clone()
    }

    fn registro(&self) -> Option<RegistroSala> {
        let e = cerrojo(&self.estado);
        let e = e.as_ref()?;
        Some(RegistroSala {
            creador: e.sala.creador.clone(),
            playgroup: e.sala.playgroup.clone(),
            formato: e.sala.formato.clone(),
            max_jugadores: u8::try_from(e.sala.max_jugadores).unwrap_or(10),
            espectadores: e.sala.espectadores,
            fase: e.sala.fase,
            epoch: e.epoch,
            host: e.host.clone(),
            sucesion: e.sucesion.clone(),
        })
    }

    /// Lo que se manda al volver a conectar con el servidor: registrar la
    /// sala (por si se reinició), la dirección y el deck.
    fn al_volver(&self) {
        let mut ms = vec![AlServidor::SalaUnirse {
            sala: self.id.clone(),
            como: self.como,
            registro: self.registro(),
        }];
        if let Ok(d) = serde_json::to_string(&self.ep.addr()) {
            ms.push(AlServidor::SalaDireccion { direccion: d });
        }
        ms.push(AlServidor::SalaDeck {
            deck: cerrojo(&self.deck).as_ref().map(|d| d.nombre.clone()),
        });
        self.cliente.al_volver(ms);
    }
}

/// Entra en una sala (tras crearla, unirse o usar su invitación).
pub fn entrar(
    app: &AppHandle,
    cliente: Arc<Cliente>,
    ep: Endpoint,
    estado: EstadoSala,
    como: Como,
    apodo: String,
) {
    let st = app.state::<AppState>();
    if let Some(viejo) = st.red.con_salon(Option::take) {
        if viejo.id == estado.sala.id {
            // La misma sala: se sigue con lo que había.
            *cerrojo(&viejo.estado) = Some(estado);
            st.red.con_salon(|s| *s = Some(viejo));
            return;
        }
    }
    let salon = Arc::new(Salon {
        id: estado.sala.id.clone(),
        yo: ep.id(),
        como,
        apodo,
        cliente: cliente.clone(),
        ep: ep.clone(),
        estado: Mutex::new(Some(estado)),
        deck: Mutex::new(None),
        epoch_hecho: AtomicU32::new(0),
        tareas: Mutex::new(Vec::new()),
    });
    if let Ok(d) = serde_json::to_string(&ep.addr()) {
        cliente.mandar(AlServidor::SalaDireccion { direccion: d });
    }
    salon.al_volver();
    let tareas = vec![
        tokio::spawn(aceptar(app.clone(), ep.clone())).abort_handle(),
        tokio::spawn(malla(app.clone(), Arc::downgrade(&salon))).abort_handle(),
        tokio::spawn(vigia(app.clone(), Arc::downgrade(&salon))).abort_handle(),
    ];
    *cerrojo(&salon.tareas) = tareas;
    st.red.con_salon(|s| *s = Some(salon));
}

/// Sale de la sala (la partida P2P la cierra quien llame, con `Red::quitar_sala`).
pub fn salir(app: &AppHandle) {
    let st = app.state::<AppState>();
    if let Some(s) = st.red.con_salon(Option::take) {
        s.cliente.al_volver(Vec::new());
    }
}

/// Llega el estado de la sala del servidor.
pub fn al_estado(app: &AppHandle, e: &EstadoSala) {
    let st = app.state::<AppState>();
    let Some(salon) = st.red.con_salon(|s| s.clone()) else {
        return;
    };
    if salon.id != e.sala.id {
        return;
    }
    *cerrojo(&salon.estado) = Some(e.clone());
    salon.al_volver();
    // Si esta app es el host, entran los de la sala.
    st.red.con_sala(|s| {
        if let Some(Sala::Anfitrion { anf, .. }) = s {
            anf.permitir_solo(nodos_de(e));
        }
    });
}

fn nodos_de(e: &EstadoSala) -> Vec<EndpointId> {
    e.miembros
        .iter()
        .filter_map(|m| m.clave.parse::<EndpointId>().ok())
        .collect()
}

/// El servidor nombra host (al empezar, `epoch` 1; luego, al migrar).
pub fn al_host(app: &AppHandle, sala: &str, epoch: u32, host: &str) {
    let st = app.state::<AppState>();
    let Some(salon) = st.red.con_salon(|s| s.clone()) else {
        return;
    };
    if salon.id != sala || epoch <= salon.epoch_hecho.load(Ordering::SeqCst) {
        return;
    }
    salon.epoch_hecho.store(epoch, Ordering::SeqCst);
    if let Some(e) = cerrojo(&salon.estado).as_mut() {
        e.epoch = epoch;
        e.host = Some(host.to_string());
        e.sala.fase = Fase::Juego;
    }
    let soy_host = host == salon.yo.to_string();
    let hay_partida = st.hay_partida();
    // Lo de antes (el invitado del host viejo, o el anfitrión si ya no lo es) se deja.
    if let Some(viejo) = st.red.quitar_sala() {
        if let Sala::Invitado { conexion, .. } = viejo {
            conexion.tarea.abort();
        } else if !soy_host {
            tauri::async_runtime::spawn(viejo.cerrar());
        }
    }
    if soy_host {
        hacer_de_host(app, &salon, epoch, hay_partida);
    } else {
        tauri::async_runtime::spawn(conectar_al_host(
            app.clone(),
            salon,
            host.to_string(),
            hay_partida,
        ));
    }
}

fn hacer_de_host(app: &AppHandle, salon: &Arc<Salon>, epoch: u32, hay_partida: bool) {
    let st = app.state::<AppState>();
    let mesa = MesaDeApp(app.clone());
    let anf = if hay_partida {
        // Migrar: sigue la partida con la copia de esta app.
        let nodos = mesa.con(|m| match m.as_ref().and_then(|s| s.red.as_ref()) {
            Some(RedSesion::Invitado(i)) => i.nodos.clone(),
            _ => Vec::new(),
        });
        let anf = Anfitrion::desde(epoch, String::new(), salon.apodo.clone(), &nodos);
        mesa.con(|m| {
            if let Some(s) = m.as_mut() {
                s.red = Some(RedSesion::Anfitrion(anf.clone()));
            }
        });
        anf
    } else {
        let anf = Anfitrion::nuevo(salon.apodo.clone(), String::new());
        if let Ok(mut d) = anf.deck.lock() {
            *d = cerrojo(&salon.deck).as_ref().map(|d| d.nombre.clone());
        }
        anf
    };
    if let Some(e) = salon.estado() {
        anf.permitir_solo(nodos_de(&e));
    }
    // El aceptador de la sala atiende la partida: esta tarea no hace nada.
    let tarea = tokio::spawn(async {}).abort_handle();
    st.red.con_sala(|s| {
        *s = Some(Sala::Anfitrion {
            anf: anf.clone(),
            tarea,
            codigo: String::new(),
        })
    });
    if !hay_partida {
        tauri::async_runtime::spawn(empezar_cuando_esten(app.clone(), salon.clone(), anf));
    }
}

/// El host empieza cuando están todos los jugadores de la sala con su deck
/// (o a los 20 s, con los que haya).
async fn empezar_cuando_esten(app: AppHandle, salon: Arc<Salon>, anf: Arc<Anfitrion>) {
    let otros = salon
        .estado()
        .map(|e| {
            e.miembros
                .iter()
                .filter(|m| {
                    m.como == Como::Jugador && m.en_linea && m.clave != salon.yo.to_string()
                })
                .count()
        })
        .unwrap_or(0);
    let hasta = tokio::time::Instant::now() + ESPERA_TODOS;
    while anf.para_empezar().len() < otros && tokio::time::Instant::now() < hasta {
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let Some(listo) = cerrojo(&salon.deck).take() else {
        tracing::warn!("el host no tiene deck: no se puede empezar");
        return;
    };
    let formato = salon
        .estado()
        .and_then(|e| {
            serde_json::from_value::<Formato>(serde_json::Value::String(e.sala.formato)).ok()
        })
        .unwrap_or(Formato::Commander);
    let config = ConfigMesa {
        jugadores: vec![JugadorMesa {
            deck_id: listo.deck_id.clone(),
            nombre: salon.apodo.clone(),
            color: colores::de_asiento(0),
            lista: listo.lista.clone(),
            equipo: None,
        }],
        formato,
        variantes: Variantes::default(),
        opciones: None,
        intensidad: Intensidad::Apagado,
        equipos: Vec::new(),
    };
    let mesa = MesaDeApp(app.clone());
    match empezar_partida(&anf, &mesa, salon.yo, config, listo.mazo.clone()) {
        Ok(()) => mesa.avisar(AvisoRed::Empezada),
        Err(e) => {
            tracing::warn!(codigo = ?e.code, "no se pudo empezar la partida de la sala");
            mesa.avisar(AvisoRed::Rechazada {
                motivo: e
                    .params
                    .get("motivo")
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", e.code)),
            });
        }
    }
    *cerrojo(&salon.deck) = Some(listo);
}

/// Se conecta al host (al empezar con el deck; al migrar, sin él: el host
/// nuevo ya tiene la partida). El host puede tardar un momento en abrir.
async fn conectar_al_host(app: AppHandle, salon: Arc<Salon>, host: String, migrar: bool) {
    let st = app.state::<AppState>();
    let Some(addr) = salon.estado().and_then(|e| {
        e.miembros
            .iter()
            .find(|m| m.clave == host)
            .and_then(|m| m.direccion.as_deref())
            .and_then(|d| serde_json::from_str::<EndpointAddr>(d).ok())
    }) else {
        tracing::warn!("el host no tiene dirección");
        return;
    };
    let rol = match salon.como {
        Como::Jugador => Rol::Jugador,
        Como::Espectador => Rol::Espectador,
    };
    let deck = if migrar || rol == Rol::Espectador {
        None
    } else {
        cerrojo(&salon.deck).as_ref().map(|d| d.mazo.clone())
    };
    let union = Union {
        codigo: Codigo {
            addr,
            secreto: String::new(),
        },
        apodo: salon.apodo.clone(),
        rol,
        deck,
    };
    for _ in 0..15 {
        match unirse(salon.ep.clone(), union.clone(), MesaDeApp(app.clone())).await {
            Ok(conexion) => {
                st.red
                    .con_sala(|s| *s = Some(Sala::Invitado { conexion, rol }));
                return;
            }
            Err(e) => tracing::info!(codigo = ?e.code, "el host aún no responde"),
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    MesaDeApp(app.clone()).avisar(AvisoRed::Perdida);
}

/// Atiende las conexiones que llegan al nodo mientras se está en una sala.
async fn aceptar(app: AppHandle, ep: Endpoint) {
    while let Some(entrante) = ep.accept().await {
        let app = app.clone();
        let yo = ep.id();
        tokio::spawn(async move {
            let Ok(conn) = entrante.await else { return };
            let alpn = conn.alpn().to_vec();
            let mesa = MesaDeApp(app.clone());
            if alpn == ALPN_PING {
                // Solo para medir: se queda abierta hasta que el otro la cierre.
                conn.closed().await;
            } else if alpn == ALPN_IMG {
                imagenes::servir(conn, mesa).await;
            } else if alpn == ALPN_MESA {
                let anf = app.state::<AppState>().red.con_sala(|s| match s {
                    Some(Sala::Anfitrion { anf, .. }) => Some(anf.clone()),
                    _ => None,
                });
                match anf {
                    Some(anf) => atender_conexion(conn, anf, mesa, yo).await,
                    None => conn.close(5u32.into(), b"no soy el host"),
                }
            }
        });
    }
}

async fn malla(app: AppHandle, salon: std::sync::Weak<Salon>) {
    let mut conexiones: BTreeMap<String, Connection> = BTreeMap::new();
    tokio::time::sleep(Duration::from_secs(1)).await;
    loop {
        let Some(s) = salon.upgrade() else { return };
        let otros: Vec<(String, EndpointAddr)> = s
            .estado()
            .map(|e| {
                e.miembros
                    .iter()
                    .filter(|m| m.clave != s.yo.to_string() && m.en_linea)
                    .filter_map(|m| {
                        let a =
                            serde_json::from_str::<EndpointAddr>(m.direccion.as_deref()?).ok()?;
                        Some((m.clave.clone(), a))
                    })
                    .collect()
            })
            .unwrap_or_default();
        conexiones.retain(|k, c| c.close_reason().is_none() && otros.iter().any(|(o, _)| o == k));
        let mut fila = Vec::new();
        for (clave, addr) in otros {
            if !conexiones.contains_key(&clave) {
                let c = tokio::time::timeout(Duration::from_secs(5), s.ep.connect(addr, ALPN_PING))
                    .await;
                if let Ok(Ok(c)) = c {
                    conexiones.insert(clave.clone(), c);
                }
            }
            if let Some((rtt, directo)) = conexiones.get(&clave).and_then(medir) {
                fila.push(Medida {
                    clave,
                    rtt_ms: rtt,
                    directo,
                });
            }
        }
        if !fila.is_empty() {
            let _ = app.emit(
                "salas://enlaces",
                fila.iter()
                    .map(|m| EnlaceSala {
                        clave: m.clave.clone(),
                        rtt_ms: m.rtt_ms,
                        directo: m.directo,
                    })
                    .collect::<Vec<_>>(),
            );
            s.cliente.mandar(AlServidor::SalaLatencias { fila });
        }
        drop(s);
        tokio::time::sleep(CADA_MEDIDA).await;
    }
}

/// Si el invitado perdió al host, lo dice al servidor hasta que haya otro.
async fn vigia(app: AppHandle, salon: std::sync::Weak<Salon>) {
    loop {
        tokio::time::sleep(CADA_VIGIA).await;
        let Some(s) = salon.upgrade() else { return };
        let Some(e) = s.estado() else { continue };
        if e.sala.fase != Fase::Juego || s.como != Como::Jugador {
            continue;
        }
        let perdido = app.state::<AppState>().red.con_sala(|x| match x {
            Some(Sala::Invitado { conexion, .. }) => conexion.conexion().is_none(),
            _ => false,
        });
        if perdido {
            s.cliente
                .mandar(AlServidor::SalaHostCaido { epoch: e.epoch });
        }
    }
}

/// Lo que llega del servidor, a la pantalla (`salas://…`) y a la sala en curso.
pub struct AvisosApp(pub AppHandle);

/// El nuevo host, para la pantalla.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct HostSala {
    pub sala: String,
    pub epoch: u32,
    pub host: String,
}

impl super::cliente::Avisos for AvisosApp {
    fn conexion(&self, conectado: bool) {
        let _ = self.0.emit("salas://conexion", conectado);
    }

    fn aviso(&self, m: super::mensajes::DelServidor) {
        use super::mensajes::DelServidor as D;
        match m {
            D::PgLista { playgroups } => {
                let _ = self.0.emit("salas://playgroups", playgroups);
            }
            D::SalaEstado(e) => {
                al_estado(&self.0, &e);
                let _ = self.0.emit("salas://sala", e);
            }
            D::SalaHost {
                sala, epoch, host, ..
            } => {
                al_host(&self.0, &sala, epoch, &host);
                let _ = self.0.emit("salas://host", HostSala { sala, epoch, host });
            }
            D::SalaFuera { sala, motivo } => {
                let st = self.0.state::<AppState>();
                if st
                    .red
                    .con_salon(|s| s.as_ref().is_some_and(|s| s.id == sala))
                {
                    salir(&self.0);
                    let _ = self.0.emit("salas://fuera", motivo);
                }
            }
            _ => {}
        }
    }
}
