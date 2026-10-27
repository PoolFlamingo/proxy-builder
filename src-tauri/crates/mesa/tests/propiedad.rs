//! Tests de propiedad del motor (A6, A9, A10): mil intenciones al azar.

mod comun;

use comun::*;
use mesa::aplicar::aplicar;
use mesa::vista::{aplicar_parche, parche, vista};
use mesa::{huella, Azar, Cambio, Evento, IdJugador, Mirador, ModoDeshacer};

fn jugar(semilla: u64, n: usize, pasos: usize, mirar: bool) -> String {
    let (mut r, _) = empezada(n, ModoDeshacer::SinLimite);
    let mut azar = Azar::new(semilla);
    let mut gen = Azar::new(semilla ^ 0xabcdef);
    let miradores: Vec<Mirador> = (0..n)
        .map(|j| Mirador::Jugador(IdJugador(j as u8)))
        .chain([Mirador::Espectador])
        .collect();
    let mut vistas: Vec<_> = miradores.iter().map(|m| vista(r.partida(), *m)).collect();
    let mut validas = 0;
    for paso in 0..pasos {
        let i = intencion_al_azar(r.partida(), &mut gen, n);
        let antes = huella(r.partida());
        match r.procesar(&i, &mut azar, 3_000 + paso as u64) {
            Ok(h) => {
                validas += 1;
                if mirar {
                    for (m, v) in miradores.iter().zip(vistas.iter_mut()) {
                        let pa = parche(r.partida(), &h.tocado, *m);
                        aplicar_parche(v, &pa);
                        let entera = vista(r.partida(), *m);
                        if *v != entera {
                            for (a, b) in v.cartas.iter().zip(entera.cartas.iter()) {
                                if a != b {
                                    panic!("carta distinta tras {:?}:\n{a:?}\n{b:?}", i.accion);
                                }
                            }
                            assert_eq!(
                                v.cartas.len(),
                                entera.cartas.len(),
                                "nº de cartas tras {:?}",
                                i.accion
                            );
                            assert_eq!(v.zonas, entera.zonas, "zonas tras {:?}", i.accion);
                            assert_eq!(*v, entera, "parche ≠ vista tras {:?}", i.accion);
                        }
                        sin_fugas(r.partida(), &entera, *m);
                    }
                }
            }
            Err(_) => assert_eq!(antes, huella(r.partida()), "un rechazo cambió la partida"),
        }
    }
    assert!(validas > pasos / 4, "muy pocas válidas: {validas}");
    huella(r.partida())
}

#[test]
fn mil_intenciones_sin_panico_y_deterministas() {
    let a = jugar(7, 4, 1_000, false);
    let b = jugar(7, 4, 1_000, false);
    assert_eq!(a, b);
    let c = jugar(8, 4, 1_000, false);
    assert_ne!(a, c);
}

#[test]
fn las_vistas_no_enseñan_lo_oculto_y_los_parches_llegan_a_la_vista() {
    jugar(11, 3, 400, true);
    jugar(12, 1, 300, true);
}

#[test]
fn eventos_arbitrarios_no_rompen_nada() {
    let (r, _) = empezada(2, ModoDeshacer::SinLimite);
    let mut p = r.partida().clone();
    let mut gen = Azar::new(5);
    let mut aplicados = 0;
    for k in 0..2_000u64 {
        let mut i = intencion_al_azar(&p, &mut gen, 2);
        // Actores que no existen, cartas inventadas…
        if k % 7 == 0 {
            i.actor = IdJugador(9);
        }
        let ev = Evento {
            seq: if k % 13 == 0 { p.seq + 5 } else { p.seq + 1 },
            actor: mesa::accion::Actor::Jugador(i.actor),
            t_ms: k,
            cambio: Cambio::Accion {
                accion: i.accion,
                azar: if k % 3 == 0 { None } else { Some(k) },
            },
        };
        let antes = huella(&p);
        match aplicar(&mut p, &ev) {
            Ok(_) => aplicados += 1,
            Err(_) => assert_eq!(antes, huella(&p)),
        }
    }
    assert!(aplicados > 100);
}
