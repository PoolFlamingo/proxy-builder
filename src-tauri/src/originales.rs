//! **El nombre original de una carta propia, esté donde esté.** Cada autor lo
//! pone a su manera: bajo el título, en el pie a la izquierda o a la derecha,
//! detrás del artista («NEO • SP ➤ NESTOR OSSANDON LEAL | Mirror Box»), entre
//! paréntesis en la línea de tipo, en versalitas, partido en dos líneas… Así
//! que no se supone nada: se prueban **todos los trozos** de **todas las
//! líneas** que lee el OCR contra el catálogo local de nombres de Scryfall
//! (sin red) y se puntúa cada acierto por dónde está y cómo es, para que una
//! palabra suelta del texto de reglas («Fog», «Shock») no se cuele.
//!
//! También se lee la colección, el número y el idioma del pie («250/302 R» +
//! «NEO • SP»), que dan la carta exacta.

use std::collections::HashSet;

use crate::commands::identify::normalizar;
use crate::ocr::{es_linea_de_tipo, parece_pie, LineaLeida, FRANJA_PIE, FRANJA_TITULO};
use crate::scryfall::nombres::Indice;

/// Lo que separa trozos en una línea (un nombre nunca los cruza).
const SEPARADORES: &[char] = &[
    '|', '•', '·', '—', '–', '/', '(', ')', '[', ']', '{', '}', '«', '»', '"', '“', '”', ':', ';',
    '✎', '★', '©', '™', '®', '➤', '►', '▶', '→', '~', '*', '@',
];
/// Cuántas palabras tiene como mucho un nombre que se busca.
const MAX_PALABRAS: usize = 6;
/// Cuántos trozos se prueban como mucho con la búsqueda aproximada (recorre
/// todo el catálogo: es lenta).
const MAX_APROXIMADOS: usize = 6;
/// Desde cuántos puntos un acierto es de fiar.
pub const PUNTOS_SEGURO: f32 = 3.0;

/// Dónde está una línea en la carta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zona {
    Titulo,
    Pie,
    Tipo,
    Reglas,
}

/// Un nombre real encontrado en la carta.
#[derive(Debug, Clone, PartialEq)]
pub struct Acierto {
    pub nombre: String,
    pub puntos: f32,
    /// Encontrado por parecido (una letra mal leída), no exacto.
    pub aproximado: bool,
}

fn zona(l: &LineaLeida) -> Zona {
    if l.y < FRANJA_TITULO {
        Zona::Titulo
    } else if l.y > FRANJA_PIE || parece_pie(&l.texto) {
        Zona::Pie
    } else if es_linea_de_tipo(&l.texto) {
        Zona::Tipo
    } else {
        Zona::Reglas
    }
}

/// Los trozos de una línea, cada uno con sus palabras. El copyright fuera.
fn trozos(texto: &str) -> Vec<Vec<String>> {
    texto
        .split(SEPARADORES)
        .flat_map(|t| t.split("   "))
        .map(|t| t.split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .filter(|p| !p.is_empty())
        .filter(|p| !normalizar(&p.join(" ")).contains("wizards"))
        .collect()
}

/// Apunta un acierto; si el nombre ya estaba, se queda con los mejores puntos.
fn apuntar(aciertos: &mut Vec<Acierto>, nombre: String, puntos: f32, aproximado: bool) {
    match aciertos.iter_mut().find(|a| a.nombre == nombre) {
        Some(a) if a.puntos >= puntos => {}
        Some(a) => {
            a.puntos = puntos;
            a.aproximado = aproximado;
        }
        None => aciertos.push(Acierto {
            nombre,
            puntos,
            aproximado,
        }),
    }
}

/// Busca nombres en todas las líneas. El mejor primero; cada nombre una vez.
pub fn buscar(lineas: &[LineaLeida], indice: &Indice) -> Vec<Acierto> {
    let alto_titulo = lineas
        .iter()
        .filter(|l| l.y < FRANJA_TITULO)
        .map(|l| l.alto)
        .fold(0.0_f32, f32::max);
    let mut aciertos: Vec<Acierto> = Vec::new();

    // Las líneas y, además, cada una unida a la siguiente si parecen la misma
    // frase partida (misma zona, letra del mismo tamaño, muy juntas).
    let mut textos: Vec<(String, Zona, f32)> = lineas
        .iter()
        .map(|l| (l.texto.clone(), zona(l), l.alto))
        .collect();
    for par in lineas.windows(2) {
        let (a, b) = (&par[0], &par[1]);
        let (za, zb) = (zona(a), zona(b));
        let alto = a.alto.max(b.alto);
        if za == zb
            && za != Zona::Reglas
            && (a.alto - b.alto).abs() <= 0.25 * alto
            && b.y - a.y <= 1.8 * alto
        {
            textos.push((format!("{} {}", a.texto, b.texto), za, alto));
        }
    }

    for (texto, z, alto) in &textos {
        let pequena = alto_titulo > 0.0 && *alto < alto_titulo * 0.7;
        for palabras in trozos(texto) {
            let n = palabras.len();
            for k in (1..=MAX_PALABRAS.min(n)).rev() {
                for i in 0..=n - k {
                    let completo = k == n;
                    // Una palabra suelta solo cuenta si es el trozo entero y
                    // no está en el texto de reglas: «Fog» en una frase no es
                    // el nombre de la carta.
                    if k == 1 && (!completo || *z == Zona::Reglas) {
                        continue;
                    }
                    let Some(nombre) = indice.exacto(&palabras[i..i + k].join(" ")) else {
                        continue;
                    };
                    let mut puntos = if k >= 2 { 3.0 + 0.5 * k as f32 } else { 1.0 };
                    if completo {
                        puntos += 1.0;
                    }
                    puntos += match z {
                        Zona::Titulo | Zona::Pie => 2.0,
                        Zona::Tipo => 1.0,
                        Zona::Reglas => -2.0,
                    };
                    if pequena {
                        puntos += 0.5;
                    }
                    if puntos >= PUNTOS_SEGURO {
                        apuntar(&mut aciertos, nombre, puntos, false);
                    }
                }
            }
        }
    }

    // Nada de dos palabras o más: lo más parecido (una letra mal leída) de
    // unos pocos trozos enteros del título y del pie.
    if !aciertos.iter().any(|a| a.nombre.contains(' ')) {
        let mut probados = 0;
        for (texto, z, _) in &textos {
            if !matches!(z, Zona::Titulo | Zona::Pie) {
                continue;
            }
            for palabras in trozos(texto) {
                if palabras.len() < 2 || probados >= MAX_APROXIMADOS {
                    continue;
                }
                probados += 1;
                if let Some(h) = indice.parecido(&palabras.join(" ")) {
                    apuntar(&mut aciertos, h.nombre, PUNTOS_SEGURO, true);
                }
            }
        }
    }

    aciertos.sort_by(|a, b| b.puntos.total_cmp(&a.puntos));
    aciertos
}

/// La colección, el número y el idioma leídos en el pie.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PieLeido {
    /// Código del set, en minúsculas («neo»).
    pub set: String,
    /// Número de colección sin ceros delante («250»).
    pub numero: String,
    /// Idioma con el código de Scryfall («es»), si viene.
    pub idioma: Option<String>,
}

/// El idioma de un pie («SP», «ES», «JP»…) con el código de Scryfall.
pub fn idioma_del_pie(codigo: &str) -> Option<&'static str> {
    Some(match codigo.to_uppercase().as_str() {
        "EN" => "en",
        "ES" | "SP" => "es",
        "FR" => "fr",
        "DE" => "de",
        "IT" => "it",
        "PT" => "pt",
        "JP" | "JA" => "ja",
        "KO" | "KR" => "ko",
        "RU" => "ru",
        "CS" => "zhs",
        "CT" => "zht",
        "PH" => "ph",
        _ => return None,
    })
}

/// El número de colección de una palabra («250/302» → «250», «0057» → «57»).
fn numero(palabra: &str) -> Option<String> {
    let n = palabra.split('/').next()?;
    if n.is_empty() || n.len() > 4 || !n.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let total = palabra.split('/').nth(1);
    if total.is_some_and(|t| t.is_empty() || !t.chars().all(|c| c.is_ascii_digit())) {
        return None;
    }
    let sin_ceros = n.trim_start_matches('0');
    Some(if sin_ceros.is_empty() { "0" } else { sin_ceros }.to_string())
}

/// Lee el set, el número y el idioma en las líneas de abajo (o en las que
/// parecen un pie aunque estén más arriba). `sets`: los códigos de Scryfall en
/// minúsculas. Sin set o sin número, nada: uno solo no dice qué carta es.
pub fn pie_leido(lineas: &[LineaLeida], sets: &HashSet<String>) -> Option<PieLeido> {
    let mut num: Option<String> = None;
    let mut set: Option<String> = None;
    let mut idioma: Option<String> = None;
    for l in lineas.iter().filter(|l| l.y > 0.85 || parece_pie(&l.texto)) {
        let palabras: Vec<String> = l
            .texto
            .split(|c: char| c.is_whitespace() || (SEPARADORES.contains(&c) && c != '/'))
            .map(|p| p.trim_matches(|c: char| !c.is_alphanumeric() && c != '/'))
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect();
        for (i, p) in palabras.iter().enumerate() {
            if num.is_none() {
                if let Some(n) = numero(p) {
                    num = Some(n);
                    continue;
                }
            }
            if set.is_some() {
                continue;
            }
            let es_codigo = p.len() >= 2
                && p.len() <= 7
                && p.chars().all(|c| c.is_ascii_alphanumeric())
                && p.chars().any(|c| c.is_ascii_alphabetic())
                && p.chars().all(|c| !c.is_ascii_lowercase());
            if !es_codigo {
                continue;
            }
            let minus = p.to_lowercase();
            if sets.contains(&minus) {
                set = Some(minus);
                idioma = palabras
                    .get(i + 1)
                    .and_then(|s| idioma_del_pie(s))
                    .map(str::to_string);
                continue;
            }
            // El set pegado al idioma («NEOSP»: el OCR se comió el «•»).
            for corte in 2..p.len() {
                let (a, b) = p.split_at(corte);
                if let (true, Some(l)) = (sets.contains(&a.to_lowercase()), idioma_del_pie(b)) {
                    set = Some(a.to_lowercase());
                    idioma = Some(l.to_string());
                    break;
                }
            }
        }
    }
    Some(PieLeido {
        set: set?,
        numero: num?,
        idioma,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indice() -> Indice {
        Indice::nuevo(
            &[
                "Mirror Box",
                "Fog",
                "Shock",
                "Opt",
                "Island",
                "Jodah, Archmage Eternal",
                "Lathliss, Dragon Queen",
                "Lightning Bolt",
            ]
            .map(String::from),
        )
    }

    fn l(texto: &str, y: f32, alto: f32) -> LineaLeida {
        LineaLeida {
            texto: texto.into(),
            y,
            alto,
        }
    }

    fn mejor(lineas: &[LineaLeida]) -> Option<String> {
        buscar(lineas, &indice()).first().map(|a| a.nombre.clone())
    }

    /// El caso real de «Espejos de Izis» (lo que leyó el OCR el 19/10/2026).
    fn espejos() -> Vec<LineaLeida> {
        vec![
            l("Espejos de Izis", 0.05, 0.03),
            l("Artefacto", 0.585, 0.022),
            l("La xregla de leyendas no Se aplica a los", 0.67, 0.02),
            l("Cada criatura legendaria que controlas", 0.74, 0.02),
            l("Cada criatura que nO sea ficha que controlas", 0.8, 0.02),
            l("250/302 R TM & © 2022 Wizards of the Coast", 0.946, 0.012),
            l("NEOSP NESTOR OSSANDON LEAI Mirror BoX", 0.963, 0.012),
        ]
    }

    #[test]
    fn el_original_detras_del_artista() {
        let a = buscar(&espejos(), &indice());
        assert_eq!(a[0].nombre, "Mirror Box");
        assert!(a[0].puntos >= PUNTOS_SEGURO);
        assert!(!a[0].aproximado);
    }

    #[test]
    fn el_original_venga_como_venga() {
        // Pie clásico: número, rareza y nombre.
        assert_eq!(
            mejor(&[
                l("Rimuru, Rey Demonio", 0.05, 0.03),
                l("0001 S Jodah, Archmage Eternal", 0.96, 0.012)
            ])
            .as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        // En letra pequeña bajo el título.
        assert_eq!(
            mejor(&[
                l("Rimuru, Rey Demonio", 0.05, 0.03),
                l("Jodah, Archmage Eternal", 0.1, 0.014)
            ])
            .as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        // Entre paréntesis en la línea de tipo.
        assert_eq!(
            mejor(&[
                l("Lathliss, la Reina Dragón", 0.05, 0.03),
                l(
                    "Criatura legendaria — Dragón (Lathliss, Dragon Queen)",
                    0.56,
                    0.02
                )
            ])
            .as_deref(),
            Some("Lathliss, Dragon Queen")
        );
        // A la izquierda del pie, en versalitas.
        assert_eq!(
            mejor(&[
                l("Espejos de Izis", 0.05, 0.03),
                l("MIRROR BOX 250/302 R", 0.96, 0.012)
            ])
            .as_deref(),
            Some("Mirror Box")
        );
        // Partido en dos líneas.
        assert_eq!(
            mejor(&[
                l("Rimuru", 0.05, 0.03),
                l("Jodah, Archmage", 0.955, 0.012),
                l("Eternal", 0.968, 0.012)
            ])
            .as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        // Una letra mal leída.
        assert_eq!(
            mejor(&[
                l("Rimuru", 0.05, 0.03),
                l("Jodah, Archmage Eterna", 0.96, 0.012)
            ])
            .as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        // El título real manda (una carta que no es propia).
        assert_eq!(
            mejor(&[
                l("Lightning Bolt", 0.05, 0.03),
                l("Lightning Bolt deals 3 damage to any target.", 0.7, 0.02)
            ])
            .as_deref(),
            Some("Lightning Bolt")
        );
    }

    #[test]
    fn una_palabra_en_las_reglas_no_es_el_nombre() {
        let a = buscar(
            &[
                l("Niebla de Izis", 0.05, 0.03),
                l("Instant", 0.57, 0.02),
                l("Cast Fog and Shock this turn.", 0.7, 0.02),
                l("Fog", 0.75, 0.02),
                l("Island", 0.8, 0.02),
            ],
            &indice(),
        );
        assert!(a.is_empty(), "{a:?}");
        // En el pie, una palabra sola sí vale («| Opt»).
        assert_eq!(
            mejor(&[
                l("Destello", 0.05, 0.03),
                l("JOSE PEREZ | Opt", 0.96, 0.012)
            ])
            .as_deref(),
            Some("Opt")
        );
    }

    #[test]
    fn el_pie_da_set_numero_e_idioma() {
        let sets: HashSet<String> = ["neo", "dmu", "m21", "jok"].map(String::from).into();
        assert_eq!(
            pie_leido(&espejos(), &sets),
            Some(PieLeido {
                set: "neo".into(),
                numero: "250".into(),
                idioma: Some("es".into()),
            })
        );
        assert_eq!(
            pie_leido(
                &[
                    l("0001 M", 0.95, 0.01),
                    l("DMU • EN ✎ Chris Rahn", 0.965, 0.01)
                ],
                &sets
            ),
            Some(PieLeido {
                set: "dmu".into(),
                numero: "1".into(),
                idioma: Some("en".into()),
            })
        );
        // Sin set conocido o sin número, nada.
        assert_eq!(pie_leido(&[l("XYZ • SP 012/100", 0.96, 0.01)], &sets), None);
        assert_eq!(pie_leido(&[l("NEO • SP", 0.96, 0.01)], &sets), None);
    }

    #[test]
    fn rapido_con_muchas_lineas() {
        let nombres: Vec<String> = (0..30_000).map(|i| format!("Carta numero {i}")).collect();
        let i = Indice::nuevo(&nombres);
        let lineas: Vec<LineaLeida> = (0..40)
            .map(|n| {
                l(
                    "Cada criatura que controlas obtiene más uno barra más uno hasta el final",
                    0.2 + n as f32 * 0.018,
                    0.02,
                )
            })
            .collect();
        let t = std::time::Instant::now();
        let _ = buscar(&lineas, &i);
        assert!(
            t.elapsed() < std::time::Duration::from_millis(500),
            "{:?}",
            t.elapsed()
        );
    }
}

/// Prueba en vivo con una imagen: `PB_OCR_DATOS=<app_data_dir> PB_OCR_IMG=<png>
/// cargo test --lib originales::en_vivo -- --ignored --nocapture`. Descarga el
/// catálogo de nombres y la lista de sets a una carpeta temporal.
#[cfg(test)]
mod en_vivo {
    use super::*;
    use crate::scryfall::client::{CardApi, Scryfall};
    use crate::scryfall::nombres;

    #[tokio::test]
    #[ignore]
    async fn nombre_original_de_una_imagen() {
        let datos = std::path::PathBuf::from(std::env::var("PB_OCR_DATOS").unwrap());
        let bytes = std::fs::read(std::env::var("PB_OCR_IMG").unwrap()).unwrap();
        let img = proxy_core::imaging::decode::decode(&bytes).unwrap();
        let leido = crate::ocr::leer_carta(&datos, &img).unwrap();
        for l in &leido.lineas {
            println!("{:.3} {:.3}  {}", l.y, l.alto, l.texto);
        }
        let cache = std::env::temp_dir().join("pb-originales");
        std::fs::create_dir_all(&cache).unwrap();
        let s = Scryfall::new().unwrap();
        let indice = nombres::indice(&s, &cache).await.unwrap();
        let t = std::time::Instant::now();
        let a = buscar(&leido.lineas, &indice);
        println!("aciertos {a:?} en {:?}", t.elapsed());
        let sets = nombres::codigos_de_sets(&s, &cache).await.unwrap();
        let pie = pie_leido(&leido.lineas, &sets);
        println!("pie {pie:?}, ficha {}", leido.ficha);
        if let Some(p) = &pie {
            let c = s.card_lang(&p.set, &p.numero, "en").await.unwrap();
            println!("carta del pie: {:?}", c.map(|c| c.name));
        }
    }
}
