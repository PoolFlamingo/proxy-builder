//! Los eventos guardados con versiones anteriores del catálogo se siguen
//! leyendo (A4): las repeticiones viejas y las partidas guardadas dependen
//! de ello. `catalogo/v<N>.json` se escribe una vez con su versión
//! (`PB_CATALOGO_ESCRIBIR=1 cargo test -p mesa --test catalogo`) y no se
//! toca nunca más.

mod comun;

use comun::*;
use mesa::accion::{Accion, Evento, Intencion};
use mesa::{Azar, IdJugador, ModoDeshacer, VERSION_CATALOGO};

fn eventos_de_ejemplo() -> Vec<Evento> {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let mut gen = Azar::new(77);
    let mut t = 0;
    while r.eventos.len() < 120 {
        t += 1;
        let i = intencion_al_azar(r.partida(), &mut gen, 2);
        let _ = r.procesar(&i, &mut azar, t);
    }
    r.procesar(
        &Intencion {
            actor: IdJugador(0),
            accion: Accion::Chat { texto: "gg".into() },
        },
        &mut azar,
        t,
    )
    .unwrap();
    // Sin las restauraciones (llevan la partida entera).
    let mut eventos: Vec<Evento> = r
        .eventos
        .into_iter()
        .filter(|e| !matches!(e.cambio, mesa::Cambio::Restaurar { .. }))
        .collect();
    // Lo que añadió cada versión, a mano, por si el azar no lo generó (v2: `TomarControl`).
    let ultimo = eventos.last().cloned().unwrap();
    eventos.push(Evento {
        seq: ultimo.seq + 1,
        cambio: mesa::Cambio::Accion {
            accion: Accion::TomarControl {
                cartas: vec![mesa::IdCarta(1)],
                hasta_fin_de_turno: true,
            },
            azar: None,
        },
        ..ultimo
    });
    // v3: las notas.
    for (k, accion) in [
        Accion::PonerNota {
            texto: "2 de maná rojo".into(),
            temporal: true,
            privada: true,
        },
        Accion::EditarNota {
            id: 1,
            texto: "3 de maná".into(),
            temporal: false,
            privada: false,
        },
        Accion::QuitarNota { id: 1 },
    ]
    .into_iter()
    .enumerate()
    {
        eventos.push(Evento {
            seq: ultimo.seq + 2 + k as u64,
            cambio: mesa::Cambio::Accion { accion, azar: None },
            ..ultimo.clone()
        });
    }
    eventos
}

#[test]
fn los_eventos_de_cada_version_se_siguen_leyendo() {
    let dir = format!("{}/tests/catalogo", env!("CARGO_MANIFEST_DIR"));
    if std::env::var("PB_CATALOGO_ESCRIBIR").is_ok() {
        let j = serde_json::to_string_pretty(&eventos_de_ejemplo()).unwrap();
        std::fs::write(format!("{dir}/v{VERSION_CATALOGO}.json"), j).unwrap();
    }
    for v in 1..=VERSION_CATALOGO {
        let txt = std::fs::read_to_string(format!("{dir}/v{v}.json"))
            .unwrap_or_else(|_| panic!("falta catalogo/v{v}.json"));
        let evs: Vec<Evento> = serde_json::from_str(&txt).unwrap();
        assert!(!evs.is_empty());
    }
}
