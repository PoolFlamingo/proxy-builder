//! **Dorados de los layouts especiales** (Fase 5 del plan de la 2.0): una
//! imagen por layout de la tabla del sub-plan, con los datos reales de la
//! carta de prueba (`dorados/layouts/datos/*.json`, bajados de Scryfall con
//! `borrador::en_vivo::datos_de_layouts`) y arte y símbolos de prueba.
//!
//! `PB_ACTUALIZAR_DORADOS=1 cargo test -p proxy-core --test layouts` los
//! reescribe (solo con el visto bueno de Pablo).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod comun;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use proxy_core::diseno::efectivo::efectivo;
use proxy_core::diseno::maquetacion::{maquetacion, Maquetacion};
use proxy_core::diseno::modelo::*;
use proxy_core::diseno::pack::Pack;
use proxy_core::diseno::render::{render, Motor, Recursos, Salida};
use proxy_core::model::{BackSpec, CardEdits, CardSize, CardSource};

use comun::*;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dorados/layouts")
}

/// Los casos: el fichero de datos y el estilo.
const CASOS: &[(&str, DesignStyle)] = &[
    ("pw-3", DesignStyle::Borderless),
    ("pw-4", DesignStyle::Extended),
    ("saga-urza", DesignStyle::Borderless),
    ("saga", DesignStyle::Borderless),
    ("clase", DesignStyle::Borderless),
    ("caso", DesignStyle::Classic),
    ("partida", DesignStyle::Borderless),
    ("fusionar", DesignStyle::Borderless),
    ("secuela", DesignStyle::Borderless),
    ("habitacion", DesignStyle::Borderless),
    ("aventura", DesignStyle::Borderless),
    ("presagio", DesignStyle::Extended),
    ("flip", DesignStyle::Borderless),
    ("transform", DesignStyle::Borderless),
    ("mdfc", DesignStyle::Borderless),
    ("batalla", DesignStyle::Borderless),
    ("meld", DesignStyle::Borderless),
    ("reversible", DesignStyle::Borderless),
    ("ficha2", DesignStyle::Borderless),
    ("leveler", DesignStyle::Borderless),
    ("prototipo", DesignStyle::Borderless),
    ("mutar", DesignStyle::Borderless),
    ("estacion", DesignStyle::Borderless),
    ("emblema", DesignStyle::Borderless),
    ("mazmorra", DesignStyle::Borderless),
    ("plano", DesignStyle::Borderless),
    ("plan", DesignStyle::Borderless),
    ("vanguard", DesignStyle::Borderless),
];

fn datos(archivo: &str) -> CardData {
    let ruta = dir().join("datos").join(format!("{archivo}.json"));
    serde_json::from_str(&std::fs::read_to_string(ruta).unwrap()).unwrap()
}

fn diseno(card: CardData, estilo: DesignStyle) -> CardDesign {
    let caras = card
        .faces
        .iter()
        .enumerate()
        .map(|(i, f)| FaceDesign {
            art: Some(ArtRef {
                image: img(&format!("arte{}", i % 8 + 1)),
                source: ArtSource::Upload,
                framing: ArtFraming::default(),
            }),
            texts: ByLang {
                es: FaceTexts {
                    name: f.printed_name_es.clone().unwrap_or_else(|| f.name.clone()),
                    ..FaceTexts::default()
                },
                en: FaceTexts {
                    name: f.name.clone(),
                    ..FaceTexts::default()
                },
                otros: Default::default(),
            },
        })
        .collect();
    CardDesign {
        version: VERSION_DISENO,
        status: DesignStatus::Ready,
        collection: None,
        style: estilo,
        estilo_pack: None,
        variante_pack: None,
        frame: None,
        faces: caras,
        before: BeforeDesign {
            source: CardSource::Blank,
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
        },
        footer: FooterOptions {
            number: if card.es_ficha() { "T001" } else { "0042" }.into(),
            ..FooterOptions::default()
        },
        card,
        options: DesignOptions {
            show_flavor: true,
            ..DesignOptions::default()
        },
        emblem: EmblemChoice::None,
        thumb_key: None,
    }
}

fn entorno() -> (Pack, proxy_core::diseno::simbolos::Simbolos, Artes) {
    let mut m = HashMap::new();
    for i in 1..=8 {
        m.insert(format!("arte{i}"), Arc::new(arte(i)));
    }
    (Pack::de_fabrica().unwrap(), simbolos(), Artes(m))
}

#[test]
fn dorados_de_los_layouts() {
    let (pack, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    let actualizar = std::env::var("PB_ACTUALIZAR_DORADOS").is_ok();
    std::fs::create_dir_all(dir()).unwrap();
    let mut fallos = Vec::new();
    for (archivo, estilo) in CASOS {
        let d = diseno(datos(archivo), *estilo);
        let (maq, desconocido) = maquetacion(&d.card);
        assert!(!desconocido, "{archivo}");
        let e = efectivo(&d, None);
        for cara in 0..maq.caras_impresas() {
            let r = render(
                &mut motor,
                &e,
                cara,
                DesignLang::Es,
                &rec,
                CardSize::MTG,
                ANCHO,
                Salida::Impresion,
            )
            .unwrap();
            let nombre = if maq.caras_impresas() == 1 {
                format!("{archivo}.png")
            } else {
                format!("{archivo}-{}.png", if cara == 0 { "a" } else { "b" })
            };
            assert!(
                !r.avisos.iter().any(|a| matches!(
                    a,
                    Aviso::PackSinLayout { .. } | Aviso::LayoutDesconocido { .. }
                )),
                "{nombre}: {:?}",
                r.avisos
            );
            let ruta = dir().join(&nombre);
            if actualizar || !ruta.exists() {
                r.imagen.save(&ruta).unwrap();
                continue;
            }
            let dorado = image::open(&ruta).unwrap().to_rgba8();
            let dif = diferencia(&r.imagen, &dorado);
            if dif >= 0.5 {
                r.imagen
                    .save(std::env::temp_dir().join(format!("nuevo-{nombre}")))
                    .unwrap();
                fallos.push(format!("{nombre}: {dif:.2} %"));
            }
        }
    }
    assert!(fallos.is_empty(), "dorados distintos: {fallos:?}");
}

#[test]
fn las_cartas_grandes_salen_a_su_medida() {
    let (pack, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    for archivo in ["plano", "plan", "vanguard"] {
        let d = diseno(datos(archivo), DesignStyle::Borderless);
        assert!(maquetacion(&d.card).0.es_grande());
        let e = efectivo(&d, None);
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            889,
            Salida::Impresion,
        )
        .unwrap();
        // 88,9 × 127 mm: a 889 px de ancho, 1270 de alto.
        assert_eq!(r.imagen.dimensions(), (889, 1270), "{archivo}");
    }
    assert_eq!(
        maquetacion(&datos("partida")).0,
        Maquetacion::Partida,
        "Fire // Ice"
    );
}

/// **Matriz de compatibilidad del pack Balatro** (Fase D): cada layout de
/// arriba con cada estilo propio del pack y cada una de sus variantes, en
/// español y en inglés. Falla si algo no se pinta o le falta marco
/// (`PackSinLayout`, `LayoutDesconocido`, `EstiloNoEnElPack`); los textos que
/// no caben solo se cuentan. Deja una hoja de contactos por estilo en
/// `PB_SALIDA` (o la carpeta temporal) para revisarla a ojo.
/// `PB_BALATRO=<carpeta del pack> cargo test -p proxy-core --test layouts
/// balatro_matriz -- --ignored --nocapture`.
#[test]
#[ignore]
fn balatro_matriz() {
    let (base, salida, pack) = pack_balatro();
    let (_, simbolos, artes) = entorno();
    let simbolos = match &pack.simbolos {
        Some(p) => simbolos.con_propios(p.clone()),
        None => simbolos,
    };
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = motor_balatro(&base);
    let propios: Vec<(String, Vec<Option<String>>)> = pack
        .def
        .estilos
        .iter()
        .filter(|(k, _)| !proxy_core::diseno::pack::ESTILOS_DE_SERIE.contains(&k.as_str()))
        .map(|(k, e)| {
            let variantes = std::iter::once(None)
                .chain(e.variantes.keys().cloned().map(Some))
                .collect();
            (k.clone(), variantes)
        })
        .collect();
    assert!(!propios.is_empty(), "el pack no tiene estilos propios");
    const LADO: u32 = 186;
    let mut fallos = Vec::new();
    let mut no_caben = 0;
    for (estilo, variantes) in &propios {
        let mut miniaturas = Vec::new();
        for variante in variantes {
            for (archivo, _) in CASOS {
                let mut d = diseno(datos(archivo), DesignStyle::Classic);
                d.estilo_pack = Some(estilo.clone());
                d.variante_pack = variante.clone();
                let (maq, _) = maquetacion(&d.card);
                let e = efectivo(&d, None);
                for cara in 0..maq.caras_impresas() {
                    for lang in [DesignLang::Es, DesignLang::En] {
                        let caso = format!("{estilo}/{variante:?}/{archivo}/{cara}/{lang:?}");
                        let r = match render(
                            &mut motor,
                            &e,
                            cara,
                            lang,
                            &rec,
                            CardSize::MTG,
                            ANCHO,
                            Salida::Pantalla,
                        ) {
                            Ok(r) => r,
                            Err(e) => {
                                fallos.push(format!("{caso}: {e:?}"));
                                continue;
                            }
                        };
                        for a in &r.avisos {
                            match a {
                                Aviso::PackSinLayout { .. }
                                | Aviso::LayoutDesconocido { .. }
                                | Aviso::EstiloNoEnElPack { .. } => {
                                    fallos.push(format!("{caso}: {a:?}"))
                                }
                                Aviso::TextoNoCabe { .. } => no_caben += 1,
                                _ => {}
                            }
                        }
                        if lang == DesignLang::Es {
                            miniaturas.push(image::imageops::thumbnail(
                                &r.imagen,
                                LADO * r.imagen.width() / r.imagen.height().max(1),
                                LADO,
                            ));
                        }
                    }
                }
            }
        }
        // La hoja: las miniaturas en filas de 10.
        let ancho_celda = miniaturas.iter().map(|m| m.width()).max().unwrap_or(LADO);
        let filas = miniaturas.len().div_ceil(10) as u32;
        let mut hoja = image::RgbaImage::from_pixel(
            ancho_celda * 10,
            LADO * filas.max(1),
            image::Rgba([40, 40, 40, 255]),
        );
        for (i, m) in miniaturas.iter().enumerate() {
            let (x, y) = ((i % 10) as u32 * ancho_celda, (i / 10) as u32 * LADO);
            image::imageops::overlay(&mut hoja, m, x.into(), y.into());
        }
        hoja.save(salida.join(format!("matriz-{estilo}.png")))
            .unwrap();
    }
    println!("{no_caben} textos que no caben (ver las hojas)");
    assert!(
        fallos.is_empty(),
        "{} fallos:\n{}",
        fallos.len(),
        fallos.join("\n")
    );
}
