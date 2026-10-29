//! Las notas de los jugadores: quién las ve, quién las toca, cuándo se
//! borran y que deshacer no se las lleva.

mod comun;

use comun::*;
use mesa::accion::{Accion, Intencion};
use mesa::{parche, vista, Azar, IdJugador, Mirador, ModoDeshacer, Rechazo, Registro};

const A: IdJugador = IdJugador(0);
const B: IdJugador = IdJugador(1);

fn pide(
    r: &mut Registro,
    azar: &mut Azar,
    actor: IdJugador,
    accion: Accion,
) -> Result<(), Rechazo> {
    let t = 10_000 + r.partida().seq;
    r.procesar(&Intencion { actor, accion }, azar, t)
        .map(|_| ())
}

fn poner(texto: &str, temporal: bool, privada: bool) -> Accion {
    Accion::PonerNota {
        texto: texto.into(),
        temporal,
        privada,
    }
}

fn textos(r: &Registro, m: Mirador) -> Vec<String> {
    vista(r.partida(), m)
        .general
        .notas
        .into_iter()
        .map(|n| n.texto)
        .collect()
}

#[test]
fn las_privadas_solo_las_ve_su_autor() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    pide(&mut r, &mut azar, A, poner("2 de maná rojo", true, true)).unwrap();
    pide(
        &mut r,
        &mut azar,
        A,
        poner("Tesoro sacrificado", false, false),
    )
    .unwrap();
    assert_eq!(
        textos(&r, Mirador::Jugador(A)),
        vec!["2 de maná rojo", "Tesoro sacrificado"]
    );
    assert_eq!(textos(&r, Mirador::Jugador(B)), vec!["Tesoro sacrificado"]);
    assert_eq!(textos(&r, Mirador::Espectador), vec!["Tesoro sacrificado"]);
    // Ni la repetición con todo a la vista enseña lo privado.
    assert_eq!(textos(&r, Mirador::Todo), vec!["Tesoro sacrificado"]);
}

#[test]
fn el_parche_de_otro_no_lleva_la_privada() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let h = r
        .procesar(
            &Intencion {
                actor: A,
                accion: poner("secreto", false, true),
            },
            &mut azar,
            5_000,
        )
        .unwrap();
    let del_otro = parche(r.partida(), &h.tocado, Mirador::Jugador(B));
    let notas = del_otro.general.map(|g| g.notas).unwrap_or_default();
    assert!(notas.is_empty());
    let propio = parche(r.partida(), &h.tocado, Mirador::Jugador(A));
    assert_eq!(propio.general.unwrap().notas.len(), 1);
}

#[test]
fn cada_uno_toca_solo_las_suyas() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    pide(&mut r, &mut azar, A, poner("mía", false, false)).unwrap();
    let id = r.partida().notas[0].id;
    let editar = |texto: &str| Accion::EditarNota {
        id,
        texto: texto.into(),
        temporal: false,
        privada: false,
    };
    assert_eq!(
        pide(&mut r, &mut azar, B, editar("de B")),
        Err(Rechazo::SinPermiso)
    );
    assert_eq!(
        pide(&mut r, &mut azar, B, Accion::QuitarNota { id }),
        Err(Rechazo::SinPermiso)
    );
    pide(&mut r, &mut azar, A, editar("  cambiada  ")).unwrap();
    assert_eq!(r.partida().notas[0].texto, "cambiada");
    pide(&mut r, &mut azar, A, Accion::QuitarNota { id }).unwrap();
    assert!(r.partida().notas.is_empty());
    assert_eq!(
        pide(&mut r, &mut azar, A, Accion::QuitarNota { id }),
        Err(Rechazo::NoExiste)
    );
}

#[test]
fn una_nota_se_puede_hacer_privada_o_temporal_despues() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    pide(&mut r, &mut azar, A, poner("x", false, false)).unwrap();
    let id = r.partida().notas[0].id;
    pide(
        &mut r,
        &mut azar,
        A,
        Accion::EditarNota {
            id,
            texto: "x".into(),
            temporal: true,
            privada: true,
        },
    )
    .unwrap();
    assert!(textos(&r, Mirador::Jugador(B)).is_empty());
    assert!(r.partida().notas[0].temporal);
}

#[test]
fn los_limites() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    assert_eq!(
        pide(&mut r, &mut azar, A, poner("   ", false, false)),
        Err(Rechazo::Limite)
    );
    let larga = "a".repeat(201);
    assert_eq!(
        pide(&mut r, &mut azar, A, poner(&larga, false, false)),
        Err(Rechazo::Limite)
    );
    for i in 0..30 {
        pide(&mut r, &mut azar, A, poner(&format!("n{i}"), false, false)).unwrap();
    }
    assert_eq!(
        pide(&mut r, &mut azar, A, poner("una más", false, false)),
        Err(Rechazo::Limite)
    );
    // El tope es de cada jugador.
    pide(&mut r, &mut azar, B, poner("de B", false, false)).unwrap();
}

#[test]
fn las_temporales_se_borran_al_pasar_el_turno() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    pide(&mut r, &mut azar, A, poner("hasta el final", true, false)).unwrap();
    pide(&mut r, &mut azar, B, poner("temporal privada", true, true)).unwrap();
    pide(&mut r, &mut azar, A, poner("fija", false, false)).unwrap();
    let activo = r.partida().turno.activo;
    pide(&mut r, &mut azar, activo, Accion::PasarTurno).unwrap();
    let quedan: Vec<_> = r.partida().notas.iter().map(|n| n.texto.as_str()).collect();
    assert_eq!(quedan, vec!["fija"]);
}

#[test]
fn deshacer_no_se_lleva_las_notas() {
    for modo in [ModoDeshacer::SinLimite, ModoDeshacer::UltimaPropia] {
        let (mut r, mut azar) = empezada(2, modo);
        pide(
            &mut r,
            &mut azar,
            A,
            Accion::Vida {
                jugador: A,
                delta: -3,
            },
        )
        .unwrap();
        pide(&mut r, &mut azar, A, poner("tras la vida", false, false)).unwrap();
        // Una nota no cuenta como «la última acción»: se deshace la vida.
        pide(&mut r, &mut azar, A, Accion::Deshacer { hasta: None }).unwrap();
        assert_eq!(r.partida().jugador(A).unwrap().vida, 40, "{modo:?}");
        assert_eq!(r.partida().notas.len(), 1, "{modo:?}");
        // Quitar una nota tampoco se deshace: no vuelve con otro deshacer.
        let id = r.partida().notas[0].id;
        pide(&mut r, &mut azar, A, Accion::QuitarNota { id }).unwrap();
        let _ = pide(&mut r, &mut azar, A, Accion::Deshacer { hasta: None });
        assert!(r.partida().notas.is_empty(), "{modo:?}");
    }
}

#[test]
fn sin_notas_la_huella_no_cambia() {
    let (r, _) = empezada(2, ModoDeshacer::SinLimite);
    let json = serde_json::to_string(r.partida()).unwrap();
    assert!(!json.contains("\"notas\""));
}
