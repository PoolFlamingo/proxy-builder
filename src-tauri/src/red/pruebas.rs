//! Pruebas de la partida en red en el mismo proceso: nodos de iroh sin relé
//! (direcciones locales), un anfitrión y sus invitados, cada uno con su
//! sesión en memoria (`MesaEnMemoria`).

use std::sync::Arc;
use std::time::Duration;

use iroh::{Endpoint, SecretKey};
use mesa::accion::{Accion, Destino, Sitio, ZonaDestino};
use mesa::modelo::{ComoGano, Equipo, Pos, RefZona, Resultado, Zona};
use mesa::{Azar, IdEquipo, IdJugador};

use super::anfitrion::{atender, empezar_partida, vigilar_ausentes, Anfitrion};
use super::codigo::{secreto_nuevo, Codigo};
use super::imagenes::{la_tengo, ruta_cruda, traer_propia, Por};
use super::invitado::{unirse, Conexion, Union};
use super::latencia;
use super::mesa::{AvisoRed, Mesa, MesaEnMemoria};
use super::nodo::{crear, Relay, ALPN_MESA};
use super::protocolo::{datagrama_de_bytes, AlHost, Datagrama, Enlace, Rol};
use crate::mesa::encuentro::VistaBanquillo;
use crate::mesa::imagenes::propia_en;
use crate::mesa::sesion::tests::mazo;
use crate::mesa::sesion::{ConfigMesa, JugadorMesa, RedSesion};
use mesa::formato::{Formato, ModoEquipos, Variantes};
use mesa::modelo::RefImagen;
use mesa::preparar::{DeckDeMesa, Seccion};
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
    /// La tarea que atiende a los que llegan al anfitrión.
    aceptador: tokio::task::JoinHandle<()>,
}

pub(super) async fn esperar(que: &str, cond: impl Fn() -> bool) {
    for _ in 0..500 {
        if cond() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("no llegó: {que}");
}

pub(super) fn seq(m: &MesaEnMemoria) -> Option<u64> {
    m.con(|s| s.as_ref().map(|s| s.partida().seq))
}

pub(super) fn huella(m: &MesaEnMemoria) -> Option<String> {
    m.con(|s| s.as_ref().map(|s| mesa::huella(s.partida())))
}

pub(super) fn pendientes(m: &MesaEnMemoria) -> usize {
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
    partida_ajustada(decks, mazo(), &[], |_| {}).await
}

/// Lo mismo, con el deck del anfitrión, el orden de los asientos de los
/// invitados (índices por orden de llegada; vacío, el de llegada) y lo que
/// el anfitrión cambie de la configuración (equipos, al mejor de…). Los
/// invitados de `config.jugadores` tras el anfitrión son lo que eligió de
/// cada uno, por asiento.
async fn partida_ajustada(
    decks: Vec<DeckDeMesa>,
    deck_host: DeckDeMesa,
    orden: &[usize],
    ajustar: impl FnOnce(&mut ConfigMesa),
) -> Partida3 {
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
    let aceptador = tokio::spawn(atender(host.ep.clone(), anf.clone(), host.mesa.clone()));
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
    let mut config = ConfigMesa {
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
        al_mejor_de: None,
        primero: None,
    };
    ajustar(&mut config);
    // Las conexiones de la sala son 1, 2, 3… por orden de llegada.
    let llegada: Vec<u64> = anf.para_empezar().into_iter().map(|(c, ..)| c).collect();
    let por_asiento: Vec<u64> = orden.iter().map(|i| llegada[*i]).collect();
    empezar_partida(
        &anf,
        &host.mesa,
        host.ep.id(),
        config,
        deck_host,
        &por_asiento,
    )
    .unwrap();
    for j in &invitados {
        esperar("la bienvenida", || seq(&j.mesa).is_some()).await;
    }
    Partida3 {
        anf,
        host,
        invitados,
        codigo,
        aceptador,
    }
}

/// Una jugada al azar del jugador `j` sobre su copia.
pub(super) fn jugada(m: &MesaEnMemoria, j: IdJugador, azar: &mut Azar) {
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

pub(super) fn es_anfitrion(m: &MesaEnMemoria) -> bool {
    m.con(|s| {
        matches!(
            s.as_ref().and_then(|s| s.red.as_ref()),
            Some(RedSesion::Anfitrion(_))
        )
    })
}

pub(super) fn epoch_invitado(m: &MesaEnMemoria) -> Option<u32> {
    m.con(|s| match s.as_ref().and_then(|s| s.red.as_ref()) {
        Some(RedSesion::Invitado(i)) => Some(i.epoch),
        _ => None,
    })
}

pub(super) fn activo(m: &MesaEnMemoria) -> bool {
    m.con(|s| match s.as_ref().and_then(|s| s.red.as_ref()) {
        Some(RedSesion::Anfitrion(a)) => a.activo(),
        _ => false,
    })
}

fn hace(m: &MesaEnMemoria, accion: Accion) -> Result<(), String> {
    m.con(|s| s.as_mut().unwrap().accion(accion, None))
        .map_err(|e| e.params.get("motivo").cloned().unwrap_or_default())
}

#[tokio::test]
async fn si_se_va_el_anfitrion_otro_sigue_la_partida_sin_servidor() {
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
    // Todos saben quién es quién, con su dirección.
    let nodos = p.invitados[1]
        .mesa
        .con(|s| match s.as_ref().and_then(|s| s.red.as_ref()) {
            Some(RedSesion::Invitado(i)) => i.nodos.clone(),
            _ => Vec::new(),
        });
    assert_eq!(nodos.len(), 3);
    assert!(nodos.iter().all(|n| n.direccion.is_some()), "{nodos:?}");
    // Se va el anfitrión (se cierra su app).
    p.host.ep.close().await;
    // B (el jugador 1) sigue de anfitrión solo, con su copia; C lo encuentra.
    let b = &p.invitados[0];
    let c = &p.invitados[1];
    esperar("que B sea el anfitrión", || es_anfitrion(&b.mesa)).await;
    esperar("que C se una a B", || epoch_invitado(&c.mesa) == Some(2)).await;
    // Con 2 de 3, B tiene la mayoría y deja jugar.
    esperar("que B juegue", || activo(&b.mesa)).await;
    assert!(avisos(&c.mesa).contains(&AvisoRed::NuevoAnfitrion {
        jugador: IdJugador(1),
        soy_yo: false
    }));
    assert!(avisos(&b.mesa).contains(&AvisoRed::NuevoAnfitrion {
        jugador: IdJugador(1),
        soy_yo: true
    }));
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
async fn con_dos_jugadores_el_que_queda_espera_sin_jugar() {
    let p = partida(1).await;
    quedarse_todos(&p).await;
    p.host.ep.close().await;
    let b = &p.invitados[0];
    esperar("que B herede la partida", || es_anfitrion(&b.mesa)).await;
    // 1 de 2 no es la mayoría: no se juega.
    tokio::time::sleep(Duration::from_millis(400)).await;
    assert!(!activo(&b.mesa));
    assert_eq!(
        hace(&b.mesa, Accion::Robar { n: 1 }),
        Err("sin_mayoria".into())
    );
    assert!(avisos(&b.mesa).contains(&AvisoRed::Mayoria {
        activa: false,
        conectados: 1,
        total: 2
    }));
}

#[tokio::test]
async fn el_espectador_sigue_mirando_con_el_anfitrion_nuevo() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let esp = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        conexion: None,
    };
    let _c = unirse(
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
    let epoch = |m: &MesaEnMemoria| m.mirada(|v| v.as_ref().map(|v| (v.epoch, v.nodos.len())));
    esperar("la vista con los nodos", || {
        epoch(&esp.mesa) == Some((1, 3))
    })
    .await;
    p.host.ep.close().await;
    let b = &p.invitados[0];
    esperar("que B juegue", || activo(&b.mesa)).await;
    esperar("que el espectador mire a B", || {
        epoch(&esp.mesa).map(|e| e.0) == Some(2)
    })
    .await;
    // Lo que se juega con B le llega.
    let antes = esp.mesa.mirada(|v| v.as_ref().map(|v| v.vista.version));
    hace(&b.mesa, Accion::Robar { n: 1 }).unwrap();
    esperar("el parche de B", || {
        esp.mesa.mirada(|v| v.as_ref().map(|v| v.vista.version)) > antes
    })
    .await;
}

#[tokio::test]
async fn el_anfitrion_que_se_queda_solo_se_para_y_luego_se_une_al_nuevo() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    // El anfitrión pierde la red: ya no atiende a nadie y se le cortan los
    // dos, pero su app sigue abierta.
    p.aceptador.abort();
    p.anf.cortar(IdJugador(1));
    p.anf.cortar(IdJugador(2));
    let (b, c) = (&p.invitados[0], &p.invitados[1]);
    // Con 1 de 3, se para: no puede seguir mientras los otros dos eligen.
    esperar("que el anfitrión se pare", || !p.anf.activo()).await;
    assert_eq!(
        hace(&p.host.mesa, Accion::Robar { n: 1 }),
        Err("sin_mayoria".into())
    );
    esperar("que B sea el anfitrión", || es_anfitrion(&b.mesa)).await;
    esperar("que C se una a B", || epoch_invitado(&c.mesa) == Some(2)).await;
    esperar("que B juegue", || activo(&b.mesa)).await;
    // El de antes se une a B como invitado.
    esperar("que el anfitrión de antes se una a B", || {
        epoch_invitado(&p.host.mesa) == Some(2)
    })
    .await;
    assert!(p.anf.acabado());
    hace(&b.mesa, Accion::Robar { n: 1 }).unwrap();
    hace(&p.host.mesa, Accion::Robar { n: 1 }).unwrap();
    for m in [&c.mesa, &p.host.mesa] {
        esperar("todos con la partida de B", || {
            pendientes(m) == 0 && seq(m) == seq(&b.mesa)
        })
        .await;
        assert_eq!(huella(m), huella(&b.mesa));
    }
}

#[tokio::test]
async fn si_el_anfitrion_se_va_a_proposito_la_partida_sigue_con_el_primero_que_queda() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let (b, c) = (&p.invitados[0], &p.invitados[1]);
    // Se va por su pie (salir de la mesa): cede, no cierra.
    assert!(p.anf.irse().await);
    esperar("que B sea el anfitrión", || es_anfitrion(&b.mesa)).await;
    esperar("que C se una a B", || epoch_invitado(&c.mesa) == Some(2)).await;
    // Con 2 de 2 (el que se fue no cuenta), B deja jugar.
    esperar("que B juegue", || activo(&b.mesa)).await;
    assert!(avisos(&c.mesa).contains(&AvisoRed::NuevoAnfitrion {
        jugador: IdJugador(1),
        soy_yo: false
    }));
    // Nadie ha visto la partida cerrada.
    assert!(!avisos(&b.mesa).contains(&AvisoRed::Cerrada));
    assert!(!avisos(&c.mesa).contains(&AvisoRed::Cerrada));
    let mut azar = Azar::new(11);
    for _ in 0..60 {
        let quien = 1 + azar.menor_que(2) as usize;
        jugada(
            &p.invitados[quien - 1].mesa,
            IdJugador(quien as u8),
            &mut azar,
        );
    }
    esperar("la misma partida en B y C", || {
        pendientes(&c.mesa) == 0 && seq(&c.mesa) == seq(&b.mesa)
    })
    .await;
    assert_eq!(huella(&c.mesa), huella(&b.mesa));
}

#[tokio::test]
async fn con_dos_jugadores_el_que_queda_sigue_sin_esperar_a_nadie_si_el_anfitrion_se_va_a_proposito(
) {
    let p = partida(1).await;
    quedarse_todos(&p).await;
    let b = &p.invitados[0];
    assert!(p.anf.irse().await);
    esperar("que B herede la partida", || es_anfitrion(&b.mesa)).await;
    // El que se fue a propósito no cuenta: 1 de 1 es mayoría (si se hubiera
    // caído, B esperaría a que volviera).
    esperar("que B juegue", || activo(&b.mesa)).await;
    hace(&b.mesa, Accion::Robar { n: 1 }).unwrap();
}

#[tokio::test]
async fn el_anfitrion_cede_a_quien_dice_aunque_haya_otro_de_asiento_mas_bajo_caido() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let (b, c) = (&p.invitados[0], &p.invitados[1]);
    // B (asiento 1) se cae; el anfitrión se va y deja la partida a C, el único
    // conectado, sin que C pierda tiempo probando con B.
    b.ep.close().await;
    esperar("que se note que falta B", || {
        p.anf.ausentes().iter().any(|a| a.jugador == IdJugador(1))
    })
    .await;
    assert!(p.anf.irse().await);
    esperar("que C sea el anfitrión", || es_anfitrion(&c.mesa)).await;
}

#[tokio::test]
async fn si_no_queda_nadie_que_siga_el_anfitrion_cierra_la_partida() {
    let p = partida(1).await;
    quedarse_todos(&p).await;
    let b = &p.invitados[0];
    p.anf.cortar(IdJugador(1));
    esperar("que se note que falta", || {
        p.anf.ausentes().iter().any(|a| a.jugador == IdJugador(1))
    })
    .await;
    assert!(!p.anf.irse().await);
    assert!(p.anf.acabado());
    // B no llega a enterarse (ya no estaba): sigue como estaba.
    assert!(!es_anfitrion(&b.mesa));
}

#[tokio::test]
async fn el_espectador_sigue_mirando_cuando_el_anfitrion_se_va_a_proposito() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let esp = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        conexion: None,
    };
    let _c = unirse(
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
    let epoch = |m: &MesaEnMemoria| m.mirada(|v| v.as_ref().map(|v| (v.epoch, v.nodos.len())));
    esperar("la vista con los nodos", || {
        epoch(&esp.mesa) == Some((1, 3))
    })
    .await;
    assert!(p.anf.irse().await);
    let b = &p.invitados[0];
    esperar("que B juegue", || activo(&b.mesa)).await;
    esperar("que el espectador mire a B", || {
        epoch(&esp.mesa).map(|e| e.0) == Some(2)
    })
    .await;
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

pub(super) fn avisos(m: &MesaEnMemoria) -> Vec<AvisoRed> {
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
            // En la misma máquina; con la máquina cargada (el compilador, el
            // escritorio) un latido tarda más de 50 ms en un runtime de un
            // solo hilo: lo que se comprueba es que se mide y es del orden local.
            assert!(f.rtt_ms < 500, "{f:?}");
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
                ..
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
            mas: vec![],
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
            mas: vec![],
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
                mas: vec![],
                x: 100,
                y: 100,
                t,
            },
        )
    };
    mover(oculta, 1);
    mover(visible, 2);
    // Varias a la vez: si una no la ve, no le llega ninguna del grupo.
    latencia::enviar(
        &conn,
        &Datagrama::Fantasma {
            jugador: IdJugador(1),
            tablero: IdJugador(1),
            carta: Some(visible),
            mas: vec![oculta],
            x: 200,
            y: 200,
            t: 3,
        },
    );
    esperar("el grupo en el anfitrión", || {
        fantasmas(&p.host.mesa).iter().any(|f| f.3 == 200)
    })
    .await;
    esperar("el fantasma de la carta visible", || {
        fantasmas(&esp.mesa).iter().any(|f| f.2 == Some(visible))
    })
    .await;
    assert!(
        !fantasmas(&esp.mesa).iter().any(|f| f.2 == Some(oculta)),
        "al espectador no se le cuenta lo que no ve"
    );
    assert!(
        !fantasmas(&esp.mesa).iter().any(|f| f.3 == 200),
        "ni un grupo del que no ve una carta"
    );
    // Los jugadores sí ven el movimiento de la oculta (desde su lado, un dorso).
    esperar("la oculta en el anfitrión", || {
        fantasmas(&p.host.mesa).iter().any(|f| f.2 == Some(oculta))
    })
    .await;
}

#[tokio::test]
async fn quien_se_va_aparece_como_ausente_para_todos_y_deja_de_estarlo_al_volver() {
    let mut p = partida(2).await;
    quedarse_todos(&p).await;
    // El jugador 2 se va (cierra su conexión).
    p.invitados[1].conexion.take().unwrap().salir().await;
    esperar("que el anfitrión lo apunte", || {
        p.anf.ausentes().iter().any(|a| a.jugador == IdJugador(2))
    })
    .await;
    let lo_ve = |m: &MesaEnMemoria| {
        m.avisos.lock().unwrap().iter().any(|a| match a {
            AvisoRed::Ausentes { filas } => filas.iter().any(|f| f.jugador == IdJugador(2)),
            _ => false,
        })
    };
    // La pantalla del anfitrión (su vigía) y la del otro jugador (el latido).
    esperar("que lo vea el anfitrión", || lo_ve(&p.host.mesa)).await;
    esperar("que lo vea el otro jugador", || lo_ve(&p.invitados[0].mesa)).await;
    // Vuelve (por su nodo, con el mismo secreto): ya no falta.
    let j = &p.invitados[1];
    unirse(
        j.ep.clone(),
        Union {
            codigo: p.codigo.clone(),
            apodo: "Invitado 1".into(),
            rol: Rol::Jugador,
            deck: None,
        },
        j.mesa.clone(),
    )
    .await
    .unwrap();
    esperar("que ya no falte", || p.anf.ausentes().is_empty()).await;
}

#[tokio::test]
async fn si_el_ausente_no_decide_su_mano_el_anfitrion_se_queda_por_el() {
    let mut p = partida(2).await;
    // Un vigía con prisa (el de verdad espera un minuto).
    vigilar_ausentes(&p.anf, p.host.mesa.clone(), Duration::from_millis(1_200));
    p.invitados[1].conexion.take().unwrap().salir().await;
    // Los otros dos deciden; falta el que se fue.
    for m in [&p.host.mesa, &p.invitados[0].mesa] {
        m.con(|s| {
            s.as_mut()
                .unwrap()
                .accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
                .unwrap()
        });
    }
    esperar("que empiece la partida sin él", || {
        p.host
            .mesa
            .con(|s| s.as_ref().is_some_and(|s| s.partida().mulligan.is_none()))
    })
    .await;
    esperar("que el otro jugador también la vea empezada", || {
        p.invitados[0]
            .mesa
            .con(|s| s.as_ref().is_some_and(|s| s.partida().mulligan.is_none()))
    })
    .await;
}

#[tokio::test]
async fn el_expulsado_queda_eliminado_se_le_corta_y_no_vuelve_a_entrar() {
    let p = partida(2).await;
    quedarse_todos(&p).await;
    let nodo = p.invitados[1].ep.id();
    let jugador = p.anf.expulsar(nodo);
    assert_eq!(jugador, Some(IdJugador(2)));
    p.host.mesa.con(|s| {
        let s = s.as_mut().unwrap();
        s.suplir(
            IdJugador(0),
            Accion::Eliminar {
                jugador: IdJugador(2),
                por: None,
            },
        )
        .unwrap()
    });
    // Los que quedan ven que está fuera.
    esperar("que lo vea el otro jugador", || {
        p.invitados[0].mesa.con(|s| {
            s.as_ref().is_some_and(|s| {
                s.partida()
                    .jugador(IdJugador(2))
                    .is_some_and(|j| j.eliminado.is_some())
            })
        })
    })
    .await;
    // No cuenta como ausente (ya está eliminado) y no puede volver.
    assert!(p.anf.ausentes().is_empty());
    let j = &p.invitados[1];
    let r = unirse(
        j.ep.clone(),
        Union {
            codigo: p.codigo.clone(),
            apodo: "Invitado 1".into(),
            rol: Rol::Jugador,
            deck: None,
        },
        MesaEnMemoria::default(),
    )
    .await;
    assert!(r.is_err(), "el expulsado no debería poder volver");
    // Solo el anfitrión puede suplir: un invitado, no.
    let e = p.invitados[0].mesa.con(|s| {
        s.as_mut().unwrap().suplir(
            IdJugador(1),
            Accion::Eliminar {
                jugador: IdJugador(0),
                por: None,
            },
        )
    });
    assert!(e.is_err());
}

/// Un mazo de 60 con tres cartas de banquillo (`banq0`…).
fn mazo_con_banquillo() -> DeckDeMesa {
    let mut m = mazo();
    for i in 0..3 {
        let mut extra = m.cartas[i].clone();
        extra.carta_deck = format!("banq{i}");
        extra.origen.carta_deck = format!("banq{i}");
        extra.origen.nombre = format!("Del banquillo {i}");
        extra.seccion = Seccion::Banquillo;
        m.cartas.push(extra);
    }
    m
}

/// Lo que un invitado le pide al anfitrión del banquillo y lo que contesta.
async fn banquillo_del_invitado(
    m: &MesaEnMemoria,
    pregunta: AlHost,
) -> Result<VistaBanquillo, String> {
    let rx = m
        .con(|s| match s.as_mut().map(|s| &mut s.red) {
            Some(Some(RedSesion::Invitado(i))) => i.pedir_banquillo(pregunta).ok(),
            _ => None,
        })
        .expect("es un invitado");
    tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("el anfitrión contesta")
        .expect("el canal sigue abierto")
}

fn gana_el_anfitrion() -> Accion {
    Accion::Terminar {
        resultado: Resultado {
            ganadores: vec![IdJugador(0)],
            como: ComoGano::Combate,
            nota: String::new(),
        },
    }
}

fn nombres(m: &MesaEnMemoria) -> Vec<String> {
    m.con(|s| {
        s.as_ref()
            .unwrap()
            .partida()
            .jugadores
            .iter()
            .map(|j| j.nombre.clone())
            .collect()
    })
}

#[tokio::test]
async fn el_anfitrion_elige_el_orden_de_los_asientos_los_equipos_y_quien_empieza() {
    let nuevo_jugador = |color: &str, equipo| JugadorMesa {
        deck_id: String::new(),
        nombre: String::new(),
        color: color.into(),
        lista: None,
        equipo,
    };
    // Llegan Invitado 0, 1 y 2; se sientan 2, 0 y 1, en dos equipos.
    let p = partida_ajustada(vec![mazo(), mazo(), mazo()], mazo(), &[2, 0, 1], |c| {
        c.variantes.equipos = ModoEquipos::Equipos { fuego_amigo: false };
        c.equipos = vec![
            Equipo {
                id: IdEquipo(0),
                nombre: "A".into(),
                color: "#111111".into(),
                vida_compartida: None,
            },
            Equipo {
                id: IdEquipo(1),
                nombre: "B".into(),
                color: "#222222".into(),
                vida_compartida: None,
            },
        ];
        c.jugadores[0].equipo = Some(IdEquipo(0));
        c.jugadores.extend([
            nuevo_jugador("#aa0000", Some(IdEquipo(0))),
            nuevo_jugador("#00bb00", Some(IdEquipo(1))),
            nuevo_jugador("#0000cc", Some(IdEquipo(1))),
        ]);
        c.primero = Some(IdJugador(2));
    })
    .await;
    let esperado = ["Host", "Invitado 2", "Invitado 0", "Invitado 1"];
    for m in std::iter::once(&p.host.mesa).chain(p.invitados.iter().map(|j| &j.mesa)) {
        assert_eq!(nombres(m), esperado);
    }
    // Cada uno sabe en qué asiento está.
    let asiento = |m: &MesaEnMemoria| m.con(|s| s.as_ref().unwrap().yo.0);
    assert_eq!(asiento(&p.invitados[0].mesa), 2);
    assert_eq!(asiento(&p.invitados[1].mesa), 3);
    assert_eq!(asiento(&p.invitados[2].mesa), 1);
    p.host.mesa.con(|s| {
        let partida = s.as_ref().unwrap().partida();
        let equipos: Vec<_> = partida.jugadores.iter().map(|j| j.equipo).collect();
        assert_eq!(
            equipos,
            vec![
                Some(IdEquipo(0)),
                Some(IdEquipo(0)),
                Some(IdEquipo(1)),
                Some(IdEquipo(1))
            ]
        );
        let colores: Vec<_> = partida.jugadores.iter().map(|j| j.color.as_str()).collect();
        assert_eq!(&colores[1..], ["#aa0000", "#00bb00", "#0000cc"]);
        assert_eq!(partida.turno.activo, IdJugador(2));
    });
    // Los invitados reciben lo mismo.
    todos_iguales(&p).await;
}

#[tokio::test]
async fn si_la_sala_cambia_entre_medias_no_empieza_nada() {
    let host = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        conexion: None,
    };
    let anf = Anfitrion::nuevo("Host".into(), secreto_nuevo());
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
        al_mejor_de: None,
        primero: None,
    };
    // Se eligió un asiento para alguien que ya no está.
    let e = empezar_partida(&anf, &host.mesa, host.ep.id(), config, mazo(), &[7]).unwrap_err();
    assert_eq!(e.params["motivo"], "sala_cambio");
    assert!(!anf.empezada());
    assert!(host.mesa.con(|s| s.is_none()));
}

#[tokio::test]
async fn el_invitado_cambia_su_banquillo_y_el_anfitrion_abre_la_siguiente_partida() {
    let p = partida_ajustada(vec![mazo_con_banquillo()], mazo_con_banquillo(), &[], |c| {
        c.al_mejor_de = Some(3)
    })
    .await;
    quedarse_todos(&p).await;
    let b = &p.invitados[0];
    // El invitado también lleva el marcador del encuentro.
    esperar("el encuentro en el invitado", || {
        b.mesa
            .con(|s| s.as_ref().is_some_and(|s| s.encuentro.is_some()))
    })
    .await;
    // Sin terminar la partida no se toca.
    assert_eq!(
        banquillo_del_invitado(&b.mesa, AlHost::PideBanquillo).await,
        Err("sin_terminar".into())
    );
    hace(&p.host.mesa, gana_el_anfitrion()).unwrap();
    esperar("el fin en el invitado", || {
        b.mesa.con(|s| s.as_ref().unwrap().partida().fin.is_some())
    })
    .await;
    let v = banquillo_del_invitado(&b.mesa, AlHost::PideBanquillo)
        .await
        .unwrap();
    assert_eq!((v.mazo.len(), v.banquillo.len()), (60, 3));
    // Un cambio que no encaja no toca nada.
    let mal = banquillo_del_invitado(
        &b.mesa,
        AlHost::CambiaBanquillo {
            sale: vec![v.mazo[0].id.clone()],
            entra: vec![v.mazo[1].id.clone()],
        },
    )
    .await;
    assert_eq!(mal, Err("cambio".into()));
    // Uno bueno: lo que devuelve es su mazo ya cambiado.
    let (sale, entra) = (v.mazo[0].id.clone(), v.banquillo[0].id.clone());
    let nuevo = banquillo_del_invitado(
        &b.mesa,
        AlHost::CambiaBanquillo {
            sale: vec![sale.clone()],
            entra: vec![entra.clone()],
        },
    )
    .await
    .unwrap();
    assert!(nuevo.mazo.iter().any(|c| c.id == entra));
    assert!(!nuevo.mazo.iter().any(|c| c.id == sale));
    // Cada uno toca solo el suyo: el anfitrión no el del invitado, ni este el
    // suyo por la sesión (lo hace por el anfitrión).
    let ajeno = p
        .host
        .mesa
        .con(|s| s.as_ref().unwrap().banquillo(IdJugador(1)))
        .unwrap_err();
    assert_eq!(ajeno.params["motivo"], "ajeno");
    let directo = b
        .mesa
        .con(|s| s.as_ref().unwrap().banquillo(IdJugador(1)))
        .unwrap_err();
    assert_eq!(directo.params["motivo"], "invitado");
    // El anfitrión abre la siguiente partida con el mazo cambiado.
    p.host
        .mesa
        .con(|s| s.as_mut().unwrap().siguiente_partida())
        .unwrap();
    esperar("la partida nueva en el invitado", || {
        b.mesa.con(|s| s.as_ref().unwrap().partida().fin.is_none())
    })
    .await;
    esperar("el marcador al día", || {
        b.mesa.con(|s| {
            s.as_ref()
                .unwrap()
                .estado_encuentro()
                .estado
                .is_some_and(|e| e.marcador == vec![1, 0])
        })
    })
    .await;
    // En juego: biblioteca y mano (el banquillo también es una zona de la mesa).
    let en_juego = b.mesa.con(|s| {
        let p = s.as_ref().unwrap().partida();
        [Zona::Biblioteca, Zona::Mano]
            .into_iter()
            .flat_map(|z| p.zona(&RefZona::de(IdJugador(1), z)).to_vec())
            .map(|c| p.cartas[&c].origen.carta_deck.clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(en_juego.len(), 60);
    assert!(
        en_juego.contains(&entra),
        "{entra} debería estar en el mazo"
    );
    assert!(!en_juego.contains(&sale), "{sale} debería haber salido");
}

/// Lo último que dijo la sala de espera a esta pantalla: (apodo, deck) de cada uno.
fn sala_vista(m: &MesaEnMemoria) -> Vec<(String, Option<String>)> {
    avisos(m)
        .into_iter()
        .rev()
        .find_map(|a| match a {
            AvisoRed::Sala { jugadores } => Some(
                jugadores
                    .into_iter()
                    .map(|j| (j.apodo, j.deck))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn en_la_sala_cada_uno_cambia_su_nombre_y_su_deck_y_todos_lo_ven_al_momento() {
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
    let aceptador = tokio::spawn(atender(host.ep.clone(), anf.clone(), host.mesa.clone()));
    let codigo = Codigo {
        addr: host.ep.addr(),
        secreto: anf.secreto().to_string(),
    };
    let mut invitados = Vec::new();
    for i in 0..2 {
        let mut j = Jugador {
            mesa: MesaEnMemoria::default(),
            ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
            conexion: None,
        };
        j.conexion = Some(
            unirse(
                j.ep.clone(),
                Union {
                    codigo: codigo.clone(),
                    apodo: format!("Invitado {i}"),
                    rol: Rol::Jugador,
                    deck: Some(mazo()),
                },
                j.mesa.clone(),
            )
            .await
            .unwrap(),
        );
        invitados.push(j);
        esperar("el deck en la sala", || anf.para_empezar().len() == i + 1).await;
    }
    let (a, b) = (&invitados[0], &invitados[1]);
    let nombres =
        |m: &MesaEnMemoria| -> Vec<String> { sala_vista(m).into_iter().map(|(n, _)| n).collect() };

    // Un invitado se cambia el nombre: el anfitrión y el otro lo ven.
    let limpio = a
        .conexion
        .as_ref()
        .unwrap()
        .cambiar_apodo("  Ana   la  Grande ");
    assert_eq!(limpio, "Ana la Grande");
    esperar("el nombre nuevo en el otro invitado", || {
        nombres(&b.mesa) == ["Host", "Ana la Grande", "Invitado 1"]
    })
    .await;
    assert_eq!(anf.vista_sala()[1].apodo, "Ana la Grande");

    // Uno vacío no cambia nada.
    assert_eq!(a.conexion.as_ref().unwrap().cambiar_apodo("   "), "");
    // El anfitrión se cambia el suyo: lo ven los dos.
    assert!(anf.poner_apodo("Pablo"));
    assert!(!anf.poner_apodo("Pablo"));
    anf.avisar_sala(&host.mesa);
    esperar("el nombre del anfitrión en los invitados", || {
        nombres(&a.mesa).first().map(String::as_str) == Some("Pablo")
            && nombres(&b.mesa).first().map(String::as_str) == Some("Pablo")
    })
    .await;
    assert_eq!(
        nombres(&host.mesa),
        ["Pablo", "Ana la Grande", "Invitado 1"]
    );

    // Un invitado cambia de deck: se ve el nuevo en todas las salas.
    let mut otro = mazo();
    otro.nombre = "Otro deck".into();
    b.conexion.as_ref().unwrap().cambiar_deck(otro);
    esperar("el deck nuevo en el otro invitado", || {
        sala_vista(&a.mesa)
            .get(2)
            .and_then(|(_, d)| d.clone())
            .as_deref()
            == Some("Otro deck")
    })
    .await;
    assert_eq!(anf.vista_sala()[2].deck.as_deref(), Some("Otro deck"));
    // Y quien no lo cambió, conserva el suyo.
    assert_eq!(anf.vista_sala()[1].deck, Some(mazo().nombre));

    aceptador.abort();
}
