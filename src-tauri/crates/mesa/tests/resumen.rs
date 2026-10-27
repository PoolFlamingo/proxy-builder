//! El resumen de la partida (H1): puestos por orden de eliminación, con
//! empates, concesiones y equipos.

mod comun;

use comun::*;
use mesa::accion::Intencion;
use mesa::formato::{Formato, ModoEquipos, Opciones, Variantes};
use mesa::modelo::{ComoGano, Equipo, Resultado};
use mesa::preparar::iniciar;
use mesa::{resumir, Accion, Azar, IdEquipo, IdJugador, ModoDeshacer, Registro};

fn empezar(mut r: Registro, n: u8) -> (Registro, Azar) {
    let mut azar = Azar::new(5);
    for j in 0..n {
        haz(
            &mut r,
            &mut azar,
            j,
            Accion::QuedarseCon { al_fondo: vec![] },
            1_000,
        );
    }
    (r, azar)
}

fn haz(r: &mut Registro, azar: &mut Azar, actor: u8, accion: Accion, t: u64) {
    r.procesar(
        &Intencion {
            actor: IdJugador(actor),
            accion,
        },
        azar,
        t,
    )
    .unwrap();
}

fn puestos(r: &Registro) -> Vec<(u8, u32)> {
    resumir(r)
        .puestos
        .iter()
        .map(|x| (x.jugador.0, x.puesto))
        .collect()
}

fn fuera(r: &mut Registro, azar: &mut Azar, j: u8, por: u8, t: u64) {
    haz(
        r,
        azar,
        por,
        Accion::Eliminar {
            jugador: IdJugador(j),
            por: Some(IdJugador(por)),
        },
        t,
    );
}

fn terminar(ganadores: &[u8], como: ComoGano) -> Accion {
    Accion::Terminar {
        resultado: Resultado {
            ganadores: ganadores.iter().map(|g| IdJugador(*g)).collect(),
            como,
            nota: String::new(),
        },
    }
}

#[test]
fn el_ultimo_en_caer_queda_mejor_aunque_caigan_en_la_misma_ronda() {
    let (mut r, mut azar) = empezar(Registro::nuevo(partida(4), ModoDeshacer::SinLimite), 4);
    fuera(&mut r, &mut azar, 2, 0, 2_000);
    fuera(&mut r, &mut azar, 1, 0, 3_000);
    fuera(&mut r, &mut azar, 3, 0, 4_000);
    // Gana el 0; cayeron 2, 1 y 3 por ese orden (los tres en la ronda 1).
    assert_eq!(puestos(&r), vec![(0, 1), (3, 2), (1, 3), (2, 4)]);
    let s = resumir(&r);
    assert!(s
        .puestos
        .iter()
        .all(|x| x.jugador.0 == 0 || x.ronda == Some(1)));
    assert_eq!(s.puestos[1].por, Some(IdJugador(0)));
    assert_eq!(s.duracion_ms, 4_000 - 1_000);
}

#[test]
fn conceder_cuenta_como_caer() {
    let (mut r, mut azar) = empezar(Registro::nuevo(partida(3), ModoDeshacer::SinLimite), 3);
    haz(&mut r, &mut azar, 1, Accion::Conceder, 2_000);
    haz(&mut r, &mut azar, 2, Accion::Conceder, 3_000);
    assert_eq!(puestos(&r), vec![(0, 1), (2, 2), (1, 3)]);
    assert_eq!(resumir(&r).puestos[2].por, None);
}

#[test]
fn con_el_resultado_mandan_los_ganadores_y_un_empate_comparte_el_primero() {
    let (mut r, mut azar) = empezar(Registro::nuevo(partida(3), ModoDeshacer::SinLimite), 3);
    fuera(&mut r, &mut azar, 2, 0, 2_000);
    haz(
        &mut r,
        &mut azar,
        0,
        terminar(&[0, 1], ComoGano::Empate),
        3_000,
    );
    assert_eq!(puestos(&r), vec![(0, 1), (1, 1), (2, 3)]);
}

#[test]
fn quien_sigue_en_pie_sin_ganar_va_detras_de_los_ganadores() {
    let (mut r, mut azar) = empezar(Registro::nuevo(partida(3), ModoDeshacer::SinLimite), 3);
    haz(&mut r, &mut azar, 0, terminar(&[1], ComoGano::Combo), 2_000);
    assert_eq!(puestos(&r), vec![(1, 1), (0, 2), (2, 2)]);
}

#[test]
fn los_companeros_de_equipo_comparten_puesto() {
    let variantes = Variantes {
        equipos: ModoEquipos::Equipos { fuego_amigo: false },
        ..Variantes::default()
    };
    let mut c = config(4);
    c.opciones = Opciones::de_fabrica(Formato::Commander, &variantes, 4);
    c.variantes = variantes;
    c.equipos = [0u8, 1]
        .into_iter()
        .map(|i| Equipo {
            id: IdEquipo(i),
            nombre: format!("E{i}"),
            color: "#fff".into(),
            vida_compartida: None,
        })
        .collect();
    for (i, j) in c.jugadores.iter_mut().enumerate() {
        j.equipo = Some(IdEquipo((i / 2) as u8));
    }
    let (mut r, mut azar) = empezar(
        Registro::nuevo(
            iniciar("p".into(), c, 7, 1_000).unwrap(),
            ModoDeshacer::SinLimite,
        ),
        4,
    );
    fuera(&mut r, &mut azar, 2, 0, 2_000);
    fuera(&mut r, &mut azar, 3, 0, 3_000);
    // El equipo 0 (0 y 1) sigue en pie; el 3 cayó más tarde que el 2, pero
    // son compañeros y comparten el puesto del mejor.
    assert_eq!(puestos(&r), vec![(0, 1), (1, 1), (2, 3), (3, 3)]);
}

#[test]
fn perder_contra_el_rival_del_solitario_no_es_ganar() {
    let (mut r, mut azar) = empezar(Registro::nuevo(partida(1), ModoDeshacer::SinLimite), 1);
    haz(
        &mut r,
        &mut azar,
        0,
        terminar(&[], ComoGano::Combate),
        2_000,
    );
    assert_eq!(puestos(&r), vec![(0, 2)]);
}

#[test]
fn un_empate_sin_nombres_lo_comparten_los_que_siguen_en_pie() {
    let (mut r, mut azar) = empezar(Registro::nuevo(partida(3), ModoDeshacer::SinLimite), 3);
    fuera(&mut r, &mut azar, 2, 0, 2_000);
    haz(&mut r, &mut azar, 0, terminar(&[], ComoGano::Empate), 3_000);
    assert_eq!(puestos(&r), vec![(0, 1), (1, 1), (2, 3)]);
}
