//! **OCR del nombre de una carta propia** (opcional): lee la franja de arriba
//! de la imagen, donde va el nombre, y la línea pequeña de abajo, donde muchas
//! cartas propias ponen el nombre original («0001 S Jodah, Archmage Eternal»
//! en el hueco del ilustrador), para proponer la carta real de Scryfall.
//!
//! Motor: `ocrs` (Rust puro, sin librerías del sistema). Sus dos modelos
//! (~12 MB) no van dentro de la app: se descargan la primera vez que se usa
//! el OCR a `app_data_dir/modelos/` y se quedan ahí. Sin OCR la app no los
//! necesita para nada.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use image::RgbaImage;
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::blobs::escribir_atomico;

const MODELOS: [(&str, &str); 2] = [
    (
        "text-detection.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.rten",
    ),
    (
        "text-recognition.rten",
        "https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.rten",
    ),
];

/// Proporción de la altura que se lee: el nombre y, en muchas cartas propias,
/// el nombre original en pequeño justo debajo.
const FRANJA_TITULO: f32 = 0.16;

/// Desde qué altura (del centro de la línea) empieza el pie: número, rareza
/// y, en las propias, el nombre original.
const FRANJA_PIE: f32 = 0.94;

static MOTOR: OnceLock<OcrEngine> = OnceLock::new();

fn fallo(e: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::Internal).with("ocr", e.to_string())
}

pub fn carpeta_modelos(datos: &Path) -> PathBuf {
    datos.join("modelos")
}

/// ¿Están ya los modelos en disco?
pub fn modelos_listos(datos: &Path) -> bool {
    let dir = carpeta_modelos(datos);
    MODELOS.iter().all(|(n, _)| {
        std::fs::metadata(dir.join(n))
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    })
}

/// Descarga los modelos que falten.
pub async fn descargar_modelos(datos: &Path) -> AppResult<()> {
    let dir = carpeta_modelos(datos);
    let http = reqwest::Client::builder()
        .user_agent(concat!("ProxyBuilder/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
    for (nombre, url) in MODELOS {
        let destino = dir.join(nombre);
        if destino.exists() {
            continue;
        }
        let r = http
            .get(url)
            .send()
            .await
            .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
        tracing::info!(
            host = "ocrs-models",
            modelo = nombre,
            estado = r.status().as_u16(),
            "red"
        );
        if !r.status().is_success() {
            return Err(AppError::new(ErrorCode::Network).with("estado", r.status().as_u16()));
        }
        let bytes = r
            .bytes()
            .await
            .map_err(|e| AppError::new(ErrorCode::Network).with("detalle", e))?;
        escribir_atomico(&destino, &bytes)?;
    }
    Ok(())
}

fn motor(datos: &Path) -> AppResult<&'static OcrEngine> {
    if let Some(m) = MOTOR.get() {
        return Ok(m);
    }
    let dir = carpeta_modelos(datos);
    let deteccion = Model::load_file(dir.join(MODELOS[0].0)).map_err(fallo)?;
    let reconocimiento = Model::load_file(dir.join(MODELOS[1].0)).map_err(fallo)?;
    let m = OcrEngine::new(OcrEngineParams {
        detection_model: Some(deteccion),
        recognition_model: Some(reconocimiento),
        ..Default::default()
    })
    .map_err(fallo)?;
    // Si dos hilos llegan a la vez, se queda el primero; el otro se tira.
    let _ = MOTOR.set(m);
    MOTOR.get().ok_or_else(|| fallo("motor"))
}

/// Lo que se ha leído en una imagen: las líneas del título y los nombres de
/// la línea de abajo.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lectura {
    pub titulo: Vec<String>,
    pub pie: Vec<String>,
    pub resto: Vec<String>,
    /// Pone «Token» o «Ficha» en algún sitio (el pie o la línea de tipo).
    pub ficha: bool,
}

/// Lee la carta entera, a la resolución original de la imagen, y reparte las
/// líneas: las de arriba son el título; las de abajo, el pie; el resto (el
/// texto de reglas, la línea de tipo…) va en `resto`, por si el nombre está en
/// otro sitio. Síncrono y pesado: se llama desde un hilo aparte.
pub fn leer_carta(datos: &Path, img: &RgbaImage) -> AppResult<Lectura> {
    let motor = motor(datos)?;
    let mut l = Lectura::default();
    for (texto, y) in lineas(motor, img.clone())? {
        if es_de_ficha(&texto) {
            l.ficha = true;
        }
        if y < FRANJA_TITULO {
            let t = limpiar(&texto);
            if parece_un_nombre(&t) {
                l.titulo.push(t);
            }
        } else if y > FRANJA_PIE || parece_pie(&texto) {
            l.pie.extend(nombre_del_pie(&texto));
        } else {
            let t = limpiar(&texto);
            if parece_un_nombre(&t) {
                l.resto.push(t);
            }
        }
    }
    Ok(l)
}

/// Las líneas de texto de una imagen, de arriba abajo, con la altura de su
/// centro (0 = arriba, 1 = abajo).
fn lineas(motor: &OcrEngine, img: RgbaImage) -> AppResult<Vec<(String, f32)>> {
    let alto = img.height().max(1) as f32;
    let rgb = image::DynamicImage::ImageRgba8(img).into_rgb8();
    let fuente = ImageSource::from_bytes(rgb.as_raw(), rgb.dimensions()).map_err(fallo)?;
    let entrada = motor.prepare_input(fuente).map_err(fallo)?;
    let palabras = motor.detect_words(&entrada).map_err(fallo)?;
    let lineas = motor.find_text_lines(&entrada, &palabras);
    let textos = motor.recognize_text(&entrada, &lineas).map_err(fallo)?;
    Ok(lineas
        .iter()
        .zip(textos)
        .filter_map(|(rects, texto)| {
            let texto = texto?.to_string();
            let y = rects.iter().map(|r| r.center().y).sum::<f32>() / rects.len().max(1) as f32;
            Some((texto, y / alto))
        })
        .collect())
}

/// ¿Empieza como la línea del pie? Número de colección (o `T002` en las
/// fichas) y la letra de rareza: «0057 S Menmarch». Hay cartas con el pie más
/// arriba de lo normal, así que no basta con la altura.
pub fn parece_pie(linea: &str) -> bool {
    let mut p = linea.split_whitespace();
    let numero = p.next().is_some_and(|n| {
        let digitos = n.chars().filter(char::is_ascii_digit).count();
        digitos >= 2 && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '/')
    });
    let rareza = p
        .next()
        .is_some_and(|r| r.chars().count() == 1 && r.chars().all(|c| c.is_ascii_uppercase()));
    numero && rareza
}

/// ¿Dice que es una ficha? («T Token», «Ficha de criatura — Guerrero»).
pub fn es_de_ficha(linea: &str) -> bool {
    linea
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w.eq_ignore_ascii_case("token") || w.eq_ignore_ascii_case("ficha"))
}

/// El nombre que pueda haber en una línea del pie: se quitan el número de
/// colección y la letra de rareza del principio («0001 S Jodah, Archmage
/// Eternal» → «Jodah, Archmage Eternal»). Las líneas sin minúsculas son el
/// código del set y el idioma («RTS ES»), y las del copyright, ruido.
pub fn nombre_del_pie(linea: &str) -> Option<String> {
    let limpia = limpiar(linea);
    let palabras: Vec<&str> = limpia.split_whitespace().collect();
    let inicio = palabras
        .iter()
        .position(|p| {
            !(p.chars().any(|c| c.is_ascii_digit())
                || p.chars().count() <= 1
                || p.eq_ignore_ascii_case("illus"))
        })
        .unwrap_or(palabras.len());
    let nombre = palabras[inicio..].join(" ");
    let minusculas = nombre.chars().any(char::is_lowercase);
    let copyright = nombre.to_lowercase().contains("wizards");
    // En las fichas, en el hueco del nombre pone «Token».
    let ficha = es_de_ficha(&nombre) && nombre.split_whitespace().count() == 1;
    (minusculas && !copyright && !ficha && parece_un_nombre(&nombre)).then_some(nombre)
}

/// Descarta el ruido típico de la franja del título: los símbolos de maná y
/// del set se leen como ristras cortas en mayúsculas («CER», «BOOO»).
pub fn parece_un_nombre(l: &str) -> bool {
    let letras: Vec<char> = l.chars().filter(|c| c.is_alphabetic()).collect();
    if letras.len() < 3 {
        return false;
    }
    let mayusculas = letras.iter().all(|c| c.is_uppercase());
    let vocal = letras
        .iter()
        .any(|c| "aeiouáéíóúàèìòùâêîôûäëïöüAEIOUÁÉÍÓÚ".contains(*c));
    // Una sola palabra corta con mayúsculas por dentro («OOofC») tampoco.
    let una_palabra = !l.contains(' ');
    let mayus_dentro = letras.iter().skip(1).any(|c| c.is_uppercase());
    vocal && !(mayusculas && letras.len() < 6) && !(una_palabra && mayus_dentro && letras.len() < 8)
}

/// Quita lo que no puede ser parte de un nombre (símbolos de maná leídos
/// como letras sueltas, comillas, cifras al final…).
pub fn limpiar(s: &str) -> String {
    let solo: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " ,'-".contains(c) {
                c
            } else {
                ' '
            }
        })
        .collect();
    let palabras: Vec<&str> = solo
        .split_whitespace()
        // Una letra suelta al final suele ser un símbolo de maná.
        .collect();
    let mut fin = palabras.len();
    while fin > 0 && palabras[fin - 1].chars().count() <= 1 {
        fin -= 1;
    }
    palabras[..fin]
        .join(" ")
        .trim_matches(|c| c == ',' || c == '-')
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ruido_no_parece_un_nombre() {
        for ruido in ["CER", "BOOO", "OOofC", "xx", "123"] {
            assert!(!parece_un_nombre(&limpiar(ruido)), "{ruido}");
        }
        for nombre in [
            "Swords to Plowshares",
            "Atraxa, Praetors' Voice",
            "Orca",
            "City of Brass",
        ] {
            assert!(parece_un_nombre(nombre), "{nombre}");
        }
    }

    #[test]
    fn el_nombre_del_pie() {
        assert_eq!(
            nombre_del_pie("0001   S  Jodah, Archmage Eternal").as_deref(),
            Some("Jodah, Archmage Eternal")
        );
        assert_eq!(
            nombre_del_pie("0123/0280 R Illus. John Avon").as_deref(),
            Some("John Avon")
        );
        assert_eq!(nombre_del_pie("RTS★ES  S.J."), None);
        assert_eq!(nombre_del_pie("0001 S"), None);
        assert_eq!(nombre_del_pie("T002 T Token"), None);
        assert!(parece_pie("0057 S Menmarch"));
        assert!(parece_pie("T002 T Token"));
        assert!(!parece_pie("Criatura artefacto legendaria"));
        assert!(!parece_pie("200 Fl permanente objetivo se"));
        assert!(es_de_ficha("Ficha de artefacto -Tesoro"));
        assert!(!es_de_ficha("Fichaje"));
        assert_eq!(nombre_del_pie("TM & © 2024 Wizards of the Coast"), None);
    }

    #[test]
    fn limpiar_quita_ruido() {
        assert_eq!(
            limpiar("Castillo Real de Herbaria  ®"),
            "Castillo Real de Herbaria"
        );
        assert_eq!(
            limpiar("Orca, Colmillo del Cielo Azul 1 U"),
            "Orca, Colmillo del Cielo Azul"
        );
        assert_eq!(limpiar("“City of Brass”"), "City of Brass");
    }
}

/// Prueba en vivo: `PB_OCR_DATOS=<carpeta con modelos/> PB_OCR_IMG=<imagen>
/// cargo test --lib ocr::en_vivo -- --ignored --nocapture`.
#[cfg(test)]
mod en_vivo {
    #[test]
    #[ignore]
    fn lee_el_titulo_de_una_imagen() {
        let datos = std::path::PathBuf::from(std::env::var("PB_OCR_DATOS").unwrap());
        let bytes = std::fs::read(std::env::var("PB_OCR_IMG").unwrap()).unwrap();
        let img = proxy_core::imaging::decode::decode(&bytes).unwrap();
        let t = std::time::Instant::now();
        let l = super::leer_carta(&datos, &img).unwrap();
        println!("{l:?} en {:?}", t.elapsed());
        assert!(!l.titulo.is_empty() || !l.pie.is_empty());
    }
}
