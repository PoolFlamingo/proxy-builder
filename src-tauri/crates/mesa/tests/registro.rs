//! Deshacer, rehacer y repetición (A8), huella (A9) y mulligan (A7).

mod comun;

use comun::*;
use mesa::accion::{Accion, Intencion};
use mesa::formato::ModoMulligan;
use mesa::modelo::{RefZona, Zona};
use mesa::{huella, Azar, IdJugador, ModoDeshacer, Rechazo, Registro};

fn n_acciones(r: &mut Registro, azar: &mut Azar, gen: &mut Azar, n: usize, jugadores: usize) {
    let mut hechas = 0;
    let mut t = 5_000;
    while hechas < n {
        let i = intencion_al_azar(r.partida(), gen, jugadores);
        t += 1;
        if matches!(i.accion, Accion::Deshacer { .. } | Accion::Rehacer) {
            continue;
        }
        if r.procesar(&i, azar, t).is_ok() {
            hechas += 1;
        }
    }
}

#[test]
fn deshacer_20_de_300_es_como_no_haberlas_hecho() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let mut gen = Azar::new(1);
    n_acciones(&mut r, &mut azar, &mut gen, 280, 2);
    let a_los_280 = huella(r.partida());
    let seq_280 = r.partida().seq;
    n_acciones(&mut r, &mut azar, &mut gen, 20, 2);
    for _ in 0..20 {
        r.procesar(
            &Intencion {
                actor: IdJugador(0),
                accion: Accion::Deshacer { hasta: None },
            },
            &mut azar,
            9_999,
        )
        .unwrap();
    }
    assert_eq!(huella(r.partida()), a_los_280);
    assert_eq!(huella(&r.en(seq_280).unwrap()), a_los_280);
    // Rehacer 10: diez eventos nuevos.
    let antes = r.eventos.len();
    for _ in 0..10 {
        r.procesar(
            &Intencion {
                actor: IdJugador(0),
                accion: Accion::Rehacer,
            },
            &mut azar,
            10_000,
        )
        .unwrap();
    }
    assert_eq!(r.eventos.len(), antes + 10);
}

#[test]
fn deshacer_hasta_una_linea_del_registro() {
    let (mut r, mut azar) = empezada(1, ModoDeshacer::SinLimite);
    let mut gen = Azar::new(2);
    n_acciones(&mut r, &mut azar, &mut gen, 30, 1);
    let seq = r.partida().seq;
    let objetivo = huella(&r.en(seq - 10).unwrap());
    // La línea `seq - 9` y todo lo de después.
    let desde = r
        .eventos
        .iter()
        .find(|e| e.seq > seq - 10)
        .map(|e| e.seq)
        .unwrap();
    r.procesar(
        &Intencion {
            actor: IdJugador(0),
            accion: Accion::Deshacer { hasta: Some(desde) },
        },
        &mut azar,
        1,
    )
    .unwrap();
    assert_eq!(huella(r.partida()), objetivo);
    // El registro que se enseña ya no tiene lo deshecho ni el `Restaurar`.
    let vigentes = r.vigentes();
    assert!(vigentes.iter().all(|e| e.seq < desde));
    assert_eq!(
        vigentes.len(),
        r.eventos.iter().filter(|e| e.seq < desde).count()
    );
}

#[test]
fn en_multijugador_solo_la_ultima_propia() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::UltimaPropia);
    let robar = |j: u8| Intencion {
        actor: IdJugador(j),
        accion: Accion::Vida {
            jugador: IdJugador(j),
            delta: 1,
        },
    };
    let deshacer = |j: u8| Intencion {
        actor: IdJugador(j),
        accion: Accion::Deshacer { hasta: None },
    };
    r.procesar(&robar(0), &mut azar, 1).unwrap();
    r.procesar(&robar(1), &mut azar, 2).unwrap();
    assert_eq!(
        r.procesar(&deshacer(0), &mut azar, 3).err(),
        Some(Rechazo::NoSePuede),
        "otro hizo algo después"
    );
    r.procesar(&deshacer(1), &mut azar, 4).unwrap();
    assert_eq!(r.partida().jugador(IdJugador(1)).unwrap().vida, 40);
    assert_eq!(
        r.procesar(
            &Intencion {
                actor: IdJugador(1),
                accion: Accion::Rehacer
            },
            &mut azar,
            5
        )
        .err(),
        Some(Rechazo::NoSePuede)
    );
}

#[test]
fn un_invitado_que_entra_tarde_sigue_al_host() {
    let (mut host, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let mut gen = Azar::new(3);
    n_acciones(&mut host, &mut azar, &mut gen, 60, 2);
    // Entra con una instantánea.
    let mut invitado = Registro::nuevo(host.partida().clone(), ModoDeshacer::UltimaPropia);
    let desde = host.eventos.len();
    n_acciones(&mut host, &mut azar, &mut gen, 20, 2);
    host.procesar(
        &Intencion {
            actor: IdJugador(0),
            accion: Accion::Deshacer { hasta: None },
        },
        &mut azar,
        1,
    )
    .unwrap();
    for ev in &host.eventos[desde..] {
        invitado.recibir(ev.clone()).unwrap();
    }
    assert_eq!(huella(invitado.partida()), huella(host.partida()));
    // Un evento fuera de orden se rechaza y no cambia nada.
    let viejo = host.eventos[desde].clone();
    let h = huella(invitado.partida());
    assert!(invitado.recibir(viejo).is_err());
    assert_eq!(huella(invitado.partida()), h);
}

#[test]
fn la_repeticion_llega_a_cualquier_momento() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let mut gen = Azar::new(4);
    n_acciones(&mut r, &mut azar, &mut gen, 130, 2);
    // Aplicando uno a uno desde el principio.
    let mut p = r.inicial.clone();
    for ev in &r.eventos {
        mesa::aplicar(&mut p, ev).unwrap();
        assert_eq!(huella(&r.en(ev.seq).unwrap()), huella(&p), "en({})", ev.seq);
    }
    assert!(!r.fotos.is_empty());
}

#[test]
fn huella_estable_y_sensible() {
    let p = partida(2);
    assert_eq!(huella(&p), huella(&p.clone()));
    assert_eq!(huella(&p).len(), 16);
    let mut q = p.clone();
    q.jugadores[0].vida -= 1;
    assert_ne!(huella(&p), huella(&q));
    // Ningún HashMap en el modelo (revisión del código).
    for f in ["modelo.rs", "formato.rs", "sparring/mod.rs"] {
        let src =
            std::fs::read_to_string(format!("{}/src/{f}", env!("CARGO_MANIFEST_DIR"))).unwrap();
        assert!(
            !src.contains("HashMap<") && !src.contains("HashSet<"),
            "{f} usa HashMap"
        );
    }
    // Sin reloj ni azar de fuera en el crate.
    for f in std::fs::read_dir(format!("{}/src", env!("CARGO_MANIFEST_DIR"))).unwrap() {
        let f = f.unwrap().path();
        if f.is_file() {
            let src = std::fs::read_to_string(&f).unwrap();
            for prohibido in ["SystemTime", "Instant::", "rand::"] {
                assert!(!src.contains(prohibido), "{f:?} usa {prohibido}");
            }
        }
    }
}

fn mulligan(modo: ModoMulligan, mulligans: usize, al_fondo: usize) -> Result<(), Rechazo> {
    let mut cfg = config(1);
    cfg.opciones.mulligan = modo;
    let p = mesa::preparar::iniciar("m".into(), cfg, 5, 0).unwrap();
    let mut r = Registro::nuevo(p, ModoDeshacer::SinLimite);
    let mut azar = Azar::new(6);
    let yo = IdJugador(0);
    for _ in 0..mulligans {
        r.procesar(
            &Intencion {
                actor: yo,
                accion: Accion::Mulligan,
            },
            &mut azar,
            0,
        )?;
        assert_eq!(r.partida().zona(&RefZona::de(yo, Zona::Mano)).len(), 7);
    }
    let mano = r.partida().zona(&RefZona::de(yo, Zona::Mano)).to_vec();
    let fondo = mano[..al_fondo].to_vec();
    r.procesar(
        &Intencion {
            actor: yo,
            accion: Accion::QuedarseCon {
                al_fondo: fondo.clone(),
            },
        },
        &mut azar,
        0,
    )?;
    let lib = r.partida().zona(&RefZona::de(yo, Zona::Biblioteca));
    assert_eq!(&lib[lib.len() - al_fondo..], &fondo[..]);
    assert!(r.partida().mulligan.is_none());
    assert_eq!(r.partida().turno.numero, 1);
    Ok(())
}

#[test]
fn mulligan_londres() {
    assert!(mulligan(ModoMulligan::Londres, 0, 0).is_ok());
    assert!(mulligan(ModoMulligan::Londres, 1, 1).is_ok());
    assert!(mulligan(ModoMulligan::Londres, 2, 2).is_ok());
    assert_eq!(mulligan(ModoMulligan::Londres, 2, 1), Err(Rechazo::Limite));
    assert!(mulligan(ModoMulligan::LondresPrimeroGratis, 1, 0).is_ok());
    assert!(mulligan(ModoMulligan::LondresPrimeroGratis, 2, 1).is_ok());
    assert_eq!(
        mulligan(ModoMulligan::LondresPrimeroGratis, 1, 1),
        Err(Rechazo::Limite)
    );
    assert!(mulligan(ModoMulligan::Amistoso, 3, 0).is_ok());
    assert!(mulligan(ModoMulligan::Libre, 1, 3).is_ok());
}

#[test]
fn nadie_empieza_hasta_que_todos_deciden() {
    let p = partida(3);
    let mut r = Registro::nuevo(p, ModoDeshacer::SinLimite);
    let mut azar = Azar::new(1);
    let quedarse = |j: u8| Intencion {
        actor: IdJugador(j),
        accion: Accion::QuedarseCon { al_fondo: vec![] },
    };
    r.procesar(&quedarse(0), &mut azar, 0).unwrap();
    assert_eq!(
        r.procesar(
            &Intencion {
                actor: IdJugador(0),
                accion: Accion::PasarTurno
            },
            &mut azar,
            0
        )
        .err(),
        Some(Rechazo::NoSePuede)
    );
    r.procesar(&quedarse(1), &mut azar, 0).unwrap();
    assert!(r.partida().mulligan.is_some());
    r.procesar(&quedarse(2), &mut azar, 0).unwrap();
    assert!(r.partida().mulligan.is_none());
    // Con 3 jugadores el primero sí roba.
    assert_eq!(
        r.partida()
            .zona(&RefZona::de(IdJugador(0), Zona::Mano))
            .len(),
        8
    );
}

#[test]
fn una_partida_guardada_se_reconstruye_igual() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let mut gen = Azar::new(9);
    n_acciones(&mut r, &mut azar, &mut gen, 120, 2);
    let json = serde_json::to_string(&(&r.inicial, &r.eventos)).unwrap();
    let (inicial, eventos): (mesa::Partida, Vec<mesa::Evento>) =
        serde_json::from_str(&json).unwrap();
    let mut otra = Registro::reconstruir(inicial, eventos, ModoDeshacer::SinLimite).unwrap();
    assert_eq!(huella(otra.partida()), huella(r.partida()));
    // Y se puede deshacer lo de antes de guardar.
    otra.procesar(
        &Intencion {
            actor: IdJugador(0),
            accion: Accion::Deshacer { hasta: None },
        },
        &mut azar,
        1,
    )
    .unwrap();
}
