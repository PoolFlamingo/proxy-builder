//! Pruebas de la partida en red en el mismo proceso: nodos de iroh sin relé
//! (direcciones locales), un anfitrión y sus invitados, cada uno con su
//! sesión en memoria (`MesaEnMemoria`).

use std::sync::Arc;
use std::time::Duration;

use iroh::{Endpoint, SecretKey};
use mesa::accion::{Accion, Destino, Sitio, ZonaDestino};
use mesa::modelo::{Pos, RefZona, Zona};
use mesa::{Azar, IdJugador};

use super::anfitrion::{atender, empezar_partida, Anfitrion};
use super::codigo::{secreto_nuevo, Codigo};
use super::imagenes::{la_tengo, ruta_cruda, traer_propia, Por};
use super::invitado::{unirse, Conexion, Union};
use super::latencia;
use super::mesa::{AvisoRed, Mesa, MesaEnMemoria};
use super::nodo::{crear, Relay, ALPN_MESA};
use super::protocolo::{datagrama_de_bytes, Datagrama, Enlace, Rol};
use crate::mesa::imagenes::propia_en;
use crate::mesa::sesion::tests::mazo;
use crate::mesa::sesion::{ConfigMesa, JugadorMesa, RedSesion};
use mesa::formato::{Formato, Variantes};
use mesa::modelo::RefImagen;
use mesa::preparar::DeckDeMesa;
use mesa::sparring::Intensidad;

struct Jugador {
    mesa: MesaEnMemoria,
    ep: Endpoint,
    conexion: Option<Conexion>,
}

struct Partida3 {
    anf: Arc<Anfitrion>,
    host: Jugador,
    invitados: Vec<Jugador>,
    codigo: Codigo,
}

async fn esperar(que: &str, cond: impl Fn() -> bool) {
    for _ in 0..500 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no llegó: {que}");
}

fn seq(m: &MesaEnMemoria) -> Option<u64> {
    m.con(|s| s.as_ref().map(|s| s.partida().seq))
}

fn huella(m: &MesaEnMemoria) -> Option<String> {
    m.con(|s| s.as_ref().map(|s| mesa::huella(s.partida())))
}

fn pendientes(m: &MesaEnMemoria) -> usize {
    m.con(|s| match s.as_ref().and_then(|s| s.red.as_ref()) {
        Some(RedSesion::Invitado(i)) => i.pendientes.len(),
        _ => 0,
    })
}

/// Un anfitrión y `n` invitados en la sala, y la partida empezada.
async fn partida(n: usize) -> Partida3 {
    partida_con((0..n).map(|_| mazo()).collect()).await
}

/// Lo mismo, con el deck de cada invitado.
async fn partida_con(decks: Vec<DeckDeMesa>) -> Partida3 {
    let n = decks.len();
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
    let host = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        conexion: None,
    };
    let anf = Anfitrion::nuevo("Host".into(), secreto_nuevo());
    *anf.deck.lock().unwrap() = Some("Prueba".into());
    tokio::spawn(atender(host.ep.clone(), anf.clone(), host.mesa.clone()));
    let codigo = Codigo {
        addr: host.ep.addr(),
        secreto: anf.secreto().to_string(),
    };
    let mut invitados = Vec::new();
    for (i, deck) in decks.into_iter().enumerate() {
        let mut j = Jugador {
            mesa: MesaEnMemoria::default(),
            ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
            conexion: None,
        };
        let c = unirse(
            j.ep.clone(),
            Union {
                codigo: codigo.clone(),
                apodo: format!("Invitado {i}"),
                rol: Rol::Jugador,
                deck: Some(deck),
            },
            j.mesa.clone(),
        )
        .await
        .unwrap();
        j.conexion = Some(c);
        invitados.push(j);
        // Que lleguen en orden (el asiento es el orden de llegada).
        esperar("el deck en la sala", || anf.para_empezar().len() == i + 1).await;
    }
    esperar("los decks de la sala", || anf.para_empezar().len() == n).await;
    let config = ConfigMesa {
        jugadores: vec![JugadorMesa {
            deck_id: "d".into(),
            nombre: "Host".into(),
            color: "#e11d48".into(),
            lista: None,
            equipo: None,
        }],
        formato: Formato::Constructed,
        variantes: Variantes::default(),
        opciones: None,
        intensidad: Intensidad::Apagado,
        equipos: Vec::new(),
    };
    empezar_partida(&anf, &host.mesa, host.ep.id(), config, mazo()).unwrap();
    for j in &invitados {
        esperar("la bienvenida", || seq(&j.mesa).is_some()).await;
    }
    Partida3 {
        anf,
        host,
        invitados,
        codigo,
    }
}

/// Una jugada al azar del jugador `j` sobre su copia.
fn jugada(m: &MesaEnMemoria, j: IdJugador, azar: &mut Azar) {
    m.con(|s| {
        let Some(s) = s.as_mut() else { return };
        let mano = s.partida().zona(&RefZona::de(j, Zona::Mano)).to_vec();
        let accion = match azar.menor_que(6) {
            0 => Accion::Robar { n: 1 },
            1 => Accion::Moler { n: 1 },
            2 => Accion::Vida {
                jugador: j,
                delta: if azar.moneda() { 1 } else { -1 },
            },
            3 => Accion::PasarTurno,
            4 if !mano.is_empty() => Accion::Mover {
                cartas: vec![mano[azar.menor_que(mano.len() as u64) as usize]],
                a: Destino {
                    zona: ZonaDestino::De(Zona::Campo),
                    jugador: Some(j),
                    sitio: Sitio::EnCampo(Pos::nueva(
                        azar.menor_que(10_001) as u16,
                        azar.menor_que(10_001) as u16,
                    )),
                },
                como: None,
            },
            _ => Accion::Barajar,
        };
        // Lo que el motor rechace (no es su turno…) no importa aquí.
        let _ = s.accion(accion, None);
    });
}

async fn todos_iguales(p: &Partida3) {
    for j in &p.invitados {
        esperar("la misma partida en todos", || {
            pendientes(&j.mesa) == 0 && seq(&j.mesa) == seq(&p.host.mesa)
        })
        .await;
        assert_eq!(huella(&j.mesa), huella(&p.host.mesa));
    }
}

async fn quedarse_todos(p: &Partida3) {
    for m in std::iter::once(&p.host.mesa).chain(p.invitados.iter().map(|j| &j.mesa)) {
        m.con(|s| {
            s.as_mut()
                .unwrap()
                .accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
                .unwrap()
        });
    }
    todos_iguales(p).await;
}

#[tokio::test]
async fn tres_jugadores_y_200_jugadas_acaban_con_la_misma_partida() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let mut azar = Azar::new(7);
    for k in 0..200 {
        let quien = azar.menor_que(3) as usize;
        let m = if quien == 0 {
            &p.host.mesa
        } else {
            &p.invitados[quien - 1].mesa
        };
        jugada(m, IdJugador(quien as u8), &mut azar);
        // De vez en cuando se deja respirar a la red.
        if k % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
    todos_iguales(&p).await;
    let n = seq(&p.host.mesa).unwrap();
    // Las que el motor rechaza (no es su turno…) no cuentan.
    assert!(n > 80, "solo {n} eventos");
    // La sala avisó a la pantalla del invitado al empezar.
    assert!(p.invitados[0]
        .mesa
        .avisos
        .lock()
        .unwrap()
        .contains(&AvisoRed::Empezada));
}

#[tokio::test]
async fn un_invitado_que_se_corta_vuelve_con_la_misma_partida() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    p.anf.cortar(IdJugador(2));
    // Mientras vuelve, los otros juegan.
    let mut azar = Azar::new(11);
    for _ in 0..40 {
        let quien = azar.menor_que(2) as usize;
        let m = if quien == 0 {
            &p.host.mesa
        } else {
            &p.invitados[0].mesa
        };
        jugada(m, IdJugador(quien as u8), &mut azar);
    }
    esperar("que intente volver", || {
        p.invitados[1]
            .mesa
            .avisos
            .lock()
            .unwrap()
            .iter()
            .any(|a| matches!(a, AvisoRed::Reconectando { .. }))
    })
    .await;
    esperar("que vuelva", || {
        p.invitados[1]
            .mesa
            .avisos
            .lock()
            .unwrap()
            .contains(&AvisoRed::Conectada)
    })
    .await;
    // Y sigue jugando.
    jugada(&p.invitados[1].mesa, IdJugador(2), &mut azar);
    todos_iguales(&p).await;
}

#[tokio::test]
async fn sin_el_secreto_o_con_otra_version_no_se_entra() {
    let p = partida(1).await;
    let ep = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
    let malo = Codigo {
        addr: p.codigo.addr.clone(),
        secreto: secreto_nuevo(),
    };
    let e = unirse(
        ep.clone(),
        Union {
            codigo: malo,
            apodo: "Intruso".into(),
            rol: Rol::Jugador,
            deck: Some(mazo()),
        },
        MesaEnMemoria::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(e.params["motivo"], "codigo");
    // Con el código bueno pero la partida ya empezada, como jugador no.
    let e = unirse(
        ep,
        Union {
            codigo: p.codigo.clone(),
            apodo: "Tarde".into(),
            rol: Rol::Jugador,
            deck: Some(mazo()),
        },
        MesaEnMemoria::default(),
    )
    .await
    .unwrap_err();
    assert_eq!(e.params["motivo"], "empezada");
}

#[tokio::test]
async fn si_se_va_el_anfitrion_otro_sigue_la_partida() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let mut azar = Azar::new(3);
    for _ in 0..100 {
        let quien = azar.menor_que(3) as usize;
        let m = if quien == 0 {
            &p.host.mesa
        } else {
            &p.invitados[quien - 1].mesa
        };
        jugada(m, IdJugador(quien as u8), &mut azar);
    }
    todos_iguales(&p).await;
    let antes = seq(&p.invitados[0].mesa).unwrap();
    // Se va el anfitrión.
    p.host.ep.close().await;
    // B (el jugador 1) sigue de anfitrión, época 2, con su copia.
    let b = &p.invitados[0];
    let nodos = b
        .mesa
        .con(|s| match s.as_ref().and_then(|s| s.red.as_ref()) {
            Some(RedSesion::Invitado(i)) => i.nodos.clone(),
            _ => Vec::new(),
        });
    assert_eq!(nodos.len(), 3);
    let anf_b = Anfitrion::desde(2, p.codigo.secreto.clone(), "B".into(), &nodos);
    b.mesa
        .con(|s| s.as_mut().unwrap().red = Some(RedSesion::Anfitrion(anf_b.clone())));
    tokio::spawn(atender(b.ep.clone(), anf_b.clone(), b.mesa.clone()));
    // C se une a B con el mismo secreto (la Fase F le dirá a quién).
    let c = &p.invitados[1];
    unirse(
        c.ep.clone(),
        Union {
            codigo: Codigo {
                addr: b.ep.addr(),
                secreto: p.codigo.secreto.clone(),
            },
            apodo: "C".into(),
            rol: Rol::Jugador,
            deck: None,
        },
        c.mesa.clone(),
    )
    .await
    .unwrap();
    esperar("que C tenga la época 2", || {
        c.mesa
            .con(|s| match s.as_ref().and_then(|s| s.red.as_ref()) {
                Some(RedSesion::Invitado(i)) => i.epoch == 2,
                _ => false,
            })
    })
    .await;
    // Otras 100 jugadas entre B y C.
    for _ in 0..100 {
        let quien = 1 + azar.menor_que(2) as usize;
        let m = &p.invitados[quien - 1].mesa;
        jugada(m, IdJugador(quien as u8), &mut azar);
    }
    esperar("la misma partida en B y C", || {
        pendientes(&c.mesa) == 0 && seq(&c.mesa) == seq(&b.mesa)
    })
    .await;
    assert_eq!(huella(&c.mesa), huella(&b.mesa));
    let despues = seq(&b.mesa).unwrap();
    assert!(despues > antes + 50, "{antes} → {despues}");
}

#[tokio::test]
async fn el_espectador_no_ve_lo_oculto_hasta_que_se_revela() {
    let p = partida(1).await;
    quedarse_todos(&p).await;
    let esp = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        conexion: None,
    };
    unirse(
        esp.ep.clone(),
        Union {
            codigo: p.codigo.clone(),
            apodo: "Mira".into(),
            rol: Rol::Espectador,
            deck: None,
        },
        esp.mesa.clone(),
    )
    .await
    .unwrap();
    let version = |m: &MesaEnMemoria| m.mirada(|v| v.as_ref().map(|v| v.vista.version));
    esperar("la vista del espectador", || version(&esp.mesa).is_some()).await;
    // Nada de una mano ni de una biblioteca lleva su identidad.
    let ocultas = |m: &MesaEnMemoria| {
        m.mirada(|v| {
            v.as_ref().map_or(0, |v| {
                v.vista
                    .cartas
                    .iter()
                    .filter(|c| {
                        matches!(
                            c.zona,
                            RefZona::De {
                                zona: Zona::Mano | Zona::Biblioteca,
                                ..
                            }
                        ) && c.origen.is_some()
                    })
                    .count()
            })
        })
    };
    assert_eq!(ocultas(&esp.mesa), 0);
    // El anfitrión baja una carta de la mano: el espectador la ve entonces.
    let carta = p.host.mesa.con(|s| {
        let s = s.as_mut().unwrap();
        let c = s.partida().zona(&RefZona::de(IdJugador(0), Zona::Mano))[0];
        s.accion(
            Accion::Mover {
                cartas: vec![c],
                a: Destino {
                    zona: ZonaDestino::De(Zona::Campo),
                    jugador: Some(IdJugador(0)),
                    sitio: Sitio::EnCampo(Pos::nueva(5_000, 5_000)),
                },
                como: None,
            },
            None,
        )
        .unwrap();
        c
    });
    let ultima = seq(&p.host.mesa);
    esperar("el parche del espectador", || version(&esp.mesa) == ultima).await;
    let vista = esp.mesa.mirada(|v| {
        v.as_ref()
            .and_then(|v| v.vista.cartas.iter().find(|c| c.id == carta).cloned())
    });
    assert!(vista.and_then(|c| c.origen).is_some());
    assert_eq!(ocultas(&esp.mesa), 0);
    // Y no puede jugar.
    let r = esp.mesa.con(|s| s.is_none());
    assert!(r, "el espectador no tiene partida");
}

#[tokio::test]
async fn las_imagenes_propias_llegan_por_el_anfitrion_y_con_su_huella() {
    use sha2::{Digest, Sha256};
    let huella = |b: &[u8]| -> String {
        Sha256::digest(b)
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect()
    };
    let img = image::RgbaImage::from_pixel(100, 140, image::Rgba([200, 30, 30, 255]));
    let jpg = proxy_core::imaging::encode::jpeg(&img, 85).unwrap();
    let buena = huella(&jpg);
    // Una que dice ser otra cosa: su fichero no tiene esa huella.
    let falsa = huella(b"otra cosa");
    let mut deck = mazo();
    deck.cartas[0].origen.caras[0] = RefImagen::Propia {
        sha256: buena.clone(),
        ancho: 100,
        alto: 140,
    };
    deck.cartas[1].origen.caras[0] = RefImagen::Propia {
        sha256: falsa.clone(),
        ancho: 100,
        alto: 140,
    };
    let p = partida_con(vec![deck, mazo()]).await;
    let b = &p.invitados[0];
    let c = &p.invitados[1];
    let escribir = |ruta: std::path::PathBuf, bytes: &[u8]| {
        std::fs::create_dir_all(ruta.parent().unwrap()).unwrap();
        std::fs::write(ruta, bytes).unwrap();
    };
    escribir(propia_en(&b.mesa.carpeta, &buena), &jpg);
    escribir(propia_en(&b.mesa.carpeta, &falsa), &jpg);
    // C se la pide al anfitrión, que se la pide a B.
    let conn = c.conexion.as_ref().unwrap().conexion().unwrap();
    traer_propia(&c.mesa, Por::Invitado(conn.clone()), &buena)
        .await
        .unwrap();
    assert!(la_tengo(&c.mesa.carpeta, &buena));
    assert_eq!(
        std::fs::read(ruta_cruda(&c.mesa.carpeta, &buena)).unwrap(),
        jpg
    );
    // El anfitrión se quedó con una copia (para el siguiente que la pida).
    assert!(la_tengo(&p.host.mesa.carpeta, &buena));
    // La falsa no llega a nadie.
    assert!(traer_propia(&c.mesa, Por::Invitado(conn), &falsa)
        .await
        .is_err());
    assert!(!la_tengo(&c.mesa.carpeta, &falsa));
    // Y lo que no es de la partida no se sirve.
    assert!(
        traer_propia(&c.mesa, Por::Anfitrion(p.anf.clone()), &huella(b"nada"))
            .await
            .is_err()
    );
    for m in [&p.host.mesa, &b.mesa, &c.mesa] {
        let _ = std::fs::remove_dir_all(&m.carpeta);
    }
}

fn avisos(m: &MesaEnMemoria) -> Vec<AvisoRed> {
    m.avisos.lock().map(|v| v.clone()).unwrap_or_default()
}

/// La última tabla de enlaces que apuntó una mesa, con su desfase.
fn ultimos_enlaces(m: &MesaEnMemoria) -> Option<(i32, Vec<Enlace>)> {
    avisos(m).into_iter().rev().find_map(|a| match a {
        AvisoRed::Enlaces { desfase_ms, filas } => Some((desfase_ms, filas)),
        _ => None,
    })
}

#[tokio::test]
async fn cada_jugador_sabe_su_latencia_y_si_va_directo() {
    let p = partida(2).await;
    for j in &p.invitados {
        esperar("la tabla de enlaces con los tres", || {
            ultimos_enlaces(&j.mesa).is_some_and(|(_, f)| f.len() == 3)
        })
        .await;
        let (desfase, filas) = ultimos_enlaces(&j.mesa).unwrap();
        // Mismo reloj, misma máquina: el desfase es casi nada.
        assert!(desfase.abs() < 200, "desfase {desfase}");
        for f in &filas {
            assert!(f.directo, "{f:?} debería ir directo");
            assert!(f.rtt_ms < 50, "{f:?}");
        }
    }
    esperar("la tabla del anfitrión", || {
        ultimos_enlaces(&p.host.mesa).is_some_and(|(d, f)| d == 0 && f.len() == 3)
    })
    .await;
}

#[tokio::test]
async fn el_reloj_del_anfitrion_se_calcula_aunque_el_de_aqui_vaya_atrasado() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
    let host = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
    let guest = crear(SecretKey::generate(), Relay::Ninguno).await.unwrap();
    // Un anfitrión de mentira: su reloj va 5 s por delante.
    let ep = host.clone();
    tokio::spawn(async move {
        while let Some(i) = ep.accept().await {
            let Ok(conn) = i.await else { continue };
            tokio::spawn(async move {
                while let Ok(b) = conn.read_datagram().await {
                    if let Some(Datagrama::Latido { t }) = datagrama_de_bytes(&b) {
                        let t_host = latencia::ahora_ms() + 5_000;
                        latencia::enviar(&conn, &Datagrama::Eco { t, t_host });
                    }
                }
            });
        }
    });
    let conn = guest.connect(host.addr(), ALPN_MESA).await.unwrap();
    let mesa = MesaEnMemoria::default();
    let tarea = tokio::spawn(latencia::atender_invitado(
        conn,
        mesa.clone(),
        Arc::new(latencia::ahora_ms),
        Duration::from_millis(50),
    ));
    esperar("diez ecos", || {
        avisos(&mesa)
            .iter()
            .filter(|a| matches!(a, AvisoRed::Enlaces { .. }))
            .count()
            >= 10
    })
    .await;
    let (desfase, filas) = ultimos_enlaces(&mesa).unwrap();
    assert!((desfase - 5_000).abs() <= 200, "desfase {desfase}");
    let mio = filas.iter().find(|f| f.jugador == IdJugador(0)).unwrap();
    assert!(mio.directo && mio.rtt_ms < 50, "{mio:?}");
    tarea.abort();
}

fn fantasmas(m: &MesaEnMemoria) -> Vec<(IdJugador, IdJugador, Option<mesa::IdCarta>, u16, u16)> {
    avisos(m)
        .into_iter()
        .filter_map(|a| match a {
            AvisoRed::Fantasma {
                jugador,
                tablero,
                carta,
                x,
                y,
            } => Some((jugador, tablero, carta, x, y)),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn el_fantasma_de_un_invitado_llega_al_anfitrion_y_a_los_demas_con_su_nombre() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let conn = p.invitados[0]
        .conexion
        .as_ref()
        .unwrap()
        .conexion()
        .unwrap();
    // Dice ser el jugador 7 y se sale del tablero: el anfitrión lo corrige.
    latencia::enviar(
        &conn,
        &Datagrama::Fantasma {
            jugador: IdJugador(7),
            tablero: IdJugador(1),
            carta: None,
            x: 60_000,
            y: 2_000,
            t: 1,
        },
    );
    let esperado = (IdJugador(1), IdJugador(1), None, 10_000, 2_000);
    esperar("el fantasma en el anfitrión", || {
        fantasmas(&p.host.mesa).contains(&esperado)
    })
    .await;
    esperar("el fantasma en el otro invitado", || {
        fantasmas(&p.invitados[1].mesa).contains(&esperado)
    })
    .await;
    // A quien lo mueve no se le devuelve.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(fantasmas(&p.invitados[0].mesa).is_empty());
    // Uno repetido o más viejo se tira.
    latencia::enviar(
        &conn,
        &Datagrama::Fantasma {
            jugador: IdJugador(1),
            tablero: IdJugador(1),
            carta: None,
            x: 1,
            y: 1,
            t: 1,
        },
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(fantasmas(&p.host.mesa).len(), 1);
}

#[tokio::test]
async fn al_espectador_solo_le_llega_el_movimiento_de_lo_que_ve() {
    let p = partida(1).await;
    quedarse_todos(&p).await;
    let esp = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        conexion: None,
    };
    unirse(
        esp.ep.clone(),
        Union {
            codigo: p.codigo.clone(),
            apodo: "Mira".into(),
            rol: Rol::Espectador,
            deck: None,
        },
        esp.mesa.clone(),
    )
    .await
    .unwrap();
    esperar("la vista del espectador", || {
        esp.mesa.mirada(|v| v.is_some())
    })
    .await;
    // Una carta de la mano del invitado (oculta) y otra del campo del anfitrión.
    let oculta = p.invitados[0].mesa.con(|s| {
        s.as_ref()
            .unwrap()
            .partida()
            .zona(&RefZona::de(IdJugador(1), Zona::Mano))[0]
    });
    let visible = p.host.mesa.con(|s| {
        let s = s.as_mut().unwrap();
        let c = s.partida().zona(&RefZona::de(IdJugador(0), Zona::Mano))[0];
        s.accion(
            Accion::Mover {
                cartas: vec![c],
                a: Destino {
                    zona: ZonaDestino::De(Zona::Campo),
                    jugador: Some(IdJugador(0)),
                    sitio: Sitio::EnCampo(Pos::nueva(5_000, 5_000)),
                },
                como: None,
            },
            None,
        )
        .unwrap();
        c
    });
    todos_iguales(&p).await;
    let conn = p.invitados[0]
        .conexion
        .as_ref()
        .unwrap()
        .conexion()
        .unwrap();
    let mover = |carta, t| {
        latencia::enviar(
            &conn,
            &Datagrama::Fantasma {
                jugador: IdJugador(1),
                tablero: IdJugador(1),
                carta: Some(carta),
                x: 100,
                y: 100,
                t,
            },
        )
    };
    mover(oculta, 1);
    mover(visible, 2);
    esperar("el fantasma de la carta visible", || {
        fantasmas(&esp.mesa).iter().any(|f| f.2 == Some(visible))
    })
    .await;
    assert!(
        !fantasmas(&esp.mesa).iter().any(|f| f.2 == Some(oculta)),
        "al espectador no se le cuenta lo que no ve"
    );
    // Los jugadores sí ven el movimiento de la oculta (desde su lado, un dorso).
    esperar("la oculta en el anfitrión", || {
        fantasmas(&p.host.mesa).iter().any(|f| f.2 == Some(oculta))
    })
    .await;
}
