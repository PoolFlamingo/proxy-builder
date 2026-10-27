//! Equipos (G4): quién cuenta como oponente y la vida compartida.

mod comun;

use comun::*;
use mesa::accion::Intencion;
use mesa::formato::{Formato, ModoEquipos, Opciones, Variantes};
use mesa::modelo::{ComoGano, Equipo, Resultado};
use mesa::preparar::iniciar;
use mesa::{Accion, Azar, IdEquipo, IdJugador, Mirador, ModoDeshacer, Registro};

const A: IdEquipo = IdEquipo(0);
const B: IdEquipo = IdEquipo(1);

/// Cuatro jugadores en dos equipos (0 y 1 contra 2 y 3), todos ya con su mano.
fn mesa_de_equipos(modo: ModoEquipos, compartida: bool) -> (Registro, Azar) {
    let variantes = Variantes {
        equipos: modo,
        ..Variantes::default()
    };
    let mut c = config(4);
    c.opciones = Opciones::de_fabrica(Formato::Commander, &variantes, 4);
    let vida = c.opciones.vida_inicial;
    c.variantes = variantes;
    c.equipos = [(A, "Rojo"), (B, "Azul")]
        .into_iter()
        .map(|(id, nombre)| Equipo {
            id,
            nombre: nombre.into(),
            color: "#fff".into(),
            vida_compartida: compartida.then_some(vida),
        })
        .collect();
    for (i, j) in c.jugadores.iter_mut().enumerate() {
        j.equipo = Some(IdEquipo((i / 2) as u8));
    }
    let mut r = Registro::nuevo(
        iniciar("p".into(), c, 7, 1_000).unwrap(),
        ModoDeshacer::SinLimite,
    );
    let mut azar = Azar::new(3);
    for j in 0..4u8 {
        r.procesar(
            &Intencion {
                actor: IdJugador(j),
                accion: Accion::QuedarseCon { al_fondo: vec![] },
            },
            &mut azar,
            2_000,
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
        3_000,
    )
    .unwrap();
}

fn vida(r: &Registro, j: u8) -> i32 {
    r.partida().jugador(IdJugador(j)).unwrap().vida
}

fn vidas(r: &Registro) -> [i32; 4] {
    [vida(r, 0), vida(r, 1), vida(r, 2), vida(r, 3)]
}

#[test]
fn sin_fuego_amigo_drenar_no_toca_al_compañero() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: false }, false);
    haz(&mut r, &mut azar, 0, Accion::Drenar { n: 2 });
    // El 1 es del equipo del 0: no pierde; el 2 y el 3 pierden 2 cada uno y el 0 gana 4.
    assert_eq!(vidas(&r), [44, 40, 38, 38]);
}

#[test]
fn con_fuego_amigo_drenar_toca_a_todos_los_demas() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: true }, false);
    haz(&mut r, &mut azar, 0, Accion::Drenar { n: 2 });
    assert_eq!(vidas(&r), [46, 38, 38, 38]);
}

#[test]
fn vida_a_todos_los_oponentes_respeta_el_fuego_amigo() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: false }, false);
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::VidaATodos {
            delta: -3,
            solo_oponentes: true,
        },
    );
    assert_eq!(vidas(&r), [37, 37, 40, 40]);

    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: true }, false);
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::VidaATodos {
            delta: -3,
            solo_oponentes: true,
        },
    );
    assert_eq!(vidas(&r), [37, 37, 40, 37]);
}

#[test]
fn sin_equipos_todos_son_oponentes() {
    let (mut r, mut azar) = {
        let mut r = Registro::nuevo(partida(4), ModoDeshacer::SinLimite);
        let mut azar = Azar::new(3);
        for j in 0..4u8 {
            haz(
                &mut r,
                &mut azar,
                j,
                Accion::QuedarseCon { al_fondo: vec![] },
            );
        }
        (r, azar)
    };
    haz(&mut r, &mut azar, 0, Accion::Drenar { n: 1 });
    assert_eq!(vidas(&r), [43, 39, 39, 39]);
}

#[test]
fn los_oponentes_dejan_fuera_a_los_eliminados() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: false }, false);
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::Eliminar {
            jugador: IdJugador(3),
            por: None,
        },
    );
    assert_eq!(
        r.partida().oponentes(IdJugador(0)),
        vec![IdJugador(2)],
        "el compañero no cuenta y el eliminado, tampoco"
    );
}

#[test]
fn con_vida_compartida_el_daño_a_uno_baja_la_del_equipo() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: false }, true);
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::Vida {
            jugador: IdJugador(0),
            delta: -7,
        },
    );
    let p = r.partida();
    let rojo = p.equipos.iter().find(|e| e.id == A).unwrap();
    assert_eq!(rojo.vida_compartida, Some(33));
    // La de cada jugador no se toca: la que vale es la del equipo.
    assert_eq!(vida(&r, 0), 40);
    assert_eq!(vida(&r, 1), 40);
    // Y los dos la leen igual en la vista, con la del otro equipo intacta.
    for yo in [0u8, 1, 2] {
        let v = mesa::vista(r.partida(), Mirador::Jugador(IdJugador(yo)));
        let rojo = v.general.equipos.iter().find(|e| e.id == A).unwrap();
        let azul = v.general.equipos.iter().find(|e| e.id == B).unwrap();
        assert_eq!(rojo.vida_compartida, Some(33));
        assert_eq!(azul.vida_compartida, Some(40));
    }
}

#[test]
fn con_vida_compartida_drenar_resta_a_cada_oponente_y_suma_al_propio_equipo() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: false }, true);
    haz(&mut r, &mut azar, 0, Accion::Drenar { n: 2 });
    let p = r.partida();
    let get = |e| {
        p.equipos
            .iter()
            .find(|x| x.id == e)
            .unwrap()
            .vida_compartida
    };
    // El azul tiene dos oponentes de 0: pierde 2 por cada uno; el rojo gana lo drenado.
    assert_eq!(get(B), Some(36));
    assert_eq!(get(A), Some(44));
}

#[test]
fn dos_cabezas_trae_30_de_vida_compartida_por_parejas() {
    let variantes = Variantes {
        equipos: ModoEquipos::DosCabezas,
        ..Variantes::default()
    };
    let mut c = config(4);
    c.opciones = Opciones::de_fabrica(Formato::Commander, &variantes, 4);
    c.variantes = variantes;
    let p = iniciar("p".into(), c, 1, 0).unwrap();
    assert_eq!(p.equipos.len(), 2);
    assert!(p.equipos.iter().all(|e| e.vida_compartida == Some(30)));
    let eq = |j: u8| p.jugador(IdJugador(j)).unwrap().equipo;
    assert_eq!(eq(0), eq(1));
    assert_eq!(eq(2), eq(3));
    assert_ne!(eq(0), eq(2));
    // Sin fuego amigo.
    assert_eq!(p.oponentes(IdJugador(0)), vec![IdJugador(2), IdJugador(3)]);
}

#[test]
fn un_equipo_gana_junto() {
    let (mut r, mut azar) = mesa_de_equipos(ModoEquipos::Equipos { fuego_amigo: false }, false);
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::Terminar {
            resultado: Resultado {
                ganadores: vec![IdJugador(0), IdJugador(1)],
                como: ComoGano::Combate,
                nota: String::new(),
            },
        },
    );
    let fin = r.partida().fin.clone().unwrap();
    assert_eq!(fin.ganadores, vec![IdJugador(0), IdJugador(1)]);
}

#[test]
fn un_equipo_que_no_existe_es_un_error_al_empezar() {
    let mut c = config(2);
    c.jugadores[0].equipo = Some(IdEquipo(5));
    assert!(iniciar("p".into(), c, 1, 0).is_err());
}
