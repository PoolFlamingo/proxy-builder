//! Lo que comparten los tests del motor: decks y partidas de ejemplo.
#![allow(dead_code)]

use mesa::accion::Intencion;
use mesa::formato::{Formato, Opciones, Variantes};
use mesa::modelo::{InfoCarta, OrigenCarta, Partida, RefImagen};
use mesa::preparar::{iniciar, CartaDeMesa, Config, DeckDeMesa, JugadorNuevo, Seccion};
use mesa::{Accion, Azar, IdJugador, Registro};

pub fn origen(nombre: &str, tipo: &str, cmc: u16) -> OrigenCarta {
    let t = tipo.to_lowercase();
    OrigenCarta {
        carta_deck: format!("deck-{nombre}"),
        nombre: nombre.into(),
        caras: vec![RefImagen::Propia {
            sha256: format!("{nombre:0>64}"),
            ancho: 745,
            alto: 1040,
        }],
        dorso: RefImagen::Generica,
        scryfall_id: Some(format!("sf-{nombre}")),
        info: InfoCarta {
            tipo: Some(tipo.into()),
            cmc_decimas: Some(cmc * 10),
            es_tierra: t.contains("land"),
            es_criatura: t.contains("creature"),
            es_legendaria: t.contains("legendary"),
            fuerza: t.contains("creature").then(|| "2".into()),
            resistencia: t.contains("creature").then(|| "2".into()),
            lealtad: t.contains("planeswalker").then(|| "3".into()),
            ..InfoCarta::default()
        },
    }
}

fn carta(nombre: &str, tipo: &str, cmc: u16, seccion: Seccion) -> CartaDeMesa {
    CartaDeMesa {
        carta_deck: format!("deck-{nombre}"),
        seccion,
        origen: origen(nombre, tipo, cmc),
    }
}

/// Un deck de 40 cartas: 16 tierras, 14 criaturas, 6 instantáneos, 2
/// artefactos, 1 planeswalker, 1 saga; un comandante y una ficha.
pub fn deck(nombre: &str) -> DeckDeMesa {
    let mut cartas = Vec::new();
    for i in 0..16 {
        cartas.push(carta(
            &format!("Forest {i}"),
            "Basic Land — Forest",
            0,
            Seccion::Mazo,
        ));
    }
    for i in 0..14 {
        cartas.push(carta(
            &format!("Bear {i}"),
            "Creature — Bear",
            2,
            Seccion::Mazo,
        ));
    }
    for i in 0..6 {
        cartas.push(carta(&format!("Shock {i}"), "Instant", 1, Seccion::Mazo));
    }
    for i in 0..2 {
        cartas.push(carta(&format!("Ring {i}"), "Artifact", 1, Seccion::Mazo));
    }
    cartas.push(carta(
        "Garruk",
        "Legendary Planeswalker — Garruk",
        4,
        Seccion::Mazo,
    ));
    cartas.push(carta("Saga", "Enchantment — Saga", 3, Seccion::Mazo));
    cartas.push(carta(
        "Yusri",
        "Legendary Creature — Efreet",
        3,
        Seccion::Comandante,
    ));
    cartas.push(carta("Pyroblast", "Instant", 1, Seccion::Banquillo));
    cartas.push(carta(
        "Treasure",
        "Token Artifact — Treasure",
        0,
        Seccion::Ficha,
    ));
    DeckDeMesa {
        nombre: nombre.into(),
        cartas,
        tapete: None,
        dorso: RefImagen::Generica,
    }
}

pub fn config(jugadores: usize) -> Config {
    let variantes = Variantes::default();
    Config {
        formato: Formato::Commander,
        opciones: Opciones::de_fabrica(Formato::Commander, &variantes, jugadores),
        variantes,
        jugadores: (0..jugadores)
            .map(|i| JugadorNuevo {
                nombre: format!("J{i}"),
                color: "#e11d48".into(),
                equipo: None,
                mazo: deck(&format!("D{i}")),
            })
            .collect(),
        equipos: Vec::new(),
        primero: Some(IdJugador(0)),
    }
}

pub fn partida(jugadores: usize) -> Partida {
    iniciar("p".into(), config(jugadores), 1234, 1_000).unwrap()
}

/// Una partida ya empezada (todos se quedan con su mano).
pub fn empezada(jugadores: usize, modo: mesa::ModoDeshacer) -> (Registro, Azar) {
    let mut r = Registro::nuevo(partida(jugadores), modo);
    let mut azar = Azar::new(99);
    for j in 0..jugadores {
        r.procesar(
            &Intencion {
                actor: IdJugador(j as u8),
                accion: Accion::QuedarseCon { al_fondo: vec![] },
            },
            &mut azar,
            2_000,
        )
        .unwrap();
    }
    (r, azar)
}

use mesa::accion::{
    Destino, Filtro, FormaExcavar, Lado, ModoDescarte, ModoExilio, OrigenFicha, RestoExcavar,
    Sitio, ZonaDestino,
};
use mesa::modelo::{Extremo, Pos, RefZona, TipoFlecha, Zona, ZonaComun};
use mesa::IdCarta;

fn una_de<T: Clone>(azar: &mut Azar, v: &[T]) -> Option<T> {
    if v.is_empty() {
        None
    } else {
        v.get(azar.menor_que(v.len() as u64) as usize).cloned()
    }
}

fn carta_al_azar(p: &Partida, azar: &mut Azar) -> IdCarta {
    // A veces una que no existe.
    if azar.menor_que(20) == 0 {
        return IdCarta(99_999);
    }
    let ids: Vec<IdCarta> = p.cartas.keys().copied().collect();
    una_de(azar, &ids).unwrap_or(IdCarta(1))
}

fn de_zona(p: &Partida, azar: &mut Azar, z: RefZona) -> IdCarta {
    let v = p.zona(&z).to_vec();
    una_de(azar, &v).unwrap_or(IdCarta(99_998))
}

fn zona_al_azar(azar: &mut Azar) -> ZonaDestino {
    let zs = [
        ZonaDestino::De(Zona::Biblioteca),
        ZonaDestino::De(Zona::Mano),
        ZonaDestino::De(Zona::Campo),
        ZonaDestino::De(Zona::Campo),
        ZonaDestino::De(Zona::Cementerio),
        ZonaDestino::De(Zona::Exilio),
        ZonaDestino::De(Zona::Mando),
        ZonaDestino::Comun(ZonaComun::Pila),
    ];
    una_de(azar, &zs).unwrap_or(ZonaDestino::De(Zona::Campo))
}

fn sitio_al_azar(azar: &mut Azar) -> Sitio {
    match azar.menor_que(7) {
        0 => Sitio::Arriba,
        1 => Sitio::Abajo,
        2 => Sitio::Indice(azar.menor_que(5) as u32),
        3 => Sitio::XDesdeArriba(azar.menor_que(4) as u32 + 1),
        4 => Sitio::AlAzar,
        5 => Sitio::BarajarDentro,
        _ => Sitio::EnCampo(Pos::nueva(
            azar.menor_que(10_001) as u16,
            azar.menor_que(10_001) as u16,
        )),
    }
}

/// Una intención al azar (válida o no) para `n` jugadores.
pub fn intencion_al_azar(p: &Partida, azar: &mut Azar, n: usize) -> Intencion {
    // Casi siempre el activo, para que haya partida.
    let actor = if azar.menor_que(3) == 0 {
        IdJugador(azar.menor_que(n as u64) as u8)
    } else {
        p.turno.activo
    };
    let campo = RefZona::de(actor, Zona::Campo);
    let mano = RefZona::de(actor, Zona::Mano);
    let lib = RefZona::de(actor, Zona::Biblioteca);
    let accion = match azar.menor_que(52) {
        0..=3 => Accion::Mover {
            cartas: vec![if azar.moneda() {
                de_zona(p, azar, mano)
            } else {
                carta_al_azar(p, azar)
            }],
            a: Destino {
                zona: zona_al_azar(azar),
                jugador: None,
                sitio: sitio_al_azar(azar),
            },
            como: None,
        },
        4 => Accion::Girar {
            cartas: vec![de_zona(p, azar, campo)],
            girada: azar.moneda(),
        },
        5 => Accion::BocaAbajo {
            cartas: vec![de_zona(p, azar, campo)],
            si: azar.moneda(),
        },
        6 => Accion::CambiarCara {
            carta: de_zona(p, azar, campo),
        },
        7 => Accion::Contador {
            cartas: vec![de_zona(p, azar, campo)],
            tipo: "+1/+1".into(),
            delta: azar.menor_que(5) as i32 - 2,
        },
        8 => Accion::Robar {
            n: azar.menor_que(4) as u32,
        },
        9 => Accion::Moler {
            n: azar.menor_que(3) as u32 + 1,
        },
        10 => Accion::Mirar {
            de: actor,
            n: 3,
            desde: if azar.moneda() {
                Lado::Arriba
            } else {
                Lado::Abajo
            },
        },
        11 => {
            let v: Vec<IdCarta> = p.zona(&lib).iter().take(3).copied().collect();
            let (a, b) = v.split_at(v.len().min(azar.menor_que(4) as usize));
            Accion::Ordenar {
                de: actor,
                arriba: b.to_vec(),
                abajo: Vec::new(),
                cementerio: a.to_vec(),
                exilio: Vec::new(),
            }
        }
        12 => Accion::Barajar,
        13..=15 => Accion::PasarTurno,
        16 => Accion::Vida {
            jugador: IdJugador(azar.menor_que(n as u64) as u8),
            delta: azar.menor_que(11) as i32 - 5,
        },
        17 => Accion::CrearFicha {
            de: if azar.moneda() {
                OrigenFicha::DelDeck { indice: 0 }
            } else {
                OrigenFicha::Libre {
                    nombre: "Soldier".into(),
                    fuerza: Some("1".into()),
                    resistencia: Some("1".into()),
                    colores: vec!['W'],
                }
            },
            cantidad: azar.menor_que(3) as u32 + 1,
            monton: azar.moneda(),
            pos: None,
        },
        18 => Accion::Excavar {
            forma: if azar.moneda() {
                FormaExcavar::HastaQueCoincida
            } else {
                FormaExcavar::RevelarN(3)
            },
            filtro: Filtro {
                no_tierra: true,
                ..Filtro::default()
            },
            parar_tras: 1,
        },
        19 => {
            let reveladas: Vec<IdCarta> = p
                .zona(&lib)
                .iter()
                .copied()
                .filter(|id| p.cartas[id].vista_por == mesa::modelo::Vista::Todos)
                .collect();
            let (a, b) = reveladas.split_at(reveladas.len().min(1));
            Accion::ResolverExcavacion {
                elegidas: a
                    .iter()
                    .map(|c| {
                        (
                            *c,
                            Destino {
                                zona: ZonaDestino::De(Zona::Mano),
                                jugador: None,
                                sitio: Sitio::Arriba,
                            },
                        )
                    })
                    .collect(),
                resto: b.to_vec(),
                a: if azar.moneda() {
                    RestoExcavar::AbajoAlAzar
                } else {
                    RestoExcavar::Cementerio
                },
            }
        }
        20 => Accion::Buscar { de: actor },
        21 => Accion::DejarDeMirar { de: actor },
        22 => Accion::RevelarArriba { n: 2 },
        23 => Accion::Descartar {
            modo: ModoDescarte::AlAzar(1),
        },
        24 => Accion::Amontonar {
            cartas: vec![de_zona(p, azar, campo), de_zona(p, azar, campo)],
        },
        25 => Accion::Separar {
            monton: de_zona(p, azar, campo),
        },
        26 => Accion::Adjuntar {
            carta: de_zona(p, azar, campo),
            a: Some(de_zona(p, azar, campo)),
        },
        27 => Accion::DarControl {
            cartas: vec![de_zona(p, azar, campo)],
            a: IdJugador(azar.menor_que(n as u64) as u8),
        },
        28 => Accion::Flecha {
            de: Extremo::Carta(de_zona(p, azar, campo)),
            a: Extremo::Jugador(IdJugador(azar.menor_que(n as u64) as u8)),
            tipo: TipoFlecha::Ataque,
        },
        29 => Accion::TirarDados {
            dados: vec![(20, 2), (6, 1)],
        },
        30 => Accion::Moneda {
            cuantas: 2,
            hasta_perder: azar.moneda(),
            pulgares: azar.menor_que(2) as u8,
        },
        31 => Accion::Resolver {
            carta: de_zona(p, azar, RefZona::comun(ZonaComun::Pila)),
            como: mesa::accion::ComoResolver::Auto,
        },
        32 => Accion::ApilarDespues {
            carta: de_zona(p, azar, mano),
        },
        33 => Accion::JugarBocaAbajo {
            carta: de_zona(p, azar, mano),
            pos: None,
        },
        34 => Accion::ExiliarDeArriba {
            n: 1,
            modo: match azar.menor_que(3) {
                0 => ModoExilio::Normal,
                1 => ModoExilio::BocaAbajo,
                _ => ModoExilio::Oculta,
            },
        },
        35 => Accion::Manifestar { n: 1 },
        36 => Accion::RevelarMano { si: azar.moneda() },
        37 => Accion::DeArribaRevelada { si: azar.moneda() },
        38 => Accion::Organizar,
        39 => Accion::EnderezarTodo,
        40 => Accion::Drenar { n: 1 },
        41 => Accion::Rueda { robar: Some(3) },
        42 => Accion::AlAzarDe {
            zona: Zona::Cementerio,
            filtro: Filtro::default(),
            a: Zona::Mano,
        },
        43 => Accion::BarajarTodoDentro {
            zona: Zona::Cementerio,
        },
        44 => Accion::HabilidadEnPila {
            carta: de_zona(p, azar, campo),
            texto: "Draw a card".into(),
        },
        47 => Accion::PorJugador {
            jugador: IdJugador(azar.menor_que(n as u64) as u8),
            accion: Box::new(Accion::Moler { n: 1 }),
        },
        49 => Accion::OcultarArriba { n: 1 },
        50 => Accion::MostrarATodos {
            cartas: vec![de_zona(p, azar, mano)],
        },
        51 => Accion::DejarDeMostrar {
            cartas: vec![de_zona(p, azar, mano)],
        },
        48 => Accion::MirarMano {
            de: IdJugador(azar.menor_que(n as u64) as u8),
            si: azar.moneda(),
        },
        46 => Accion::TomarControl {
            cartas: vec![carta_al_azar(p, azar)],
            hasta_fin_de_turno: azar.moneda(),
        },
        _ => Accion::CopiaFicha {
            cartas: vec![de_zona(p, azar, campo)],
        },
    };
    Intencion { actor, accion }
}

use mesa::visibilidad::Mirador;
use mesa::vista::VistaMesa;

/// La regla de los equipos escrita otra vez a mano: compañeros (mismo equipo
/// y distinto jugador) y la mano no se ha hecho privada.
fn son_companeros_con_mano_abierta(p: &Partida, yo: Option<IdJugador>, dueno: IdJugador) -> bool {
    let Some(yo) = yo else { return false };
    if yo == dueno || p.variantes.mano_privada_en_equipo {
        return false;
    }
    let (a, b) = (
        p.jugador(yo).unwrap().equipo,
        p.jugador(dueno).unwrap().equipo,
    );
    a.is_some() && a == b
}

/// Comprueba, con las reglas escritas otra vez a mano, que la vista no
/// enseña nada que el mirador no deba ver.
pub fn sin_fugas(p: &Partida, v: &VistaMesa, m: Mirador) {
    let yo = m.jugador();
    // Las notas privadas son de su autor y de nadie más (tampoco de «todo a la vista»).
    for n in &v.general.notas {
        assert!(
            !n.privada || Some(n.autor) == yo,
            "nota privada de {:?} para {:?}",
            n.autor,
            m
        );
    }
    for z in &v.zonas {
        let publica = z.zona.es_publica();
        let mia = match z.zona {
            RefZona::De { jugador, zona } => {
                (zona == Zona::Mano || zona == Zona::Banquillo)
                    && (Some(jugador) == yo
                        || (zona == Zona::Mano
                            && (p.jugador(jugador).unwrap().mano_revelada
                                || son_companeros_con_mano_abierta(p, yo, jugador))))
            }
            _ => false,
        };
        if !publica && !mia {
            assert!(z.cartas.is_none(), "zona oculta con ids: {:?}", z.zona);
        }
        assert_eq!(z.cuantas as usize, p.zona(&z.zona).len());
    }
    for c in &v.cartas {
        let real = &p.cartas[&c.id];
        let (zona, i) = p.donde(c.id).unwrap();
        if c.origen.is_none() {
            continue;
        }
        let ensenada = real.vista_por.incluye(yo, real.dueno);
        let ok = match zona {
            z if z.es_publica() => !real.boca_abajo || ensenada,
            RefZona::De {
                jugador,
                zona: Zona::Mano,
            } => {
                ensenada
                    || Some(jugador) == yo
                    || p.jugador(jugador).unwrap().mano_revelada
                    || son_companeros_con_mano_abierta(p, yo, jugador)
            }
            RefZona::De {
                jugador,
                zona: Zona::Banquillo,
            } => ensenada || Some(jugador) == yo,
            RefZona::De {
                jugador,
                zona: Zona::Biblioteca,
            } => ensenada || (i == 0 && p.jugador(jugador).unwrap().de_arriba_revelada),
            _ => ensenada,
        };
        assert!(ok, "fuga: {:?} en {:?} para {:?}", c.id, zona, m);
    }
}
