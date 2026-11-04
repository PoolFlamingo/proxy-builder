//! **Recibir un deck**: el lado de quien espera.
//!
//! «Recibir» abre un modo de espera de 5 minutos: un código `PBD1-…` con su
//! QR (el secreto vale una sola vez) y, si hay mDNS, un anuncio en la red
//! local. Entra **una** conexión a la vez, y solo si trae el secreto o si la
//! persona acepta el código de 6 cifras. Después:
//!
//! 1. llega la **oferta** (los decks, con su peso y si traen proxies), se
//!    enseña con lo que ya hay aquí del mismo linaje y la persona elige por
//!    deck: con proxies, solo cartas originales o no, y si ya existe,
//!    reemplazar o guardar los dos;
//! 2. se piden solo los aceptados y llegan uno a uno, a un fichero de la
//!    caché, con su tamaño y su SHA-256 comprobados;
//! 3. cada uno se importa con `package::import_con` (que no se fía de nada).
//!
//! 🛑 Nada llega a la biblioteca a medias: un paquete se importa solo si ha
//! llegado entero y con su huella; si se corta, se borra lo que hubiera en la
//! caché y el deck que se estaba recibiendo no existe. Los que ya se habían
//! recibido enteros antes del corte se quedan (cada uno es un deck completo).

use std::time::Duration;

use iroh::endpoint::Connection;
use iroh::Endpoint;
use iroh::EndpointId;
use tokio::sync::oneshot;
use tokio::task::AbortHandle;
use tokio::time::timeout;

use super::encontrar::{self, Anuncio};
use super::protocolo::{
    apodo_limpio, escribir, leer, recibir_bytes, validar_oferta, AlEmisor, AlReceptor, MAX_PAQUETE,
    VERSION,
};
use super::tipos::{
    Conflicto, Decision, DeckOfrecido, DeckRevisado, Eleccion, EstadoReceptor, EstadoResultado,
    FaseReceptor, MasReciente, Progreso, Remoto, Resolucion, Resultado,
};
use super::{
    cerrojo, registrar, retirar, Entorno, Respuesta, Temporal, CIERRE_CANCELADO, CIERRE_FIN,
    CIERRE_NO_ESPERADO,
};
use crate::android::UsoMulticast;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::package::{self, Destino, Opciones};
use crate::red::codigo::{mismo_secreto, secreto_nuevo, Codigo};
use crate::store::{ahora_ms, decks, linaje};

/// Cuánto dura el código (y el anuncio) si nadie llega.
pub const CADUCIDAD: Duration = Duration::from_secs(5 * 60);
const ESPERA_HOLA: Duration = Duration::from_secs(10);
const ESPERA_PETICION: Duration = Duration::from_secs(60);
const ESPERA_OFERTA: Duration = Duration::from_secs(60);
const ESPERA_DECISION: Duration = Duration::from_secs(15 * 60);
/// Quien envía hace cada paquete justo antes de mandarlo, y comprimirlo puede
/// llevar minutos en un móvil. Si se va, la conexión se cierra y se nota antes.
const ESPERA_PAQUETE: Duration = Duration::from_secs(10 * 60);
const ESPERA_FIN: Duration = Duration::from_secs(20);
/// Cada cuánto se avisa de cómo va un paquete.
const CADA_AVISO: Duration = Duration::from_millis(100);

/// El modo «Recibir» abierto (uno por app).
pub(crate) struct Receptor {
    pub gen: u64,
    pub ep_id: EndpointId,
    /// El secreto del código, hasta que alguien lo usa.
    pub secreto: Option<String>,
    pub estado: EstadoReceptor,
    /// Hay un intercambio en curso (no entra otro).
    pub ocupado: bool,
    pub peticion: Option<Respuesta<bool>>,
    pub decisiones: Option<Respuesta<Vec<Decision>>>,
    pub conn: Option<Connection>,
    pub anuncio: Option<Anuncio>,
    pub tareas: Vec<AbortHandle>,
}

impl Receptor {
    fn parar(&mut self) {
        self.anuncio = None;
        for t in self.tareas.drain(..) {
            t.abort();
        }
    }
}

fn es_final(f: FaseReceptor) -> bool {
    matches!(
        f,
        FaseReceptor::Terminado
            | FaseReceptor::Cancelado
            | FaseReceptor::Caducado
            | FaseReceptor::Error
    )
}

/// Cambia el estado del modo (si sigue siendo el mismo, `gen`) y avisa.
fn mutar<E: Entorno, R>(e: &E, gen: u64, f: impl FnOnce(&mut Receptor) -> R) -> Option<R> {
    let (r, snap) = {
        let mut g = cerrojo(&e.estado().pasar.receptor);
        let r = g.as_mut().filter(|r| r.gen == gen)?;
        let out = f(r);
        (out, r.estado.clone())
    };
    e.emitir_receptor(&snap);
    Some(r)
}

/// ¿Está abierto «Recibir» esperando a alguien?
pub(crate) fn esperando<E: Entorno>(e: &E) -> bool {
    cerrojo(&e.estado().pasar.receptor)
        .as_ref()
        .is_some_and(|r| r.estado.fase == FaseReceptor::Esperando && !r.ocupado)
}

pub fn estado<E: Entorno>(e: &E) -> Option<EstadoReceptor> {
    cerrojo(&e.estado().pasar.receptor)
        .as_ref()
        .map(|r| r.estado.clone())
}

/// Abre «Recibir»: el código, su QR y el anuncio en la red local. Cierra el
/// que hubiera.
pub async fn abrir<E: Entorno>(
    e: &E,
    ep: Endpoint,
    apodo: &str,
    con_rele: bool,
) -> AppResult<EstadoReceptor> {
    cerrar(e);
    // Con relé, el código lleva el suyo: se espera (un poco) a tenerlo.
    if con_rele {
        let _ = timeout(Duration::from_secs(5), ep.online()).await;
    }
    let st = e.estado();
    // Lo que quedara de un intercambio anterior (si no hay otro en marcha).
    if cerrojo(&st.pasar.emisor).is_none() {
        let _ = std::fs::remove_dir_all(super::carpeta_temporal(st));
    }
    let secreto = secreto_nuevo();
    let texto = Codigo {
        addr: encontrar::recortar(&ep.addr(), encontrar::MAX_IP),
        secreto: secreto.clone(),
    }
    .a_texto_deck()?;
    let qr = encontrar::qr_svg(&texto)?;
    e.multicast(UsoMulticast::Recibir, true);
    let anuncio = match Anuncio::nuevo(&ep, apodo) {
        Ok(a) => Some(a),
        Err(err) => {
            tracing::info!(codigo = ?err.code, "pasar: sin anuncio en la red local");
            None
        }
    };
    let gen = st.pasar.nueva_generacion();
    let estado = EstadoReceptor {
        fase: FaseReceptor::Esperando,
        codigo: texto,
        qr,
        caduca: ahora_ms() + CADUCIDAD.as_millis() as i64,
        cerca: anuncio.is_some(),
        remoto: None,
        codigo6: None,
        oferta: Vec::new(),
        progreso: None,
        resultados: Vec::new(),
        error: None,
    };
    let ep_id = ep.id();
    let caducidad = {
        let e = e.clone();
        tokio::spawn(async move {
            tokio::time::sleep(CADUCIDAD).await;
            let mut libre = false;
            mutar(&e, gen, |r| {
                if r.estado.fase == FaseReceptor::Esperando && !r.ocupado {
                    r.estado.fase = FaseReceptor::Caducado;
                    r.secreto = None;
                    r.parar();
                    libre = true;
                }
            });
            if libre {
                e.multicast(UsoMulticast::Recibir, false);
            }
        })
        .abort_handle()
    };
    *cerrojo(&st.pasar.receptor) = Some(Receptor {
        gen,
        ep_id,
        secreto: Some(secreto),
        estado: estado.clone(),
        ocupado: false,
        peticion: None,
        decisiones: None,
        conn: None,
        anuncio,
        tareas: vec![caducidad],
    });
    registrar(e, ep);
    e.emitir_receptor(&estado);
    Ok(estado)
}

/// Cierra «Recibir»: corta lo que hubiera en curso y deja de anunciarse.
pub fn cerrar<E: Entorno>(e: &E) {
    let viejo = cerrojo(&e.estado().pasar.receptor).take();
    if let Some(mut r) = viejo {
        if let Some(c) = r.conn.take() {
            c.close(CIERRE_CANCELADO.into(), b"cerrado");
        }
        r.parar();
        e.multicast(UsoMulticast::Recibir, false);
        // Sin «Recibir», el bucle propio no atiende el nodo: así no corta lo
        // que llegue para la mesa.
        retirar(r.ep_id);
    }
}

/// Cancela lo que haya: un intercambio en curso queda «cancelado» (se ve el
/// resultado); sin intercambio, se cierra el modo.
pub fn cancelar<E: Entorno>(e: &E) {
    let (gen, conn, hay) = {
        let mut g = cerrojo(&e.estado().pasar.receptor);
        match g.as_mut() {
            None => return,
            Some(r) => (r.gen, r.conn.take(), r.ocupado),
        }
    };
    if !hay {
        cerrar(e);
        return;
    }
    if let Some(c) = conn {
        c.close(CIERRE_CANCELADO.into(), b"cancelado");
    }
    let mut libre = false;
    let id = mutar(e, gen, |r| {
        if !es_final(r.estado.fase) {
            r.estado.fase = FaseReceptor::Cancelado;
        }
        r.peticion = None;
        r.decisiones = None;
        r.parar();
        libre = true;
        r.ep_id
    });
    if let Some(id) = id {
        retirar(id);
    }
    if libre {
        e.multicast(UsoMulticast::Recibir, false);
    }
}

/// Cambia el nombre con el que «Recibir» se anuncia en la red local.
pub fn renombrar<E: Entorno>(e: &E, nombre: &str) {
    if let Some(a) = cerrojo(&e.estado().pasar.receptor)
        .as_ref()
        .and_then(|r| r.anuncio.as_ref())
    {
        a.cambiar_apodo(nombre);
    }
}

/// Lo que elige la persona de la oferta.
pub fn responder<E: Entorno>(e: &E, decisiones: Vec<Decision>) -> AppResult<()> {
    let tx = cerrojo(&e.estado().pasar.receptor)
        .as_mut()
        .and_then(|r| r.decisiones.take())
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_oferta"))?;
    tx.send(decisiones)
        .map_err(|_| AppError::new(ErrorCode::NotFound).with("motivo", "sin_oferta"))
}

// ── Un intercambio ──────────────────────────────────────────────────────

/// Lo que se espera a que aparezca el camino directo (al principio la
/// conexión puede ir aún por el relé mientras iroh prueba las directas).
const ESPERA_CAMINO: Duration = Duration::from_secs(3);

/// Una IP de la red local: privada, de enlace, la propia máquina o el
/// espacio compartido `100.64.0.0/10` (CGNAT, y las VPN de malla como
/// Tailscale), que tampoco se enruta por internet.
pub(crate) fn ip_de_la_red_local(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v) => {
            v.is_private()
                || v.is_link_local()
                || v.is_loopback()
                || (v.octets()[0] == 100 && (v.octets()[1] & 0xc0) == 64)
        }
        std::net::IpAddr::V6(v) => match v.to_ipv4_mapped() {
            Some(v4) => ip_de_la_red_local(std::net::IpAddr::V4(v4)),
            None => v.is_loopback() || v.is_unique_local() || v.is_unicast_link_local(),
        },
    }
}

/// ¿Va la conexión por un camino directo a una dirección de la red local?
/// Espera un poco a que aparezca.
async fn camino_local(conn: &Connection) -> bool {
    let hay = || {
        conn.paths().iter().any(
            |p| matches!(p.remote_addr(), iroh::TransportAddr::Ip(a) if ip_de_la_red_local(a.ip())),
        )
    };
    let hasta = tokio::time::Instant::now() + ESPERA_CAMINO;
    loop {
        if hay() {
            return true;
        }
        if tokio::time::Instant::now() >= hasta || conn.close_reason().is_some() {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Cómo acaba un intercambio.
enum Salida {
    /// Se rechazó antes de empezar (código malo, otro equipo, …): se sigue
    /// esperando a quien toque.
    Rechazo,
    Cancelado,
    Error(AppError),
    Fin,
}

impl From<crate::red::protocolo::ErrorTrama> for Salida {
    fn from(e: crate::red::protocolo::ErrorTrama) -> Self {
        // Un corte es que el otro canceló (o se fue la red).
        match e {
            // Sin bytes en `PLAZO_SIN_BYTES`: el emisor se quedó parado.
            crate::red::protocolo::ErrorTrama::Io(io)
                if io.kind() == std::io::ErrorKind::TimedOut =>
            {
                Salida::Error(crate::error::red_sin_http("tiempo", None))
            }
            crate::red::protocolo::ErrorTrama::Io(_) => Salida::Cancelado,
            otro => Salida::Error(
                AppError::new(ErrorCode::InvalidInput)
                    .with("motivo", "protocolo")
                    .with("detalle", otro),
            ),
        }
    }
}

fn malo(motivo: &str) -> Salida {
    Salida::Error(AppError::new(ErrorCode::InvalidInput).with("motivo", motivo))
}

/// Reserva el modo para este intercambio. `None`: no se espera a nadie o ya
/// hay otro.
fn reservar<E: Entorno>(e: &E, conn: &Connection) -> Option<u64> {
    let (gen, snap) = {
        let mut g = cerrojo(&e.estado().pasar.receptor);
        let r = g.as_mut()?;
        if r.estado.fase != FaseReceptor::Esperando || r.ocupado {
            return None;
        }
        r.ocupado = true;
        r.conn = Some(conn.clone());
        (r.gen, r.estado.clone())
    };
    e.emitir_receptor(&snap);
    Some(gen)
}

pub(crate) async fn sesion<E: Entorno>(e: E, conn: Connection) {
    let Some(gen) = reservar(&e, &conn) else {
        conn.close(CIERRE_NO_ESPERADO.into(), b"ocupado");
        return;
    };
    let salida = interior(&e, gen, &conn).await;
    let mut libre = false;
    match salida {
        Salida::Rechazo => {
            // Nadie entró: se sigue esperando (y ya puede llegar otro).
            mutar(&e, gen, |r| {
                r.ocupado = false;
                r.conn = None;
                r.peticion = None;
                r.estado.fase = FaseReceptor::Esperando;
                r.estado.remoto = None;
                r.estado.codigo6 = None;
            });
            // Que le dé tiempo a leer el rechazo antes de cerrar.
            tokio::time::sleep(Duration::from_millis(200)).await;
            conn.close(CIERRE_NO_ESPERADO.into(), b"rechazado");
        }
        Salida::Fin => {
            conn.close(CIERRE_FIN.into(), b"fin");
            mutar(&e, gen, |r| {
                if !es_final(r.estado.fase) {
                    r.estado.fase = FaseReceptor::Terminado;
                }
                r.estado.progreso = None;
                r.conn = None;
                r.parar();
                libre = true;
            });
        }
        Salida::Cancelado => {
            conn.close(CIERRE_CANCELADO.into(), b"cancelado");
            mutar(&e, gen, |r| {
                if !es_final(r.estado.fase) {
                    r.estado.fase = FaseReceptor::Cancelado;
                }
                r.conn = None;
                r.parar();
                libre = true;
            });
        }
        Salida::Error(err) => {
            tracing::warn!(codigo = ?err.code, params = ?err.params, "pasar: se corta lo que se recibía");
            conn.close(CIERRE_CANCELADO.into(), b"error");
            mutar(&e, gen, |r| {
                if !es_final(r.estado.fase) {
                    r.estado.fase = FaseReceptor::Error;
                    r.estado.error = Some(err);
                }
                r.conn = None;
                r.parar();
                libre = true;
            });
        }
    }
    if libre {
        e.multicast(UsoMulticast::Recibir, false);
    }
}

async fn rechazar(
    tx: &mut iroh::endpoint::SendStream,
    motivo: &str,
) -> Result<(), crate::red::protocolo::ErrorTrama> {
    let r = escribir(
        tx,
        &AlEmisor::Rechazo {
            motivo: motivo.into(),
        },
    )
    .await;
    let _ = tx.finish();
    r
}

async fn interior<E: Entorno>(e: &E, gen: u64, conn: &Connection) -> Salida {
    let yo = conn.remote_id();
    let Ok(Ok((mut tx, mut rx))) = timeout(ESPERA_HOLA, conn.accept_bi()).await else {
        return Salida::Rechazo;
    };
    // Lo primero, el Hola (con tope y plazo): sin él no se lee nada más.
    let hola: AlReceptor = match timeout(ESPERA_HOLA, leer(&mut rx)).await {
        Ok(Ok(h)) => h,
        _ => return Salida::Rechazo,
    };
    let AlReceptor::Hola {
        version,
        secreto,
        apodo,
    } = hola
    else {
        return Salida::Rechazo;
    };
    if version != VERSION {
        let _ = rechazar(&mut tx, "version").await;
        return Salida::Rechazo;
    }
    let remoto = Remoto {
        apodo: apodo_limpio(&apodo),
        huella: encontrar::huella(&yo),
    };

    // ¿Puede entrar?
    match secreto {
        Some(s) => {
            let motivo = {
                let mut g = cerrojo(&e.estado().pasar.receptor);
                match g.as_mut().filter(|r| r.gen == gen) {
                    None => Some("caducado"),
                    Some(r) if ahora_ms() >= r.estado.caduca => Some("caducado"),
                    Some(r) => match r.secreto.as_deref() {
                        Some(bueno) if mismo_secreto(bueno, &s) => {
                            // Un solo uso.
                            r.secreto = None;
                            None
                        }
                        _ => Some("codigo"),
                    },
                }
            };
            if let Some(m) = motivo {
                let _ = rechazar(&mut tx, m).await;
                return Salida::Rechazo;
            }
        }
        None => {
            // Sin secreto solo vale desde la red local y si la persona lo
            // acepta: «Recibir» se anuncia por mDNS y la conexión llega por
            // un camino directo a una dirección de la red local (no por el
            // relé ni desde una IP pública).
            let cerca = cerrojo(&e.estado().pasar.receptor)
                .as_ref()
                .is_some_and(|r| r.gen == gen && r.estado.cerca);
            if !cerca || !camino_local(conn).await {
                let _ = rechazar(&mut tx, "codigo").await;
                return Salida::Rechazo;
            }
            let (pregunta, respuesta) = oneshot::channel::<bool>();
            let anunciado = mutar(e, gen, |r| {
                if !r.estado.cerca {
                    return false;
                }
                r.peticion = Some(pregunta);
                r.estado.fase = FaseReceptor::Peticion;
                r.estado.remoto = Some(remoto.clone());
                r.estado.codigo6 = Some(encontrar::codigo6(r.ep_id, yo));
                true
            })
            .unwrap_or(false);
            if !anunciado {
                let _ = rechazar(&mut tx, "codigo").await;
                return Salida::Rechazo;
            }
            let acepta = tokio::select! {
                r = timeout(ESPERA_PETICION, respuesta) => matches!(r, Ok(Ok(true))),
                _ = conn.closed() => return Salida::Cancelado,
            };
            if !acepta {
                let _ = rechazar(&mut tx, "rechazado").await;
                return Salida::Rechazo;
            }
        }
    }

    // Dentro. Desde aquí, lo que pase es un intercambio entero.
    let mi_apodo = super::nombre_equipo(e);
    mutar(e, gen, |r| {
        r.estado.fase = FaseReceptor::Conectado;
        r.estado.remoto = Some(remoto.clone());
        r.estado.codigo6 = Some(encontrar::codigo6(r.ep_id, yo));
    });
    if let Err(err) = escribir(&mut tx, &AlEmisor::Listo { apodo: mi_apodo }).await {
        return err.into();
    }

    // La oferta.
    let decks_ofrecidos = match timeout(ESPERA_OFERTA, leer::<_, AlReceptor>(&mut rx)).await {
        Ok(Ok(AlReceptor::Oferta { decks })) => decks,
        Ok(Ok(_)) => return malo("oferta"),
        Ok(Err(err)) => return err.into(),
        Err(_) => return malo("tiempo"),
    };
    if let Err(m) = validar_oferta(&decks_ofrecidos) {
        return malo(m);
    }
    let revisados: Vec<DeckRevisado> = decks_ofrecidos
        .into_iter()
        .map(|mut d| {
            d.portada = d.portada.take().and_then(|p| sanear_portada(&p));
            DeckRevisado {
                conflicto: conflicto_de(e, &d),
                deck: d,
            }
        })
        .collect();

    // Qué quiere la persona.
    let (elegir, esperar) = oneshot::channel::<Vec<Decision>>();
    mutar(e, gen, |r| {
        r.decisiones = Some(elegir);
        r.estado.oferta = revisados.clone();
        r.estado.fase = FaseReceptor::Oferta;
    });
    let decisiones = tokio::select! {
        r = timeout(ESPERA_DECISION, esperar) => match r {
            Ok(Ok(d)) => d,
            _ => return Salida::Cancelado,
        },
        _ = conn.closed() => return Salida::Cancelado,
    };
    let plan = match plan_de(&revisados, &decisiones) {
        Ok(p) => p,
        Err(m) => return malo(m),
    };
    let elecciones: Vec<Eleccion> = plan.iter().map(|p| p.eleccion).collect();
    if let Err(err) = escribir(
        &mut tx,
        &AlEmisor::Respuesta {
            decisiones: elecciones,
        },
    )
    .await
    {
        return err.into();
    }

    // Los paquetes, uno a uno.
    let aceptados = plan.iter().filter(|p| p.eleccion != Eleccion::No).count() as u32;
    let mut resultados: Vec<Resultado> = Vec::new();
    for (i, p) in plan.iter().enumerate() {
        if p.eleccion == Eleccion::No {
            resultados.push(Resultado {
                nombre: revisados[i].deck.nombre.clone(),
                deck_id: None,
                estado: EstadoResultado::Rechazado,
                error: None,
                faltan_imagenes: 0,
                reemplazado: false,
            });
        }
    }
    mutar(e, gen, |r| {
        r.estado.fase = FaseReceptor::Recibiendo;
        r.estado.resultados = resultados.clone();
    });
    let mut hechos = 0u32;
    for (i, p) in plan.iter().enumerate() {
        if p.eleccion == Eleccion::No {
            continue;
        }
        let nombre = revisados[i].deck.nombre.clone();
        let r = recibir_uno(
            e,
            gen,
            &mut rx,
            &mut tx,
            i as u32,
            &nombre,
            p,
            (hechos, aceptados),
        )
        .await;
        match r {
            Ok(res) => {
                resultados.push(res);
                hechos += 1;
                mutar(e, gen, |r| r.estado.resultados = resultados.clone());
            }
            Err(s) => {
                // Lo que ya llegó entero se queda; se enseña lo hecho.
                mutar(e, gen, |r| r.estado.resultados = resultados.clone());
                return s;
            }
        }
    }
    mutar(e, gen, |r| r.estado.progreso = None);

    // El final: el emisor dice `Fin` y se despide.
    match timeout(ESPERA_FIN, leer::<_, AlReceptor>(&mut rx)).await {
        Ok(Ok(AlReceptor::Fin)) | Ok(Err(_)) | Err(_) => {}
        Ok(Ok(_)) => return malo("fin"),
    }
    let _ = escribir(&mut tx, &AlEmisor::Adios).await;
    let _ = tx.finish();
    let _ = timeout(Duration::from_secs(5), conn.closed()).await;
    Salida::Fin
}

/// Una decisión ya comprobada: qué hacer con cada deck de la oferta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Plan {
    pub eleccion: Eleccion,
    /// El deck de aquí que se reemplaza, o `None` si se guarda uno nuevo.
    pub destino: Destino,
}

/// Comprueba las decisiones de la persona contra la oferta (una por deck, sin
/// repetir, y la resolución de los que ya existen) y las pone en orden.
pub(crate) fn plan_de(
    oferta: &[DeckRevisado],
    decisiones: &[Decision],
) -> Result<Vec<Plan>, &'static str> {
    if decisiones.len() != oferta.len() {
        return Err("decisiones");
    }
    let mut plan: Vec<Option<Plan>> = vec![None; oferta.len()];
    for d in decisiones {
        let i = d.indice as usize;
        let deck = oferta.get(i).ok_or("decisiones")?;
        if plan[i].is_some() {
            return Err("decisiones");
        }
        // «Solo originales» de un deck sin proxies es lo mismo que el deck.
        let eleccion = match d.eleccion {
            Eleccion::SoloOriginales if !deck.deck.proxies => Eleccion::ConProxies,
            otra => otra,
        };
        let destino = match (&deck.conflicto, d.resolucion) {
            _ if eleccion == Eleccion::No => Destino::Nuevo,
            (Some(c), Some(Resolucion::Reemplazar)) => Destino::Reemplazar(c.deck_id.clone()),
            (Some(_), Some(Resolucion::GuardarLosDos)) => Destino::GuardarLosDos,
            (Some(_), None) => return Err("falta_resolucion"),
            (None, _) => Destino::Nuevo,
        };
        plan[i] = Some(Plan { eleccion, destino });
    }
    plan.into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or("decisiones")
}

#[allow(clippy::too_many_arguments)]
async fn recibir_uno<E: Entorno>(
    e: &E,
    gen: u64,
    rx: &mut iroh::endpoint::RecvStream,
    tx: &mut iroh::endpoint::SendStream,
    indice: u32,
    nombre: &str,
    plan: &Plan,
    (decks_hechos, decks_total): (u32, u32),
) -> Result<Resultado, Salida> {
    let trama = match timeout(ESPERA_PAQUETE, leer::<_, AlReceptor>(rx)).await {
        Ok(Ok(t)) => t,
        Ok(Err(err)) => return Err(err.into()),
        Err(_) => return Err(malo("tiempo")),
    };
    let AlReceptor::Paquete {
        indice: i,
        tamano,
        huella,
    } = trama
    else {
        return Err(malo("paquete"));
    };
    if i != indice
        || tamano == 0
        || tamano > MAX_PAQUETE
        || huella.len() != 64
        || !huella.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(malo("paquete"));
    }
    let temporal = Temporal::nueva(e.estado()).map_err(|er| Salida::Error(er.into()))?;
    let ruta = temporal.0.join("deck.pbdeck");
    let mut fichero = tokio::fs::File::create(&ruta)
        .await
        .map_err(|er| Salida::Error(er.into()))?;
    // El avance, sin avisar más de 10 veces por segundo.
    let mut ultimo = std::time::Instant::now() - CADA_AVISO;
    let nombre_p = nombre.to_string();
    let llegada = recibir_bytes(rx, &mut fichero, tamano, |hecho| {
        if ultimo.elapsed() >= CADA_AVISO || hecho == tamano {
            ultimo = std::time::Instant::now();
            mutar(e, gen, |r| {
                r.estado.progreso = Some(Progreso {
                    indice,
                    nombre: nombre_p.clone(),
                    hechos: hecho,
                    total: tamano,
                    decks_hechos,
                    decks_total,
                });
            });
        }
    })
    .await;
    let real = match llegada {
        Ok(h) => h,
        Err(err) => return Err(err.into()),
    };
    drop(fichero);
    if !real.eq_ignore_ascii_case(&huella) {
        let _ = escribir(
            tx,
            &AlEmisor::Hecho {
                indice,
                ok: false,
                motivo: Some("huella".into()),
            },
        )
        .await;
        return Err(malo("huella"));
    }

    // Importarlo. Si algo falla, el deck no existe y se sigue con el resto.
    let opciones = Opciones {
        solo_originales: plan.eleccion == Eleccion::SoloOriginales,
        destino: plan.destino.clone(),
    };
    let st = e.estado();
    let importado = super::bloqueante(|| -> AppResult<_> {
        let f = std::fs::File::open(&ruta)?;
        package::import_con(std::io::BufReader::new(f), &st.blobs, &st.db, &opciones)
    });
    drop(temporal);
    let res = match importado {
        Ok(deck) => {
            let _ = escribir(
                tx,
                &AlEmisor::Hecho {
                    indice,
                    ok: true,
                    motivo: None,
                },
            )
            .await;
            Resultado {
                nombre: nombre.to_string(),
                deck_id: Some(deck.id.clone()),
                estado: EstadoResultado::Hecho,
                error: None,
                faltan_imagenes: proxy_core::compartir::pendientes(&deck).len() as u32,
                reemplazado: matches!(plan.destino, Destino::Reemplazar(_)),
            }
        }
        Err(err) => {
            tracing::warn!(codigo = ?err.code, params = ?err.params, "pasar: no se pudo importar lo recibido");
            let _ = escribir(
                tx,
                &AlEmisor::Hecho {
                    indice,
                    ok: false,
                    motivo: Some("importar".into()),
                },
            )
            .await;
            Resultado {
                nombre: nombre.to_string(),
                deck_id: None,
                estado: EstadoResultado::Fallo,
                error: Some(err),
                faltan_imagenes: 0,
                reemplazado: false,
            }
        }
    };
    Ok(res)
}

/// El deck de aquí que ya es el mismo que el que llega (mismo linaje).
pub(crate) fn conflicto_de<E: Entorno>(e: &E, d: &DeckOfrecido) -> Option<Conflicto> {
    conflicto_en(&e.estado().db, d)
}

pub(crate) fn conflicto_en(db: &crate::store::db::Db, d: &DeckOfrecido) -> Option<Conflicto> {
    let l = d.linaje.as_deref()?;
    let (deck_id, actualizado) = linaje::decks_de(db, l).ok()?.into_iter().next()?;
    let nombre = decks::list(db)
        .ok()?
        .into_iter()
        .find(|x| x.id == deck_id)?
        .name;
    let mas_reciente = match d.actualizado {
        None => MasReciente::Desconocido,
        Some(a) if a == actualizado => MasReciente::Igual,
        Some(a) if a > actualizado => MasReciente::Llega,
        Some(_) => MasReciente::Aqui,
    };
    Some(Conflicto {
        deck_id,
        nombre,
        actualizado,
        mas_reciente,
    })
}

/// La portada de fuera, vuelta a hacer aquí: lo que ve la pantalla es siempre
/// un JPEG pequeño que ha generado esta app, nunca los bytes de otro equipo.
pub(crate) fn sanear_portada(b64: &str) -> Option<String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64).ok()?;
    if bytes.len() > super::protocolo::MAX_PORTADA {
        return None;
    }
    // Medidas de cabecera antes de decodificar: nada de imágenes enormes.
    let t = imagesize::blob_size(&bytes).ok()?;
    if t.width == 0 || t.height == 0 || t.width > 512 || t.height > 512 {
        return None;
    }
    let jpeg = package::miniatura(&bytes)?;
    Some(base64::engine::general_purpose::STANDARD.encode(jpeg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_las_ips_de_la_red_local_entran_sin_secreto() {
        for ip in [
            "192.168.1.20",
            "10.0.0.5",
            "172.16.3.4",
            "169.254.1.1",
            "127.0.0.1",
            "100.64.0.5",
            "::1",
            "fd12:3456::1",
            "fe80::1",
            "::ffff:192.168.0.2",
        ] {
            assert!(ip_de_la_red_local(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "8.8.8.8",
            "100.128.0.1",
            "2001:db8::1",
            "2a00:1450::1",
            "::ffff:8.8.8.8",
        ] {
            assert!(!ip_de_la_red_local(ip.parse().unwrap()), "{ip}");
        }
    }

    fn oferta(proxies: bool, conflicto: bool) -> DeckRevisado {
        DeckRevisado {
            deck: DeckOfrecido {
                nombre: "D".into(),
                cartas: 1,
                proxies,
                tamano: 1,
                tamano_originales: 1,
                linaje: Some("l".into()),
                actualizado: Some(5),
                portada: None,
                optimizado: false,
            },
            conflicto: conflicto.then(|| Conflicto {
                deck_id: "local".into(),
                nombre: "D".into(),
                actualizado: 3,
                mas_reciente: MasReciente::Llega,
            }),
        }
    }

    fn dec(i: u32, e: Eleccion, r: Option<Resolucion>) -> Decision {
        Decision {
            indice: i,
            eleccion: e,
            resolucion: r,
        }
    }

    #[test]
    fn el_plan_exige_una_decision_por_deck_y_la_resolucion_de_los_que_existen() {
        let of = vec![
            oferta(true, false),
            oferta(true, true),
            oferta(false, false),
        ];
        let p = plan_de(
            &of,
            &[
                dec(2, Eleccion::SoloOriginales, None),
                dec(0, Eleccion::SoloOriginales, None),
                dec(1, Eleccion::ConProxies, Some(Resolucion::Reemplazar)),
            ],
        )
        .unwrap();
        assert_eq!(p[0].eleccion, Eleccion::SoloOriginales);
        assert_eq!(p[1].destino, Destino::Reemplazar("local".into()));
        // Sin proxies no hay «solo originales»: es el deck tal cual.
        assert_eq!(p[2].eleccion, Eleccion::ConProxies);
        // Guardar los dos.
        let p = plan_de(
            &of[1..2],
            &[dec(
                0,
                Eleccion::ConProxies,
                Some(Resolucion::GuardarLosDos),
            )],
        )
        .unwrap();
        assert_eq!(p[0].destino, Destino::GuardarLosDos);
        // «No» no necesita resolución.
        assert!(plan_de(&of[1..2], &[dec(0, Eleccion::No, None)]).is_ok());
        // Faltan, sobran, se repiten o no existen.
        assert!(plan_de(&of, &[dec(0, Eleccion::No, None)]).is_err());
        assert!(plan_de(
            &of[..1],
            &[dec(0, Eleccion::No, None), dec(0, Eleccion::No, None)]
        )
        .is_err());
        assert!(plan_de(&of[..1], &[dec(1, Eleccion::No, None)]).is_err());
        // Uno que existe y se acepta, sin decir qué hacer, no vale.
        assert_eq!(
            plan_de(&of[1..2], &[dec(0, Eleccion::ConProxies, None)]).unwrap_err(),
            "falta_resolucion"
        );
    }

    #[test]
    fn una_portada_de_fuera_se_vuelve_a_hacer_o_se_tira() {
        use base64::Engine;
        let b = base64::engine::general_purpose::STANDARD;
        assert!(sanear_portada("esto no es base64 !!").is_none());
        assert!(sanear_portada(&b.encode(b"no es una imagen")).is_none());
        // Una imagen de verdad pequeña se acepta y sale JPEG.
        let png = crate::store::blobs::tests::png(30, 42, 3);
        let s = sanear_portada(&b.encode(&png)).unwrap();
        assert!(b.decode(s).unwrap().starts_with(&[0xff, 0xd8]));
        // Una grande, no.
        let grande = crate::store::blobs::tests::png(600, 700, 3);
        assert!(!grande.is_empty());
        assert!(sanear_portada(&b.encode(&grande)).is_none());
    }
}
