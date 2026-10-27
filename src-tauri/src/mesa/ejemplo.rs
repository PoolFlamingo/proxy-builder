//! **Un deck de ejemplo para la mesa**, sin red: cartas diseñadas con el
//! editor (marco, nombre, tipo y reglas pintados por el motor). Sirve para
//! probar la mesa y sacar capturas en una máquina sin Scryfall:
//!
//! `PB_DATOS=~/.local/share/com.estigio.proxybuilder PB_CACHE=~/.cache/com.estigio.proxybuilder cargo test --lib deck_de_ejemplo -- --ignored`
//!
//! `deck_grande_de_ejemplo` hace otro de 110 cartas (dos variantes del
//! comandante, alternativas, básicas) para probar las listas para jugar.

#[cfg(test)]
mod tests {
    use proxy_core::diseno::modelo::{ByLang, CardData, FaceData};
    use proxy_core::model::{
        BackSpec, CardEdits, CardMeta, CardSource, Deck, DeckCard, PrintSettings,
        DECK_SCHEMA_VERSION,
    };

    use crate::state::{AppState, Paths};

    fn carta(
        i: usize,
        nombre: &str,
        coste: &str,
        tipo: (&str, &str),
        reglas: &str,
        pt: Option<(&str, &str)>,
    ) -> DeckCard {
        let mut c = DeckCard {
            id: format!("ejemplo-{i}"),
            source: CardSource::Custom {
                scryfall_id: None,
                externa: None,
            },
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: nombre.into(),
                type_line: Some(tipo.1.into()),
                ..CardMeta::default()
            },
            design: None,
            marca: None,
        };
        let datos = CardData {
            scryfall_id: String::new(),
            layout: "normal".into(),
            rarity: "common".into(),
            set: "pb".into(),
            collector_number: format!("{i}"),
            faces: vec![FaceData {
                name: nombre.into(),
                mana_cost: coste.into(),
                type_line: ByLang {
                    es: tipo.0.into(),
                    en: tipo.1.into(),
                    otros: Default::default(),
                },
                rules: ByLang {
                    es: reglas.into(),
                    en: reglas.into(),
                    otros: Default::default(),
                },
                power: pt.map(|p| p.0.into()),
                toughness: pt.map(|p| p.1.into()),
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        c.design = Some(crate::diseno::borrador::montar(&c, datos, None, &[]).design);
        c
    }

    #[test]
    #[ignore]
    fn deck_de_ejemplo() {
        let datos = std::env::var("PB_DATOS").expect("PB_DATOS");
        let cache = std::env::var("PB_CACHE").expect("PB_CACHE");
        let s = AppState::open(Paths {
            data: datos.into(),
            cache: cache.into(),
        })
        .unwrap();
        let mut cards = Vec::new();
        let mut i = 0;
        let poner = |c: DeckCard, cards: &mut Vec<DeckCard>| {
            cards.push(c);
        };
        for k in 0..14 {
            i += 1;
            let (n, t) = if k % 2 == 0 {
                ("Bosque", "Forest")
            } else {
                ("Isla", "Island")
            };
            let es = format!("Tierra básica — {n}");
            let en = format!("Basic Land — {t}");
            poner(carta(i, n, "", (&es, &en), "", None), &mut cards);
        }
        let criaturas = [
            ("Oso pardo", "{1}{G}", "2", "2"),
            ("Elfo de Llanowar", "{G}", "1", "1"),
            ("Tritón vigía", "{1}{U}", "1", "3"),
            ("Bestia del bosque", "{3}{G}{G}", "5", "5"),
            ("Draco de las nubes", "{3}{U}", "2", "3"),
            ("Lobo gris", "{1}{G}", "2", "1"),
            ("Mago aprendiz", "{U}", "1", "1"),
            ("Ent anciano", "{4}{G}{G}", "6", "7"),
        ];
        for (n, coste, f, r) in criaturas {
            for _ in 0..2 {
                i += 1;
                poner(
                    carta(
                        i,
                        n,
                        coste,
                        ("Criatura — Bestia", "Creature — Beast"),
                        "",
                        Some((f, r)),
                    ),
                    &mut cards,
                );
            }
        }
        for (n, coste, reglas) in [
            ("Contrahechizo", "{U}{U}", "Counter target spell."),
            (
                "Crecimiento gigante",
                "{G}",
                "Target creature gets +3/+3 until end of turn.",
            ),
            ("Pensar dos veces", "{1}{U}", "Draw a card."),
        ] {
            for _ in 0..2 {
                i += 1;
                poner(
                    carta(i, n, coste, ("Instantáneo", "Instant"), reglas, None),
                    &mut cards,
                );
            }
        }
        i += 1;
        poner(
            carta(
                i,
                "Anillo solar",
                "{1}",
                ("Artefacto", "Artifact"),
                "{T}: Add {C}{C}.",
                None,
            ),
            &mut cards,
        );
        i += 1;
        poner(
            carta(
                i,
                "Espada del bosque",
                "{2}",
                ("Artefacto — Equipo", "Artifact — Equipment"),
                "Equipped creature gets +2/+2.\nEquip {2}",
                None,
            ),
            &mut cards,
        );
        i += 1;
        let mut pw = carta(
            i,
            "Nissa, la que escucha",
            "{2}{G}{G}",
            (
                "Planeswalker legendario — Nissa",
                "Legendary Planeswalker — Nissa",
            ),
            "+1: Untap target land.",
            None,
        );
        if let Some(d) = pw.design.as_mut() {
            if let Some(f) = d.card.faces.first_mut() {
                f.loyalty = Some("4".into());
            }
        }
        poner(pw, &mut cards);
        i += 1;
        poner(
            carta(
                i,
                "Kruphix, dios del horizonte",
                "{3}{G}{U}",
                ("Criatura legendaria — Dios", "Legendary Creature — God"),
                "You have no maximum hand size.",
                Some(("4", "7")),
            ),
            &mut cards,
        );
        // Planos (Planechase) y planes (Archenemy): van aparte del mazo.
        for (n, tipo, layout) in [
            ("Arboria", "Plane — Dominaria", "planar"),
            ("Mercadia", "Plane — Mercadia", "planar"),
            ("Ravnica", "Plane — Ravnica", "planar"),
            ("Inundación", "Scheme", "scheme"),
            ("Tormenta de espinas", "Scheme", "scheme"),
        ] {
            i += 1;
            let mut c = carta(i, n, "", (tipo, tipo), "", None);
            if let Some(d) = c.design.as_mut() {
                d.card.layout = layout.into();
            }
            poner(c, &mut cards);
        }
        // El banquillo (para los encuentros al mejor de varias partidas).
        for (n, coste, tipo, reglas) in [
            (
                "Hongo de reserva",
                "{G}",
                (("Criatura — Hongo", "Creature — Fungus"), Some(("1", "1"))),
                "",
            ),
            (
                "Cancelación",
                "{1}{U}",
                (("Instantáneo", "Instant"), None),
                "Counter target spell.",
            ),
        ] {
            for _ in 0..2 {
                i += 1;
                let mut c = carta(i, n, coste, tipo.0, reglas, tipo.1);
                c.marca = Some(proxy_core::model::MarcaCarta::Banquillo);
                poner(c, &mut cards);
            }
        }
        let mut deck = crate::store::decks::create(&s.db, "Mesa de prueba", None).unwrap();
        deck.decklist_text = "Commander\n1 Kruphix, dios del horizonte\n".into();
        deck.cards = cards;
        deck.settings = PrintSettings::default();
        deck.schema_version = DECK_SCHEMA_VERSION;
        let _: &Deck = &deck;
        crate::store::decks::save(&s.db, &deck).unwrap();
        println!("deck {} con {} cartas", deck.id, deck.cards.len());
    }

    #[test]
    #[ignore]
    fn deck_grande_de_ejemplo() {
        let datos = std::env::var("PB_DATOS").expect("PB_DATOS");
        let cache = std::env::var("PB_CACHE").expect("PB_CACHE");
        let s = AppState::open(Paths {
            data: datos.into(),
            cache: cache.into(),
        })
        .unwrap();
        let mut cards = Vec::new();
        let mut i = 0;
        // El comandante, en dos variantes de diseño (la segunda, marcada).
        for k in 0..2 {
            i += 1;
            let mut c = carta(
                i,
                "Kruphix, dios del horizonte",
                "{3}{G}{U}",
                ("Criatura legendaria — Dios", "Legendary Creature — God"),
                if k == 0 {
                    "You have no maximum hand size."
                } else {
                    "You have no maximum hand size. (Variante)"
                },
                Some(("4", "7")),
            );
            if let Some(d) = c.design.as_mut() {
                d.card.color_identity = vec!["G".into(), "U".into()];
            }
            if k == 1 {
                c.marca = Some(proxy_core::model::MarcaCarta::Variante);
            }
            cards.push(c);
        }
        // 30 básicas.
        for k in 0..30 {
            i += 1;
            let (n, t) = if k % 2 == 0 {
                ("Bosque", "Forest")
            } else {
                ("Isla", "Island")
            };
            let es = format!("Tierra básica — {n}");
            let en = format!("Basic Land — {t}");
            cards.push(carta(i, n, "", (&es, &en), "", None));
        }
        // 78 criaturas distintas (9 de ellas, de más): 110 en total.
        for k in 0..78 {
            i += 1;
            let mut c = carta(
                i,
                &format!("Criatura {k:02}"),
                "{1}{G}",
                ("Criatura — Bestia", "Creature — Beast"),
                "",
                Some(("2", "2")),
            );
            if let Some(d) = c.design.as_mut() {
                d.card.color_identity = vec![if k % 3 == 0 { "U" } else { "G" }.into()];
            }
            cards.push(c);
        }
        let mut deck = crate::store::decks::create(&s.db, "Mesa grande", None).unwrap();
        deck.decklist_text = "Commander\n1 Kruphix, dios del horizonte\n".into();
        deck.cards = cards;
        deck.settings = PrintSettings::default();
        deck.schema_version = DECK_SCHEMA_VERSION;
        crate::store::decks::save(&s.db, &deck).unwrap();
        println!("deck {} con {} cartas", deck.id, deck.cards.len());
    }
}
