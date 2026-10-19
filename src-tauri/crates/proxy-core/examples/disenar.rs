//! Pinta unas cartas de prueba con el motor de verdad (Fase 2), para mirarlas.
//!
//! `cargo run --release -p proxy-core --example disenar -- <simbolos> <artes> <salida> [ancho]`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use image::RgbaImage;
use proxy_core::diseno::efectivo::efectivo;
use proxy_core::diseno::modelo::*;
use proxy_core::diseno::pack::Pack;
use proxy_core::diseno::render::{render, Imagenes, Motor, Recursos, Salida};
use proxy_core::diseno::simbolos::Simbolos;
use proxy_core::model::{BackSpec, CardEdits, CardSize, CardSource, FaceImage, ImageOrigin};

struct Disco(HashMap<String, Arc<RgbaImage>>);

impl Imagenes for Disco {
    fn imagen(&self, blob: &str) -> Option<Arc<RgbaImage>> {
        self.0.get(blob).cloned()
    }
}

fn img(blob: &str) -> FaceImage {
    FaceImage {
        blob: blob.into(),
        width: 1,
        height: 1,
        origin: ImageOrigin::Upload,
    }
}

fn cara(
    name: &str,
    es_name: &str,
    coste: &str,
    tipo: (&str, &str),
    reglas: (&str, &str),
    colores: &[&str],
    fr: Option<(&str, &str)>,
) -> FaceData {
    FaceData {
        name: name.into(),
        mana_cost: coste.into(),
        type_line: ByLang {
            es: tipo.0.into(),
            en: tipo.1.into(),
        },
        rules: ByLang {
            es: reglas.0.into(),
            en: reglas.1.into(),
        },
        printed_name_es: Some(es_name.into()),
        colors: colores.iter().map(|c| c.to_string()).collect(),
        power: fr.map(|f| f.0.into()),
        toughness: fr.map(|f| f.1.into()),
        loyalty: None,
        defense: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn diseno(
    estilo: DesignStyle,
    datos: FaceData,
    rareza: &str,
    legendaria: bool,
    nombre: (&str, &str),
    sabor: &str,
    arte: Option<&str>,
    numero: &str,
    layout: &str,
) -> CardDesign {
    CardDesign {
        version: VERSION_DISENO,
        status: DesignStatus::Draft,
        collection: Some("rts".into()),
        style: estilo,
        estilo_pack: None,
        variante_pack: None,
        frame: None,
        faces: vec![FaceDesign {
            art: arte.map(|a| ArtRef {
                image: img(a),
                source: ArtSource::Upload,
                framing: ArtFraming::default(),
            }),
            texts: ByLang {
                es: FaceTexts {
                    name: nombre.0.into(),
                    flavor: sabor.into(),
                    rules_override: None,
                },
                en: FaceTexts {
                    name: nombre.1.into(),
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
            layout: layout.into(),
            rarity: rareza.into(),
            frame_effects: if legendaria {
                vec!["legendary".into()]
            } else {
                vec![]
            },
            colors: datos.colors.clone(),
            color_identity: datos.colors.clone(),
            set: "con".into(),
            collector_number: "1".into(),
            artist: Some("Artista".into()),
            faces: vec![datos],
            ..CardData::default()
        },
        footer: FooterOptions {
            number: numero.into(),
            ..FooterOptions::default()
        },
        options: DesignOptions {
            show_flavor: true,
            ..DesignOptions::default()
        },
        emblem: EmblemChoice::Inherit,
        thumb_key: None,
    }
}

fn coleccion(original: OriginalName, pie: &str) -> Collection {
    Collection {
        id: "rts".into(),
        name: "Tensura 2".into(),
        default_style: DesignStyle::Borderless,
        estilo_pack_defecto: None,
        frame: FrameOptions::default(),
        emblem: Some(img("emblema")),
        set_code: "RTS".into(),
        rarity_letter: Some("S".into()),
        year: Some("2024".into()),
        artist: Some("S.J.".into()),
        credit: Some("Bandai Namco (Isekai Memories)".into()),
        universe: None,
        original_name: original,
        footer_template: ByLang {
            es: pie.into(),
            en: pie.into(),
        },
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

fn simbolos(dir: &Path) -> Simbolos {
    let mut s = Simbolos::nuevos();
    for f in std::fs::read_dir(dir).unwrap().flatten() {
        let p = f.path();
        if p.extension().is_some_and(|e| e == "svg") {
            let n = p.file_stem().unwrap().to_string_lossy().replace('_', "/");
            let _ = s.cargar(&n, &std::fs::read(&p).unwrap());
        }
    }
    s
}

fn main() {
    let a: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let (simb, artes, salida) = (&a[0], &a[1], &a[2]);
    let ancho: u32 = a
        .get(3)
        .and_then(|s| s.to_str()?.parse().ok())
        .unwrap_or(1060);
    std::fs::create_dir_all(salida).unwrap();
    let mut mapa = HashMap::new();
    for n in ["veldanava", "milim", "reino", "emblema"] {
        let i = image::open(artes.join(format!("{n}.png")))
            .unwrap()
            .to_rgba8();
        mapa.insert(n.to_string(), Arc::new(i));
    }
    let disco = Disco(mapa);
    let pack = Pack::de_fabrica().unwrap();
    let simbolos = simbolos(simb);
    let rec = Recursos {
        pack: &pack,
        simbolos: &simbolos,
        imagenes: &disco,
    };
    let mut motor = Motor::de_fabrica().unwrap();

    let rts = coleccion(
        OriginalName::Footer,
        "{num}\t{rarity}\t{original}\n{set}★{lang}\t{brush} {artist}",
    );
    let isekai = coleccion(
        OriginalName::Ribbon,
        "{year} • MTG • {lang} • {brush} {credit}",
    );

    let veldanava = diseno(
        DesignStyle::Borderless,
        cara(
            "Progenitus",
            "Progenitus",
            "{W}{W}{U}{U}{B}{B}{R}{R}{G}{G}",
            ("Criatura legendaria — Avatar Hidra", "Legendary Creature — Hydra Avatar"),
            (
                "Protección contra todo.\nSi Progenitus fuera a ir a un cementerio desde cualquier lugar, revela a Progenitus y barájalo en la biblioteca de su propietario en su lugar.",
                "Protection from everything\nIf Progenitus would be put into a graveyard from anywhere, reveal Progenitus and shuffle it into its owner's library instead.",
            ),
            &["W", "U", "B", "R", "G"],
            Some(("10", "10")),
        ),
        "mythic",
        true,
        ("Veldanava, Dragón de la Creación", "Veldanava, Dragon of Creation"),
        "“Te daré mi destrucción y la llamaré una bendición.”",
        Some("veldanava"),
        "0002",
        "normal",
    );
    let mut milim = diseno(
        DesignStyle::Extended,
        cara(
            "Arcades, the Strategist",
            "Arcades, el Estratega",
            "{1}{G}{W}{U}",
            ("Criatura legendaria — Dragón anciano", "Legendary Creature — Elder Dragon"),
            (
                "Vuela, vigilancia.\nSiempre que una criatura con defensor entre al campo de batalla bajo tu control, roba una carta.\nCada criatura que controlas con defensor asigna una cantidad de daño de combate igual a su resistencia en vez de su fuerza, y puede atacar como si no tuviera defensor.",
                "Flying, vigilance\nWhenever a creature with defender enters the battlefield under your control, draw a card.\nEach creature you control with defender assigns combat damage equal to its toughness rather than its power and can attack as though it didn't have defender.",
            ),
            &["G", "W", "U"],
            Some(("3", "5")),
        ),
        "mythic",
        true,
        ("Milim, Señora Demonio Dragonoide", "Milim, Dragonoid Demon Lord"),
        "",
        Some("milim"),
        "0001",
        "normal",
    );
    milim.collection = Some("rts".into());
    let reino = diseno(
        DesignStyle::FullArt,
        cara(
            "Plains",
            "Llanura",
            "",
            ("Tierra básica — Llanura", "Basic Land — Plains"),
            ("", ""),
            &[],
            None,
        ),
        "common",
        false,
        ("Reino Orco de Orbic", "Orc Kingdom of Orbic"),
        "",
        Some("reino"),
        "0076",
        "normal",
    );
    let mut reino = reino;
    reino.card.color_identity = vec!["W".into()];
    let zegion = diseno(
        DesignStyle::Borderless,
        cara(
            "Moonsilver Key",
            "Llave de plata lunar",
            "{2}",
            ("Artefacto", "Artifact"),
            (
                "{1}, {T}, sacrificar la Llave de plata lunar: Busca en tu biblioteca una carta de artefacto con una habilidad de maná o una carta de tierra básica, muéstrala, ponla en tu mano y luego baraja.",
                "{1}, {T}, Sacrifice Moonsilver Key: Search your library for an artifact card with a mana ability or a basic land card, reveal it, put it into your hand, then shuffle.",
            ),
            &[],
            None,
        ),
        "uncommon",
        false,
        ("Zegion, General Insecto", "Zegion, Insect General"),
        "“Esta es mi poder, no olvides que soy el Señor de la Niebla. Puedo convertir la realidad en ilusiones y la ilusión en realidad”.",
        Some("milim"),
        "0058",
        "normal",
    );
    let mut bicolor = diseno(
        DesignStyle::Classic,
        cara(
            "Wall of Mulch",
            "Muro de mantillo",
            "{1}{G}{U}",
            ("Criatura — Muro", "Creature — Wall"),
            (
                "Defensor. (Esta criatura no puede atacar.)\n{G}, sacrificar un Muro: Roba una carta.",
                "Defender\n{G}, Sacrifice a Wall: Draw a card.",
            ),
            &["G", "U"],
            Some(("0", "4")),
        ),
        "uncommon",
        false,
        ("Direwolves en entrenamiento", "Direwolves in Training"),
        "",
        Some("reino"),
        "0003",
        "normal",
    );
    bicolor.collection = Some("rts".into());
    let ficha = diseno(
        DesignStyle::Borderless,
        cara(
            "Warrior",
            "Guerrero",
            "",
            ("Criatura ficha — Guerrero", "Token Creature — Warrior"),
            ("", ""),
            &["W"],
            Some(("1", "1")),
        ),
        "common",
        false,
        ("Cruzado (Fritz)", "Crusader (Fritz)"),
        "",
        Some("veldanava"),
        "T001",
        "token",
    );

    let casos: Vec<(&str, &CardDesign, &Collection)> = vec![
        ("veldanava", &veldanava, &rts),
        ("milim", &milim, &isekai),
        ("reino", &reino, &rts),
        ("zegion", &zegion, &rts),
        ("bicolor", &bicolor, &isekai),
        ("ficha", &ficha, &rts),
    ];
    for (n, d, c) in casos {
        let e = efectivo(d, Some(c));
        for lang in [DesignLang::Es, DesignLang::En] {
            let t = Instant::now();
            let r = render(
                &mut motor,
                &e,
                0,
                lang,
                &rec,
                CardSize::MTG,
                ancho,
                Salida::Pantalla,
            )
            .unwrap();
            let l = if lang == DesignLang::Es { "es" } else { "en" };
            println!("{n} {l}: {:?} {:?}", t.elapsed(), r.avisos);
            r.imagen.save(salida.join(format!("{n}-{l}.png"))).unwrap();
        }
    }
}
