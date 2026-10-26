//! La vista y los parches de una partida de ejemplo, para el test del store
//! de la pantalla (`src/stores/__tests__/mesa-store.test.ts`): aplicar los
//! parches sobre la primera vista tiene que dar la última. Se reescribe con
//! `PB_FIXTURES=1 cargo test -p mesa --test fixtures`.

mod comun;

use comun::*;
use mesa::vista::{parche, vista};
use mesa::{Azar, IdJugador, Mirador, ModoDeshacer};

#[test]
fn vista_y_parches_de_ejemplo() {
    let (mut r, mut azar) = empezada(2, ModoDeshacer::SinLimite);
    let yo = Mirador::Jugador(IdJugador(0));
    let inicial = vista(r.partida(), yo);
    let mut gen = Azar::new(21);
    let mut parches = Vec::new();
    let mut t = 0;
    while parches.len() < 60 {
        t += 1;
        let i = intencion_al_azar(r.partida(), &mut gen, 2);
        if let Ok(h) = r.procesar(&i, &mut azar, t) {
            if matches!(h.evento.cambio, mesa::Cambio::Restaurar { .. }) {
                continue;
            }
            parches.push(parche(r.partida(), &h.tocado, yo));
        }
    }
    let fin = vista(r.partida(), yo);
    let mut v = inicial.clone();
    for p in &parches {
        mesa::vista::aplicar_parche(&mut v, p);
    }
    assert_eq!(v, fin);
    if std::env::var("PB_FIXTURES").is_ok() {
        let j = serde_json::json!({ "inicial": inicial, "parches": parches, "final": fin });
        let ruta = format!(
            "{}/../../../src/stores/__tests__/fixtures/mesa-parches.json",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::write(ruta, serde_json::to_string(&j).unwrap()).unwrap();
    }
}
