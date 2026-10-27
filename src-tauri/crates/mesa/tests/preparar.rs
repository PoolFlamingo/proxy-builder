//! Del deck de la app a la mesa (A2, A3).

mod comun;

use comun::*;
use mesa::lista::ListaDeJuego;
use mesa::modelo::{RefImagen, RefZona, Zona};
use mesa::preparar::{candidatas_de_lista, secciones_de_mesa, CartaDeMesa, Seccion};
use mesa::IdJugador;
use proxy_core::diseno::modelo::CardDesign;
use proxy_core::model::{
    BackSpec, CardEdits, CardMeta, CardSource, Deck, DeckCard, MarcaCarta, PrintSettings,
};

fn carta(nombre: &str) -> DeckCard {
    DeckCard {
        id: format!("{nombre}-{}", nombre.len()),
        source: CardSource::Scryfall {
            scryfall_id: "x".into(),
        },
        front: None,
        back: BackSpec::DeckDefault,
        edits: CardEdits::default(),
        locked: false,
        meta: CardMeta {
            name: nombre.into(),
            type_line: Some("Creature".into()),
            ..CardMeta::default()
        },
        design: None,
        marca: None,
    }
}

/// El deck de los tests de `exportar.rs` de proxy-core.
fn yusri() -> Deck {
    let mut ficha = carta("Treasure");
    ficha.source = CardSource::Token {
        scryfall_id: "t".into(),
    };
    let mut sol2 = carta("Sol Ring");
    sol2.id = "sol2".into();
    Deck {
        id: "d".into(),
        name: "Yusri".into(),
        decklist_text: "Commander\n1 Yusri, Fortune's Flame\n\nDeck\n2 Sol Ring\n1 Delver of Secrets\n\nSideboard\n1 Pyroblast\n".into(),
        cards: vec![
            carta("Yusri, Fortune's Flame"),
            carta("Sol Ring"),
            sol2,
            carta("Delver of Secrets // Insectile Aberration"),
            carta("Pyroblast"),
            ficha,
            DeckCard {
                source: CardSource::Blank,
                ..carta("")
            },
        ],
        settings: PrintSettings::default(),
        default_back: None,
        schema_version: 1,
    }
}

fn de(deck: &Deck, s: Seccion) -> Vec<String> {
    secciones_de_mesa(deck, None)
        .into_iter()
        .filter(|(_, x)| *x == s)
        .map(|(c, _)| c.meta.name.clone())
        .collect()
}

#[test]
fn las_secciones_salen_de_la_lista() {
    let d = yusri();
    assert_eq!(de(&d, Seccion::Comandante), vec!["Yusri, Fortune's Flame"]);
    assert_eq!(de(&d, Seccion::Banquillo), vec!["Pyroblast"]);
    assert_eq!(de(&d, Seccion::Ficha), vec!["Treasure"]);
    assert_eq!(de(&d, Seccion::Mazo).len(), 3);
    assert_eq!(
        secciones_de_mesa(&d, None).len(),
        6,
        "el hueco en blanco, fuera"
    );
}

#[test]
fn las_marcas_cuentan_como_cuentan() {
    let mut d = yusri();
    let mut v1 = carta("Brainstorm");
    v1.marca = Some(MarcaCarta::Variante);
    let mut v2 = carta("Brainstorm");
    v2.id = "b2".into();
    v2.marca = Some(MarcaCarta::Variante);
    let mut op = carta("Opt");
    op.marca = Some(MarcaCarta::Opcional);
    let mut sb = carta("Lightning Bolt");
    sb.marca = Some(MarcaCarta::Banquillo);
    d.cards.extend([v1, v2, op, sb]);
    assert_eq!(
        de(&d, Seccion::Mazo)
            .iter()
            .filter(|n| *n == "Brainstorm")
            .count(),
        1
    );
    assert!(!de(&d, Seccion::Mazo).contains(&"Opt".to_string()));
    assert!(de(&d, Seccion::Banquillo).contains(&"Lightning Bolt".to_string()));
}

#[test]
fn con_una_lista_solo_va_lo_elegido() {
    let mut d = yusri();
    // Otra variante de diseño de Yusri y una alternativa que no se juega.
    let mut yusri2 = carta("Yusri, Fortune's Flame");
    yusri2.id = "yusri-2".into();
    yusri2.marca = Some(MarcaCarta::Variante);
    let mut sb = carta("Lightning Bolt");
    sb.marca = Some(MarcaCarta::Banquillo);
    d.cards.extend([yusri2, carta("Opt"), sb]);
    let lista = ListaDeJuego {
        id: "l".into(),
        nombre: "Torneo".into(),
        comandantes: vec!["yusri-2".into()],
        cartas: vec!["sol2".into(), "Opt-3".into()],
    };
    let de_lista = |s: Seccion| -> Vec<String> {
        secciones_de_mesa(&d, Some(&lista))
            .into_iter()
            .filter(|(_, x)| *x == s)
            .map(|(c, _)| c.id.clone())
            .collect()
    };
    assert_eq!(de_lista(Seccion::Comandante), vec!["yusri-2"]);
    assert_eq!(de_lista(Seccion::Mazo), vec!["sol2", "Opt-3"]);
    // Fichas y banquillo marcado siguen; el Pyroblast de la sección
    // Sideboard de la lista escrita no estaba elegido ni marcado: fuera.
    assert_eq!(de_lista(Seccion::Ficha).len(), 1);
    assert_eq!(de_lista(Seccion::Banquillo), vec!["Lightning Bolt-14"]);

    // Un compañero elegido sigue yendo aparte.
    let mut d2 = yusri();
    d2.decklist_text
        .push_str("\nCompanion\n1 Lurrus of the Dream-Den\n");
    d2.cards.push(carta("Lurrus of the Dream-Den"));
    let con_lurrus = ListaDeJuego {
        cartas: vec!["Lurrus of the Dream-Den-23".into()],
        ..lista.clone()
    };
    assert_eq!(
        secciones_de_mesa(&d2, Some(&con_lurrus))
            .into_iter()
            .filter(|(_, x)| *x == Seccion::Companero)
            .count(),
        1
    );
}

#[test]
fn un_plano_disenado_va_a_los_planos() {
    let mut diseno: CardDesign = serde_json::from_str(include_str!(
        "../../proxy-core/tests/compatibilidad/diseno-2.5.9.json"
    ))
    .unwrap();
    diseno.card.layout = "planar".into();
    let mut d = yusri();
    let mut plano = carta("Tazeem");
    plano.design = Some(diseno);
    let mut plan = carta("Behold the Power of Destruction");
    plan.meta.layout = Some("scheme".into());
    d.cards.extend([plano, plan]);
    assert_eq!(de(&d, Seccion::Plano), vec!["Tazeem"]);
    assert_eq!(
        de(&d, Seccion::Plan),
        vec!["Behold the Power of Destruction"]
    );
}

#[test]
fn iniciar_reparte_la_mesa() {
    let p = partida(2);
    let yo = IdJugador(0);
    assert_eq!(p.zona(&RefZona::de(yo, Zona::Mano)).len(), 7);
    assert_eq!(p.zona(&RefZona::de(yo, Zona::Biblioteca)).len(), 40 - 7);
    assert_eq!(p.zona(&RefZona::de(yo, Zona::Mando)).len(), 1);
    assert_eq!(p.zona(&RefZona::de(yo, Zona::Banquillo)).len(), 1);
    assert_eq!(p.jugadores[0].fichas.len(), 1);
    assert_eq!(p.jugadores[0].vida, 40);
    assert!(p.mulligan.is_some() && p.fantasma.is_none());
    let yusri = p.zona(&RefZona::de(yo, Zona::Mando))[0];
    assert!(p.cartas[&yusri].origen.info.es_comandante);
    // Ida y vuelta en JSON.
    let j = serde_json::to_string(&p).unwrap();
    let q: mesa::Partida = serde_json::from_str(&j).unwrap();
    assert_eq!(p, q);
    // Solitario: con fantasma; once jugadores: no.
    assert!(partida(1).fantasma.is_some());
    assert!(mesa::preparar::iniciar("x".into(), config(11), 1, 0).is_err());
    // La misma semilla, la misma partida.
    assert_eq!(mesa::huella(&partida(3)), mesa::huella(&partida(3)));
}

#[test]
fn las_mazmorras_son_fichas_y_no_cuentan_para_el_mazo() {
    let mut d = yusri();
    let antes = de(&d, Seccion::Mazo).len();
    let mut mina = carta("Lost Mine of Phandelver");
    mina.meta.type_line = Some("Dungeon".into());
    let mut loca = carta("Dungeon of the Mad Mage");
    loca.meta.type_line = Some("Dungeon".into());
    d.cards.extend([mina, loca]);
    assert_eq!(de(&d, Seccion::Mazo).len(), antes);
    assert_eq!(
        de(&d, Seccion::Ficha),
        vec![
            "Treasure",
            "Lost Mine of Phandelver",
            "Dungeon of the Mad Mage"
        ]
    );
    // Ni se pueden elegir para una lista para jugar: no son del mazo.
    let candidatas: Vec<&str> = candidatas_de_lista(&d)
        .iter()
        .map(|c| c.meta.name.as_str())
        .collect();
    assert!(!candidatas
        .iter()
        .any(|n| n.contains("Mine") || n.contains("Dungeon")));
}

#[test]
fn las_fichas_propias_disenadas_como_ficha_y_sus_variantes_van_a_las_fichas() {
    let mut diseno: CardDesign = serde_json::from_str(include_str!(
        "../../proxy-core/tests/compatibilidad/diseno-2.5.9.json"
    ))
    .unwrap();
    diseno.card.layout = "token".into();
    let mut d = yusri();
    let propia = |id: &str, marca: Option<MarcaCarta>| {
        let mut c = carta("Mi ficha");
        c.id = id.into();
        c.source = CardSource::Custom {
            scryfall_id: None,
            externa: None,
        };
        c.design = Some(diseno.clone());
        c.marca = marca;
        c
    };
    d.cards.extend([
        propia("a", None),
        propia("b", Some(MarcaCarta::Variante)),
        propia("c", Some(MarcaCarta::Variante)),
    ]);
    let fichas = de(&d, Seccion::Ficha);
    assert_eq!(fichas.iter().filter(|n| *n == "Mi ficha").count(), 3);
    assert_eq!(de(&d, Seccion::Mazo).len(), 3, "no entran en el mazo");
    assert!(!candidatas_de_lista(&d)
        .iter()
        .any(|c| c.meta.name == "Mi ficha"));
}

#[test]
fn en_la_partida_salen_las_variantes_de_una_ficha_y_las_copias_iguales_una_vez() {
    let mut config = config(1);
    let mazo = &mut config.jugadores[0].mazo;
    mazo.cartas.retain(|c| c.seccion != Seccion::Ficha);
    let ficha = |id: &str, imagen: &str| {
        let mut o = origen("Mi ficha", "Token Creature — Soldier", 0);
        o.carta_deck = id.into();
        o.caras = vec![RefImagen::Propia {
            sha256: format!("{imagen:0>64}"),
            ancho: 745,
            alto: 1040,
        }];
        CartaDeMesa {
            carta_deck: id.into(),
            seccion: Seccion::Ficha,
            origen: o,
        }
    };
    // Dos copias iguales y dos variantes con otra imagen.
    mazo.cartas.extend([
        ficha("a", "1"),
        ficha("b", "1"),
        ficha("c", "2"),
        ficha("d", "3"),
    ]);
    let p = mesa::preparar::iniciar("p".into(), config, 1, 0).unwrap();
    let nombres: Vec<&str> = p.jugadores[0]
        .fichas
        .iter()
        .map(|f| f.carta_deck.as_str())
        .collect();
    assert_eq!(nombres, vec!["a", "c", "d"]);
}
