//! **Dorados del motor de diseño** (Paso 2.9 del plan de la 2.0): una imagen
//! de referencia por caso base y estilo, para no romper sin querer el
//! aspecto aprobado. Arte y símbolos de prueba hechos aquí (nada con
//! derechos), letras y pack de la casa.
//!
//! `PB_ACTUALIZAR_DORADOS=1 cargo test -p proxy-core --test dorados` los
//! reescribe (solo con el visto bueno de Pablo).

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod comun;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use proxy_core::diseno::efectivo::efectivo;
use proxy_core::diseno::modelo::*;
use proxy_core::diseno::pack::Pack;
use proxy_core::diseno::render::{render, Motor, Recursos, Salida};
use proxy_core::diseno::simbolos::Simbolos;
use proxy_core::model::{BackSpec, CardEdits, CardSize, CardSource};

use comun::*;

struct Caso {
    archivo: &'static str,
    estilo: DesignStyle,
    nombre: &'static str,
    original: &'static str,
    coste: &'static str,
    tipo: &'static str,
    tipo_en: &'static str,
    reglas: &'static str,
    sabor: &'static str,
    colores: &'static [&'static str],
    fr: Option<(&'static str, &'static str)>,
    legendaria: bool,
    layout: &'static str,
    arte: u32,
    /// El nombre original en la cinta en vez de en el pie.
    cinta: bool,
}

fn diseno(c: &Caso) -> CardDesign {
    let cara = FaceData {
        name: c.original.into(),
        mana_cost: c.coste.into(),
        type_line: ByLang {
            es: c.tipo.into(),
            en: c.tipo_en.into(),
        },
        rules: ByLang {
            es: c.reglas.into(),
            en: c.reglas.into(),
        },
        printed_name_es: Some(c.original.into()),
        colors: c.colores.iter().map(|x| x.to_string()).collect(),
        power: c.fr.map(|f| f.0.into()),
        toughness: c.fr.map(|f| f.1.into()),
        loyalty: None,
        defense: None,
    };
    let basica = c.tipo_en.starts_with("Basic Land");
    CardDesign {
        version: VERSION_DISENO,
        status: DesignStatus::Ready,
        collection: Some("col".into()),
        style: c.estilo,
        frame: None,
        faces: vec![FaceDesign {
            art: Some(ArtRef {
                image: img(&format!("arte{}", c.arte)),
                source: ArtSource::Upload,
                framing: ArtFraming::default(),
            }),
            texts: ByLang {
                es: FaceTexts {
                    name: c.nombre.into(),
                    flavor: c.sabor.into(),
                    rules_override: None,
                },
                en: FaceTexts::default(),
            },
        }],
        before: BeforeDesign {
            source: CardSource::Blank,
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
        },
        card: CardData {
            scryfall_id: "x".into(),
            layout: c.layout.into(),
            rarity: "rare".into(),
            frame_effects: if c.legendaria {
                vec!["legendary".into()]
            } else {
                vec![]
            },
            colors: cara.colors.clone(),
            color_identity: if basica {
                vec!["W".into()]
            } else {
                cara.colors.clone()
            },
            set: "tst".into(),
            collector_number: "1".into(),
            artist: Some("Prueba".into()),
            faces: vec![cara],
            ..CardData::default()
        },
        footer: FooterOptions {
            number: if c.layout == "token" { "T001" } else { "0007" }.into(),
            ..FooterOptions::default()
        },
        options: DesignOptions {
            show_flavor: true,
            original_name: if c.cinta {
                OriginalName::Ribbon
            } else {
                OriginalName::Auto
            },
            ..DesignOptions::default()
        },
        emblem: EmblemChoice::Inherit,
        thumb_key: None,
    }
}

fn coleccion() -> Collection {
    Collection {
        id: "col".into(),
        name: "Prueba".into(),
        default_style: DesignStyle::Borderless,
        frame: FrameOptions::default(),
        emblem: Some(img("emblema")),
        set_code: "TST".into(),
        rarity_letter: Some("S".into()),
        year: Some("2024".into()),
        artist: Some("P.B.".into()),
        credit: None,
        universe: None,
        original_name: OriginalName::Footer,
        footer_template: ByLang::default(),
        font_title: None,
        font_rules: None,
        title_squeeze_min: ESTRECHADO_DEFECTO,
        title_squeeze_base: ESTRECHADO_BASE_DEFECTO,
        weight_title: None,
        weight_type: None,
        weight_rules: None,
        fine_tuning: None,
    }
}

const LEGENDARIA: &str = "Vuela, vigilancia.\nSiempre que una criatura con defensor entre al campo de batalla bajo tu control, roba una carta.";
const CONJURO: &str = "Busca en tu biblioteca una carta y pon esa carta en tu mano. Luego baraja.";
const ARTEFACTO: &str = "{1}, {T}, sacrificar el Talismán: Busca en tu biblioteca una carta de tierra básica, muéstrala y ponla en tu mano.";
const MURO: &str =
    "Defensor. (Esta criatura no puede atacar.)\n{G}, sacrificar un Muro: Roba una carta.";
const TIERRA: &str = "{T}: Agrega {C}.\n{T}: Agrega {G} o {U}. La Cueva te hace 1 punto de daño.";

fn casos() -> Vec<Caso> {
    let base = |archivo, estilo| Caso {
        archivo,
        estilo,
        nombre: "Señora de Prueba, Dragón Antiguo",
        original: "Arcades, the Strategist",
        coste: "{1}{G}{W}{U}",
        tipo: "Criatura legendaria — Dragón anciano",
        tipo_en: "Legendary Creature — Elder Dragon",
        reglas: LEGENDARIA,
        sabor: "«Las alas de un dragón no conocen el miedo.»",
        colores: &["G", "W", "U"],
        fr: Some(("3", "5")),
        legendaria: true,
        layout: "normal",
        arte: 1,
        cinta: false,
    };
    vec![
        base("despejado-legendaria", DesignStyle::Borderless),
        Caso {
            cinta: true,
            ..base("despejado-cinta", DesignStyle::Borderless)
        },
        base("extendido-legendaria", DesignStyle::Extended),
        Caso {
            colores: &["G", "U"],
            coste: "{1}{G}{U}",
            legendaria: false,
            tipo: "Criatura — Muro",
            tipo_en: "Creature — Wall",
            reglas: MURO,
            sabor: "",
            fr: Some(("0", "4")),
            nombre: "Muro de Prueba",
            original: "Wall of Mulch",
            ..base("despejado-bicolor", DesignStyle::Borderless)
        },
        Caso {
            nombre: "Guardián Eterno del Bosque Profundo de las Brumas",
            coste: "{2}{G}{G}{W}{W}{U}{U}",
            ..base("despejado-titulo-largo", DesignStyle::Borderless)
        },
        Caso {
            colores: &["B"],
            coste: "{2}{B}",
            legendaria: false,
            tipo: "Conjuro",
            tipo_en: "Sorcery",
            reglas: CONJURO,
            sabor: "«Deseo servir como tu más humilde sirviente.»\n—Diablo",
            fr: None,
            nombre: "Pacto de Prueba",
            original: "Diabolic Tutor",
            arte: 2,
            ..base("despejado-conjuro", DesignStyle::Borderless)
        },
        Caso {
            colores: &["U"],
            coste: "{3}{U}{U}",
            legendaria: false,
            tipo: "Criatura — Cambiaformas",
            tipo_en: "Creature — Shapeshifter",
            reglas: "{2}{U}{U}, sacrificar a Prueba: Elige un tipo de criatura. Revela cartas de la parte superior de tu biblioteca hasta que reveles una carta de criatura de ese tipo.",
            nombre: "Prueba, el Gran Maestro",
            original: "Riptide Shapeshifter",
            fr: Some(("3", "3")),
            arte: 3,
            ..base("clasico-criatura", DesignStyle::Classic)
        },
        Caso {
            colores: &[],
            coste: "{2}",
            legendaria: false,
            tipo: "Artefacto",
            tipo_en: "Artifact",
            reglas: ARTEFACTO,
            sabor: "",
            fr: None,
            nombre: "Talismán de Prueba",
            original: "Moonsilver Key",
            arte: 4,
            ..base("despejado-artefacto", DesignStyle::Borderless)
        },
        Caso {
            colores: &[],
            coste: "{3}",
            legendaria: false,
            tipo: "Artefacto",
            tipo_en: "Artifact",
            reglas: ARTEFACTO,
            sabor: "",
            fr: None,
            nombre: "Reunión de Prueba",
            original: "Chromatic Orrery",
            arte: 4,
            ..base("clasico-artefacto", DesignStyle::Classic)
        },
        Caso {
            colores: &[],
            coste: "",
            legendaria: false,
            tipo: "Tierra",
            tipo_en: "Land",
            reglas: TIERRA,
            sabor: "",
            fr: None,
            nombre: "Cueva de Prueba",
            original: "Yavimaya Coast",
            arte: 5,
            ..base("extendido-tierra", DesignStyle::Extended)
        },
        Caso {
            colores: &[],
            coste: "",
            legendaria: false,
            tipo: "Tierra básica — Llanura",
            tipo_en: "Basic Land — Plains",
            reglas: "",
            sabor: "",
            fr: None,
            nombre: "Reino de Prueba",
            original: "Plains",
            arte: 6,
            ..base("paisaje-basica", DesignStyle::FullArt)
        },
        Caso {
            colores: &["W"],
            coste: "",
            legendaria: false,
            tipo: "Criatura ficha — Muro",
            tipo_en: "Token Creature — Wall",
            reglas: "Defensor.",
            sabor: "",
            fr: Some(("0", "3")),
            nombre: "Cruzado (Prueba)",
            original: "Wall",
            layout: "token",
            arte: 7,
            ..base("extendido-ficha", DesignStyle::Extended)
        },
        Caso {
            colores: &["R"],
            coste: "",
            legendaria: false,
            tipo: "Criatura ficha — Trasgo",
            tipo_en: "Token Creature — Goblin",
            reglas: "",
            sabor: "",
            fr: Some(("1", "1")),
            nombre: "Trasgo de Prueba",
            original: "Goblin",
            layout: "token",
            arte: 8,
            ..base("clasico-ficha", DesignStyle::Classic)
        },
    ]
}

fn entorno() -> (Pack, Simbolos, Artes) {
    let mut m = HashMap::new();
    for i in 1..=8 {
        m.insert(format!("arte{i}"), Arc::new(arte(i)));
    }
    m.insert("emblema".into(), Arc::new(emblema()));
    (Pack::de_fabrica().unwrap(), simbolos(), Artes(m))
}

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dorados")
}

#[test]
fn dorados_de_los_casos_base() {
    let (pack, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    let col = coleccion();
    let actualizar = std::env::var("PB_ACTUALIZAR_DORADOS").is_ok();
    std::fs::create_dir_all(dir()).unwrap();
    let mut fallos = Vec::new();
    for c in casos() {
        let e = efectivo(&diseno(&c), Some(&col));
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            ANCHO,
            Salida::Impresion,
        )
        .unwrap();
        assert!(
            !r.avisos
                .iter()
                .any(|a| matches!(a, Aviso::FaltanSimbolos { .. })),
            "{}: {:?}",
            c.archivo,
            r.avisos
        );
        let ruta = dir().join(format!("{}.png", c.archivo));
        if actualizar || !ruta.exists() {
            r.imagen.save(&ruta).unwrap();
            continue;
        }
        let dorado = image::open(&ruta).unwrap().to_rgba8();
        let d = diferencia(&r.imagen, &dorado);
        if d >= 0.5 {
            r.imagen
                .save(std::env::temp_dir().join(format!("{}-nuevo.png", c.archivo)))
                .unwrap();
            fallos.push(format!("{}: {d:.2} %", c.archivo));
        }
    }
    assert!(fallos.is_empty(), "dorados distintos: {fallos:?}");
}

#[test]
fn dos_renders_iguales_dan_los_mismos_bytes() {
    let (pack, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    let c = &casos()[0];
    let e = efectivo(&diseno(c), Some(&coleccion()));
    let a = render(
        &mut motor,
        &e,
        0,
        DesignLang::Es,
        &rec,
        CardSize::MTG,
        300,
        Salida::Impresion,
    )
    .unwrap();
    let b = render(
        &mut motor,
        &e,
        0,
        DesignLang::Es,
        &rec,
        CardSize::MTG,
        300,
        Salida::Impresion,
    )
    .unwrap();
    assert_eq!(a.imagen.as_raw(), b.imagen.as_raw());
    // Para imprimir, opaca: las esquinas no son transparentes.
    assert_eq!(a.imagen.get_pixel(0, 0).0[3], 255);
    let p = render(
        &mut motor,
        &e,
        0,
        DesignLang::Es,
        &rec,
        CardSize::MTG,
        300,
        Salida::Pantalla,
    )
    .unwrap();
    assert_eq!(p.imagen.get_pixel(0, 0).0[3], 0);
    // Tamaño póker: la imagen sale con su proporción.
    let k = render(
        &mut motor,
        &e,
        0,
        DesignLang::Es,
        &rec,
        CardSize::POKER,
        635,
        Salida::Impresion,
    )
    .unwrap();
    assert_eq!(k.imagen.dimensions(), (635, 889));
}

#[test]
fn cambiar_un_color_del_pack_cambia_el_dorado() {
    let (pack, simbolos, artes) = entorno();
    let mut ficheros = pack.ficheros.clone();
    let mut v: serde_json::Value = serde_json::from_str(&ficheros["pack.json"]).unwrap();
    v["paletas"]["gold"]["medio"] = "#ff00ff".into();
    ficheros.insert("pack.json".into(), serde_json::to_string(&v).unwrap());
    let otro = Pack::cargar(ficheros).unwrap();
    assert_ne!(otro.huella, pack.huella);
    let mut motor = Motor::de_fabrica().unwrap();
    let c = &casos()[0];
    let e = efectivo(&diseno(c), Some(&coleccion()));
    let r = |p: &Pack, m: &mut Motor| {
        let rec = Recursos {
            pack: p,
            simbolos: &simbolos,
            imagenes: &artes,
        };
        render(
            m,
            &e,
            0,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            ANCHO,
            Salida::Impresion,
        )
        .unwrap()
        .imagen
    };
    let a = r(&pack, &mut motor);
    let b = r(&otro, &mut motor);
    assert!(diferencia(&a, &b) > 0.5);
}

/// Los tiempos del plan: `cargo test --release -p proxy-core --test dorados -- --ignored --nocapture`.
#[test]
#[ignore]
fn tiempos() {
    let (pack, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    let c = &casos()[0];
    let e = efectivo(&diseno(c), Some(&coleccion()));
    for ancho in [600u32, 2232] {
        let n = 5;
        let t = std::time::Instant::now();
        for _ in 0..n {
            render(
                &mut motor,
                &e,
                0,
                DesignLang::Es,
                &rec,
                CardSize::MTG,
                ancho,
                Salida::Impresion,
            )
            .unwrap();
        }
        println!("{ancho} px: {:?} por carta", t.elapsed() / n);
    }
}

/// Prueba a ojo del pack «Original» con sus imágenes de verdad:
/// `PB_ORIGINAL=<carpeta img/frames de Card Conjurer> cargo test -p proxy-core
/// --test dorados original -- --ignored`. Deja las cartas en el temporal.
#[test]
#[ignore]
fn original_con_sus_imagenes() {
    use proxy_core::diseno::pack::{ORIGINAL, ORIGINAL_IMAGENES};
    let base = PathBuf::from(std::env::var("PB_ORIGINAL").expect("PB_ORIGINAL"));
    let lista: serde_json::Value = serde_json::from_str(ORIGINAL_IMAGENES).unwrap();
    let imagenes = lista["imagenes"]
        .as_object()
        .unwrap()
        .keys()
        .map(|r| (r.clone(), Arc::new(std::fs::read(base.join(r)).unwrap())))
        .collect();
    let ficheros = ORIGINAL
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    let pack = Pack::cargar_con_imagenes(ficheros, imagenes).unwrap();
    let (_, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    let col = coleccion();
    for c in casos() {
        for estilo in [DesignStyle::Classic, DesignStyle::Borderless] {
            let mut d = diseno(&c);
            d.style = estilo;
            let e = efectivo(&d, Some(&col));
            let t = std::time::Instant::now();
            let r = render(
                &mut motor,
                &e,
                0,
                DesignLang::Es,
                &rec,
                CardSize::MTG,
                745,
                Salida::Pantalla,
            )
            .unwrap();
            println!("{} {estilo:?}: {:?} {:?}", c.archivo, t.elapsed(), r.avisos);
            r.imagen
                .save(std::env::temp_dir().join(format!("original-{}-{estilo:?}.png", c.archivo)))
                .unwrap();
        }
    }
    // Con ajustes finos: radios y caja más opaca.
    for radio in [0.0, 4.0, 12.0] {
        let c = &casos()[0];
        let mut d = diseno(c);
        d.style = DesignStyle::Borderless;
        d.options.fine_tuning = Some(proxy_core::diseno::modelo::FineTuning {
            art_radius: Some(radio),
            card_radius: Some(0.0),
            rules_box_opacity: 0.5,
            ..Default::default()
        });
        let e = efectivo(&d, Some(&col));
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            745,
            Salida::Pantalla,
        )
        .unwrap();
        r.imagen
            .save(std::env::temp_dir().join(format!("original-radio-{radio}.png")))
            .unwrap();
    }
}

/// Prueba a ojo de las letras libres del catálogo de la app: `PB_GF=<copia
/// de google/fonts/ofl> cargo test -p proxy-core --test dorados letras_libres
/// -- --ignored`. Deja una carta por letra en el temporal.
#[test]
#[ignore]
fn letras_libres_a_ojo() {
    let base = PathBuf::from(std::env::var("PB_GF").expect("PB_GF"));
    let catalogo: serde_json::Value =
        serde_json::from_str(include_str!("../../../assets/letras-libres.json")).unwrap();
    let (pack, simbolos, artes) = entorno();
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = Motor::de_fabrica().unwrap();
    for f in catalogo["familias"].as_array().unwrap() {
        for x in f["ficheros"].as_array().unwrap() {
            let ruta = x["ruta"].as_str().unwrap();
            motor
                .letras
                .cargar(std::fs::read(base.join(ruta)).unwrap())
                .unwrap();
        }
    }
    let c = &casos()[0];
    for f in catalogo["familias"].as_array().unwrap() {
        let familia = f["familia"].as_str().unwrap();
        let mut col = coleccion();
        col.font_title = Some(familia.into());
        // Las de título solo van en el título; las demás, en las dos.
        if f["uso"] != "titulo" {
            col.font_rules = Some(familia.into());
        }
        let e = efectivo(&diseno(c), Some(&col));
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::Es,
            &rec,
            CardSize::MTG,
            600,
            Salida::Pantalla,
        )
        .unwrap();
        println!("{familia}: {:?}", r.avisos);
        r.imagen
            .save(std::env::temp_dir().join(format!("letra-{}.png", f["id"].as_str().unwrap())))
            .unwrap();
    }
}
