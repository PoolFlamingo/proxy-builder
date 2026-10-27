//! Quién está fuera de juego (G8): vida, veneno y daño de comandante, con la
//! vida compartida de los equipos, y que la vista lo cuente.

mod comun;

use comun::*;
use mesa::accion::Intencion;
use mesa::formato::{Formato, ModoEquipos, Opciones, Variantes};
use mesa::modelo::{Equipo, RefZona, Zona};
use mesa::preparar::iniciar;
use mesa::{
    fuera_de_juego, parche, vista, Accion, Azar, Causa, IdCarta, IdEquipo, IdJugador, Mirador,
    ModoDeshacer, Registro,
};

fn mesa(n: usize) -> (Registro, Azar) {
    let mut r = Registro::nuevo(partida(n), ModoDeshacer::SinLimite);
    let mut azar = Azar::new(5);
    for j in 0..n as u8 {
        haz(
            &mut r,
            &mut azar,
            j,
            Accion::QuedarseCon { al_fondo: vec![] },
        );
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

fn comandante(r: &Registro, de: u8) -> IdCarta {
    *r.partida()
        .zona(&RefZona::de(IdJugador(de), Zona::Mando))
        .first()
        .expect("el comandante está en la zona de mando")
}

fn causa(r: &Registro, j: u8) -> Option<Causa> {
    fuera_de_juego(r.partida(), IdJugador(j))
}

#[test]
fn nadie_esta_fuera_al_empezar() {
    let (r, _) = mesa(3);
    assert!((0..3).all(|j| causa(&r, j).is_none()));
}

#[test]
fn a_cero_vidas_o_menos() {
    let (mut r, mut azar) = mesa(2);
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::Vida {
            jugador: IdJugador(0),
            delta: -39,
        },
    );
    assert_eq!(causa(&r, 0), None, "con 1 vida sigue en juego");
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::Vida {
            jugador: IdJugador(0),
            delta: -5,
        },
    );
    assert_eq!(causa(&r, 0), Some(Causa::Vida));
    assert_eq!(causa(&r, 1), None);
}

#[test]
fn al_limite_de_veneno() {
    let (mut r, mut azar) = mesa(2);
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::Veneno {
            jugador: IdJugador(0),
            delta: 9,
        },
    );
    assert_eq!(causa(&r, 0), None);
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::Veneno {
            jugador: IdJugador(0),
            delta: 1,
        },
    );
    assert_eq!(causa(&r, 0), Some(Causa::Veneno));
}

#[test]
fn a_21_de_un_mismo_comandante_y_dice_de_quien_es() {
    let (mut r, mut azar) = mesa(3);
    let de_dos = comandante(&r, 2);
    // 20 de uno no bastan, ni sumar 11 + 11 de comandantes distintos.
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::DanoComandante {
            jugador: IdJugador(0),
            comandante: de_dos,
            delta: 20,
        },
    );
    assert_eq!(causa(&r, 0), None);
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::DanoComandante {
            jugador: IdJugador(0),
            comandante: de_dos,
            delta: 1,
        },
    );
    assert_eq!(
        causa(&r, 0),
        Some(Causa::Comandante {
            de: IdJugador(2),
            carta: de_dos
        })
    );
}

#[test]
fn quien_ya_esta_eliminado_no_se_vuelve_a_avisar() {
    let (mut r, mut azar) = mesa(2);
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::Vida {
            jugador: IdJugador(0),
            delta: -50,
        },
    );
    assert!(causa(&r, 0).is_some());
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::Eliminar {
            jugador: IdJugador(0),
            por: Some(IdJugador(1)),
        },
    );
    assert_eq!(causa(&r, 0), None);
    assert_eq!(
        r.partida().jugador(IdJugador(0)).unwrap().eliminado,
        Some(mesa::modelo::Eliminacion {
            ronda: 1,
            por: Some(IdJugador(1))
        })
    );
}

#[test]
fn con_vida_compartida_manda_la_del_equipo_para_los_dos() {
    let variantes = Variantes {
        equipos: ModoEquipos::Equipos { fuego_amigo: false },
        ..Variantes::default()
    };
    let mut c = config(4);
    c.opciones = Opciones::de_fabrica(Formato::Commander, &variantes, 4);
    let vida = c.opciones.vida_inicial;
    c.variantes = variantes;
    c.equipos = [0u8, 1]
        .into_iter()
        .map(|i| Equipo {
            id: IdEquipo(i),
            nombre: format!("E{i}"),
            color: "#fff".into(),
            vida_compartida: Some(vida),
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
        haz(
            &mut r,
            &mut azar,
            j,
            Accion::QuedarseCon { al_fondo: vec![] },
        );
    }
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::Vida {
            jugador: IdJugador(0),
            delta: -vida,
        },
    );
    // La vida de cada uno no se toca, pero los dos del equipo están fuera.
    assert_eq!(causa(&r, 0), Some(Causa::Vida));
    assert_eq!(causa(&r, 1), Some(Causa::Vida));
    assert_eq!(causa(&r, 2), None);
    // Y el parche lleva también al compañero, que no cambió como jugador.
    let tocado = mesa::aplicar::Tocado {
        general: true,
        ..Default::default()
    };
    let p = parche(r.partida(), &tocado, Mirador::Todo);
    let en_parche = |id: u8| p.jugadores.iter().find(|j| j.id == IdJugador(id));
    assert!(en_parche(1).is_some_and(|j| j.fuera_de_juego == Some(Causa::Vida)));
    assert!(
        en_parche(2).is_some_and(|j| j.fuera_de_juego.is_none()),
        "el otro equipo también va (cambió general), sin avisos"
    );
}

#[test]
fn la_vista_lleva_la_causa_a_cada_mirador() {
    let (mut r, mut azar) = mesa(2);
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::Veneno {
            jugador: IdJugador(0),
            delta: 10,
        },
    );
    for m in [
        Mirador::Todo,
        Mirador::Jugador(IdJugador(1)),
        Mirador::Espectador,
    ] {
        let v = vista(r.partida(), m);
        assert_eq!(v.jugadores[0].fuera_de_juego, Some(Causa::Veneno));
        assert_eq!(v.jugadores[1].fuera_de_juego, None);
    }
}

#[test]
fn quedarse_por_el_ausente_cumple_lo_que_le_toca_en_el_mulligan() {
    use mesa::ausencia::quedarse_por_defecto;
    let mut r = Registro::nuevo(partida(2), ModoDeshacer::SinLimite);
    let mut azar = Azar::new(5);
    // Con la mano de 7 y sin mulligans: se queda con todas.
    let Some(Accion::QuedarseCon { al_fondo }) = quedarse_por_defecto(r.partida(), IdJugador(0))
    else {
        panic!("en el mulligan se puede quedar");
    };
    assert!(al_fondo.is_empty());
    // Dos mulligans (el primero gratis): una al fondo, y se acepta tal cual.
    haz(&mut r, &mut azar, 0, Accion::Mulligan);
    haz(&mut r, &mut azar, 0, Accion::Mulligan);
    let Some(a) = quedarse_por_defecto(r.partida(), IdJugador(0)) else {
        panic!("sigue sin decidir");
    };
    match &a {
        Accion::QuedarseCon { al_fondo } => assert_eq!(al_fondo.len(), 1),
        otra => panic!("{otra:?}"),
    }
    haz(&mut r, &mut azar, 0, a);
    // Ya decidió: no hay nada que hacer por él; y el otro sigue pendiente.
    assert!(quedarse_por_defecto(r.partida(), IdJugador(0)).is_none());
    assert!(quedarse_por_defecto(r.partida(), IdJugador(1)).is_some());
    // Fuera del mulligan tampoco.
    haz(
        &mut r,
        &mut azar,
        1,
        Accion::QuedarseCon { al_fondo: vec![] },
    );
    assert!(quedarse_por_defecto(r.partida(), IdJugador(1)).is_none());
}
