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
        estilo_pack: None,
        variante_pack: None,
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
        estilo_pack_defecto: None,
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

/// Cartas de texto largo para los packs con cajas fijas (Balatro).
fn casos_largos() -> Vec<Caso> {
    vec![Caso {
        archivo: "largo-yusri",
        estilo: DesignStyle::Classic,
        nombre: "Yusri, Llama de la Fortuna",
        original: "Yusri, Fortune's Flame",
        coste: "{1}{U}{R}",
        tipo: "Criatura legendaria — Efrit",
        tipo_en: "Legendary Creature — Efreet",
        reglas: "Vuela.\nSiempre que Yusri, Llama de la Fortuna ataque, elige un número entre el 1 y el 5. Lanza esa cantidad de monedas a cara o cruz. Por cada lanzamiento que ganes, roba una carta. Por cada lanzamiento que pierdas, Yusri te hace 2 puntos de daño. Si ganaste cinco lanzamientos de esta manera, puedes lanzar hechizos desde tu mano este turno sin pagar sus costes de maná.",
        sabor: "",
        colores: &["U", "R"],
        fr: Some(("2", "3")),
        legendaria: true,
        layout: "normal",
        arte: 3,
        cinta: false,
    }]
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

/// El pack de Balatro con sus imágenes (Fase D): cada caso con cada estilo
/// propio, en español y en inglés, a `PB_SALIDA` (o la carpeta temporal).
/// `PB_BALATRO=<carpeta del pack> cargo test -p proxy-core --test dorados
/// balatro -- --ignored --nocapture`.
#[test]
#[ignore]
fn balatro_con_sus_imagenes() {
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
    let col = coleccion();
    let estilos: Vec<String> = pack
        .def
        .estilos
        .keys()
        .filter(|k| !proxy_core::diseno::pack::ESTILOS_DE_SERIE.contains(&k.as_str()))
        .cloned()
        .collect();
    for c in casos().into_iter().chain(casos_largos()) {
        for estilo in &estilos {
            for lang in [DesignLang::Es, DesignLang::En] {
                let mut d = diseno(&c);
                d.estilo_pack = Some(estilo.clone());
                let e = efectivo(&d, Some(&col));
                let r = render(
                    &mut motor,
                    &e,
                    0,
                    lang,
                    &rec,
                    CardSize::MTG,
                    745,
                    Salida::Pantalla,
                )
                .unwrap();
                let avisos: Vec<_> = r
                    .avisos
                    .iter()
                    .filter(|a| !matches!(a, Aviso::SinTextoOficial | Aviso::FaltaNombre { .. }))
                    .collect();
                if !avisos.is_empty() {
                    println!("{} {estilo} {lang:?}: {avisos:?}", c.archivo);
                }
                r.imagen
                    .save(salida.join(format!("{estilo}-{}-{lang:?}.png", c.archivo)))
                    .unwrap();
            }
        }
    }
}

/// Las cartas del autor de las plantillas, pintadas con el pack con su
/// propia ilustración (recortada de su carta) y sus mismos datos, para
/// ponerlas al lado del original. `PB_AUTOR=<carpeta del Drive>`.
#[test]
#[ignore]
fn balatro_contra_el_autor() {
    let (base, salida, pack) = pack_balatro();
    let autor = PathBuf::from(std::env::var("PB_AUTOR").expect("PB_AUTOR"));
    let refs: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(base.join("source/references.json")).unwrap(),
    )
    .unwrap();
    let mut artes = HashMap::new();
    let mut casos = Vec::new();
    for (i, r) in refs.as_array().unwrap().iter().enumerate() {
        let s = |k: &str| r[k].as_str().unwrap_or("").to_string();
        let lista = |k: &str| -> Vec<String> {
            r[k].as_array()
                .map(|v| {
                    v.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };
        let estilo = s("estilo");
        // La ilustración: la carta del autor recortada a la ventana del arte.
        let carta = image::open(autor.join(s("archivo"))).unwrap().to_rgba8();
        let carta = if carta.width() == 1365 {
            image::imageops::crop_imm(&carta, 62, 62, 1240, 1730).to_image()
        } else {
            carta
        };
        let def = pack.estilo(&estilo).unwrap();
        let layout_basica = r["tipo"].as_str().unwrap_or("").starts_with("Basic");
        let arte = if layout_basica {
            def.layouts
                .get("basica")
                .and_then(|l| l.arte.clone())
                .unwrap_or(def.arte.clone())
        } else {
            def.arte.clone()
        };
        let num = |v: &proxy_core::diseno::pack::Valor| match v {
            proxy_core::diseno::pack::Valor::Num(n) => *n,
            proxy_core::diseno::pack::Valor::Expr(_) => 0.0,
        };
        let (kx, ky) = (carta.width() as f32 / 63.0, carta.height() as f32 / 88.0);
        let recorte = image::imageops::crop_imm(
            &carta,
            (num(&arte.x) * kx) as u32,
            (num(&arte.y) * ky) as u32,
            (num(&arte.w) * kx) as u32,
            (num(&arte.h) * ky) as u32,
        )
        .to_image();
        let blob = format!("ref{i}");
        artes.insert(blob.clone(), Arc::new(recorte));
        let fr: Vec<String> = lista("fr");
        let cara = FaceData {
            name: s("nombre"),
            mana_cost: s("coste"),
            type_line: ByLang {
                es: s("tipo"),
                en: s("tipo"),
            },
            rules: ByLang {
                es: s("reglas"),
                en: s("reglas"),
            },
            printed_name_es: None,
            colors: lista("colores"),
            power: fr.first().cloned(),
            toughness: fr.get(1).cloned(),
            loyalty: r["lealtad"].as_str().map(String::from),
            defense: None,
        };
        let identidad = {
            let i = lista("identidad");
            if i.is_empty() {
                cara.colors.clone()
            } else {
                i
            }
        };
        let mut d = CardDesign {
            version: VERSION_DISENO,
            status: DesignStatus::Ready,
            collection: Some("col".into()),
            style: DesignStyle::Classic,
            estilo_pack: Some(estilo.clone()),
            variante_pack: r["variante"].as_str().map(String::from),
            frame: None,
            faces: vec![FaceDesign {
                art: Some(ArtRef {
                    image: img(&blob),
                    source: ArtSource::Upload,
                    framing: ArtFraming::default(),
                }),
                texts: ByLang {
                    es: FaceTexts {
                        name: s("nombre"),
                        flavor: String::new(),
                        rules_override: None,
                    },
                    en: FaceTexts {
                        name: s("nombre"),
                        flavor: String::new(),
                        rules_override: None,
                    },
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
                layout: r["layout"].as_str().unwrap_or("normal").into(),
                rarity: s("rareza"),
                frame_effects: if r["legendaria"].as_bool().unwrap_or(false) {
                    vec!["legendary".into()]
                } else {
                    vec![]
                },
                colors: cara.colors.clone(),
                color_identity: identidad,
                set: "jok".into(),
                collector_number: "1".into(),
                artist: None,
                faces: vec![cara],
                ..CardData::default()
            },
            footer: FooterOptions::default(),
            options: DesignOptions::default(),
            emblem: EmblemChoice::Inherit,
            thumb_key: None,
        };
        d.options.show_flavor = false;
        casos.push((s("archivo"), d));
    }
    let (_, simbolos, _) = entorno();
    let simbolos = match &pack.simbolos {
        Some(p) => simbolos.con_propios(p.clone()),
        None => simbolos,
    };
    let artes = Artes(artes);
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &artes,
    };
    let mut motor = motor_balatro(&base);
    let col = coleccion();
    for (archivo, d) in casos {
        let e = efectivo(&d, Some(&col));
        let r = render(
            &mut motor,
            &e,
            0,
            DesignLang::En,
            &rec,
            CardSize::MTG,
            745,
            Salida::Pantalla,
        )
        .unwrap();
        println!("{archivo}: {:?}", r.avisos);
        let nombre = archivo.replace('/', "_");
        r.imagen.save(salida.join(format!("ref-{nombre}"))).unwrap();
    }
}
