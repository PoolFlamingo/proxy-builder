//! Un test por acción del catálogo (A4, A6): estado antes → evento →
//! estado después. `nombre` es un `match` exhaustivo: una variante nueva no
//! compila hasta que tenga su caso aquí.

mod comun;

use std::collections::BTreeSet;

use comun::*;
use mesa::accion::*;
use mesa::modelo::*;
use mesa::sparring::{Decision, Intensidad};
use mesa::{Azar, IdCarta, IdJugador, ModoDeshacer, Rechazo, Registro};

const YO: IdJugador = IdJugador(0);
const OTRO: IdJugador = IdJugador(1);

fn nombre(a: &Accion) -> &'static str {
    match a {
        Accion::Mover { .. } => "Mover",
        Accion::Girar { .. } => "Girar",
        Accion::BocaAbajo { .. } => "BocaAbajo",
        Accion::CambiarCara { .. } => "CambiarCara",
        Accion::Contador { .. } => "Contador",
        Accion::FijarContador { .. } => "FijarContador",
        Accion::MultiplicarContadores { .. } => "MultiplicarContadores",
        Accion::ContadorClave { .. } => "ContadorClave",
        Accion::Pt { .. } => "Pt",
        Accion::MasXMasY { .. } => "MasXMasY",
        Accion::Etiqueta { .. } => "Etiqueta",
        Accion::PonerBolita { .. } => "PonerBolita",
        Accion::MoverBolita { .. } => "MoverBolita",
        Accion::QuitarBolita { .. } => "QuitarBolita",
        Accion::Adjuntar { .. } => "Adjuntar",
        Accion::FaseFuera { .. } => "FaseFuera",
        Accion::NoEndereza { .. } => "NoEndereza",
        Accion::Resaltar { .. } => "Resaltar",
        Accion::CopiaFicha { .. } => "CopiaFicha",
        Accion::DarControl { .. } => "DarControl",
        Accion::TomarControl { .. } => "TomarControl",
        Accion::PorJugador { .. } => "PorJugador",
        Accion::MirarMano { .. } => "MirarMano",
        Accion::HabilidadEnPila { .. } => "HabilidadEnPila",
        Accion::Flecha { .. } => "Flecha",
        Accion::QuitarFlechas { .. } => "QuitarFlechas",
        Accion::Robar { .. } => "Robar",
        Accion::Moler { .. } => "Moler",
        Accion::Manifestar { .. } => "Manifestar",
        Accion::ExiliarDeArriba { .. } => "ExiliarDeArriba",
        Accion::Mirar { .. } => "Mirar",
        Accion::Ordenar { .. } => "Ordenar",
        Accion::RevelarArriba { .. } => "RevelarArriba",
        Accion::OcultarArriba { .. } => "OcultarArriba",
        Accion::Excavar { .. } => "Excavar",
        Accion::ResolverExcavacion { .. } => "ResolverExcavacion",
        Accion::Buscar { .. } => "Buscar",
        Accion::DejarDeMirar { .. } => "DejarDeMirar",
        Accion::DeArribaRevelada { .. } => "DeArribaRevelada",
        Accion::Barajar => "Barajar",
        Accion::Rueda { .. } => "Rueda",
        Accion::EnderezarTodo => "EnderezarTodo",
        Accion::Organizar => "Organizar",
        Accion::CrearFicha { .. } => "CrearFicha",
        Accion::Mulligan => "Mulligan",
        Accion::QuedarseCon { .. } => "QuedarseCon",
        Accion::Vida { .. } => "Vida",
        Accion::VidaATodos { .. } => "VidaATodos",
        Accion::Drenar { .. } => "Drenar",
        Accion::Extorsion { .. } => "Extorsion",
        Accion::Veneno { .. } => "Veneno",
        Accion::DanoComandante { .. } => "DanoComandante",
        Accion::ContadorJugador { .. } => "ContadorJugador",
        Accion::Designar { .. } => "Designar",
        Accion::DiaNoche { .. } => "DiaNoche",
        Accion::Lanzamientos { .. } => "Lanzamientos",
        Accion::SinCosteExtra { .. } => "SinCosteExtra",
        Accion::RevelarMano { .. } => "RevelarMano",
        Accion::RevelarA { .. } => "RevelarA",
        Accion::MostrarATodos { .. } => "MostrarATodos",
        Accion::DejarDeMostrar { .. } => "DejarDeMostrar",
        Accion::JugarBocaAbajo { .. } => "JugarBocaAbajo",
        Accion::Descartar { .. } => "Descartar",
        Accion::Eliminar { .. } => "Eliminar",
        Accion::Conceder => "Conceder",
        Accion::VidaFantasma { .. } => "VidaFantasma",
        Accion::DanoComandanteFantasma { .. } => "DanoComandanteFantasma",
        Accion::VerFantasma { .. } => "VerFantasma",
        Accion::Senal { .. } => "Senal",
        Accion::PasarTurno => "PasarTurno",
        Accion::TurnoExtra { .. } => "TurnoExtra",
        Accion::Pausar { .. } => "Pausar",
        Accion::SaltarTurno { .. } => "SaltarTurno",
        Accion::Reiniciar => "Reiniciar",
        Accion::TirarDados { .. } => "TirarDados",
        Accion::Moneda { .. } => "Moneda",
        Accion::DadoPlanar => "DadoPlanar",
        Accion::Planeswalk => "Planeswalk",
        Accion::PonerEnMarcha => "PonerEnMarcha",
        Accion::AbandonarPlan { .. } => "AbandonarPlan",
        Accion::Resolver { .. } => "Resolver",
        Accion::ApilarDespues { .. } => "ApilarDespues",
        Accion::Amontonar { .. } => "Amontonar",
        Accion::SacarDelMonton { .. } => "SacarDelMonton",
        Accion::CambiarMonton { .. } => "CambiarMonton",
        Accion::Separar { .. } => "Separar",
        Accion::Agrupar { .. } => "Agrupar",
        Accion::AlAzarDe { .. } => "AlAzarDe",
        Accion::TodoAlFondoAlAzar { .. } => "TodoAlFondoAlAzar",
        Accion::BarajarTodoDentro { .. } => "BarajarTodoDentro",
        Accion::ExiliarTodo { .. } => "ExiliarTodo",
        Accion::IntensidadSparring { .. } => "IntensidadSparring",
        Accion::DecidirSparring { .. } => "DecidirSparring",
        Accion::Chat { .. } => "Chat",
        Accion::Aviso { .. } => "Aviso",
        Accion::PonerNota { .. } => "PonerNota",
        Accion::EditarNota { .. } => "EditarNota",
        Accion::QuitarNota { .. } => "QuitarNota",
        Accion::Colocar { .. } => "Colocar",
        Accion::NotaEnCarta { .. } => "NotaEnCarta",
        Accion::AtarNota { .. } => "AtarNota",
        Accion::VenenoFantasma { .. } => "VenenoFantasma",
        Accion::Terminar { .. } => "Terminar",
        Accion::Deshacer { .. } => "Deshacer",
        Accion::Rehacer => "Rehacer",
    }
}

/// Una mesa de 2 ya empezada, con lo que hace falta para los tests.
struct Mesa {
    r: Registro,
    azar: Azar,
    hechas: BTreeSet<&'static str>,
}

impl Mesa {
    fn nueva(n: usize) -> Self {
        let (r, azar) = empezada(n, ModoDeshacer::SinLimite);
        Mesa {
            r,
            azar,
            hechas: BTreeSet::new(),
        }
    }
    fn p(&self) -> &Partida {
        self.r.partida()
    }
    fn como(&mut self, actor: IdJugador, a: Accion) -> Result<(), Rechazo> {
        let n = nombre(&a);
        let t = 10_000 + self.p().seq;
        self.r
            .procesar(&Intencion { actor, accion: a }, &mut self.azar, t)?;
        self.hechas.insert(n);
        Ok(())
    }
    fn haz(&mut self, a: Accion) {
        let n = nombre(&a);
        if let Err(e) = self.como(YO, a) {
            panic!("{n}: {e:?}");
        }
    }
    fn zona(&self, j: IdJugador, z: Zona) -> Vec<IdCarta> {
        self.p().zona(&RefZona::de(j, z)).to_vec()
    }
    fn c(&self, id: IdCarta) -> &Carta {
        &self.p().cartas[&id]
    }
    /// Una carta de mi mano al campo.
    fn al_campo(&mut self, que: &str) -> IdCarta {
        let id = *self
            .zona(YO, Zona::Mano)
            .iter()
            .chain(self.zona(YO, Zona::Biblioteca).iter())
            .find(|id| self.c(**id).origen.nombre.starts_with(que))
            .unwrap_or_else(|| panic!("no hay {que}"));
        self.haz(Accion::Mover {
            cartas: vec![id],
            a: campo(None),
            como: None,
        });
        id
    }
}

/// Una carta cualquiera del campo propio (para colocarla).
fn atada_a_colocar(m: &Mesa) -> IdCarta {
    m.zona(YO, Zona::Campo)[0]
}

fn campo(pos: Option<Pos>) -> Destino {
    Destino {
        zona: ZonaDestino::De(Zona::Campo),
        jugador: None,
        sitio: pos.map_or(Sitio::Arriba, Sitio::EnCampo),
    }
}

fn a(z: Zona, sitio: Sitio) -> Destino {
    Destino {
        zona: ZonaDestino::De(z),
        jugador: None,
        sitio,
    }
}

#[test]
fn cada_accion_del_catalogo() {
    let mut m = Mesa::nueva(2);
    // Mover: de la mano al campo, con posición.
    let oso = m.al_campo("Bear");
    assert!(m.zona(YO, Zona::Campo).contains(&oso));
    assert!(m.c(oso).pos.is_some());
    m.haz(Accion::Mover {
        cartas: vec![oso],
        a: campo(Some(Pos::nueva(5_000, 5_000))),
        como: None,
    });
    assert_eq!(m.c(oso).pos, Some(Pos::nueva(5_000, 5_000)));
    // Girar, boca abajo, cambiar de cara.
    m.haz(Accion::Girar {
        cartas: vec![oso],
        girada: true,
    });
    assert!(m.c(oso).girada);
    m.haz(Accion::BocaAbajo {
        cartas: vec![oso],
        si: true,
    });
    assert!(m.c(oso).boca_abajo);
    m.haz(Accion::BocaAbajo {
        cartas: vec![oso],
        si: false,
    });
    m.haz(Accion::CambiarCara { carta: oso });
    assert_eq!(m.c(oso).cara, 0, "una sola cara: se queda en la 0");
    // Contadores.
    m.haz(Accion::Contador {
        cartas: vec![oso],
        tipo: "+1/+1".into(),
        delta: 2,
    });
    m.haz(Accion::MultiplicarContadores { carta: oso, por: 2 });
    assert_eq!(m.c(oso).contadores["+1/+1"], 4);
    m.haz(Accion::FijarContador {
        carta: oso,
        tipo: "+1/+1".into(),
        valor: 0,
    });
    assert!(m.c(oso).contadores.is_empty());
    m.haz(Accion::ContadorClave {
        cartas: vec![oso],
        palabra: "Flying".into(),
        si: true,
    });
    assert!(m.c(oso).palabras_clave.contains("Flying"));
    m.haz(Accion::Pt {
        carta: oso,
        fila: FilaPt::Fijo,
        valor: Some((5, 5)),
    });
    m.haz(Accion::MasXMasY {
        cartas: vec![oso],
        x: 2,
        y: 2,
        fila: FilaPt::HastaFinDeTurno,
    });
    assert_eq!(m.c(oso).pt.hasta_fin_de_turno, (2, 2));
    m.haz(Accion::Etiqueta {
        carta: oso,
        etiqueta: Etiqueta {
            texto: "Copia".into(),
            color: 2,
        },
        poner: true,
    });
    assert_eq!(m.c(oso).etiquetas.len(), 1);
    // Bolitas.
    m.haz(Accion::PonerBolita {
        campo_de: YO,
        pos: Pos::nueva(10, 10),
        color: 3,
        tamano: Tamano::M,
    });
    let b = m.p().bolitas[0].id;
    m.haz(Accion::MoverBolita {
        id: b,
        pos: Pos::nueva(20, 20),
    });
    m.haz(Accion::QuitarBolita { id: b });
    assert!(m.p().bolitas.is_empty());
    // Notas.
    m.haz(Accion::PonerNota {
        texto: "2 de maná".into(),
        temporal: true,
        privada: true,
    });
    let nota = m.p().notas[0].id;
    m.haz(Accion::EditarNota {
        id: nota,
        texto: "3 de maná".into(),
        temporal: false,
        privada: false,
    });
    assert_eq!(m.p().notas[0].texto, "3 de maná");
    m.haz(Accion::QuitarNota { id: nota });
    assert!(m.p().notas.is_empty());
    // Soltar varias a la vez: un solo evento, cada una en su punto.
    let antes = m.p().seq;
    let otra = m.al_campo("Bear");
    m.haz(Accion::Colocar {
        cartas: vec![
            (atada_a_colocar(&m), Pos::nueva(100, 100)),
            (otra, Pos::nueva(900, 900)),
        ],
        jugador: None,
    });
    assert_eq!(
        m.p().seq,
        antes + 2,
        "un evento para traer el oso y otro para colocar"
    );
    assert_eq!(m.c(otra).pos, Some(Pos::nueva(900, 900)));
    // Notas atadas a una carta del campo: se sueltan y se vuelven a atar, y
    // se sueltan solas cuando la carta sale del campo.
    let atada = m.al_campo("Ring");
    m.haz(Accion::NotaEnCarta {
        texto: "+2/+0".into(),
        temporal: false,
        privada: false,
        carta: Some(atada),
    });
    let nota = m.p().notas[0].id;
    assert_eq!(m.p().notas[0].carta, Some(atada));
    m.haz(Accion::AtarNota {
        id: nota,
        carta: None,
    });
    assert_eq!(m.p().notas[0].carta, None);
    m.haz(Accion::AtarNota {
        id: nota,
        carta: Some(atada),
    });
    assert_eq!(
        m.como(
            YO,
            Accion::AtarNota {
                id: nota,
                carta: Some(m.zona(YO, Zona::Biblioteca)[0]),
            }
        ),
        Err(Rechazo::NoSePuede),
        "solo a cartas del campo"
    );
    m.haz(Accion::Mover {
        cartas: vec![atada],
        a: a(Zona::Cementerio, Sitio::Arriba),
        como: None,
    });
    assert_eq!(m.p().notas[0].carta, None, "se suelta al salir del campo");
    m.haz(Accion::QuitarNota { id: nota });
    m.haz(Accion::Mover {
        cartas: vec![atada],
        a: campo(None),
        como: None,
    });
    // Adjuntar.
    let anillo = m.al_campo("Ring");
    m.haz(Accion::Adjuntar {
        carta: anillo,
        a: Some(oso),
    });
    assert_eq!(m.c(anillo).adjunta_a, Some(oso));
    assert_eq!(
        m.como(
            YO,
            Accion::Adjuntar {
                carta: oso,
                a: Some(anillo)
            }
        ),
        Err(Rechazo::NoSePuede),
        "sin ciclos"
    );
    m.haz(Accion::FaseFuera {
        cartas: vec![oso],
        si: true,
    });
    m.haz(Accion::NoEndereza {
        cartas: vec![oso],
        si: true,
    });
    m.haz(Accion::Resaltar {
        cartas: vec![oso],
        color: Some(1),
    });
    assert!(m.c(oso).fase_fuera && m.c(oso).no_endereza && m.c(oso).resaltada == Some(1));
    // Copia ficha.
    let antes = m.zona(YO, Zona::Campo).len();
    m.haz(Accion::CopiaFicha { cartas: vec![oso] });
    let copia = *m.zona(YO, Zona::Campo).last().unwrap();
    assert_eq!(m.zona(YO, Zona::Campo).len(), antes + 1);
    assert!(m.c(copia).es_ficha && m.c(copia).copia_de == Some(oso));
    // Habilidad a la pila y resolverla: desaparece.
    m.haz(Accion::HabilidadEnPila {
        carta: oso,
        texto: "Tap: draw".into(),
    });
    let h = *m.p().zona(&RefZona::comun(ZonaComun::Pila)).last().unwrap();
    m.haz(Accion::Resolver {
        carta: h,
        como: ComoResolver::Auto,
    });
    assert!(!m.p().cartas.contains_key(&h));
    // Flechas.
    m.haz(Accion::Flecha {
        de: Extremo::Carta(oso),
        a: Extremo::Jugador(OTRO),
        tipo: TipoFlecha::Ataque,
    });
    assert_eq!(m.p().flechas.len(), 1);
    m.haz(Accion::QuitarFlechas { de: None });
    assert!(m.p().flechas.is_empty());
    // Dar el control: al campo del otro, sigue siendo mía.
    m.haz(Accion::DarControl {
        cartas: vec![copia],
        a: OTRO,
    });
    assert!(m.zona(OTRO, Zona::Campo).contains(&copia));
    assert_eq!(m.c(copia).controlador, OTRO);
    // Tomar el control: vuelve al campo de quien la toma.
    m.haz(Accion::TomarControl {
        cartas: vec![copia],
        hasta_fin_de_turno: false,
    });
    assert!(m.zona(YO, Zona::Campo).contains(&copia));
    assert_eq!(m.c(copia).controlador, YO);
    // Por otro jugador: que robe una; y mirar su mano.
    let suya = m.zona(OTRO, Zona::Mano).len();
    m.haz(Accion::PorJugador {
        jugador: OTRO,
        accion: Box::new(Accion::Robar { n: 1 }),
    });
    assert_eq!(m.zona(OTRO, Zona::Mano).len(), suya + 1);
    m.haz(Accion::MirarMano { de: OTRO, si: true });
    m.haz(Accion::MirarMano {
        de: OTRO,
        si: false,
    });
    // Enseñar una carta de la mano a todos y taparla.
    let ensenada = m.zona(YO, Zona::Mano)[0];
    m.haz(Accion::MostrarATodos {
        cartas: vec![ensenada],
    });
    m.haz(Accion::DejarDeMostrar {
        cartas: vec![ensenada],
    });

    // ── Biblioteca ──
    let mano = m.zona(YO, Zona::Mano).len();
    m.haz(Accion::Robar { n: 2 });
    assert_eq!(m.zona(YO, Zona::Mano).len(), mano + 2);
    m.haz(Accion::Moler { n: 2 });
    assert_eq!(m.zona(YO, Zona::Cementerio).len(), 2);
    m.haz(Accion::Manifestar { n: 1 });
    let man = *m.zona(YO, Zona::Campo).last().unwrap();
    assert!(m.c(man).boca_abajo);
    m.haz(Accion::ExiliarDeArriba {
        n: 1,
        modo: ModoExilio::Oculta,
    });
    let ex = *m.zona(YO, Zona::Exilio).last().unwrap();
    assert!(m.c(ex).boca_abajo && m.c(ex).vista_por == Vista::Nadie);
    m.haz(Accion::Mirar {
        de: YO,
        n: 3,
        desde: Lado::Arriba,
    });
    let tres: Vec<IdCarta> = m.zona(YO, Zona::Biblioteca)[..3].to_vec();
    assert!(tres.iter().all(|c| m.c(*c).vista_por.incluye(Some(YO), YO)));
    m.haz(Accion::Ordenar {
        de: YO,
        arriba: vec![tres[2], tres[1]],
        abajo: vec![tres[0]],
        cementerio: vec![],
        exilio: vec![],
    });
    let lib = m.zona(YO, Zona::Biblioteca);
    assert_eq!(&lib[..2], &[tres[2], tres[1]]);
    assert_eq!(*lib.last().unwrap(), tres[0]);
    m.haz(Accion::RevelarArriba { n: 1 });
    let arriba = m.zona(YO, Zona::Biblioteca)[0];
    assert_eq!(m.c(arriba).vista_por, Vista::Todos);
    m.haz(Accion::OcultarArriba { n: 1 });
    assert_eq!(
        m.c(arriba).vista_por,
        Vista::Nadie,
        "deja de estar revelada"
    );
    m.haz(Accion::RevelarArriba { n: 1 });
    assert_eq!(m.c(lib[0]).vista_por, Vista::Todos);
    m.haz(Accion::Excavar {
        forma: FormaExcavar::HastaQueCoincida,
        filtro: Filtro {
            no_tierra: true,
            ..Filtro::default()
        },
        parar_tras: 1,
    });
    let reveladas: Vec<IdCarta> = m
        .zona(YO, Zona::Biblioteca)
        .into_iter()
        .take_while(|c| m.c(*c).vista_por == Vista::Todos)
        .collect();
    assert!(!m.c(*reveladas.last().unwrap()).origen.info.es_tierra);
    let (ultima, resto) = reveladas.split_last().unwrap();
    m.haz(Accion::ResolverExcavacion {
        elegidas: vec![(*ultima, a(Zona::Mano, Sitio::Arriba))],
        resto: resto.to_vec(),
        a: RestoExcavar::AbajoAlAzar,
    });
    assert!(m.zona(YO, Zona::Mano).contains(ultima));
    m.haz(Accion::Buscar { de: YO });
    assert!(m
        .zona(YO, Zona::Biblioteca)
        .iter()
        .all(|c| m.c(*c).vista_por.incluye(Some(YO), YO)));
    m.haz(Accion::DejarDeMirar { de: YO });
    m.haz(Accion::DeArribaRevelada { si: true });
    m.haz(Accion::Barajar);
    assert!(m
        .zona(YO, Zona::Biblioteca)
        .iter()
        .all(|c| m.c(*c).vista_por == Vista::Nadie));
    let mano = m.zona(YO, Zona::Mano).len();
    m.haz(Accion::Rueda { robar: None });
    assert_eq!(m.zona(YO, Zona::Mano).len(), mano);

    // ── Campo entero ──
    m.haz(Accion::EnderezarTodo);
    m.haz(Accion::Organizar);
    m.haz(Accion::CrearFicha {
        de: OrigenFicha::DelDeck { indice: 0 },
        cantidad: 5,
        monton: true,
        pos: None,
    });
    let monton = *m.zona(YO, Zona::Campo).last().unwrap();
    assert_eq!(m.c(monton).cantidad, 5);
    m.haz(Accion::SacarDelMonton { monton, n: 2 });
    assert_eq!(m.c(monton).cantidad, 3);
    m.haz(Accion::CambiarMonton { monton, delta: 4 });
    assert_eq!(m.c(monton).cantidad, 7);
    m.haz(Accion::CrearFicha {
        de: OrigenFicha::Libre {
            nombre: "Soldier".into(),
            fuerza: Some("1".into()),
            resistencia: Some("1".into()),
            colores: vec!['W'],
        },
        cantidad: 2,
        monton: false,
        pos: None,
    });
    let soldados: Vec<IdCarta> = m.zona(YO, Zona::Campo).into_iter().rev().take(2).collect();
    m.haz(Accion::Amontonar {
        cartas: soldados.clone(),
    });
    assert_eq!(m.c(soldados[0]).cantidad, 2);
    assert!(!m.p().cartas.contains_key(&soldados[1]));
    m.haz(Accion::Separar {
        monton: soldados[0],
    });
    assert_eq!(m.c(soldados[0]).cantidad, 1);

    // ── Jugadores ──
    m.haz(Accion::Vida {
        jugador: OTRO,
        delta: -3,
    });
    assert_eq!(m.p().jugador(OTRO).unwrap().vida, 37);
    m.haz(Accion::VidaATodos {
        delta: -1,
        solo_oponentes: true,
    });
    m.haz(Accion::Drenar { n: 2 });
    m.haz(Accion::Extorsion { n: 1 });
    assert_eq!(m.p().jugador(OTRO).unwrap().vida, 37 - 1 - 2 - 1);
    assert_eq!(m.p().jugador(YO).unwrap().vida, 40 + 2 + 1);
    m.haz(Accion::Veneno {
        jugador: OTRO,
        delta: 3,
    });
    let yusri_otro = m.zona(OTRO, Zona::Mando)[0];
    m.haz(Accion::DanoComandante {
        jugador: YO,
        comandante: yusri_otro,
        delta: 5,
    });
    assert_eq!(m.p().jugador(YO).unwrap().dano_comandante[&yusri_otro], 5);
    assert_eq!(m.p().jugador(YO).unwrap().vida, 38);
    m.haz(Accion::ContadorJugador {
        jugador: YO,
        tipo: "speed".into(),
        delta: 9,
    });
    assert_eq!(m.p().jugador(YO).unwrap().contadores["speed"], 4);
    m.haz(Accion::Designar {
        tipo: Designacion::Monarca,
        a: Some(YO),
    });
    m.haz(Accion::DiaNoche {
        valor: Some(DiaNoche::Noche),
    });
    let yusri = m.zona(YO, Zona::Mando)[0];
    m.haz(Accion::Mover {
        cartas: vec![yusri],
        a: Destino {
            zona: ZonaDestino::Comun(ZonaComun::Pila),
            jugador: None,
            sitio: Sitio::Arriba,
        },
        como: Some(ComoMover::Lanzar),
    });
    assert_eq!(m.p().jugador(YO).unwrap().lanzamientos[&yusri], 1);
    m.haz(Accion::Resolver {
        carta: yusri,
        como: ComoResolver::Auto,
    });
    assert!(
        m.zona(YO, Zona::Campo).contains(&yusri),
        "criatura al campo"
    );
    m.haz(Accion::Lanzamientos {
        comandante: yusri,
        delta: 2,
    });
    m.haz(Accion::SinCosteExtra {
        comandante: yusri,
        si: true,
    });
    m.haz(Accion::RevelarMano { si: true });
    let en_mano = m.zona(YO, Zona::Mano)[0];
    m.haz(Accion::RevelarA {
        cartas: vec![en_mano],
        a: vec![OTRO],
    });
    m.haz(Accion::JugarBocaAbajo {
        carta: en_mano,
        pos: None,
    });
    assert!(m.c(en_mano).boca_abajo);
    m.haz(Accion::Descartar {
        modo: ModoDescarte::AlAzar(1),
    });
    assert_eq!(
        m.como(YO, Accion::VidaFantasma { delta: -1 }),
        Err(Rechazo::NoExiste),
        "sin fantasma con dos jugadores"
    );
    m.haz(Accion::Senal {
        senal: Some(Senal::Espera),
    });
    m.haz(Accion::Pausar { si: true });
    m.haz(Accion::Pausar { si: false });
    m.haz(Accion::TurnoExtra { jugador: YO });
    m.haz(Accion::PasarTurno);
    assert_eq!(m.p().turno.activo, YO, "turno extra");
    m.haz(Accion::PasarTurno);
    assert_eq!(m.p().turno.activo, OTRO);
    assert_eq!(m.como(YO, Accion::PasarTurno), Err(Rechazo::FueraDeTurno));
    m.haz(Accion::SaltarTurno { jugador: OTRO });
    assert_eq!(m.p().turno.activo, YO);
    // ── Azar ──
    m.haz(Accion::TirarDados {
        dados: vec![(20, 3), (6, 1)],
    });
    m.haz(Accion::Moneda {
        cuantas: 2,
        hasta_perder: false,
        pulgares: 1,
    });
    m.haz(Accion::DadoPlanar);
    assert_eq!(m.p().turno.tiradas_planar, 1);
    // ── Pila ──
    let inst = *m
        .zona(YO, Zona::Mano)
        .iter()
        .chain(m.zona(YO, Zona::Biblioteca).iter())
        .find(|c| m.c(**c).origen.nombre.starts_with("Shock"))
        .unwrap();
    m.haz(Accion::ApilarDespues { carta: inst });
    m.haz(Accion::Resolver {
        carta: inst,
        como: ComoResolver::Auto,
    });
    assert!(
        m.zona(YO, Zona::Cementerio).contains(&inst),
        "instantáneo al cementerio"
    );
    // ── Exilio, cementerio ──
    m.haz(Accion::Agrupar {
        cartas: vec![ex],
        bajo: Some(oso),
    });
    m.haz(Accion::AlAzarDe {
        zona: Zona::Cementerio,
        filtro: Filtro::default(),
        a: Zona::Exilio,
    });
    m.haz(Accion::TodoAlFondoAlAzar {
        zona: Zona::Cementerio,
    });
    assert!(m.zona(YO, Zona::Cementerio).is_empty());
    m.haz(Accion::Moler { n: 3 });
    m.haz(Accion::BarajarTodoDentro {
        zona: Zona::Cementerio,
    });
    m.haz(Accion::Moler { n: 1 });
    m.haz(Accion::ExiliarTodo {
        zona: Zona::Cementerio,
    });
    // ── Comunicación y fin ──
    m.haz(Accion::Chat {
        texto: "hola".into(),
    });
    assert_eq!(
        m.como(
            YO,
            Accion::Chat {
                texto: "x".repeat(501)
            }
        ),
        Err(Rechazo::Limite)
    );
    m.haz(Accion::Aviso {
        tipo: AvisoRapido::Ok,
    });
    m.haz(Accion::Eliminar {
        jugador: OTRO,
        por: Some(YO),
    });
    assert!(m.p().jugador(OTRO).unwrap().eliminado.is_some());
    m.haz(Accion::Deshacer { hasta: None });
    m.haz(Accion::Rehacer);
    m.haz(Accion::Terminar {
        resultado: Resultado {
            ganadores: vec![YO],
            como: ComoGano::Combate,
            nota: String::new(),
        },
    });
    assert!(m.p().fin.is_some());
    let mut hechas = m.hechas.clone();

    // Lo del solitario y Planechase/Archenemy, en su mesa.
    let mut s = Mesa::nueva(1);
    s.haz(Accion::VidaFantasma { delta: -5 });
    let yusri = s.zona(YO, Zona::Mando)[0];
    s.haz(Accion::DanoComandanteFantasma {
        comandante: yusri,
        delta: 3,
    });
    assert_eq!(s.p().fantasma.as_ref().unwrap().vida, 32);
    s.haz(Accion::VenenoFantasma { delta: 3 });
    s.haz(Accion::VenenoFantasma { delta: -5 });
    assert_eq!(s.p().fantasma.as_ref().unwrap().veneno, 0, "nunca negativo");
    s.haz(Accion::VenenoFantasma { delta: 2 });
    let atacante = s.zona(YO, Zona::Mando)[0];
    s.haz(Accion::Flecha {
        de: Extremo::Carta(atacante),
        a: Extremo::Fantasma,
        tipo: TipoFlecha::Ataque,
    });
    assert_eq!(s.p().flechas.last().unwrap().a, Extremo::Fantasma);
    s.haz(Accion::VerFantasma { si: false });
    s.haz(Accion::IntensidadSparring {
        intensidad: Intensidad::Realista,
    });
    assert_eq!(
        s.como(
            YO,
            Accion::DecidirSparring {
                reaccion: 1,
                decision: Decision::Aceptar,
                eleccion: None
            }
        ),
        Err(Rechazo::NoExiste)
    );
    s.hechas.insert("DecidirSparring");
    s.haz(Accion::Conceder);
    hechas.extend(s.hechas);
    // Planos y planes: una partida con ellos.
    let mut cfg = config(1);
    cfg.variantes.planechase = true;
    for (i, nom) in ["Plano A", "Plano B"].iter().enumerate() {
        let mut o = origen(nom, "Plane — X", 0);
        o.carta_deck = format!("pl{i}");
        cfg.jugadores[0]
            .mazo
            .cartas
            .push(mesa::preparar::CartaDeMesa {
                carta_deck: o.carta_deck.clone(),
                seccion: mesa::preparar::Seccion::Plano,
                origen: o,
            });
    }
    let plan = origen("Plan", "Scheme", 0);
    cfg.jugadores[0]
        .mazo
        .cartas
        .push(mesa::preparar::CartaDeMesa {
            carta_deck: "plan".into(),
            seccion: mesa::preparar::Seccion::Plan,
            origen: plan,
        });
    let p = mesa::preparar::iniciar("x".into(), cfg, 1, 0).unwrap();
    let mut pc = Mesa {
        r: Registro::nuevo(p, ModoDeshacer::SinLimite),
        azar: Azar::new(3),
        hechas: BTreeSet::new(),
    };
    pc.haz(Accion::Mulligan);
    assert_eq!(pc.p().jugador(YO).unwrap().mulligans, 1);
    pc.haz(Accion::QuedarseCon { al_fondo: vec![] });
    let activo = pc
        .p()
        .zona(&RefZona::comun(ZonaComun::PlanoActivo))
        .to_vec();
    assert_eq!(activo.len(), 1);
    pc.haz(Accion::Planeswalk);
    assert_ne!(pc.p().zona(&RefZona::comun(ZonaComun::PlanoActivo)), activo);
    pc.haz(Accion::PonerEnMarcha);
    let plan = pc.zona(YO, Zona::PlanesEnMarcha)[0];
    pc.haz(Accion::AbandonarPlan { carta: plan });
    hechas.extend(pc.hechas.clone());
    // Reiniciar lo hace la sesión: el motor lo rechaza.
    assert_eq!(pc.como(YO, Accion::Reiniciar), Err(Rechazo::NoSePuede));
    hechas.insert("Reiniciar");

    let todas: BTreeSet<&str> = TODAS.iter().copied().collect();
    let faltan: Vec<_> = todas.difference(&hechas).collect();
    assert!(faltan.is_empty(), "acciones sin probar: {faltan:?}");
}

const TODAS: &[&str] = &[
    "Mover",
    "Girar",
    "BocaAbajo",
    "CambiarCara",
    "Contador",
    "FijarContador",
    "MultiplicarContadores",
    "ContadorClave",
    "Pt",
    "MasXMasY",
    "Etiqueta",
    "PonerBolita",
    "MoverBolita",
    "QuitarBolita",
    "Adjuntar",
    "FaseFuera",
    "NoEndereza",
    "Resaltar",
    "CopiaFicha",
    "DarControl",
    "TomarControl",
    "PorJugador",
    "MirarMano",
    "HabilidadEnPila",
    "Flecha",
    "QuitarFlechas",
    "Robar",
    "Moler",
    "Manifestar",
    "ExiliarDeArriba",
    "Mirar",
    "Ordenar",
    "RevelarArriba",
    "OcultarArriba",
    "Excavar",
    "ResolverExcavacion",
    "Buscar",
    "DejarDeMirar",
    "DeArribaRevelada",
    "Barajar",
    "Rueda",
    "EnderezarTodo",
    "Organizar",
    "CrearFicha",
    "Mulligan",
    "QuedarseCon",
    "Vida",
    "VidaATodos",
    "Drenar",
    "Extorsion",
    "Veneno",
    "DanoComandante",
    "ContadorJugador",
    "Designar",
    "DiaNoche",
    "Lanzamientos",
    "SinCosteExtra",
    "RevelarMano",
    "RevelarA",
    "MostrarATodos",
    "DejarDeMostrar",
    "JugarBocaAbajo",
    "Descartar",
    "Eliminar",
    "Conceder",
    "VidaFantasma",
    "DanoComandanteFantasma",
    "VerFantasma",
    "Senal",
    "PasarTurno",
    "TurnoExtra",
    "Pausar",
    "SaltarTurno",
    "Reiniciar",
    "TirarDados",
    "Moneda",
    "DadoPlanar",
    "Planeswalk",
    "PonerEnMarcha",
    "AbandonarPlan",
    "Resolver",
    "ApilarDespues",
    "Amontonar",
    "SacarDelMonton",
    "CambiarMonton",
    "Separar",
    "Agrupar",
    "AlAzarDe",
    "TodoAlFondoAlAzar",
    "BarajarTodoDentro",
    "ExiliarTodo",
    "IntensidadSparring",
    "DecidirSparring",
    "Chat",
    "Aviso",
    "PonerNota",
    "EditarNota",
    "QuitarNota",
    "Colocar",
    "NotaEnCarta",
    "AtarNota",
    "VenenoFantasma",
    "Terminar",
    "Deshacer",
    "Rehacer",
];

#[test]
fn ida_y_vuelta_json_de_cada_accion() {
    let ejemplos = vec![
        Accion::Mover {
            cartas: vec![IdCarta(1)],
            a: campo(Some(Pos::nueva(1, 2))),
            como: Some(ComoMover::Lanzar),
        },
        Accion::Excavar {
            forma: FormaExcavar::RevelarN(3),
            filtro: Filtro {
                cmc: Some((Comparacion::Menor, 4)),
                ..Filtro::default()
            },
            parar_tras: 1,
        },
        Accion::Descartar {
            modo: ModoDescarte::Elegidas(vec![IdCarta(3)]),
        },
        Accion::CrearFicha {
            de: OrigenFicha::Libre {
                nombre: "X".into(),
                fuerza: None,
                resistencia: None,
                colores: vec![],
            },
            cantidad: 1,
            monton: false,
            pos: None,
        },
        Accion::Deshacer { hasta: Some(4) },
    ];
    for a in ejemplos {
        let j = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Accion>(&j).unwrap(), a, "{j}");
    }
    assert_eq!(TODAS.len(), TODAS.iter().collect::<BTreeSet<_>>().len());
}

#[test]
fn robar_lo_controlado_por_otro_vuelve_al_dueno() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    m.haz(Accion::DarControl {
        cartas: vec![oso],
        a: OTRO,
    });
    m.haz(Accion::PasarTurno);
    // El otro lo manda «a su mano»: va a la mía.
    m.como(
        OTRO,
        Accion::Mover {
            cartas: vec![oso],
            a: a(Zona::Mano, Sitio::Arriba),
            como: None,
        },
    )
    .unwrap();
    assert!(m.zona(YO, Zona::Mano).contains(&oso));
    assert_eq!(m.c(oso).controlador, YO);
}

#[test]
fn sin_permiso_no_se_toca_lo_ajeno() {
    // Lo oculto de otro (su mano) no se toca mientras no se ve.
    let mut m = Mesa::nueva(2);
    let suya = m.zona(OTRO, Zona::Mano)[0];
    assert_eq!(
        m.como(
            YO,
            Accion::Mover {
                cartas: vec![suya],
                a: campo(None),
                como: None
            }
        ),
        Err(Rechazo::SinPermiso)
    );
    // Mirada (Thoughtseize), sí: la descarto y va a su cementerio.
    m.haz(Accion::MirarMano { de: OTRO, si: true });
    m.haz(Accion::Mover {
        cartas: vec![suya],
        a: a(Zona::Cementerio, Sitio::Arriba),
        como: None,
    });
    assert!(m.zona(OTRO, Zona::Cementerio).contains(&suya));
    // Mirar su biblioteca también se puede (Gonti, Bribery…).
    m.haz(Accion::Mirar {
        de: OTRO,
        n: 1,
        desde: Lado::Arriba,
    });
    // Y su vida, siempre.
    m.haz(Accion::Vida {
        jugador: OTRO,
        delta: -2,
    });
}

#[test]
fn lo_que_esta_a_la_vista_es_de_todos() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    // El otro gira mi criatura, le pone contadores y la manda a mi mano.
    for accion in [
        Accion::Girar {
            cartas: vec![oso],
            girada: true,
        },
        Accion::Contador {
            cartas: vec![oso],
            tipo: "-1/-1".into(),
            delta: 1,
        },
    ] {
        m.como(OTRO, accion).unwrap();
    }
    assert!(m.c(oso).girada);
    // Y la copia que hace de ella es suya (Clone).
    m.como(OTRO, Accion::CopiaFicha { cartas: vec![oso] })
        .unwrap();
    let copia = *m.zona(OTRO, Zona::Campo).last().unwrap();
    assert_eq!(m.c(copia).controlador, OTRO);
    m.como(
        OTRO,
        Accion::Mover {
            cartas: vec![oso],
            a: a(Zona::Mano, Sitio::Arriba),
            como: None,
        },
    )
    .unwrap();
    assert!(
        m.zona(YO, Zona::Mano).contains(&oso),
        "a la mano de su dueño"
    );
}

#[test]
fn por_otro_jugador_como_si_lo_hiciera_el() {
    let mut m = Mesa::nueva(2);
    let por = |accion: Accion| Accion::PorJugador {
        jugador: OTRO,
        accion: Box::new(accion),
    };
    let lib = m.zona(OTRO, Zona::Biblioteca).len();
    m.haz(por(Accion::Moler { n: 2 }));
    assert_eq!(m.zona(OTRO, Zona::Biblioteca).len(), lib - 2);
    assert_eq!(m.zona(OTRO, Zona::Cementerio).len(), 2);
    // Bojuka Bog: exiliar su cementerio.
    m.haz(por(Accion::ExiliarTodo {
        zona: Zona::Cementerio,
    }));
    assert!(m.zona(OTRO, Zona::Cementerio).is_empty());
    // Una ficha para él (Beast Within, Forbidden Orchard…).
    m.haz(por(Accion::CrearFicha {
        de: OrigenFicha::Libre {
            nombre: "Beast".into(),
            fuerza: Some("3".into()),
            resistencia: Some("3".into()),
            colores: vec!['G'],
        },
        cantidad: 1,
        monton: false,
        pos: None,
    }));
    let ficha = *m.zona(OTRO, Zona::Campo).last().unwrap();
    assert_eq!(m.c(ficha).controlador, OTRO);
    // Descarte al azar (Hymn to Tourach).
    let mano = m.zona(OTRO, Zona::Mano).len();
    m.haz(por(Accion::Descartar {
        modo: ModoDescarte::AlAzar(1),
    }));
    assert_eq!(m.zona(OTRO, Zona::Mano).len(), mano - 1);
    // Lo del turno no se hace por nadie.
    assert_eq!(m.como(YO, por(Accion::PasarTurno)), Err(Rechazo::NoSePuede));
}

#[test]
fn al_salir_del_campo_pierde_todo_y_las_fichas_desaparecen() {
    let mut m = Mesa::nueva(1);
    let oso = m.al_campo("Bear");
    m.haz(Accion::Contador {
        cartas: vec![oso],
        tipo: "+1/+1".into(),
        delta: 1,
    });
    m.haz(Accion::Girar {
        cartas: vec![oso],
        girada: true,
    });
    m.haz(Accion::CopiaFicha { cartas: vec![oso] });
    let ficha = *m.zona(YO, Zona::Campo).last().unwrap();
    m.haz(Accion::Mover {
        cartas: vec![oso, ficha],
        a: a(Zona::Cementerio, Sitio::Arriba),
        como: None,
    });
    let c = m.c(oso);
    assert!(!c.girada && c.contadores.is_empty() && c.pos.is_none());
    assert!(!m.p().cartas.contains_key(&ficha));
    assert_eq!(m.zona(YO, Zona::Cementerio), vec![oso]);
}

#[test]
fn quitar_mas_dano_de_comandante_del_apuntado_solo_devuelve_lo_apuntado() {
    let mut m = Mesa::nueva(2);
    let de_otro = m.zona(OTRO, Zona::Mando)[0];
    let vida = m.p().jugador(YO).unwrap().vida;
    m.haz(Accion::DanoComandante {
        jugador: YO,
        comandante: de_otro,
        delta: 3,
    });
    assert_eq!(m.p().jugador(YO).unwrap().vida, vida - 3);
    m.haz(Accion::DanoComandante {
        jugador: YO,
        comandante: de_otro,
        delta: -5,
    });
    let yo = m.p().jugador(YO).unwrap();
    assert!(yo.dano_comandante.is_empty());
    assert_eq!(yo.vida, vida, "recupera los 3 que había, no 5");
    // Sin nada apuntado, quitar no cambia la vida.
    m.haz(Accion::DanoComandante {
        jugador: YO,
        comandante: de_otro,
        delta: -4,
    });
    assert_eq!(m.p().jugador(YO).unwrap().vida, vida);
}

#[test]
fn lo_mismo_con_el_dano_de_comandante_del_fantasma() {
    let mut m = Mesa::nueva(1);
    let yusri = m.zona(YO, Zona::Mando)[0];
    let vida = m.p().fantasma.as_ref().unwrap().vida;
    m.haz(Accion::DanoComandanteFantasma {
        comandante: yusri,
        delta: 3,
    });
    m.haz(Accion::DanoComandanteFantasma {
        comandante: yusri,
        delta: -5,
    });
    let f = m.p().fantasma.as_ref().unwrap();
    assert!(f.dano_comandante.is_empty());
    assert_eq!(f.vida, vida);
}

#[test]
fn una_carta_puesta_arriba_es_la_siguiente_que_se_roba_mientras_no_se_baraje() {
    let mut m = Mesa::nueva(1);
    // Un tutor: una carta cualquiera de la biblioteca, arriba.
    let elegida = m.zona(YO, Zona::Biblioteca)[7];
    m.haz(Accion::Mover {
        cartas: vec![elegida],
        a: a(Zona::Biblioteca, Sitio::Arriba),
        como: None,
    });
    assert_eq!(m.zona(YO, Zona::Biblioteca)[0], elegida);
    m.haz(Accion::Robar { n: 1 });
    assert_eq!(m.zona(YO, Zona::Mano).last(), Some(&elegida));
    // Robada por error: vuelve arriba y es la que se roba otra vez.
    m.haz(Accion::Mover {
        cartas: vec![elegida],
        a: a(Zona::Biblioteca, Sitio::Arriba),
        como: None,
    });
    assert_eq!(m.zona(YO, Zona::Biblioteca)[0], elegida);
    m.haz(Accion::Robar { n: 1 });
    assert_eq!(m.zona(YO, Zona::Mano).last(), Some(&elegida));
    // Con varias a la vez, la última de la lista queda arriba del todo.
    let lib = m.zona(YO, Zona::Biblioteca);
    let (x, y) = (lib[3], lib[4]);
    m.haz(Accion::Mover {
        cartas: vec![x, y],
        a: a(Zona::Biblioteca, Sitio::Arriba),
        como: None,
    });
    assert_eq!(&m.zona(YO, Zona::Biblioteca)[..2], &[y, x]);
}

#[test]
fn ayudas_al_pasar_el_turno_y_contadores_al_entrar() {
    let mut m = Mesa::nueva(2);
    let pw = m.al_campo("Garruk");
    assert_eq!(m.c(pw).contadores.get("loyalty"), Some(&3));
    let saga = m.al_campo("Saga");
    assert_eq!(m.c(saga).contadores.get("lore"), Some(&1));
    let oso = m.al_campo("Bear");
    m.haz(Accion::Girar {
        cartas: vec![oso],
        girada: true,
    });
    m.haz(Accion::MasXMasY {
        cartas: vec![oso],
        x: 1,
        y: 1,
        fila: FilaPt::HastaFinDeTurno,
    });
    let mano_otro = m.zona(OTRO, Zona::Mano).len();
    m.haz(Accion::PasarTurno);
    assert_eq!(m.c(oso).pt.hasta_fin_de_turno, (0, 0));
    assert_eq!(
        m.zona(OTRO, Zona::Mano).len(),
        mano_otro + 1,
        "roba al empezar"
    );
    assert!(m.c(oso).girada, "lo mío no se endereza en su turno");
    m.como(OTRO, Accion::PasarTurno).unwrap();
    assert!(!m.c(oso).girada, "se endereza en el mío");
    assert_eq!(m.p().turno.ronda, 2);
}

#[test]
fn los_contadores_temporales_se_pierden_al_pasar_el_turno() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    for tipo in ["~mana-R", "+1/+1", "~lealtad"] {
        m.haz(Accion::Contador {
            cartas: vec![oso],
            tipo: tipo.into(),
            delta: 3,
        });
    }
    // Se editan como cualquier otro: fijar el valor y dejarlo en 0 lo quita.
    m.haz(Accion::FijarContador {
        carta: oso,
        tipo: "~mana-R".into(),
        valor: 1,
    });
    assert_eq!(m.c(oso).contadores["~mana-R"], 1);
    m.haz(Accion::FijarContador {
        carta: oso,
        tipo: "~lealtad".into(),
        valor: 0,
    });
    assert!(!m.c(oso).contadores.contains_key("~lealtad"));
    for palabra in ["~Flying", "Haste"] {
        m.haz(Accion::ContadorClave {
            cartas: vec![oso],
            palabra: palabra.into(),
            si: true,
        });
    }
    m.haz(Accion::PasarTurno);
    assert!(
        !m.c(oso).contadores.contains_key("~mana-R"),
        "el temporal no sobrevive al fin del turno"
    );
    assert_eq!(m.c(oso).contadores["+1/+1"], 3, "el permanente sí");
    assert!(!m.c(oso).palabras_clave.contains("~Flying"));
    assert!(m.c(oso).palabras_clave.contains("Haste"));
}

fn con_contador(m: &mut Mesa, id: IdCarta, tipo: &str, delta: i32) {
    m.haz(Accion::Contador {
        cartas: vec![id],
        tipo: tipo.into(),
        delta,
    });
}

#[test]
fn al_dar_el_control_van_con_ella_sus_contadores_y_lo_que_lleva_adjunto() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    let espada = m.al_campo("Forest");
    let bicho = m.al_campo("Bear");
    con_contador(&mut m, oso, "+1/+1", 3);
    con_contador(&mut m, espada, "oil", 2);
    m.haz(Accion::Adjuntar {
        carta: espada,
        a: Some(oso),
    });
    m.haz(Accion::Amontonar {
        cartas: vec![oso, bicho],
    });
    m.haz(Accion::DarControl {
        cartas: vec![oso],
        a: OTRO,
    });
    for id in [oso, espada, bicho] {
        assert!(
            m.zona(OTRO, Zona::Campo).contains(&id),
            "{id:?} en su campo"
        );
        assert!(!m.zona(YO, Zona::Campo).contains(&id));
        assert_eq!(m.c(id).controlador, OTRO);
    }
    assert_eq!(
        m.c(oso).contadores["+1/+1"],
        3,
        "sus contadores van con ella"
    );
    assert_eq!(m.c(espada).contadores["oil"], 2, "y los del equipo");
    assert_eq!(m.c(espada).adjunta_a, Some(oso), "sigue adjunto");
    assert_eq!(m.c(bicho).agrupada_bajo, Some(oso), "y amontonado");
    // El dueño sigue siendo el mismo.
    assert_eq!(m.c(oso).dueno, YO);
}

#[test]
fn lo_adjunto_de_otro_se_queda_en_su_campo_al_cambiar_de_control() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    let aura = m.al_campo("Forest");
    m.haz(Accion::DarControl {
        cartas: vec![oso],
        a: OTRO,
    });
    // Mi aura, en mi campo, sobre la criatura que ahora controla el otro.
    m.haz(Accion::Adjuntar {
        carta: aura,
        a: Some(oso),
    });
    // Cuando el otro me la devuelve, lo mío ya estaba aquí: no se mueve ni se suelta.
    m.como(
        OTRO,
        Accion::DarControl {
            cartas: vec![oso],
            a: YO,
        },
    )
    .unwrap();
    assert_eq!(m.c(aura).adjunta_a, Some(oso), "sigue adjunta");
    assert_eq!(m.c(aura).controlador, YO);
    assert!(m.zona(YO, Zona::Campo).contains(&aura));
}

#[test]
fn el_control_robado_hasta_el_final_del_turno_vuelve_al_pasar_el_turno() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    m.haz(Accion::DarControl {
        cartas: vec![oso],
        a: OTRO,
    });
    m.haz(Accion::TomarControl {
        cartas: vec![oso],
        hasta_fin_de_turno: true,
    });
    assert_eq!(m.c(oso).controlador, YO);
    assert_eq!(m.c(oso).devolver_a, Some(OTRO));
    // Tomar lo que ya es mío no hace nada.
    assert_eq!(
        m.como(
            YO,
            Accion::TomarControl {
                cartas: vec![oso],
                hasta_fin_de_turno: false
            }
        ),
        Err(Rechazo::NoSePuede)
    );
    m.haz(Accion::PasarTurno);
    assert_eq!(m.c(oso).controlador, OTRO, "vuelve al pasar el turno");
    assert!(m.zona(OTRO, Zona::Campo).contains(&oso));
    assert_eq!(m.c(oso).devolver_a, None);
}

#[test]
fn tomar_el_control_para_siempre_no_se_devuelve() {
    let mut m = Mesa::nueva(2);
    let oso = m.al_campo("Bear");
    m.haz(Accion::DarControl {
        cartas: vec![oso],
        a: OTRO,
    });
    m.haz(Accion::TomarControl {
        cartas: vec![oso],
        hasta_fin_de_turno: false,
    });
    m.haz(Accion::PasarTurno);
    assert_eq!(m.c(oso).controlador, YO);
    assert_eq!(m.c(oso).devolver_a, None);
}

#[test]
fn se_roba_del_cementerio_ajeno_al_campo_y_lo_que_va_a_una_mano_va_a_la_de_su_dueno() {
    let mut m = Mesa::nueva(2);
    let id = *m.zona(OTRO, Zona::Biblioteca).first().unwrap();
    m.como(
        OTRO,
        Accion::Mover {
            cartas: vec![id],
            a: a(Zona::Cementerio, Sitio::Arriba),
            como: None,
        },
    )
    .unwrap();
    // A mi campo: sí (reanimar lo del otro).
    m.haz(Accion::Mover {
        cartas: vec![id],
        a: campo(None),
        como: None,
    });
    assert!(m.zona(YO, Zona::Campo).contains(&id));
    assert_eq!(m.c(id).controlador, YO);
    assert_eq!(m.c(id).dueno, OTRO, "sigue siendo del otro");
    // Y si va a la mano, vuelve a la del dueño.
    m.haz(Accion::Mover {
        cartas: vec![id],
        a: a(Zona::Mano, Sitio::Arriba),
        como: None,
    });
    assert!(m.zona(OTRO, Zona::Mano).contains(&id));
}

#[test]
fn una_carta_de_la_mano_se_muestra_a_todos_y_se_vuelve_a_tapar() {
    use mesa::visibilidad::{ve, Mirador};
    let mut m = Mesa::nueva(2);
    let id = m.zona(YO, Zona::Mano)[0];
    let veria = |m: &Mesa, mirador| {
        let p = m.p();
        let (zona, i) = p.donde(id).unwrap();
        ve(p, &p.cartas[&id], &zona, i, mirador)
    };
    assert!(!veria(&m, Mirador::Jugador(OTRO)), "oculta");
    m.haz(Accion::MostrarATodos { cartas: vec![id] });
    assert!(veria(&m, Mirador::Jugador(OTRO)), "la ve el otro");
    assert!(veria(&m, Mirador::Espectador), "y quien solo mira");
    m.haz(Accion::DejarDeMostrar { cartas: vec![id] });
    assert!(!veria(&m, Mirador::Jugador(OTRO)), "tapada otra vez");
    assert!(veria(&m, Mirador::Jugador(YO)), "yo la sigo viendo");
    // No se muestra lo ajeno que no se ve.
    let suya = m.zona(OTRO, Zona::Mano)[0];
    assert_eq!(
        m.como(YO, Accion::MostrarATodos { cartas: vec![suya] }),
        Err(Rechazo::SinPermiso)
    );
}
