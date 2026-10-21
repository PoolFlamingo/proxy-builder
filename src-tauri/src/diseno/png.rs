//! **Exportar imágenes PNG** de las cartas diseñadas (Fase 6, paso 6.2 del
//! plan de la 2.0): cada cara a la medida exacta del corte y a los PPP
//! pedidos, cada idioma en su carpeta (`es/`, `en/`, `fr/`…), con el nombre
//! `NN - Nombre.png`.

use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use proxy_core::diseno::componer::medida;
use proxy_core::diseno::maquetacion::caras_impresas;
use proxy_core::diseno::modelo::DesignLang;
use proxy_core::diseno::render::Salida;
use proxy_core::model::{CardSize, DeckCard};
use serde::Serialize;
use ts_rs::TS;

use super::pintar::{colecciones_por_id, efectivo_de, pintar};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DesignPngProgressEvent {
    pub job_id: u32,
    pub done: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct DesignPngDoneEvent {
    pub job_id: u32,
    /// Ficheros escritos.
    pub files: u32,
    pub error: Option<AppError>,
}

/// Un nombre de fichero sin lo que los sistemas no admiten.
pub fn nombre_de_fichero(s: &str) -> String {
    let limpio: String = s
        .replace(" // ", " - ")
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    limpio.trim().trim_end_matches('.').to_string()
}

/// Cada idioma en su carpeta, con su código (`es/`, `en/`, `fr/`…).
fn carpeta(lang: DesignLang) -> &'static str {
    lang.codigo()
}

/// Lo que hay que escribir: una entrada por diseño distinto, cara e idioma
/// (las copias de una carta salen una vez).
struct Tarea {
    card: DeckCard,
    cara: u32,
    lang: DesignLang,
}

fn tareas(cartas: &[DeckCard], langs: &[DesignLang]) -> Vec<Tarea> {
    let mut vistos = HashSet::new();
    let mut out = Vec::new();
    for c in cartas {
        let Some(d) = &c.design else { continue };
        // Lo que no cambia la imagen no cuenta: dos copias iguales salen una vez.
        let mut solo = d.clone();
        solo.thumb_key = None;
        solo.status = proxy_core::diseno::modelo::DesignStatus::Draft;
        solo.before = proxy_core::diseno::modelo::BeforeDesign {
            source: proxy_core::model::CardSource::Blank,
            front: None,
            back: proxy_core::model::BackSpec::DeckDefault,
            edits: proxy_core::model::CardEdits::default(),
        };
        let clave = serde_json::to_string(&solo).unwrap_or_default();
        if !vistos.insert(clave) {
            continue;
        }
        let caras = caras_impresas(&d.card) as u32;
        for &lang in langs {
            for cara in 0..caras {
                out.push(Tarea {
                    card: c.clone(),
                    cara,
                    lang,
                });
            }
        }
    }
    out
}

/// Escribe los PNG. `progreso(hechos, total)` tras cada fichero.
#[allow(clippy::too_many_arguments)]
pub fn exportar(
    state: &AppState,
    cartas: &[DeckCard],
    destino: &Path,
    card: CardSize,
    ppp: u32,
    langs: &[DesignLang],
    cancelado: &AtomicBool,
    progreso: &dyn Fn(u32, u32),
) -> AppResult<u32> {
    let cols = colecciones_por_id(state)?;
    let lista = tareas(cartas, langs);
    let total = lista.len() as u32;
    let io = |e: std::io::Error| AppError::new(ErrorCode::Io).with("detalle", e);
    for lang in langs {
        std::fs::create_dir_all(destino.join(carpeta(*lang))).map_err(io)?;
    }
    let mut escritos = 0;
    let mut usados: HashSet<String> = HashSet::new();
    for (i, t) in lista.iter().enumerate() {
        if cancelado.load(Ordering::SeqCst) {
            return Err(AppError::new(ErrorCode::Cancelled));
        }
        let Some(d) = &t.card.design else { continue };
        let e = efectivo_de(d, &cols);
        let (w_mm, _) = medida(&e, card);
        let ancho = (w_mm / 25.4 * f64::from(ppp)).round() as u32;
        let r = pintar(state, &e, t.cara, t.lang, card, ancho, Salida::Impresion)?;
        let nombre = proxy_core::diseno::borrador::nombre_efectivo(d, t.cara as usize, t.lang);
        let base = nombre_de_fichero(&format!("{} - {}", d.footer.number, nombre));
        let base = if caras_impresas(&d.card) > 1 {
            format!("{base} ({})", t.cara + 1)
        } else {
            base
        };
        // Dos diseños distintos con el mismo número y nombre no se pisan.
        let mut fichero = format!("{base}.png");
        let mut n = 2;
        while !usados.insert(format!("{}/{fichero}", carpeta(t.lang))) {
            fichero = format!("{base} ~{n}.png");
            n += 1;
        }
        let bytes = proxy_core::imaging::encode::png(&r.imagen)
            .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
        std::fs::write(destino.join(carpeta(t.lang)).join(fichero), bytes).map_err(io)?;
        escritos += 1;
        progreso(i as u32 + 1, total);
    }
    Ok(escritos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Paths;
    use proxy_core::diseno::modelo::{ByLang, CardData, FaceData};
    use proxy_core::model::{BackSpec, CardEdits, CardMeta, CardSource};

    #[test]
    fn nombres_validos() {
        assert_eq!(
            nombre_de_fichero("0001 - Fuego // Hielo"),
            "0001 - Fuego - Hielo"
        );
        assert_eq!(nombre_de_fichero("¿Qué? <x>."), "¿Qué- -x-");
    }

    /// Medida exacta del corte a los PPP pedidos, carpetas por idioma y
    /// mismos bytes al repetir.
    #[test]
    fn png_a_su_medida_y_siempre_iguales() {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        let c = DeckCard {
            id: "c".into(),
            source: CardSource::Scryfall {
                scryfall_id: "x".into(),
            },
            front: None,
            back: BackSpec::DeckDefault,
            edits: CardEdits::default(),
            locked: false,
            meta: CardMeta::default(),
            design: None,
            marca: None,
        };
        let datos = CardData {
            scryfall_id: "x".into(),
            layout: "normal".into(),
            rarity: "common".into(),
            set: "tst".into(),
            collector_number: "1".into(),
            faces: vec![FaceData {
                name: "Sol Ring".into(),
                type_line: ByLang {
                    es: "Artefacto".into(),
                    en: "Artifact".into(),
                    otros: Default::default(),
                },
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        let mut c2 = c.clone();
        c2.design = Some(crate::diseno::borrador::montar(&c, datos, None, &[]).design);
        let cartas = vec![c2.clone(), c2];
        let destino = dir.path().join("png");
        let n = exportar(
            &s,
            &cartas,
            &destino,
            CardSize::MTG,
            300,
            &[DesignLang::Es, DesignLang::En],
            &AtomicBool::new(false),
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(n, 2, "las copias salen una vez, en los dos idiomas");
        let es = destino.join("es/0001 - Sol Ring.png");
        let img = image::open(&es).unwrap();
        // 63 × 88 mm a 300 PPP.
        assert_eq!((img.width(), img.height()), (744, 1039));
        assert!(destino.join("en/0001 - Sol Ring.png").exists());
        let antes = std::fs::read(&es).unwrap();
        exportar(
            &s,
            &cartas,
            &destino,
            CardSize::MTG,
            300,
            &[DesignLang::Es],
            &AtomicBool::new(false),
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(std::fs::read(&es).unwrap(), antes);
        // Otro idioma, en su carpeta.
        let n = exportar(
            &s,
            &cartas,
            &destino,
            CardSize::MTG,
            300,
            &[DesignLang::Fr],
            &AtomicBool::new(false),
            &|_, _| {},
        )
        .unwrap();
        assert_eq!(n, 1);
        assert!(destino.join("fr/0001 - Sol Ring.png").exists());
    }
}
