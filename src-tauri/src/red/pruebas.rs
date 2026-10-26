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
use super::invitado::{unirse, Union};
use super::mesa::{AvisoRed, Mesa, MesaEnMemoria};
use super::nodo::{crear, Relay};
use super::protocolo::Rol;
use crate::mesa::sesion::tests::mazo;
use crate::mesa::sesion::{ConfigMesa, JugadorMesa, RedSesion};
use mesa::formato::{Formato, Variantes};
use mesa::sparring::Intensidad;

struct Jugador {
    mesa: MesaEnMemoria,
    ep: Endpoint,
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
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();
    let host = Jugador {
        mesa: MesaEnMemoria::default(),
        ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
    };
    let anf = Anfitrion::nuevo("Host".into(), secreto_nuevo());
    *anf.deck.lock().unwrap() = Some("Prueba".into());
    tokio::spawn(atender(host.ep.clone(), anf.clone(), host.mesa.clone()));
    let codigo = Codigo {
        addr: host.ep.addr(),
        secreto: anf.secreto().to_string(),
    };
    let mut invitados = Vec::new();
    for i in 0..n {
        let j = Jugador {
            mesa: MesaEnMemoria::default(),
            ep: crear(SecretKey::generate(), Relay::Ninguno).await.unwrap(),
        };
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
        .unwrap();
        invitados.push(j);
    }
    esperar("los decks de la sala", || anf.para_empezar().len() == n).await;
    let config = ConfigMesa {
        jugadores: vec![JugadorMesa {
            deck_id: "d".into(),
            nombre: "Host".into(),
            color: "#e11d48".into(),
            lista: None,
        }],
        formato: Formato::Constructed,
        variantes: Variantes::default(),
        opciones: None,
        intensidad: Intensidad::Apagado,
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
