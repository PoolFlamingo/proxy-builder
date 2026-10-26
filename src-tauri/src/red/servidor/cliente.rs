//! **El cliente del servidor de salas** (F8): un WebSocket que se mantiene
//! abierto mientras la app lo necesite.
//!
//! - Responde el reto firmando con la clave de iroh de la instalación.
//! - Reconecta solo, con espera creciente (1, 2, 4… hasta 30 s), y al volver
//!   manda lo de `al_volver` (registrar otra vez la sala, su dirección…).
//! - `pedir` manda un mensaje con su `id` y espera la respuesta (10 s): un
//!   `error` del servidor llega como `ErrorCode::Server` con `params.code`;
//!   sin conexión, `ErrorCode::Network`.
//! - Lo que el servidor manda por su cuenta va a `Avisos` (en la app, eventos
//!   `salas://…` para la pantalla y la sala en curso).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use iroh::SecretKey;
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::Connector;

use super::mensajes::{
    texto_a_firmar, AlServidor, DelServidor, Llegada, Peticion, Plataforma, VERSION_PROTOCOLO,
};
use crate::error::{AppError, AppResult, ErrorCode};

const ESPERA_RESPUESTA: Duration = Duration::from_secs(10);
const ESPERA_MAX: Duration = Duration::from_secs(30);
/// Un `ping` propio cada tanto (nginx corta lo que lleva 120 s callado).
const LATIDO: Duration = Duration::from_secs(20);

/// Lo que el servidor manda sin que se le pida (y si hay conexión o no).
pub trait Avisos: Send + Sync + 'static {
    fn conexion(&self, conectado: bool);
    fn aviso(&self, m: DelServidor);
}

type Pendientes = Arc<Mutex<BTreeMap<u64, oneshot::Sender<DelServidor>>>>;

fn cerrojo<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

pub struct Cliente {
    salida: mpsc::UnboundedSender<Peticion>,
    pendientes: Pendientes,
    siguiente: AtomicU64,
    conectado: Arc<AtomicBool>,
    al_volver: Arc<Mutex<Vec<AlServidor>>>,
    tarea: tokio::task::AbortHandle,
}

impl Drop for Cliente {
    fn drop(&mut self) {
        self.tarea.abort();
    }
}

/// La configuración TLS: rustls con el verificador del sistema (como reqwest).
fn tls() -> AppResult<Connector> {
    let proveedor = Arc::new(rustls::crypto::ring::default_provider());
    let red = |e: rustls::Error| AppError::new(ErrorCode::Network).with("detalle", e);
    let cfg = rustls::ClientConfig::builder_with_provider(proveedor)
        .with_safe_default_protocol_versions()
        .map_err(red)?
        .with_platform_verifier()
        .map_err(red)?
        .with_no_client_auth();
    Ok(Connector::Rustls(Arc::new(cfg)))
}

impl Cliente {
    /// Arranca el cliente: conecta (y reconecta) en una tarea propia.
    pub fn arrancar(
        url: String,
        clave: SecretKey,
        apodo: String,
        plataforma: Plataforma,
        avisos: impl Avisos,
    ) -> Cliente {
        let (salida, entrada) = mpsc::unbounded_channel();
        let pendientes: Pendientes = Arc::default();
        let conectado = Arc::new(AtomicBool::new(false));
        let al_volver: Arc<Mutex<Vec<AlServidor>>> = Arc::default();
        let tarea = tokio::spawn(bucle(
            Datos {
                url,
                clave,
                apodo,
                plataforma,
            },
            Arc::new(avisos),
            Compartido {
                pendientes: pendientes.clone(),
                conectado: conectado.clone(),
                al_volver: al_volver.clone(),
            },
            entrada,
        ));
        Cliente {
            salida,
            pendientes,
            siguiente: AtomicU64::new(1),
            conectado,
            al_volver,
            tarea: tarea.abort_handle(),
        }
    }

    pub fn conectado(&self) -> bool {
        self.conectado.load(Ordering::SeqCst)
    }

    /// Lo que se manda cada vez que se vuelve a conectar (sustituye lo de antes).
    pub fn al_volver(&self, ms: Vec<AlServidor>) {
        *cerrojo(&self.al_volver) = ms;
    }

    /// Manda sin esperar respuesta (si no hay conexión, se pierde).
    pub fn mandar(&self, m: AlServidor) {
        let id = self.siguiente.fetch_add(1, Ordering::SeqCst);
        let _ = self.salida.send(Peticion { id, m });
    }

    /// Manda y espera la respuesta.
    pub async fn pedir(&self, m: AlServidor) -> AppResult<DelServidor> {
        if !self.conectado() {
            return Err(AppError::new(ErrorCode::Network).with("motivo", "sin_servidor"));
        }
        let id = self.siguiente.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = oneshot::channel();
        cerrojo(&self.pendientes).insert(id, tx);
        if self.salida.send(Peticion { id, m }).is_err() {
            cerrojo(&self.pendientes).remove(&id);
            return Err(AppError::new(ErrorCode::Network).with("motivo", "sin_servidor"));
        }
        let r = tokio::time::timeout(ESPERA_RESPUESTA, rx).await;
        cerrojo(&self.pendientes).remove(&id);
        match r {
            Ok(Ok(DelServidor::Error { code, params })) => {
                let mut e = AppError::new(ErrorCode::Server).with("code", code);
                for (k, v) in params.unwrap_or_default() {
                    e = e.with(&k, v);
                }
                Err(e)
            }
            Ok(Ok(m)) => Ok(m),
            _ => Err(AppError::new(ErrorCode::Network).with("motivo", "sin_respuesta")),
        }
    }
}

struct Datos {
    url: String,
    clave: SecretKey,
    apodo: String,
    plataforma: Plataforma,
}

struct Compartido {
    pendientes: Pendientes,
    conectado: Arc<AtomicBool>,
    al_volver: Arc<Mutex<Vec<AlServidor>>>,
}

async fn bucle<A: Avisos>(
    d: Datos,
    avisos: Arc<A>,
    c: Compartido,
    mut entrada: mpsc::UnboundedReceiver<Peticion>,
) {
    let mut espera = Duration::from_secs(1);
    loop {
        match sesion(&d, avisos.as_ref(), &c, &mut entrada).await {
            Ok(()) => espera = Duration::from_secs(1),
            Err(e) => tracing::info!(error = %e, "sin conexión con el servidor de salas"),
        }
        if c.conectado.swap(false, Ordering::SeqCst) {
            espera = Duration::from_secs(1);
        }
        avisos.conexion(false);
        // Lo que esperaba respuesta ya no la tendrá.
        cerrojo(&c.pendientes).clear();
        // Lo mandado sin conexión se tira.
        while entrada.try_recv().is_ok() {}
        tokio::time::sleep(espera).await;
        espera = (espera * 2).min(ESPERA_MAX);
    }
}

/// Una conexión entera: el reto, el `hola` y luego ir y venir hasta que se corta.
async fn sesion<A: Avisos>(
    d: &Datos,
    avisos: &A,
    c: &Compartido,
    entrada: &mut mpsc::UnboundedReceiver<Peticion>,
) -> AppResult<()> {
    let red = |e: tokio_tungstenite::tungstenite::Error| {
        AppError::new(ErrorCode::Network).with("detalle", e)
    };
    let (mut ws, _) = tokio::time::timeout(
        ESPERA_RESPUESTA,
        tokio_tungstenite::connect_async_tls_with_config(d.url.as_str(), None, false, Some(tls()?)),
    )
    .await
    .map_err(|_| AppError::new(ErrorCode::Network).with("motivo", "sin_respuesta"))?
    .map_err(red)?;

    let leer = |m: Message| -> Option<Llegada> {
        match m {
            Message::Text(t) => serde_json::from_str(t.as_str()).ok(),
            _ => None,
        }
    };
    // El reto.
    let nonce = loop {
        let m = tokio::time::timeout(ESPERA_RESPUESTA, ws.next())
            .await
            .map_err(|_| AppError::new(ErrorCode::Network).with("motivo", "sin_respuesta"))?
            .ok_or_else(|| AppError::new(ErrorCode::Network))?
            .map_err(red)?;
        if let Some(Llegada {
            m: DelServidor::Reto { nonce },
            ..
        }) = leer(m)
        {
            break nonce;
        }
    };
    let firma = d.clave.sign(texto_a_firmar(&nonce).as_bytes());
    let hola = Peticion {
        id: 0,
        m: AlServidor::Hola {
            version: VERSION_PROTOCOLO,
            clave: d.clave.public().to_string(),
            apodo: d.apodo.clone(),
            plataforma: d.plataforma,
            firma: firma
                .to_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        },
    };
    let json = |p: &Peticion| {
        serde_json::to_string(p).map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))
    };
    ws.send(Message::text(json(&hola)?)).await.map_err(red)?;
    loop {
        let m = tokio::time::timeout(ESPERA_RESPUESTA, ws.next())
            .await
            .map_err(|_| AppError::new(ErrorCode::Network).with("motivo", "sin_respuesta"))?
            .ok_or_else(|| AppError::new(ErrorCode::Network))?
            .map_err(red)?;
        match leer(m).map(|l| l.m) {
            Some(DelServidor::Bienvenido { .. }) => break,
            Some(DelServidor::Error { code, .. }) => {
                return Err(AppError::new(ErrorCode::Server).with("code", code));
            }
            _ => {}
        }
    }
    c.conectado.store(true, Ordering::SeqCst);
    avisos.conexion(true);
    let volver = cerrojo(&c.al_volver).clone();
    for (i, m) in volver.into_iter().enumerate() {
        let p = Peticion {
            id: u64::MAX - i as u64,
            m,
        };
        ws.send(Message::text(json(&p)?)).await.map_err(red)?;
    }

    let mut latido = tokio::time::interval(LATIDO);
    latido.tick().await;
    loop {
        tokio::select! {
            m = ws.next() => {
                let Some(m) = m else { return Ok(()) };
                let m = m.map_err(red)?;
                if let Message::Close(_) = m {
                    return Ok(());
                }
                let Some(l) = leer(m) else { continue };
                let esperado = l.re.and_then(|re| cerrojo(&c.pendientes).remove(&re));
                match esperado {
                    Some(tx) => {
                        // Las listas y estados también van a los avisos: la
                        // pantalla y la sala se ponen al día igual.
                        if matches!(l.m, DelServidor::PgLista { .. } | DelServidor::SalaEstado(_)) {
                            avisos.aviso(l.m.clone());
                        }
                        let _ = tx.send(l.m);
                    }
                    None => match l.m {
                        DelServidor::Ok | DelServidor::Pong | DelServidor::Error { .. } => {}
                        m => avisos.aviso(m),
                    },
                }
            }
            p = entrada.recv() => {
                let Some(p) = p else { return Ok(()) };
                ws.send(Message::text(json(&p)?)).await.map_err(red)?;
            }
            _ = latido.tick() => {
                let p = Peticion { id: 0, m: AlServidor::Ping };
                ws.send(Message::text(json(&p)?)).await.map_err(red)?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    use super::super::mensajes::{Como, Fase};
    use super::*;

    /// El servidor de `server/` en un puerto libre (o `None` si no está).
    struct Servidor {
        hijo: Child,
        puerto: u16,
    }

    impl Drop for Servidor {
        fn drop(&mut self) {
            let _ = self.hijo.kill();
        }
    }

    fn servidor() -> Option<Servidor> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../server");
        let tsx = dir.join("node_modules/.bin/tsx");
        if !tsx.exists() {
            eprintln!("aviso: sin server/node_modules no se prueba contra el servidor");
            return None;
        }
        let puerto = std::net::TcpListener::bind("127.0.0.1:0")
            .ok()?
            .local_addr()
            .ok()?
            .port();
        let hijo = Command::new(tsx)
            .arg("src/index.ts")
            .current_dir(&dir)
            .env("PB_PUERTO", puerto.to_string())
            .env("PB_DB", ":memory:")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        Some(Servidor { hijo, puerto })
    }

    #[derive(Clone, Default)]
    struct Anotados(Arc<Mutex<Vec<DelServidor>>>);

    impl Avisos for Anotados {
        fn conexion(&self, _: bool) {}
        fn aviso(&self, m: DelServidor) {
            cerrojo(&self.0).push(m);
        }
    }

    async fn conectado(c: &Cliente) {
        for _ in 0..200 {
            if c.conectado() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("no conectó");
    }

    #[tokio::test]
    async fn crea_un_playgroup_invita_y_otra_identidad_entra() {
        let Some(s) = servidor() else { return };
        let url = format!("ws://127.0.0.1:{}/v1/ws", s.puerto);
        let avisos_a = Anotados::default();
        let a = Cliente::arrancar(
            url.clone(),
            SecretKey::generate(),
            "Ana".into(),
            Plataforma::Escritorio,
            avisos_a.clone(),
        );
        let b = Cliente::arrancar(
            url,
            SecretKey::generate(),
            "Berto".into(),
            Plataforma::Escritorio,
            Anotados::default(),
        );
        conectado(&a).await;
        conectado(&b).await;
        let DelServidor::PgLista { playgroups } = a
            .pedir(AlServidor::PgCrear {
                nombre: "Jueves".into(),
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        let pg = playgroups[0].id.clone();
        let DelServidor::Inv { codigo, .. } = a
            .pedir(AlServidor::InvCrear {
                playgroup: Some(pg.clone()),
                sala: None,
                caduca_horas: None,
                usos_max: Some(1),
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        let DelServidor::PgLista { playgroups } = b
            .pedir(AlServidor::InvUsar {
                codigo: codigo.clone(),
                como: None,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        let apodos: Vec<_> = playgroups[0]
            .miembros
            .iter()
            .map(|m| m.apodo.as_str())
            .collect();
        assert_eq!(apodos, ["Ana", "Berto"]);
        // Usada una vez, ya no vale: el error llega con su código.
        let e = a
            .pedir(AlServidor::InvUsar { codigo, como: None })
            .await
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::Server);
        assert_eq!(e.params["code"], "caducada");
        // Una sala del playgroup: B entra por el id y A recibe el estado.
        let DelServidor::SalaEstado(e) = b
            .pedir(AlServidor::SalaCrear {
                playgroup: Some(pg),
                formato: "commander".into(),
                max_jugadores: 4,
                espectadores: false,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(e.sala.fase, Fase::Espera);
        let DelServidor::SalaEstado(e) = a
            .pedir(AlServidor::SalaUnirse {
                sala: e.sala.id,
                como: Como::Jugador,
                registro: None,
            })
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(e.miembros.len(), 2);
        // Y el aviso de su playgroup con la sala llegó solo.
        assert!(cerrojo(&avisos_a.0).iter().any(
            |m| matches!(m, DelServidor::PgLista { playgroups } if playgroups[0].salas.len() == 1)
        ));
        drop(s);
    }
}
