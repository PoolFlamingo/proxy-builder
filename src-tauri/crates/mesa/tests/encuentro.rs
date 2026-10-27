//! Encuentros (G6): el marcador y el cambio de cartas con el banquillo.

mod comun;

use comun::*;
use mesa::encuentro::{cambiar, Encuentro, ErrorEncuentro};
use mesa::modelo::{Partida, RefZona, Zona};
use mesa::preparar::{iniciar, Seccion};
use mesa::IdJugador;

const A: IdJugador = IdJugador(0);
const B: IdJugador = IdJugador(1);

fn en_biblioteca(p: &Partida, j: IdJugador) -> Vec<String> {
    p.zona(&RefZona::de(j, Zona::Biblioteca))
        .iter()
        .map(|c| p.cartas[c].origen.carta_deck.clone())
        .collect()
}

fn cuantas(p: &Partida, j: IdJugador) -> usize {
    p.zona(&RefZona::de(j, Zona::Biblioteca)).len() + p.zona(&RefZona::de(j, Zona::Mano)).len()
}

fn en_partida(p: &Partida, j: IdJugador) -> Vec<String> {
    let mut v = en_biblioteca(p, j);
    v.extend(
        p.zona(&RefZona::de(j, Zona::Mano))
            .iter()
            .map(|c| p.cartas[c].origen.carta_deck.clone()),
    );
    v
}

#[test]
fn solo_se_ofrecen_1_3_y_5() {
    for ok in [1u8, 3, 5] {
        assert!(Encuentro::nuevo(ok).is_ok());
    }
    for mal in [0u8, 2, 4, 7] {
        assert_eq!(Encuentro::nuevo(mal), Err(ErrorEncuentro::Duracion));
    }
}

#[test]
fn al_mejor_de_tres_gana_quien_llega_a_dos() {
    let mut e = Encuentro::nuevo(3).unwrap();
    assert_eq!(e.necesarias(), 2);
    // Primera partida en curso: aún sin terminar.
    let s = e.estado(2, None);
    assert_eq!(
        (s.marcador.clone(), s.jugadas, s.terminado),
        (vec![0, 0], 0, false)
    );
    // Termina: la cuenta ya cuenta la de ahora, sin cerrarla.
    let s = e.estado(2, Some(&[A]));
    assert_eq!((s.marcador, s.jugadas, s.terminado), (vec![1, 0], 1, false));
    e.cerrar(vec![A]);
    e.cerrar(vec![B]);
    let s = e.estado(2, None);
    assert_eq!(s.marcador, vec![1, 1]);
    assert!(!s.terminado);
    // La tercera, para B: 1–2.
    let s = e.estado(2, Some(&[B]));
    assert_eq!(s.marcador, vec![1, 2]);
    assert!(s.terminado);
    assert_eq!(s.ganadores, vec![B]);
}

#[test]
fn a_una_sola_partida_se_acaba_al_terminar_la_primera() {
    let e = Encuentro::nuevo(1).unwrap();
    assert!(!e.estado(2, None).terminado);
    assert!(e.estado(2, Some(&[A])).terminado);
}

#[test]
fn con_tablas_se_acaba_al_agotar_las_partidas() {
    let mut e = Encuentro::nuevo(3).unwrap();
    e.cerrar(vec![]);
    e.cerrar(vec![]);
    let s = e.estado(2, Some(&[]));
    assert_eq!(s.marcador, vec![0, 0]);
    assert!(s.terminado);
    assert!(s.ganadores.is_empty());
}

#[test]
fn un_equipo_gana_junto() {
    let mut e = Encuentro::nuevo(3).unwrap();
    e.cerrar(vec![IdJugador(0), IdJugador(1)]);
    let s = e.estado(4, Some(&[IdJugador(0), IdJugador(1)]));
    assert_eq!(s.marcador, vec![2, 2, 0, 0]);
    assert_eq!(s.ganadores, vec![IdJugador(0), IdJugador(1)]);
}

#[test]
fn empieza_la_siguiente_quien_perdio() {
    assert_eq!(Encuentro::primero_siguiente(2, &[A]), Some(B));
    assert_eq!(Encuentro::primero_siguiente(2, &[B]), Some(A));
    // Un equipo ganador deja empezar al primero de los que perdieron.
    assert_eq!(
        Encuentro::primero_siguiente(4, &[IdJugador(0), IdJugador(1)]),
        Some(IdJugador(2))
    );
    // Tablas: al azar.
    assert_eq!(Encuentro::primero_siguiente(2, &[]), None);
}

#[test]
fn cambiar_dos_cartas_las_deja_en_la_biblioteca_de_la_partida_siguiente() {
    let mut d = deck("D0");
    let antes: usize = d
        .cartas
        .iter()
        .filter(|c| c.seccion == Seccion::Mazo)
        .count();
    // Entran dos del banquillo (una añadida a la de siempre) y salen dos tierras.
    let mut extra = d
        .cartas
        .iter()
        .find(|c| c.seccion == Seccion::Banquillo)
        .unwrap()
        .clone();
    extra.carta_deck = "deck-Pyroblast 2".into();
    extra.origen.carta_deck = "deck-Pyroblast 2".into();
    d.cartas.push(extra);
    let sale = vec!["deck-Forest 0".to_string(), "deck-Forest 1".to_string()];
    let entra = vec!["deck-Pyroblast".to_string(), "deck-Pyroblast 2".to_string()];
    cambiar(&mut d, &sale, &entra).unwrap();

    let mut cfg = config(2);
    cfg.formato = mesa::formato::Formato::Constructed;
    cfg.jugadores[0].mazo = d;
    let p = iniciar("p".into(), cfg, 7, 0).unwrap();
    let cartas = en_partida(&p, A);
    for id in &entra {
        assert!(cartas.contains(id), "{id} debería estar en el mazo");
    }
    for id in &sale {
        assert!(!cartas.contains(id), "{id} debería estar fuera");
    }
    // El resto sigue donde estaba y el tamaño es el mismo.
    assert!(cartas.contains(&"deck-Forest 2".to_string()));
    assert_eq!(cuantas(&p, A), antes);
}

#[test]
fn cambiar_valida_antes_de_tocar_nada() {
    let mut d = deck("D0");
    let original = d.clone();
    let uno = |s: &str| vec![s.to_string()];
    assert_eq!(
        cambiar(&mut d, &uno("deck-Forest 0"), &[]),
        Err(ErrorEncuentro::Tamano)
    );
    assert_eq!(
        cambiar(&mut d, &uno("deck-Pyroblast"), &uno("deck-Pyroblast")),
        Err(ErrorEncuentro::Repetida("deck-Pyroblast".into()))
    );
    // Lo que sale tiene que estar en el mazo (no en el banquillo, ni ser el comandante).
    assert!(matches!(
        cambiar(&mut d, &uno("deck-Pyroblast"), &uno("deck-Forest 0")),
        Err(ErrorEncuentro::NoEstaEnElMazo(_))
    ));
    assert!(matches!(
        cambiar(&mut d, &uno("deck-Yusri"), &uno("deck-Pyroblast")),
        Err(ErrorEncuentro::NoEstaEnElMazo(_))
    ));
    // Lo que entra tiene que estar en el banquillo.
    assert!(matches!(
        cambiar(&mut d, &uno("deck-Forest 0"), &uno("deck-Forest 1")),
        Err(ErrorEncuentro::NoEstaEnElBanquillo(_))
    ));
    assert!(matches!(
        cambiar(&mut d, &uno("deck-Forest 0"), &uno("no-existe")),
        Err(ErrorEncuentro::NoEstaEnElBanquillo(_))
    ));
    // Y si una de dos falla, no se ha movido ninguna.
    assert!(cambiar(
        &mut d,
        &["deck-Forest 0".to_string(), "deck-Forest 1".to_string()],
        &["deck-Pyroblast".to_string(), "no-existe".to_string()],
    )
    .is_err());
    assert_eq!(d, original);
}
