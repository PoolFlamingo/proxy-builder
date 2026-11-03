//! **Pasar un deck a otro equipo con o sin proxies** (puro, sin red ni disco).
//!
//! «Sin proxies» (solo cartas originales) deja un deck que pesa una fracción:
//!
//! - las de Scryfall y las fichas, igual;
//! - las propias **vinculadas** (y las diseñadas con vínculo) pasan a su carta
//!   de Scryfall, sin imagen ni diseño: quien recibe las baja;
//! - las de **otra fuente** (las fichas de Cockatrice) conservan su referencia
//!   y van sin imagen: quien recibe las baja de allí;
//! - las propias **sin vínculo** pasan a un hueco con su nombre (`meta.name`),
//!   para que quien recibe sepa qué carta era;
//! - ni colecciones ni imágenes propias (tampoco el reverso propio del deck:
//!   queda el de la app, el que genera ella misma).
//!
//! 🛑 **Cada carta conserva su id**: las listas para jugar (que apuntan a ids
//! de carta, también el comandante), la portada y las marcas siguen valiendo.

use crate::model::{
    BackSpec, CardEdits, CardSource, Deck, DeckCard, FaceImage, ImageOrigin, RefExterna,
};

/// De dónde se baja la imagen de una carta que llegó sin ella.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origen {
    /// Una impresión de Scryfall (`card_fetch`).
    Scryfall(String),
    /// Una ficha de Scryfall.
    Ficha(String),
    /// Una carta de otra fuente (`card_fetch_externa`).
    Externa(RefExterna),
}

/// Una carta del deck a la que le falta la imagen y de dónde sacarla.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pendiente {
    /// El id de la carta en el deck (se conserva al completarla).
    pub carta: String,
    pub origen: Origen,
}

/// Lo que el deck tiene de «proxy» (hecho por quien lo hizo, no de Scryfall).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumenProxies {
    /// Cartas propias con su imagen (subida o de otra fuente), vinculadas o no.
    pub propias: u32,
    /// Cartas con un diseño del editor de cartas.
    pub disenadas: u32,
    /// Cartas con un reverso propio.
    pub reversos: u32,
    /// El deck lleva un reverso propio por defecto.
    pub reverso_del_deck: bool,
}

impl ResumenProxies {
    pub fn hay(&self) -> bool {
        self.propias > 0 || self.disenadas > 0 || self.reversos > 0 || self.reverso_del_deck
    }
}

/// Lo que lleva el deck que no es de Scryfall.
pub fn resumen_proxies(deck: &Deck) -> ResumenProxies {
    let mut r = ResumenProxies {
        reverso_del_deck: deck.default_back.as_ref().is_some_and(es_propia),
        ..Default::default()
    };
    for c in &deck.cards {
        if matches!(c.source, CardSource::Custom { .. }) && c.front.is_some() {
            r.propias += 1;
        }
        if c.design.is_some() {
            r.disenadas += 1;
        }
        if matches!(c.back, BackSpec::Custom { .. }) {
            r.reversos += 1;
        }
    }
    r
}

/// La imagen no la puso la app ni la bajó de Scryfall.
fn es_propia(i: &FaceImage) -> bool {
    !matches!(
        i.origin,
        ImageOrigin::Generated | ImageOrigin::ScryfallPng | ImageOrigin::ScryfallLarge
    )
}

/// El mismo deck sin proxies (ver el módulo). Es idempotente.
pub fn sin_proxies(deck: &Deck) -> Deck {
    let mut d = deck.clone();
    d.cards = deck.cards.iter().map(carta_sin_proxy).collect();
    if d.default_back.as_ref().is_some_and(es_propia) {
        d.default_back = None;
    }
    d
}

fn vinculo(c: &DeckCard) -> Option<&str> {
    match &c.source {
        CardSource::Custom {
            scryfall_id: Some(id),
            ..
        } if !id.trim().is_empty() => Some(id),
        _ => None,
    }
}

/// La carta sin su imagen, su diseño ni sus retoques, con esta fuente.
fn hueco_de(c: &DeckCard, source: CardSource) -> DeckCard {
    let mut meta = c.meta.clone();
    meta.image_lang = None;
    meta.image_status = None;
    meta.highres = false;
    if matches!(source, CardSource::Blank) {
        meta.is_dfc = false;
    }
    DeckCard {
        id: c.id.clone(),
        source,
        front: None,
        back: BackSpec::DeckDefault,
        edits: CardEdits::default(),
        locked: c.locked,
        meta,
        design: None,
        marca: c.marca,
    }
}

fn carta_sin_proxy(c: &DeckCard) -> DeckCard {
    match &c.source {
        CardSource::Custom { externa, .. } => {
            if let Some(id) = vinculo(c) {
                hueco_de(
                    c,
                    CardSource::Scryfall {
                        scryfall_id: id.to_string(),
                    },
                )
            } else if let Some(e) = externa {
                hueco_de(
                    c,
                    CardSource::Custom {
                        scryfall_id: None,
                        externa: Some(e.clone()),
                    },
                )
            } else {
                hueco_de(c, CardSource::Blank)
            }
        }
        // Una de Scryfall con diseño (no pasa, pero por si acaso): sin él.
        CardSource::Scryfall { .. } | CardSource::Token { .. } if c.design.is_some() => {
            hueco_de(c, c.source.clone())
        }
        CardSource::Scryfall { .. } | CardSource::Token { .. } => {
            let mut c = c.clone();
            // Un reverso elegido a mano es una imagen propia.
            if matches!(c.back, BackSpec::Custom { .. }) {
                c.back = BackSpec::DeckDefault;
            }
            c
        }
        CardSource::Blank => {
            let mut c = c.clone();
            c.front = None;
            c.design = None;
            c.back = BackSpec::DeckDefault;
            c
        }
    }
}

/// Las cartas que se pueden completar bajando su imagen: sin frente y con una
/// fuente de la que bajarla.
pub fn pendientes(deck: &Deck) -> Vec<Pendiente> {
    deck.cards
        .iter()
        .filter(|c| c.front.is_none())
        .filter_map(|c| {
            let origen = match &c.source {
                CardSource::Scryfall { scryfall_id } => Origen::Scryfall(scryfall_id.clone()),
                CardSource::Token { scryfall_id } => Origen::Ficha(scryfall_id.clone()),
                CardSource::Custom {
                    externa: Some(e), ..
                } => Origen::Externa(e.clone()),
                CardSource::Custom {
                    scryfall_id: Some(id),
                    ..
                } if !id.trim().is_empty() => Origen::Scryfall(id.clone()),
                _ => return None,
            };
            Some(Pendiente {
                carta: c.id.clone(),
                origen,
            })
        })
        .collect()
}

/// Pone en `carta` lo que trae la bajada (fuente, imágenes, datos de la
/// carta) y conserva lo suyo: el id, la marca, si está fijada y su sitio en
/// el deck.
pub fn completar(carta: &mut DeckCard, traida: DeckCard) {
    carta.source = traida.source;
    carta.front = traida.front;
    carta.back = traida.back;
    carta.meta = traida.meta;
    carta.edits = CardEdits::default();
    carta.design = None;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diseno::modelo::tests::diseno_de_prueba;
    use crate::diseno::modelo::{CardData, CardDesign};
    use crate::model::{CardMeta, MarcaCarta, PrintSettings, DECK_SCHEMA_VERSION};

    fn img(n: u8, origen: ImageOrigin) -> FaceImage {
        FaceImage {
            blob: format!("{n:064x}"),
            width: 745,
            height: 1040,
            origin: origen,
        }
    }

    fn carta(id: &str, nombre: &str, source: CardSource, front: Option<FaceImage>) -> DeckCard {
        DeckCard {
            id: id.into(),
            source,
            front,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta {
                name: nombre.into(),
                ..CardMeta::default()
            },
            design: None,
            marca: None,
        }
    }

    fn deck_de_todo() -> Deck {
        let mut cartas = vec![
            // 0: Scryfall tal cual.
            carta(
                "c0",
                "Sol Ring",
                CardSource::Scryfall {
                    scryfall_id: "sol".into(),
                },
                Some(img(1, ImageOrigin::ScryfallPng)),
            ),
            // 1: ficha.
            carta(
                "c1",
                "Treasure",
                CardSource::Token {
                    scryfall_id: "tes".into(),
                },
                Some(img(2, ImageOrigin::ScryfallLarge)),
            ),
            // 2: propia vinculada.
            carta(
                "c2",
                "Lightning Bolt",
                CardSource::Custom {
                    scryfall_id: Some("bolt".into()),
                    externa: None,
                },
                Some(img(3, ImageOrigin::Upload)),
            ),
            // 3: propia de otra fuente.
            carta(
                "c3",
                "Kelp",
                CardSource::Custom {
                    scryfall_id: None,
                    externa: Some(RefExterna {
                        fuente: crate::model::FuenteCarta::Cockatrice,
                        id: "kelp".into(),
                    }),
                },
                Some(img(4, ImageOrigin::Upload)),
            ),
            // 4: propia sin vínculo.
            carta(
                "c4",
                "Mi carta rara",
                CardSource::Custom {
                    scryfall_id: None,
                    externa: None,
                },
                Some(img(5, ImageOrigin::Upload)),
            ),
            // 5: hueco.
            carta("c5", "", CardSource::Blank, None),
            // 6: diseñada con vínculo, con arte propio.
            carta(
                "c6",
                "Diseñada",
                CardSource::Custom {
                    scryfall_id: Some("dis".into()),
                    externa: None,
                },
                Some(img(6, ImageOrigin::Designed)),
            ),
            // 7: diseñada desde cero (vínculo vacío).
            carta(
                "c7",
                "Desde cero",
                CardSource::Custom {
                    scryfall_id: Some(String::new()),
                    externa: None,
                },
                Some(img(7, ImageOrigin::Designed)),
            ),
        ];
        let diseno = |scryfall: &str| CardDesign {
            card: CardData {
                scryfall_id: scryfall.into(),
                ..CardData::default()
            },
            ..diseno_de_prueba()
        };
        cartas[6].design = Some(diseno("dis"));
        cartas[7].design = Some(diseno(""));
        cartas[2].back = BackSpec::Custom {
            image: img(8, ImageOrigin::Upload),
        };
        cartas[0].back = BackSpec::Dfc {
            image: img(9, ImageOrigin::ScryfallPng),
        };
        cartas[1].marca = Some(MarcaCarta::Copia);
        cartas[2].marca = Some(MarcaCarta::Banquillo);
        cartas[2].locked = true;
        Deck {
            id: "d".into(),
            name: "Todo".into(),
            decklist_text: String::new(),
            cards: cartas,
            settings: PrintSettings::default(),
            default_back: Some(img(10, ImageOrigin::Upload)),
            schema_version: DECK_SCHEMA_VERSION,
        }
    }

    #[test]
    fn cada_tipo_de_carta_pasa_a_lo_que_toca() {
        let d = sin_proxies(&deck_de_todo());
        let c = &d.cards;
        // Scryfall y ficha, igual (y la cara trasera de Scryfall se queda).
        assert_eq!(c[0], deck_de_todo().cards[0]);
        assert_eq!(c[1], deck_de_todo().cards[1]);
        // Propia vinculada: su carta de Scryfall, sin imagen, sin reverso propio.
        assert_eq!(
            c[2].source,
            CardSource::Scryfall {
                scryfall_id: "bolt".into()
            }
        );
        assert!(c[2].front.is_none());
        assert_eq!(c[2].back, BackSpec::DeckDefault);
        assert_eq!(c[2].meta.name, "Lightning Bolt");
        // De otra fuente: su referencia y sin imagen.
        assert!(matches!(
            &c[3].source,
            CardSource::Custom {
                scryfall_id: None,
                externa: Some(e)
            } if e.id == "kelp"
        ));
        assert!(c[3].front.is_none());
        // Sin vínculo: un hueco con su nombre.
        assert_eq!(c[4].source, CardSource::Blank);
        assert_eq!(c[4].meta.name, "Mi carta rara");
        assert!(c[4].front.is_none());
        // El hueco sigue igual.
        assert_eq!(c[5].source, CardSource::Blank);
        // Diseñada con vínculo: Scryfall sin diseño; desde cero: hueco.
        assert_eq!(
            c[6].source,
            CardSource::Scryfall {
                scryfall_id: "dis".into()
            }
        );
        assert!(c[6].design.is_none() && c[6].front.is_none());
        assert_eq!(c[7].source, CardSource::Blank);
        assert!(c[7].design.is_none());
        // El reverso propio del deck se va.
        assert!(d.default_back.is_none());
        assert!(!resumen_proxies(&d).hay());
    }

    #[test]
    fn los_ids_las_marcas_y_las_fijadas_no_cambian() {
        let antes = deck_de_todo();
        let despues = sin_proxies(&antes);
        let ids = |d: &Deck| d.cards.iter().map(|c| c.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&antes), ids(&despues));
        for (a, b) in antes.cards.iter().zip(&despues.cards) {
            assert_eq!(a.marca, b.marca, "{}", a.id);
            assert_eq!(a.locked, b.locked, "{}", a.id);
        }
        assert_eq!(antes.id, despues.id);
        assert_eq!(antes.name, despues.name);
        assert_eq!(antes.settings, despues.settings);
    }

    #[test]
    fn es_idempotente_y_el_reverso_generado_se_queda() {
        let mut d = deck_de_todo();
        d.default_back = Some(img(11, ImageOrigin::Generated));
        let una = sin_proxies(&d);
        assert_eq!(una.default_back, d.default_back);
        assert_eq!(sin_proxies(&una), una);
    }

    #[test]
    fn el_resumen_cuenta_lo_que_es_proxy() {
        let r = resumen_proxies(&deck_de_todo());
        assert_eq!(r.propias, 5);
        assert_eq!(r.disenadas, 2);
        assert_eq!(r.reversos, 1);
        assert!(r.reverso_del_deck && r.hay());
        let limpio = Deck {
            cards: vec![deck_de_todo().cards.remove(0)],
            default_back: None,
            ..deck_de_todo()
        };
        assert!(!resumen_proxies(&limpio).hay());
    }

    #[test]
    fn los_pendientes_son_las_que_se_pueden_bajar() {
        let p = pendientes(&sin_proxies(&deck_de_todo()));
        let por_id = |id: &str| p.iter().find(|x| x.carta == id).map(|x| x.origen.clone());
        // Scryfall y ficha traían imagen: no faltan.
        assert_eq!(por_id("c0"), None);
        assert_eq!(por_id("c1"), None);
        assert_eq!(por_id("c2"), Some(Origen::Scryfall("bolt".into())));
        assert!(matches!(por_id("c3"), Some(Origen::Externa(_))));
        // Un hueco no se puede completar.
        assert_eq!(por_id("c4"), None);
        assert_eq!(por_id("c5"), None);
        assert_eq!(por_id("c6"), Some(Origen::Scryfall("dis".into())));
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn completar_conserva_el_id_la_marca_y_lo_fijado() {
        let d = sin_proxies(&deck_de_todo());
        let mut c = d.cards[2].clone();
        let traida = carta(
            "otro-id",
            "Lightning Bolt",
            CardSource::Scryfall {
                scryfall_id: "bolt".into(),
            },
            Some(img(20, ImageOrigin::ScryfallPng)),
        );
        completar(&mut c, traida);
        assert_eq!(c.id, "c2");
        assert_eq!(c.marca, Some(MarcaCarta::Banquillo));
        assert!(c.locked);
        assert_eq!(c.front, Some(img(20, ImageOrigin::ScryfallPng)));
    }
}
