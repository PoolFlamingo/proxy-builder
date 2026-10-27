//! Formatos multijugador (G5): los valores de fábrica, Archenemy, Planechase
//! y el daño de comandante.

mod comun;

use comun::*;
use mesa::accion::Intencion;
use mesa::formato::{Formato, ModoEquipos, Opciones, Variantes, VIDA_ARCHIENEMIGO};
use mesa::modelo::{Equipo, RefZona, Zona, ZonaComun};
use mesa::preparar::{iniciar, CartaDeMesa, Config, Seccion};
use mesa::{Accion, Azar, IdCarta, IdEquipo, IdJugador, ModoDeshacer, Rechazo, Registro};

fn plano(config: &mut Config, jugador: usize, nombre: &str, tipo: &str, seccion: Seccion) {
    let mut o = origen(nombre, tipo, 0);
    o.carta_deck = format!("x-{nombre}");
    config.jugadores[jugador].mazo.cartas.push(CartaDeMesa {
        carta_deck: o.carta_deck.clone(),
        seccion,
        origen: o,
    });
}

fn empezar(config: Config) -> (Registro, Azar) {
    let n = config.jugadores.len();
    let mut r = Registro::nuevo(
        iniciar("p".into(), config, 5, 1_000).unwrap(),
        ModoDeshacer::SinLimite,
    );
    let mut azar = Azar::new(8);
    for j in 0..n as u8 {
        haz(
            &mut r,
            &mut azar,
            j,
            Accion::QuedarseCon { al_fondo: vec![] },
        )
        .unwrap();
    }
    (r, azar)
}

fn haz(r: &mut Registro, azar: &mut Azar, actor: u8, accion: Accion) -> Result<(), Rechazo> {
    r.procesar(
        &Intencion {
            actor: IdJugador(actor),
            accion,
        },
        azar,
        3_000,
    )
    .map(|_| ())
}

fn activo(r: &Registro) -> Vec<IdCarta> {
    r.partida()
        .zona(&RefZona::comun(ZonaComun::PlanoActivo))
        .to_vec()
}

fn nombre(r: &Registro, id: IdCarta) -> String {
    r.partida().cartas[&id].origen.nombre.clone()
}

#[test]
fn valores_de_fabrica_de_cada_formato() {
    let ninguno = Variantes::default();
    let o = |f, v: &Variantes, n| Opciones::de_fabrica(f, v, n);
    assert_eq!(o(Formato::Commander, &ninguno, 4).vida_inicial, 40);
    assert_eq!(o(Formato::Constructed, &ninguno, 2).vida_inicial, 20);
    let dos = Variantes {
        equipos: ModoEquipos::DosCabezas,
        ..Variantes::default()
    };
    assert_eq!(o(Formato::Constructed, &dos, 4).vida_inicial, 30);
    // Con 3 o más se roba el primero; en duelo, no.
    assert!(!o(Formato::Commander, &ninguno, 4).primero_no_roba);
    assert!(o(Formato::Constructed, &ninguno, 2).primero_no_roba);
    assert_eq!(VIDA_ARCHIENEMIGO, 40);
}

#[test]
fn el_archienemigo_empieza_con_40_y_los_demas_con_la_de_fabrica() {
    let variantes = Variantes {
        equipos: ModoEquipos::Equipos { fuego_amigo: false },
        archenemy: Some(IdJugador(0)),
        planechase: false,
    };
    let mut c = config(4);
    c.formato = Formato::Constructed;
    c.opciones = Opciones::de_fabrica(Formato::Constructed, &variantes, 4);
    c.variantes = variantes;
    let p = iniciar("p".into(), c, 1, 0).unwrap();
    let vida = |j: u8| p.jugador(IdJugador(j)).unwrap().vida;
    assert_eq!([vida(0), vida(1), vida(2), vida(3)], [40, 20, 20, 20]);
}

#[test]
fn contra_el_archienemigo_los_demas_son_un_equipo_sin_fuego_amigo() {
    let variantes = Variantes {
        equipos: ModoEquipos::Equipos { fuego_amigo: false },
        archenemy: Some(IdJugador(0)),
        planechase: false,
    };
    let mut c = config(4);
    c.opciones = Opciones::de_fabrica(Formato::Commander, &variantes, 4);
    c.variantes = variantes;
    c.equipos = vec![Equipo {
        id: IdEquipo(0),
        nombre: "Héroes".into(),
        color: "#3b82f6".into(),
        vida_compartida: None,
    }];
    for j in c.jugadores.iter_mut().skip(1) {
        j.equipo = Some(IdEquipo(0));
    }
    let p = iniciar("p".into(), c, 1, 0).unwrap();
    assert_eq!(
        p.oponentes(IdJugador(0)),
        vec![IdJugador(1), IdJugador(2), IdJugador(3)]
    );
    for j in 1..4u8 {
        assert_eq!(p.oponentes(IdJugador(j)), vec![IdJugador(0)]);
    }
}

#[test]
fn los_planes_van_al_mazo_del_archienemigo_y_se_ponen_en_marcha() {
    let mut c = config(3);
    c.variantes.archenemy = Some(IdJugador(0));
    for i in 0..3 {
        plano(&mut c, 0, &format!("Plan {i}"), "Scheme", Seccion::Plan);
    }
    let (mut r, mut azar) = empezar(c);
    let planes = RefZona::de(IdJugador(0), Zona::Planes);
    let en_marcha = RefZona::de(IdJugador(0), Zona::PlanesEnMarcha);
    assert_eq!(r.partida().zona(&planes).len(), 3);
    // Sin planes, nada que poner en marcha.
    assert_eq!(
        haz(&mut r, &mut azar, 1, Accion::PonerEnMarcha),
        Err(Rechazo::ZonaVacia)
    );
    haz(&mut r, &mut azar, 0, Accion::PonerEnMarcha).unwrap();
    assert_eq!(r.partida().zona(&planes).len(), 2);
    let plan = r.partida().zona(&en_marcha)[0];
    // Abandonar: vuelve al fondo del mazo de planes.
    haz(&mut r, &mut azar, 0, Accion::AbandonarPlan { carta: plan }).unwrap();
    assert!(r.partida().zona(&en_marcha).is_empty());
    assert_eq!(r.partida().zona(&planes).last(), Some(&plan));
}

#[test]
fn planechase_empieza_con_un_plano_y_planeswalk_rota_el_mazo() {
    let mut c = config(2);
    c.variantes.planechase = true;
    for i in 0..3 {
        plano(
            &mut c,
            (i % 2) as usize,
            &format!("Plano {i}"),
            "Plane — X",
            Seccion::Plano,
        );
    }
    let (mut r, mut azar) = empezar(c);
    let comun = RefZona::comun(ZonaComun::Planos);
    assert_eq!(activo(&r).len(), 1);
    assert_eq!(r.partida().zona(&comun).len(), 2);
    let primero = activo(&r)[0];
    let mut vistos = vec![nombre(&r, primero)];
    for _ in 0..2 {
        haz(&mut r, &mut azar, 0, Accion::Planeswalk).unwrap();
        assert_eq!(activo(&r).len(), 1);
        vistos.push(nombre(&r, activo(&r)[0]));
    }
    // Pasan los tres, y al cuarto salto vuelve el primero.
    vistos.sort();
    vistos.dedup();
    assert_eq!(vistos.len(), 3);
    haz(&mut r, &mut azar, 1, Accion::Planeswalk).unwrap();
    assert_eq!(activo(&r), vec![primero]);
}

#[test]
fn sin_planechase_los_planos_esperan_en_el_mazo_comun() {
    let mut c = config(2);
    for i in 0..2 {
        plano(
            &mut c,
            0,
            &format!("Plano {i}"),
            "Plane — X",
            Seccion::Plano,
        );
    }
    let (r, _) = empezar(c);
    assert!(activo(&r).is_empty());
    assert_eq!(
        r.partida().zona(&RefZona::comun(ZonaComun::Planos)).len(),
        2
    );
}

#[test]
fn planeswalk_sin_planos_se_rechaza() {
    let (mut r, mut azar) = empezar(config(2));
    assert_eq!(
        haz(&mut r, &mut azar, 0, Accion::Planeswalk),
        Err(Rechazo::ZonaVacia)
    );
}

#[test]
fn el_dado_planar_cuesta_una_mas_cada_vez_y_vuelve_a_cero_con_el_turno() {
    let (mut r, mut azar) = empezar(config(2));
    assert_eq!(r.partida().turno.tiradas_planar, 0);
    haz(&mut r, &mut azar, 0, Accion::DadoPlanar).unwrap();
    haz(&mut r, &mut azar, 0, Accion::DadoPlanar).unwrap();
    assert_eq!(r.partida().turno.tiradas_planar, 2);
    haz(&mut r, &mut azar, 0, Accion::PasarTurno).unwrap();
    assert_eq!(r.partida().turno.tiradas_planar, 0);
}

#[test]
fn las_seis_caras_del_dado_planar_son_una_de_planeswalk_una_de_caos_y_cuatro_vacias() {
    let mut cuentas = [0u32; 3];
    for v in 0..6 {
        match mesa::aplicar::cara_planar(v) {
            mesa::accion::CaraPlanar::Planeswalk => cuentas[0] += 1,
            mesa::accion::CaraPlanar::Caos => cuentas[1] += 1,
            mesa::accion::CaraPlanar::Nada => cuentas[2] += 1,
        }
    }
    assert_eq!(cuentas, [1, 1, 4]);
}

#[test]
fn el_dano_de_comandante_baja_la_vida_y_se_cuenta_por_comandante() {
    let (mut r, mut azar) = empezar(config(3));
    let yusri = |j: u8| r.partida().zona(&RefZona::de(IdJugador(j), Zona::Mando))[0];
    let (c0, c2) = (yusri(0), yusri(2));
    for _ in 0..3 {
        haz(
            &mut r,
            &mut azar,
            0,
            Accion::DanoComandante {
                jugador: IdJugador(1),
                comandante: c0,
                delta: 7,
            },
        )
        .unwrap();
    }
    haz(
        &mut r,
        &mut azar,
        2,
        Accion::DanoComandante {
            jugador: IdJugador(1),
            comandante: c2,
            delta: 4,
        },
    )
    .unwrap();
    let j1 = r.partida().jugador(IdJugador(1)).unwrap().clone();
    assert_eq!(j1.dano_comandante[&c0], 21);
    assert_eq!(j1.dano_comandante[&c2], 4);
    assert_eq!(j1.vida, 40 - 21 - 4);
    // Quitar exactamente lo que había devuelve la vida y borra la entrada. (Pedir
    // más de lo que hay devolvería vida de más; la pantalla nunca lo pide.)
    haz(
        &mut r,
        &mut azar,
        0,
        Accion::DanoComandante {
            jugador: IdJugador(1),
            comandante: c0,
            delta: -21,
        },
    )
    .unwrap();
    let j1 = r.partida().jugador(IdJugador(1)).unwrap();
    assert!(!j1.dano_comandante.contains_key(&c0));
    assert_eq!(j1.vida, 40 - 4);
}
