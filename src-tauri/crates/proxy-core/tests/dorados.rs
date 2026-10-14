//! **Dorados del motor de diseño** (Paso 2.9 del plan de la 2.0): una imagen
//! de referencia por caso base y estilo, para no romper sin querer el
//! aspecto aprobado. Arte y símbolos de prueba hechos aquí (nada con
//! derechos), letras y pack de la casa.
//!
//! `PB_ACTUALIZAR_DORADOS=1 cargo test -p proxy-core --test dorados` los
//! reescribe (solo con el visto bueno de Pablo).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use image::{Rgba, RgbaImage};
use proxy_core::diseno::efectivo::efectivo;
use proxy_core::diseno::modelo::*;
use proxy_core::diseno::pack::Pack;
use proxy_core::diseno::render::{render, Imagenes, Motor, Recursos, Salida};
use proxy_core::diseno::simbolos::Simbolos;
use proxy_core::model::{BackSpec, CardEdits, CardSize, CardSource, FaceImage, ImageOrigin};

const ANCHO: u32 = 420;

struct Artes(HashMap<String, Arc<RgbaImage>>);

impl Imagenes for Artes {
    fn imagen(&self, blob: &str) -> Option<Arc<RgbaImage>> {
        self.0.get(blob).cloned()
    }
}

/// Un «arte» de prueba: degradado con círculos, siempre igual.
fn arte(semilla: u32) -> RgbaImage {
    RgbaImage::from_fn(600, 840, |x, y| {
        let (fx, fy) = (x as f32 / 600.0, y as f32 / 840.0);
        let s = semilla as f32;
        let cx = 300.0 + 120.0 * (s * 1.7).sin();
        let cy = 380.0 + 90.0 * (s * 2.3).cos();
        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
        let anillo = if ((d / 40.0) as u32).is_multiple_of(2) {
            30.0
        } else {
            0.0
        };
        Rgba([
            (60.0 + 150.0 * fx + anillo + s * 10.0) as u8,
            (40.0 + 120.0 * fy + anillo) as u8,
            (120.0 + 90.0 * (1.0 - fx) * fy) as u8,
            255,
        ])
    })
}

/// Un emblema de prueba: un óvalo azul con brillo.
fn emblema() -> RgbaImage {
    RgbaImage::from_fn(90, 60, |x, y| {
        let (dx, dy) = ((x as f32 - 45.0) / 44.0, (y as f32 - 30.0) / 28.0);
        if dx * dx + dy * dy <= 1.0 {
            Rgba([90, 160, 240, 255])
        } else {
            Rgba([0, 0, 0, 0])
        }
    })
}

/// Símbolos de prueba dibujados aquí: un círculo de color y una marca
/// distinta por símbolo (sin texto, que usvg no pinta sin letras).
fn simbolos() -> Simbolos {
    let mut s = Simbolos::nuevos();
    let colores = [
        (
            "W",
            "#f8f6d8",
            "M50 20 L58 42 L80 50 L58 58 L50 80 L42 58 L20 50 L42 42 Z",
        ),
        (
            "U",
            "#aae0fa",
            "M50 18 C66 44 72 58 50 82 C28 58 34 44 50 18 Z",
        ),
        ("B", "#cbc2bf", "M30 35 H70 V65 H30 Z"),
        ("R", "#f9aa8f", "M50 20 L80 75 H20 Z"),
        (
            "G",
            "#9bd3ae",
            "M50 18 L74 50 L60 50 L74 80 H26 L40 50 L26 50 Z",
        ),
        ("C", "#cccccc", "M50 25 L75 50 L50 75 L25 50 Z"),
        (
            "T",
            "#cac5c0",
            "M30 30 C55 20 75 40 70 60 L80 60 L65 78 L52 60 L60 60 C62 45 50 35 30 40 Z",
        ),
        ("1", "#cac5c0", "M46 25 H56 V75 H46 Z"),
        (
            "2",
            "#cac5c0",
            "M35 30 H65 V50 H45 V65 H65 V75 H35 V45 H55 V40 H35 Z",
        ),
        (
            "3",
            "#cac5c0",
            "M35 25 H65 V75 H35 V65 H55 V55 H40 V45 H55 V35 H35 Z",
        ),
    ];
    for (c, fondo, marca) in colores {
        let svg = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="48" fill="{fondo}"/><path d="{marca}" fill="#111111"/></svg>"##
        );
        s.cargar(c, svg.as_bytes()).unwrap();
    }
    s
}

fn img(blob: &str) -> FaceImage {
    FaceImage {
        blob: blob.into(),
        width: 1,
        height: 1,
        origin: ImageOrigin::Upload,
    }
}

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
            ..DesignOptions::default()
        },
        emblem: EmblemChoice::Inherit,
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
    };
    vec![
        base("despejado-legendaria", DesignStyle::Borderless),
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

/// Porcentaje de píxeles con algún canal más de 8/255 distinto.
fn diferencia(a: &RgbaImage, b: &RgbaImage) -> f64 {
    if a.dimensions() != b.dimensions() {
        return 100.0;
    }
    let distintos = a
        .pixels()
        .zip(b.pixels())
        .filter(|(p, q)| p.0.iter().zip(q.0.iter()).any(|(x, y)| x.abs_diff(*y) > 8))
        .count();
    distintos as f64 * 100.0 / (a.width() * a.height()) as f64
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
    let j = ficheros["pack.json"].replace("\"medio\": \"#d9ab3c\"", "\"medio\": \"#ff00ff\"");
    ficheros.insert("pack.json".into(), j);
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
