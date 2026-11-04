//! **Enviar decks**: el lado de quien manda.
//!
//! Se elige a quién (uno de los equipos cerca o un código `PBD1-…` pegado o
//! leído de un QR), qué decks y en qué modo (`package::Modo`). Después: `Hola` →
//! `Oferta` (lo que cuenta cada deck, ≤ 64 KiB) → espera lo que elija quien
//! recibe → un paquete `.pbdeck` por cada deck aceptado, hecho en la caché
//! justo antes de mandarlo y borrado al acabar.
//!
//! Se puede cancelar en cualquier momento; el otro lado lo ve y no importa
//! nada de lo que no haya llegado entero.

use std::time::Duration;

use base64::Engine;
use iroh::endpoint::Connection;
use iroh::Endpoint;
use tokio::task::AbortHandle;
use tokio::time::timeout;

use super::encontrar;
use super::protocolo::{
    apodo_limpio, enviar_bytes, escribir, hex, leer, AlEmisor, AlReceptor, MAX_DECKS, MAX_PAQUETE,
    MAX_TOTAL, TOPE_TRAMA, VERSION,
};
use super::tipos::{
    DeckOfrecido, Destino, Eleccion, EstadoEmisor, EstadoResultado, FaseEmisor, Progreso, Remoto,
    Resultado,
};
use super::{cerrojo, Entorno, Temporal, CIERRE_CANCELADO, CIERRE_FIN, CIERRE_NO_ESPERADO};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::package::{self, Modo};
use crate::red::codigo::Codigo;
use crate::red::nodo::ALPN_DECK;
use crate::store::{decks, linaje};

const ESPERA_CONEXION: Duration = Duration::from_secs(30);
const ESPERA_LISTO: Duration = Duration::from_secs(30);
/// Con el código de 6 cifras, la persona del otro lado tiene que aceptar.
const ESPERA_ACEPTAR: Duration = Duration::from_secs(90);
const ESPERA_RESPUESTA: Duration = Duration::from_secs(15 * 60);
const ESPERA_HECHO: Duration = Duration::from_secs(10 * 60);
const ESPERA_ADIOS: Duration = Duration::from_secs(15);
const CADA_AVISO: Duration = Duration::from_millis(100);
/// Las portadas se quitan si la oferta no cabe con ellas.
const OFERTA_SIN_PORTADAS: usize = TOPE_TRAMA - 4 * 1024;

/// El envío en marcha (uno por app).
pub(crate) struct Emisor {
    pub gen: u64,
    pub estado: EstadoEmisor,
    pub conn: Option<Connection>,
    pub tarea: Option<AbortHandle>,
}

fn es_final(f: FaseEmisor) -> bool {
    matches!(
        f,
        FaseEmisor::Terminado | FaseEmisor::Cancelado | FaseEmisor::Error
    )
}

fn mutar<E: Entorno, R>(e: &E, gen: u64, f: impl FnOnce(&mut Emisor) -> R) -> Option<R> {
    let (r, snap) = {
        let mut g = cerrojo(&e.estado().pasar.emisor);
        let r = g.as_mut().filter(|r| r.gen == gen)?;
        let out = f(r);
        (out, r.estado.clone())
    };
    e.emitir_emisor(&snap);
    Some(r)
}

pub fn estado<E: Entorno>(e: &E) -> Option<EstadoEmisor> {
    cerrojo(&e.estado().pasar.emisor)
        .as_ref()
        .map(|r| r.estado.clone())
}

/// Quita el envío terminado (para empezar otro o cerrar la pantalla).
pub fn olvidar<E: Entorno>(e: &E) {
    let mut g = cerrojo(&e.estado().pasar.emisor);
    if g.as_ref().is_some_and(|r| es_final(r.estado.fase)) {
        *g = None;
    }
}

/// Cancela el envío en marcha.
pub fn cancelar<E: Entorno>(e: &E) {
    let (gen, conn, tarea) = {
        let mut g = cerrojo(&e.estado().pasar.emisor);
        match g.as_mut() {
            Some(r) if !es_final(r.estado.fase) => (r.gen, r.conn.take(), r.tarea.take()),
            _ => return,
        }
    };
    if let Some(c) = conn {
        c.close(CIERRE_CANCELADO.into(), b"cancelado");
    }
    if let Some(t) = tarea {
        t.abort();
    }
    mutar(e, gen, |r| r.estado.fase = FaseEmisor::Cancelado);
}

/// Un fallo de red sin error de fondo (lo apunta `error::red_sin_http`).
fn red(motivo: &str) -> AppError {
    crate::error::red_sin_http(motivo, None)
}

/// Los motivos de `Hecho` que se le creen al otro equipo; cualquier otro
/// texto de fuera pasa a `importar`.
fn motivo_de_hecho(m: Option<String>) -> &'static str {
    match m.as_deref() {
        Some("huella") => "huella",
        _ => "importar",
    }
}

/// Lo que quien recibe rechazaría por tamaño (`protocolo::validar_oferta`),
/// antes de llamar a nadie: así el emisor dice por qué y no «Cancelado».
fn comprobar_tamanos(tamanos: &[(u64, u64)]) -> AppResult<()> {
    let mut total = 0u64;
    for &(tamano, originales) in tamanos {
        if tamano > MAX_PAQUETE || originales > MAX_PAQUETE {
            return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "paquete_grande"));
        }
        total = total.saturating_add(tamano);
    }
    if total > MAX_TOTAL {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "envio_grande"));
    }
    Ok(())
}

/// Empieza a enviar `ids` a `destino`. Vuelve en cuanto está lanzado; lo que
/// pase llega por `pasar://emisor`.
pub async fn enviar<E: Entorno>(
    e: &E,
    ep: Endpoint,
    destino: Destino,
    ids: Vec<String>,
    modo: Modo,
) -> AppResult<()> {
    let st = e.estado();
    if ids.is_empty() || ids.len() > MAX_DECKS {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "decks"));
    }
    // Los decks tienen que existir ya, antes de llamar a nadie, y caber.
    let mut tamanos = Vec::with_capacity(ids.len());
    for id in &ids {
        let deck = decks::open(&st.db, id)?;
        let sin = package::tamano_estimado(&deck, &st.blobs, &st.db, Modo::SinProxies);
        let con = package::tamano_estimado(&deck, &st.blobs, &st.db, modo);
        tamanos.push((con, sin));
    }
    comprobar_tamanos(&tamanos)?;
    // A quién.
    let (addr, secreto, con_aceptacion) = match destino {
        Destino::Codigo { texto } => {
            let c = Codigo::de_texto_deck(&texto)?;
            (c.addr, Some(c.secreto), false)
        }
        Destino::Cerca { id } => {
            let a = cerrojo(&st.pasar.busqueda)
                .as_ref()
                .and_then(|b| b.direccion(&id))
                .ok_or_else(|| red("no_esta"))?;
            (a, None, true)
        }
    };
    if addr.id == ep.id() {
        return Err(red("uno_mismo"));
    }
    let gen = st.pasar.nueva_generacion();
    {
        let mut g = cerrojo(&st.pasar.emisor);
        if g.as_ref().is_some_and(|r| !es_final(r.estado.fase)) {
            return Err(AppError::new(ErrorCode::Busy).with("motivo", "enviando"));
        }
        *g = Some(Emisor {
            gen,
            estado: EstadoEmisor {
                fase: FaseEmisor::Conectando,
                remoto: None,
                codigo6: None,
                decks: Vec::new(),
                progreso: None,
                resultados: Vec::new(),
                error: None,
            },
            conn: None,
            tarea: None,
        });
    }
    // Lo que quedara de un intercambio anterior (si no se está recibiendo).
    if cerrojo(&st.pasar.receptor).is_none() {
        let _ = std::fs::remove_dir_all(super::carpeta_temporal(st));
    }
    let e2 = e.clone();
    let tarea = tokio::spawn(async move {
        let r = sesion(&e2, gen, ep, addr, secreto, con_aceptacion, ids, modo).await;
        match r {
            Ok(()) => {
                mutar(&e2, gen, |r| {
                    if !es_final(r.estado.fase) {
                        r.estado.fase = FaseEmisor::Terminado;
                    }
                    r.estado.progreso = None;
                    r.conn = None;
                });
            }
            Err(Fin::Cancelado) => {
                mutar(&e2, gen, |r| {
                    if !es_final(r.estado.fase) {
                        r.estado.fase = FaseEmisor::Cancelado;
                    }
                    r.conn = None;
                });
            }
            Err(Fin::Error(err)) => {
                tracing::warn!(codigo = ?err.code, params = ?err.params, "pasar: se corta el envío");
                mutar(&e2, gen, |r| {
                    if !es_final(r.estado.fase) {
                        r.estado.fase = FaseEmisor::Error;
                        r.estado.error = Some(err);
                    }
                    r.conn = None;
                });
            }
        }
    })
    .abort_handle();
    mutar(e, gen, |r| r.tarea = Some(tarea));
    Ok(())
}

enum Fin {
    Cancelado,
    Error(AppError),
}

impl From<crate::red::protocolo::ErrorTrama> for Fin {
    fn from(e: crate::red::protocolo::ErrorTrama) -> Self {
        match e {
            // El otro cortó (o se fue la red): para esta pantalla es que
            // se canceló desde allí.
            crate::red::protocolo::ErrorTrama::Io(_) => Fin::Cancelado,
            otro => Fin::Error(
                AppError::new(ErrorCode::InvalidInput)
                    .with("motivo", "protocolo")
                    .with("detalle", otro),
            ),
        }
    }
}

impl From<AppError> for Fin {
    fn from(e: AppError) -> Self {
        Fin::Error(e)
    }
}

/// El nombre de un deck en una oferta de fuera no puede tener saltos de línea
/// ni ser vacío: el de aquí tampoco los lleva, pero por si acaso.
fn nombre_para_oferta(n: &str) -> String {
    let limpio = apodo_limpio_largo(n);
    if limpio.is_empty() {
        "—".into()
    } else {
        limpio
    }
}

fn apodo_limpio_largo(n: &str) -> String {
    n.chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(super::protocolo::MAX_NOMBRE)
        .collect()
}

/// La portada del deck para la oferta: un JPEG pequeño en base64.
fn portada_de(estado: &crate::state::AppState, deck: &proxy_core::model::Deck) -> Option<String> {
    let elegida = decks::portada_de(&estado.db, &deck.id).ok().flatten();
    let carta = elegida
        .as_deref()
        .and_then(|id| deck.cards.iter().find(|c| c.id == id && c.front.is_some()))
        .or_else(|| deck.cards.iter().find(|c| c.front.is_some()))?;
    let f = carta.front.as_ref()?;
    let (bytes, _) = estado.leer_blob(&f.blob).ok()?;
    let jpeg = package::miniatura(&bytes)?;
    Some(base64::engine::general_purpose::STANDARD.encode(jpeg))
}

/// Lo que se cuenta de cada deck.
pub(crate) fn oferta_de<E: Entorno>(
    e: &E,
    ids: &[String],
    modo: Modo,
) -> AppResult<Vec<DeckOfrecido>> {
    let st = e.estado();
    let mut out = Vec::new();
    for id in ids {
        let deck = decks::open(&st.db, id)?;
        let lin = linaje::de(&st.db, id)?;
        let hay = proxy_core::compartir::resumen_proxies(&deck).hay();
        let sin = package::tamano_estimado(&deck, &st.blobs, &st.db, Modo::SinProxies);
        let manda_proxies = hay && modo != Modo::SinProxies;
        out.push(DeckOfrecido {
            nombre: nombre_para_oferta(&deck.name),
            cartas: deck.cards.len() as u32,
            proxies: manda_proxies,
            tamano: if manda_proxies {
                package::tamano_estimado(&deck, &st.blobs, &st.db, modo)
            } else {
                sin
            },
            tamano_originales: sin,
            linaje: Some(lin.linaje),
            actualizado: Some(lin.actualizado),
            portada: portada_de(st, &deck),
            optimizado: manda_proxies && modo == Modo::Optimizado,
        });
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
async fn sesion<E: Entorno>(
    e: &E,
    gen: u64,
    ep: Endpoint,
    addr: iroh::EndpointAddr,
    secreto: Option<String>,
    con_aceptacion: bool,
    ids: Vec<String>,
    modo: Modo,
) -> Result<(), Fin> {
    let st = e.estado();
    let conn = match timeout(ESPERA_CONEXION, ep.connect(addr, ALPN_DECK)).await {
        Ok(Ok(c)) => c,
        Ok(Err(err)) => {
            return Err(Fin::Error(crate::error::red_sin_http(
                "conexion",
                Some(&err),
            )));
        }
        Err(_) => return Err(Fin::Error(red("tiempo"))),
    };
    mutar(e, gen, |r| r.conn = Some(conn.clone()));
    let remoto_id = conn.remote_id();
    let codigo6 = encontrar::codigo6(ep.id(), remoto_id);
    let yo = super::nombre_equipo(e);

    let (mut tx, mut rx) = conn.open_bi().await.map_err(|_| Fin::Cancelado)?;
    escribir(
        &mut tx,
        &AlReceptor::Hola {
            version: VERSION,
            secreto,
            apodo: yo,
        },
    )
    .await?;
    if con_aceptacion {
        mutar(e, gen, |r| {
            r.estado.fase = FaseEmisor::EsperandoAceptacion;
            r.estado.codigo6 = Some(codigo6.clone());
        });
    }
    let espera = if con_aceptacion {
        ESPERA_ACEPTAR
    } else {
        ESPERA_LISTO
    };
    let respuesta = match timeout(espera, leer::<_, AlEmisor>(&mut rx)).await {
        Ok(Ok(r)) => r,
        Ok(Err(crate::red::protocolo::ErrorTrama::Io(_))) => {
            // Sin respuesta y con la conexión cerrada por el otro: no espera
            // un deck, o es otro intercambio.
            return Err(match conn.close_reason() {
                Some(iroh::endpoint::ConnectionError::ApplicationClosed(c))
                    if c.error_code == CIERRE_NO_ESPERADO.into() =>
                {
                    Fin::Error(red("no_espera"))
                }
                _ => Fin::Cancelado,
            });
        }
        Ok(Err(err)) => return Err(err.into()),
        Err(_) => return Err(Fin::Error(red("tiempo"))),
    };
    let apodo_otro = match respuesta {
        AlEmisor::Listo { apodo } => apodo_limpio(&apodo),
        AlEmisor::Rechazo { motivo } => {
            let m = match motivo.as_str() {
                m @ ("version" | "codigo" | "caducado" | "rechazado" | "ocupado") => m,
                _ => "rechazado",
            };
            return Err(Fin::Error(red(m)));
        }
        _ => return Err(Fin::Error(red("protocolo"))),
    };
    mutar(e, gen, |r| {
        r.estado.remoto = Some(Remoto {
            apodo: apodo_otro,
            huella: encontrar::huella(&remoto_id),
        });
        r.estado.codigo6 = Some(codigo6.clone());
    });

    // La oferta.
    let mut oferta = super::bloqueante(|| oferta_de(e, &ids, modo))?;
    let mut trama = serde_json::to_vec(&AlReceptor::Oferta {
        decks: oferta.clone(),
    })
    .map_err(|er| Fin::Error(AppError::new(ErrorCode::Internal).with("detalle", er)))?;
    if trama.len() > OFERTA_SIN_PORTADAS {
        for d in &mut oferta {
            d.portada = None;
        }
        trama = serde_json::to_vec(&AlReceptor::Oferta {
            decks: oferta.clone(),
        })
        .map_err(|er| Fin::Error(AppError::new(ErrorCode::Internal).with("detalle", er)))?;
    }
    if trama.len() > TOPE_TRAMA {
        return Err(Fin::Error(
            AppError::new(ErrorCode::InvalidInput).with("motivo", "oferta_grande"),
        ));
    }
    crate::red::protocolo::escribir_trama(&mut tx, &trama, TOPE_TRAMA).await?;
    mutar(e, gen, |r| {
        r.estado.decks = oferta.clone();
        r.estado.fase = FaseEmisor::EsperandoRespuesta;
    });

    // Lo que elige el otro.
    let decisiones = tokio::select! {
        r = timeout(ESPERA_RESPUESTA, leer::<_, AlEmisor>(&mut rx)) => match r {
            Ok(Ok(AlEmisor::Respuesta { decisiones })) => decisiones,
            Ok(Ok(_)) => return Err(Fin::Error(red("protocolo"))),
            Ok(Err(err)) => return Err(err.into()),
            Err(_) => return Err(Fin::Error(red("tiempo"))),
        },
        _ = conn.closed() => return Err(Fin::Cancelado),
    };
    if decisiones.len() != oferta.len() {
        return Err(Fin::Error(red("protocolo")));
    }

    let aceptados = decisiones.iter().filter(|d| **d != Eleccion::No).count() as u32;
    let mut resultados: Vec<Resultado> = oferta
        .iter()
        .zip(&decisiones)
        .filter(|(_, d)| **d == Eleccion::No)
        .map(|(o, _)| Resultado {
            nombre: o.nombre.clone(),
            deck_id: None,
            estado: EstadoResultado::Rechazado,
            error: None,
            faltan_imagenes: 0,
            reemplazado: false,
        })
        .collect();
    mutar(e, gen, |r| {
        r.estado.fase = FaseEmisor::Preparando;
        r.estado.resultados = resultados.clone();
    });

    // Los paquetes.
    let mut hechos = 0u32;
    for (i, (id, eleccion)) in ids.iter().zip(&decisiones).enumerate() {
        if *eleccion == Eleccion::No {
            continue;
        }
        let nombre = oferta[i].nombre.clone();
        // Lo que el emisor decidió manda: nunca más proxies de los que quiso.
        let modo_deck = if *eleccion == Eleccion::SoloOriginales {
            Modo::SinProxies
        } else {
            modo
        };
        let temporal = Temporal::nueva(st).map_err(|er| Fin::Error(er.into()))?;
        let ruta = temporal.0.join("deck.pbdeck");
        // Comprimir puede llevar un rato: se ve cómo va y se puede cancelar.
        let avisar = |hechas: u32, total: u32| {
            mutar(e, gen, |r| {
                if es_final(r.estado.fase) {
                    return false;
                }
                r.estado.fase = FaseEmisor::Preparando;
                r.estado.progreso = Some(Progreso {
                    indice: i as u32,
                    nombre: nombre.clone(),
                    hechos: u64::from(hechas),
                    total: u64::from(total),
                    decks_hechos: hechos,
                    decks_total: aceptados,
                });
                true
            })
            .unwrap_or(false)
        };
        let (tamano, huella) = super::bloqueante(|| -> AppResult<(u64, String)> {
            let deck = decks::open(&st.db, id)?;
            let tamano =
                package::export_en_fichero(&deck, &ruta, &st.blobs, &st.db, modo_deck, &avisar)?;
            Ok((tamano, huella_de(&ruta)?))
        })?;
        mutar(e, gen, |r| r.estado.fase = FaseEmisor::Enviando);
        // Lo estimado podía caber y el paquete de verdad no: el otro lo
        // cortaría a medias.
        if tamano > MAX_PAQUETE {
            return Err(Fin::Error(
                AppError::new(ErrorCode::InvalidInput).with("motivo", "paquete_grande"),
            ));
        }
        crate::red::protocolo::escribir_trama(
            &mut tx,
            &serde_json::to_vec(&AlReceptor::Paquete {
                indice: i as u32,
                tamano,
                huella,
            })
            .map_err(|er| Fin::Error(AppError::new(ErrorCode::Internal).with("detalle", er)))?,
            TOPE_TRAMA,
        )
        .await?;
        let mut fichero = tokio::fs::File::open(&ruta)
            .await
            .map_err(|er| Fin::Error(er.into()))?;
        let mut ultimo = std::time::Instant::now() - CADA_AVISO;
        let nombre_p = nombre.clone();
        enviar_bytes(&mut fichero, &mut tx, tamano, |hecho| {
            if ultimo.elapsed() >= CADA_AVISO || hecho == tamano {
                ultimo = std::time::Instant::now();
                mutar(e, gen, |r| {
                    r.estado.progreso = Some(Progreso {
                        indice: i as u32,
                        nombre: nombre_p.clone(),
                        hechos: hecho,
                        total: tamano,
                        decks_hechos: hechos,
                        decks_total: aceptados,
                    });
                });
            }
        })
        .await?;
        drop(fichero);
        drop(temporal);
        // Quien recibe lo importa y avisa.
        let hecho = match timeout(ESPERA_HECHO, leer::<_, AlEmisor>(&mut rx)).await {
            Ok(Ok(AlEmisor::Hecho { indice, ok, motivo })) if indice == i as u32 => (ok, motivo),
            Ok(Ok(_)) => return Err(Fin::Error(red("protocolo"))),
            Ok(Err(err)) => return Err(err.into()),
            Err(_) => return Err(Fin::Error(red("tiempo"))),
        };
        resultados.push(Resultado {
            nombre,
            deck_id: None,
            estado: if hecho.0 {
                EstadoResultado::Hecho
            } else {
                EstadoResultado::Fallo
            },
            error: (!hecho.0).then(|| {
                AppError::new(ErrorCode::InvalidInput).with("motivo", motivo_de_hecho(hecho.1))
            }),
            faltan_imagenes: 0,
            reemplazado: false,
        });
        hechos += 1;
        mutar(e, gen, |r| r.estado.resultados = resultados.clone());
    }
    mutar(e, gen, |r| r.estado.progreso = None);

    // El final.
    escribir(&mut tx, &AlReceptor::Fin).await?;
    let _ = tx.finish();
    let _ = timeout(ESPERA_ADIOS, leer::<_, AlEmisor>(&mut rx)).await;
    conn.close(CIERRE_FIN.into(), b"fin");
    Ok(())
}

/// El SHA-256 de un fichero, en hexadecimal.
fn huella_de(ruta: &std::path::Path) -> AppResult<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut f = std::fs::File::open(ruta)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex(&h.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_que_no_cabe_se_rechaza_antes_de_llamar() {
        assert!(comprobar_tamanos(&[(10, 5), (MAX_PAQUETE, MAX_PAQUETE)]).is_ok());
        let e = comprobar_tamanos(&[(MAX_PAQUETE + 1, 5)]).unwrap_err();
        assert_eq!(e.params["motivo"], "paquete_grande");
        // Aunque se mande con proxies, quien recibe mira también el de sin.
        let e = comprobar_tamanos(&[(5, MAX_PAQUETE + 1)]).unwrap_err();
        assert_eq!(e.params["motivo"], "paquete_grande");
        let muchos = vec![(MAX_PAQUETE, 1); (MAX_TOTAL / MAX_PAQUETE) as usize + 1];
        let e = comprobar_tamanos(&muchos).unwrap_err();
        assert_eq!(e.params["motivo"], "envio_grande");
    }

    #[test]
    fn el_motivo_de_fuera_va_por_una_lista_cerrada() {
        assert_eq!(motivo_de_hecho(Some("huella".into())), "huella");
        assert_eq!(motivo_de_hecho(Some("importar".into())), "importar");
        assert_eq!(motivo_de_hecho(None), "importar");
        assert_eq!(
            motivo_de_hecho(Some("<script>\u{1b}[31m".into())),
            "importar"
        );
    }
}
