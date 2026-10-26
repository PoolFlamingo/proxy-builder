//! El sparring con partidas simuladas: un guion de jugadas (una tierra por
//! ronda, los hechizos que dé el maná, pasar el turno) y el fantasma
//! reaccionando con cada intensidad.

mod comun;

use comun::origen;
use mesa::accion::{ComoMover, Destino, Sitio, ZonaDestino};
use mesa::formato::{Formato, Opciones, Variantes};
use mesa::modelo::{InfoCarta, Partida, Pos, RefZona, Zona, ZonaComun};
use mesa::preparar::{iniciar, CartaDeMesa, Config, DeckDeMesa, JugadorNuevo, Seccion};
use mesa::sparring::amenaza::{protegida, tipos};
use mesa::sparring::cartas::por_nombre;
use mesa::sparring::{reaccionar, Decision, Intensidad, ReaccionSparring, TipoReaccion};
use mesa::{Accion, Azar, Cambio, IdCarta, IdJugador, Intencion, ModoDeshacer, Registro};

const J: IdJugador = IdJugador(0);
const RONDAS: u32 = 10;
const PARTIDAS: u64 = 500;

fn carta(nombre: &str, tipo: &str, cmc: u16, extra: impl Fn(&mut InfoCarta)) -> CartaDeMesa {
    let mut o = origen(nombre, tipo, cmc);
    extra(&mut o.info);
    CartaDeMesa {
        carta_deck: format!("deck-{nombre}"),
        seccion: Seccion::Mazo,
        origen: o,
    }
}

/// Un deck de Commander de 99 + comandante, con curva y de todo un poco:
/// rocas y elfos (que no se quitan), una criatura con antimaleficio y otra
/// indestructible.
fn deck() -> DeckDeMesa {
    let mut c = Vec::new();
    for i in 0..37 {
        c.push(carta(
            &format!("Island {i}"),
            "Basic Land — Island",
            0,
            |_| {},
        ));
    }
    for (k, (cmc, n)) in [(1u16, 4), (2, 8), (3, 7), (4, 5), (5, 4), (6, 2)]
        .into_iter()
        .enumerate()
    {
        for i in 0..n {
            c.push(carta(
                &format!("Criatura {k}-{i}"),
                "Creature — Beast",
                cmc,
                |inf| {
                    if i % 3 == 0 {
                        inf.funciones = vec!["draw".into()];
                    }
                },
            ));
        }
    }
    for i in 0..3 {
        c.push(carta(
            &format!("Elfo {i}"),
            "Creature — Elf Druid",
            1,
            |inf| inf.funciones = vec!["ramp".into()],
        ));
        c.push(carta(&format!("Roca {i}"), "Artifact", 2, |inf| {
            inf.funciones = vec!["ramp".into()]
        }));
    }
    for i in 0..4 {
        c.push(carta(&format!("Artefacto {i}"), "Artifact", 2 + i, |_| {}));
        c.push(carta(
            &format!("Encantamiento {i}"),
            "Enchantment",
            2 + i,
            |_| {},
        ));
    }
    for i in 0..10 {
        c.push(carta(
            &format!("Instantáneo {i}"),
            "Instant",
            1 + i % 3,
            |inf| inf.funciones = vec![if i % 2 == 0 { "removal" } else { "draw" }.into()],
        ));
    }
    for i in 0..8 {
        c.push(carta(
            &format!("Conjuro {i}"),
            "Sorcery",
            2 + i % 4,
            |inf| {
                if i % 4 == 0 {
                    inf.funciones = vec!["tutor".into()]
                }
            },
        ));
    }
    c.push(carta(
        "Planeswalker",
        "Legendary Planeswalker — Jace",
        4,
        |_| {},
    ));
    c.push(carta("Invisible", "Creature — Faerie", 3, |inf| {
        inf.palabras_clave = vec!["Hexproof".into()]
    }));
    c.push(carta("Eterno", "Creature — God", 4, |inf| {
        inf.palabras_clave = vec!["Indestructible".into()]
    }));
    let mut jefe = carta("Jefe", "Legendary Creature — Human", 4, |inf| {
        inf.es_comandante = true;
        inf.funciones = vec!["draw".into()];
    });
    jefe.seccion = Seccion::Comandante;
    c.push(jefe);
    DeckDeMesa {
        nombre: "Prueba".into(),
        cartas: c,
        tapete: None,
        dorso: mesa::modelo::RefImagen::Generica,
    }
}

fn partida(semilla: u64) -> Partida {
    let variantes = Variantes::default();
    let config = Config {
        formato: Formato::Commander,
        opciones: Opciones::de_fabrica(Formato::Commander, &variantes, 1),
        variantes,
        jugadores: vec![JugadorNuevo {
            nombre: "Yo".into(),
            color: "#e11d48".into(),
            equipo: None,
            mazo: deck(),
        }],
        equipos: Vec::new(),
        primero: Some(J),
    };
    iniciar("p".into(), config, semilla, 0).unwrap()
}

fn destino(zona: ZonaDestino) -> Destino {
    Destino {
        zona,
        jugador: Some(J),
        sitio: match zona {
            ZonaDestino::De(Zona::Campo) => Sitio::EnCampo(Pos::nueva(5_000, 5_000)),
            _ => Sitio::Arriba,
        },
    }
}

fn mover(id: IdCarta, z: ZonaDestino, como: Option<ComoMover>) -> Accion {
    Accion::Mover {
        cartas: vec![id],
        a: destino(z),
        como,
    }
}

const PILA: ZonaDestino = ZonaDestino::Comun(ZonaComun::Pila);
const CAMPO: ZonaDestino = ZonaDestino::De(Zona::Campo);
const CEMENTERIO: ZonaDestino = ZonaDestino::De(Zona::Cementerio);

struct Juego {
    r: Registro,
    azar: Azar,
    fantasma: Azar,
    reacciones: Vec<ReaccionSparring>,
}

impl Juego {
    fn nuevo(semilla: u64, intensidad: Intensidad) -> Self {
        let mut j = Juego {
            r: Registro::nuevo(partida(semilla), ModoDeshacer::SinLimite),
            azar: Azar::new(semilla ^ 0xa5a5),
            fantasma: Azar::new(semilla.wrapping_mul(31).wrapping_add(7)),
            reacciones: Vec::new(),
        };
        j.hacer(Accion::QuedarseCon { al_fondo: vec![] });
        j.hacer(Accion::IntensidadSparring { intensidad });
        j
    }

    fn p(&self) -> &Partida {
        self.r.partida()
    }

    /// Hace una jugada y, si el fantasma reacciona, la resuelve.
    fn hacer(&mut self, accion: Accion) {
        let antes = self.p().clone();
        let Ok(h) = self
            .r
            .procesar(&Intencion { actor: J, accion }, &mut self.azar, 0)
        else {
            return;
        };
        let Some(re) = reaccionar(&antes, self.r.partida(), &h.evento, &mut self.fantasma) else {
            return;
        };
        comprobar(self.p(), &re);
        self.r
            .de_la_mesa(
                Cambio::Sparring {
                    reaccion: re.clone(),
                },
                0,
            )
            .unwrap();
        let eleccion = (re.tipo == TipoReaccion::Sacrificio).then(|| {
            self.p()
                .zona(&RefZona::de(J, Zona::Campo))
                .iter()
                .copied()
                .find(|c| tipos(&self.p().cartas[c].origen.info).criatura)
                .unwrap()
        });
        self.r
            .procesar(
                &Intencion {
                    actor: J,
                    accion: Accion::DecidirSparring {
                        reaccion: re.id,
                        decision: Decision::Aceptar,
                        eleccion,
                    },
                },
                &mut self.azar,
                0,
            )
            .unwrap();
        self.reacciones.push(re);
    }

    fn en(&self, z: Zona) -> Vec<IdCarta> {
        self.p().zona(&RefZona::de(J, z)).to_vec()
    }

    fn info(&self, id: IdCarta) -> &InfoCarta {
        &self.p().cartas[&id].origen.info
    }

    /// Lanza una carta y, si sigue en la pila, la resuelve.
    fn lanzar(&mut self, id: IdCarta, como: Option<ComoMover>) {
        self.hacer(mover(id, PILA, como));
        let pila = RefZona::comun(ZonaComun::Pila);
        if !self.p().zona(&pila).contains(&id) {
            return;
        }
        let t = tipos(self.info(id));
        self.hacer(mover(
            id,
            if t.instantaneo || t.conjuro {
                CEMENTERIO
            } else {
                CAMPO
            },
            None,
        ));
    }

    /// Una ronda del guion.
    fn ronda(&mut self) {
        let ronda = self.p().turno.ronda;
        if let Some(t) = self
            .en(Zona::Mano)
            .into_iter()
            .find(|c| self.info(*c).es_tierra)
        {
            self.hacer(mover(t, CAMPO, None));
        }
        let mut mana = u32::try_from(
            self.en(Zona::Campo)
                .iter()
                .filter(|c| self.info(**c).es_tierra)
                .count(),
        )
        .unwrap();
        if ronda >= 4 && mana >= 4 {
            if let Some(&jefe) = self.en(Zona::Mando).first() {
                self.lanzar(jefe, Some(ComoMover::Lanzar));
                mana -= 4;
            }
        }
        let mut mano = self.en(Zona::Mano);
        mano.sort_by_key(|c| std::cmp::Reverse(self.info(*c).cmc()));
        for c in mano {
            let cmc = u32::from(self.info(c).cmc());
            if !self.info(c).es_tierra && cmc <= mana {
                mana -= cmc;
                self.lanzar(c, None);
            }
        }
        self.hacer(Accion::PasarTurno);
    }
}

/// Los límites del fantasma, en cada reacción.
fn comprobar(p: &Partida, re: &ReaccionSparring) {
    let f = por_nombre(&re.carta.nombre).expect("una carta de la lista");
    assert_eq!(f.tipo, re.tipo);
    assert!(
        f.se_puede_pagar(p.turno.ronda),
        "{} en la ronda {}",
        f.nombre,
        p.turno.ronda
    );
    let campo = RefZona::de(J, Zona::Campo);
    let pila = RefZona::comun(ZonaComun::Pila);
    for id in &re.objetivo {
        let c = &p.cartas[id];
        let info = &c.origen.info;
        assert!(!tipos(info).tierra, "nunca a una tierra");
        let (z, _) = p.donde(*id).expect("está en la mesa");
        match re.tipo {
            TipoReaccion::Contrarrestar => assert!(z == pila || z == campo),
            _ => assert_eq!(z, campo, "lo que quita está en el campo"),
        }
        if matches!(re.tipo, TipoReaccion::Eliminar | TipoReaccion::Devolver) {
            assert!(
                !protegida(info, Some(c)) || f.no_apunta,
                "respeta el antimaleficio"
            );
            assert!(f.vale_para(info));
        }
    }
    if re.tipo == TipoReaccion::Sacrificio {
        assert!(re.objetivo.is_empty());
    }
}

fn jugar(semilla: u64, intensidad: Intensidad) -> Vec<ReaccionSparring> {
    let mut j = Juego::nuevo(semilla, intensidad);
    for _ in 0..RONDAS {
        j.ronda();
    }
    j.reacciones
}

struct Resumen {
    media: f64,
    reparto: [f64; 5],
}

fn resumen(intensidad: Intensidad) -> Resumen {
    let mut n = [0u32; 5];
    let mut total = 0u32;
    for s in 0..PARTIDAS {
        for re in jugar(1_000 + s, intensidad) {
            let i = match re.tipo {
                TipoReaccion::Eliminar => 0,
                TipoReaccion::Contrarrestar => 1,
                TipoReaccion::Barrer => 2,
                TipoReaccion::Devolver => 3,
                TipoReaccion::Sacrificio => 4,
            };
            n[i] += 1;
            total += 1;
        }
    }
    let t = f64::from(total.max(1));
    Resumen {
        media: f64::from(total) / PARTIDAS as f64,
        reparto: n.map(|x| f64::from(x) * 100.0 / t),
    }
}

/// Las bandas de reacciones por partida (10 rondas) de cada intensidad.
const BANDAS: [(Intensidad, f64, f64); 3] = [
    (Intensidad::Suave, 1.0, 3.0),
    (Intensidad::Realista, 2.5, 5.0),
    (Intensidad::Despiadado, 4.5, 8.5),
];
const PLAYGROUP: [f64; 5] = [58.0, 19.0, 13.0, 7.0, 3.0];

#[test]
fn cada_intensidad_cae_en_su_banda_y_con_el_reparto_de_playgroup() {
    let todos: Vec<_> = BANDAS.iter().map(|b| (b, resumen(b.0))).collect();
    for ((intensidad, _, _), r) in &todos {
        eprintln!(
            "{intensidad:?}: {:.2} por partida, reparto {:?}",
            r.media,
            r.reparto.map(|x| (x * 10.0).round() / 10.0)
        );
    }
    for (&(intensidad, min, max), r) in todos {
        assert!(
            (min..=max).contains(&r.media),
            "{intensidad:?}: {:.2} reacciones por partida",
            r.media
        );
        for (k, (real, objetivo)) in r.reparto.iter().zip(PLAYGROUP).enumerate() {
            assert!(
                (real - objetivo).abs() <= 5.0,
                "{intensidad:?}: tipo {k} al {real:.1} % (Playgroup, {objetivo} %)"
            );
        }
    }
}

#[test]
fn apagado_nunca_reacciona() {
    for s in 0..50 {
        assert!(jugar(s, Intensidad::Apagado).is_empty());
    }
}

#[test]
fn con_la_misma_semilla_las_mismas_reacciones() {
    for s in 0..20 {
        assert_eq!(
            jugar(s, Intensidad::Despiadado),
            jugar(s, Intensidad::Despiadado)
        );
    }
}

#[test]
fn resolver_una_reaccion_y_deshacer_lo_devuelve_todo() {
    // Hasta que una partida «Despiadada» traiga una eliminación.
    for s in 0..200 {
        let mut j = Juego::nuevo(s, Intensidad::Despiadado);
        for _ in 0..RONDAS {
            let ya = j.reacciones.len();
            j.ronda();
            let Some(re) = j.reacciones.get(ya).cloned() else {
                continue;
            };
            if re.tipo != TipoReaccion::Eliminar {
                continue;
            }
            // Resuelta, la carta ya no está en el campo.
            let id = re.objetivo[0];
            let (z, _) = j.p().donde(id).unwrap();
            assert_ne!(z, RefZona::de(J, Zona::Campo));
            // Deshacer hasta antes de la reacción la deja como estaba.
            j.r.procesar(
                &Intencion {
                    actor: J,
                    accion: Accion::Deshacer { hasta: Some(re.id) },
                },
                &mut j.azar,
                0,
            )
            .unwrap();
            assert!(j
                .p()
                .sparring
                .reacciones
                .iter()
                .all(|r| r.reaccion.id != re.id));
            assert_eq!(
                j.p().donde(id).map(|(z, _)| z),
                Some(RefZona::de(J, Zona::Campo))
            );
            return;
        }
    }
    panic!("ninguna eliminación en 200 partidas");
}
