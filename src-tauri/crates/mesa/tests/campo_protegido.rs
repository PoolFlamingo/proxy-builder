//! `Opciones::campo_ajeno_protegido` (red): en el campo solo se toca lo que se
//! controla; lo de otro se coge con `TomarControl` y, ya suyo, no lo toca su
//! dueño. Cementerio, exilio y pila siguen abiertos. Apagado, todo igual.

mod comun;

use comun::*;
use mesa::accion::*;
use mesa::modelo::*;
use mesa::preparar::iniciar;
use mesa::{Azar, IdCarta, IdJugador, ModoDeshacer, Rechazo, Registro};

const YO: IdJugador = IdJugador(0);
const OTRO: IdJugador = IdJugador(1);

struct Banco {
    r: Registro,
    azar: Azar,
}

impl Banco {
    /// Dos jugadores con su mano, con la protección encendida o no.
    fn nuevo(protegido: bool) -> Self {
        let mut cfg = config(2);
        cfg.opciones.campo_ajeno_protegido = protegido;
        let mut r = Registro::nuevo(
            iniciar("p".into(), cfg, 1234, 1_000).unwrap(),
            ModoDeshacer::SinLimite,
        );
        let mut azar = Azar::new(99);
        for j in [YO, OTRO] {
            r.procesar(
                &Intencion {
                    actor: j,
                    accion: Accion::QuedarseCon { al_fondo: vec![] },
                },
                &mut azar,
                2_000,
            )
            .unwrap();
        }
        Banco { r, azar }
    }

    fn como(&mut self, actor: IdJugador, accion: Accion) -> Result<(), Rechazo> {
        let t = 10_000 + self.r.partida().seq;
        self.r
            .procesar(&Intencion { actor, accion }, &mut self.azar, t)
            .map(|_| ())
    }

    /// Una carta de la mano de `j` al campo de `j`.
    fn al_campo(&mut self, j: IdJugador) -> IdCarta {
        let id = self.r.partida().zona(&RefZona::de(j, Zona::Mano))[0];
        self.mover(j, id, Zona::Campo).unwrap();
        id
    }

    fn mover(&mut self, actor: IdJugador, id: IdCarta, a: Zona) -> Result<(), Rechazo> {
        self.como(
            actor,
            Accion::Mover {
                cartas: vec![id],
                a: Destino {
                    zona: ZonaDestino::De(a),
                    jugador: None,
                    sitio: Sitio::Arriba,
                },
                como: None,
            },
        )
    }

    fn girar(&mut self, actor: IdJugador, id: IdCarta) -> Result<(), Rechazo> {
        self.como(
            actor,
            Accion::Girar {
                cartas: vec![id],
                girada: true,
            },
        )
    }

    fn controla(&self, id: IdCarta) -> IdJugador {
        self.r.partida().cartas[&id].controlador
    }
}

#[test]
fn el_campo_de_otro_se_cierra_en_red() {
    let mut b = Banco::nuevo(true);
    let suya = b.al_campo(OTRO);
    assert_eq!(b.girar(YO, suya), Err(Rechazo::SinPermiso));
    assert_eq!(
        b.mover(YO, suya, Zona::Cementerio),
        Err(Rechazo::SinPermiso)
    );
    // Lo suyo, sí.
    b.girar(OTRO, suya).unwrap();
    // Y lo mío, también.
    let mia = b.al_campo(YO);
    b.girar(YO, mia).unwrap();
}

#[test]
fn tomar_el_control_abre_la_carta_y_cierra_a_su_dueno() {
    let mut b = Banco::nuevo(true);
    let suya = b.al_campo(OTRO);
    b.como(
        YO,
        Accion::TomarControl {
            cartas: vec![suya],
            hasta_fin_de_turno: false,
        },
    )
    .unwrap();
    assert_eq!(b.controla(suya), YO);
    // Ya es mía: la toco yo y su dueño no.
    b.girar(YO, suya).unwrap();
    assert_eq!(b.girar(OTRO, suya), Err(Rechazo::SinPermiso));
    // Para recuperarla, el dueño la toma de vuelta (queda en el registro).
    b.como(
        OTRO,
        Accion::TomarControl {
            cartas: vec![suya],
            hasta_fin_de_turno: false,
        },
    )
    .unwrap();
    assert_eq!(b.controla(suya), OTRO);
}

#[test]
fn cementerio_y_exilio_siguen_abiertos() {
    let mut b = Banco::nuevo(true);
    let suya = b.al_campo(OTRO);
    b.mover(OTRO, suya, Zona::Cementerio).unwrap();
    // Reanimarla desde su cementerio, a mi campo.
    b.mover(YO, suya, Zona::Campo).unwrap();
    assert_eq!(b.controla(suya), YO);
    // Y exiliar la de su cementerio.
    let otra = b.al_campo(OTRO);
    b.mover(OTRO, otra, Zona::Cementerio).unwrap();
    b.mover(YO, otra, Zona::Exilio).unwrap();
}

#[test]
fn apagado_todo_sigue_como_antes() {
    let mut b = Banco::nuevo(false);
    let suya = b.al_campo(OTRO);
    b.girar(YO, suya).unwrap();
    b.mover(YO, suya, Zona::Cementerio).unwrap();
}

#[test]
fn una_partida_guardada_sin_la_opcion_la_trae_apagada() {
    let mut v = serde_json::to_value(config(2).opciones).unwrap();
    v.as_object_mut().unwrap().remove("campoAjenoProtegido");
    let o: mesa::formato::Opciones = serde_json::from_value(v).unwrap();
    assert!(!o.campo_ajeno_protegido);
}
