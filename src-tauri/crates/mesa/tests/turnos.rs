//! El orden de los turnos (G7): se juega por asientos desde el primer
//! jugador, se salta a los eliminados y la ronda sube al pasar por el primero.

mod comun;

use comun::*;
use mesa::accion::Intencion;
use mesa::preparar::iniciar;
use mesa::{Accion, Azar, IdJugador, ModoDeshacer, Registro};

/// `n` jugadores que ya se han quedado con su mano y empieza `primero`.
fn mesa(n: usize, primero: u8) -> (Registro, Azar) {
    let mut c = config(n);
    c.primero = Some(IdJugador(primero));
    let mut r = Registro::nuevo(
        iniciar("p".into(), c, 5, 0).unwrap(),
        ModoDeshacer::SinLimite,
    );
    let mut azar = Azar::new(1);
    for j in 0..n as u8 {
        r.procesar(
            &Intencion {
                actor: IdJugador(j),
                accion: Accion::QuedarseCon { al_fondo: vec![] },
            },
            &mut azar,
            1_000,
        )
        .unwrap();
    }
    (r, azar)
}

fn haz(r: &mut Registro, azar: &mut Azar, actor: u8, accion: Accion) {
    r.procesar(
        &Intencion {
            actor: IdJugador(actor),
            accion,
        },
        azar,
        2_000,
    )
    .unwrap();
}

/// Pasa el turno `veces` veces y devuelve (quién juega, ronda) tras cada una.
fn pasa(r: &mut Registro, azar: &mut Azar, veces: usize) -> Vec<(u8, u32)> {
    (0..veces)
        .map(|_| {
            let activo = r.partida().turno.activo.0;
            haz(r, azar, activo, Accion::PasarTurno);
            let t = &r.partida().turno;
            (t.activo.0, t.ronda)
        })
        .collect()
}

#[test]
fn cinco_jugadores_con_uno_eliminado_se_salta_y_la_ronda_sube_al_pasar_por_el_primero() {
    // Empieza el asiento 2 y el 4 está fuera.
    let (mut r, mut azar) = mesa(5, 2);
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::Eliminar {
            jugador: IdJugador(4),
            por: None,
        },
    );
    assert_eq!(r.partida().turno.activo, IdJugador(2));
    assert_eq!(r.partida().turno.ronda, 1);
    let v = pasa(&mut r, &mut azar, 8);
    assert_eq!(
        v,
        vec![
            (3, 1), // 2 → 3
            (0, 1), // 3 → 0 (pasa por el asiento 0, pero el primero es el 2)
            (1, 1), // 0 → 1
            (2, 2), // 1 → salta al 4 → 2: ronda nueva
            (3, 2),
            (0, 2),
            (1, 2),
            (2, 3),
        ]
    );
}

#[test]
fn si_el_primero_se_va_la_ronda_sigue_subiendo_en_su_asiento() {
    let (mut r, mut azar) = mesa(4, 1);
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::Eliminar {
            jugador: IdJugador(1),
            por: None,
        },
    );
    // El que empezaba era el 1 y ha salido: pasa el turno solo, al 2.
    let t = &r.partida().turno;
    assert_eq!((t.activo.0, t.ronda), (2, 1));
    let v = pasa(&mut r, &mut azar, 6);
    assert_eq!(
        v,
        vec![
            (3, 1),
            (0, 1),
            (2, 2), // del 0 pasa por el asiento 1 (vacío): ronda nueva
            (3, 2),
            (0, 2),
            (2, 3),
        ]
    );
}

#[test]
fn con_empiece_en_el_asiento_cero_todo_sigue_igual() {
    let (mut r, mut azar) = mesa(3, 0);
    let v = pasa(&mut r, &mut azar, 4);
    assert_eq!(v, vec![(1, 1), (2, 1), (0, 2), (1, 2)]);
}

#[test]
fn solo_en_la_mesa_cada_turno_es_una_ronda() {
    let (mut r, mut azar) = mesa(2, 1);
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::Eliminar {
            jugador: IdJugador(0),
            por: None,
        },
    );
    let v = pasa(&mut r, &mut azar, 2);
    assert_eq!(v, vec![(1, 2), (1, 3)]);
}
