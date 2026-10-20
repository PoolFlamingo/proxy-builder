//! **Vista previa en directo** del editor: la pantalla registra el diseño
//! que se está editando y pide la imagen por `pbimg://…/d/<huella>/<ancho>`.
//! Así no viajan bytes por el IPC y WebKit cachea por URL. Se guardan los 64
//! diseños más recientes (crece con cada tecla).

use std::collections::{HashMap, VecDeque};

use image::RgbaImage;
use proxy_core::diseno::efectivo::DisenoEfectivo;
use proxy_core::diseno::maquetacion::maquetacion;
use proxy_core::diseno::modelo::{ArtFraming, Aviso, CardDesign, DesignLang, DesignStatus};
use proxy_core::diseno::render::Salida;
use proxy_core::model::{BackSpec, CardEdits, CardSize, CardSource, DeckCard, ImageOrigin};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::pintar::{colecciones_por_id, efectivo_de, huella_de, pintar, pintar_capa};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::state::AppState;

const MAX: usize = 64;
/// Ancho de la miniatura que se guarda como `front` de la carta.
pub const ANCHO_MINIATURA: u32 = 1000;

#[derive(Default)]
pub struct Vistas {
    m: HashMap<String, (DisenoEfectivo, u32, DesignLang)>,
    orden: VecDeque<String>,
}

impl Vistas {
    fn meter(&mut self, huella: String, v: (DisenoEfectivo, u32, DesignLang)) {
        if !self.m.contains_key(&huella) {
            self.orden.push_back(huella.clone());
        }
        self.m.insert(huella, v);
        while self.orden.len() > MAX {
            if let Some(v) = self.orden.pop_front() {
                self.m.remove(&v);
            }
        }
    }

    pub fn get(&self, huella: &str) -> Option<(DisenoEfectivo, u32, DesignLang)> {
        self.m.get(huella).cloned()
    }
}

/// Registra un diseño para la vista previa y devuelve su huella.
pub fn registrar(
    state: &AppState,
    d: &CardDesign,
    cara: u32,
    lang: DesignLang,
) -> AppResult<String> {
    let cols = colecciones_por_id(state)?;
    let e = efectivo_de(d, &cols);
    let h = huella_de(state, &e, cara, lang)?;
    state
        .vistas
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?
        .meter(h.clone(), (e, cara, lang));
    Ok(h)
}

/// La imagen de la vista previa (PNG con las esquinas redondeadas), de la
/// caché si ya está.
pub fn imagen(state: &AppState, huella: &str, ancho: u32) -> AppResult<Vec<u8>> {
    let ruta = state
        .carpeta_renders()
        .join("d")
        .join(format!("{huella}-{ancho}.png"));
    if let Ok(b) = std::fs::read(&ruta) {
        return Ok(b);
    }
    let (e, cara, lang) = state
        .vistas
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?
        .get(huella)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("diseno", huella))?;
    let r = pintar(
        state,
        &e,
        cara,
        lang,
        CardSize::MTG,
        ancho,
        Salida::Pantalla,
    )?;
    let bytes = proxy_core::imaging::encode::png(&r.imagen)?;
    let _ = crate::store::blobs::escribir_atomico(&ruta, &bytes);
    Ok(bytes)
}

/// Para mover la ilustración sin esperar a Rust: la carta sin ilustración
/// (`pbimg://…/c/<huella>/<ancho>`) y dónde va la ilustración. La huella no
/// depende del encuadre, así que vale mientras solo cambia el encuadre.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CapaInfo {
    pub huella: String,
    /// El hueco de la ilustración en fracciones de la carta: x, y, ancho, alto.
    pub arte: [f32; 4],
    /// Cuánto sigue la ilustración por debajo del hueco (extensión del
    /// difuminado), en fracciones del alto de la carta.
    #[serde(default)]
    pub extension: f32,
    /// El radio de las esquinas en fracciones del ancho y del alto.
    pub radio: [f32; 2],
    /// La cara del diseño cuya ilustración va en el hueco.
    pub cara: u32,
}

/// Lados aceptados de la ilustración reducida (`/b/<blob>/<lado>`).
pub const LADOS_ARTE: [u32; 2] = [1000, 2000];

fn ruta_capa(state: &AppState, huella: &str, ancho: u32) -> std::path::PathBuf {
    state
        .carpeta_renders()
        .join("c")
        .join(format!("{huella}-{ancho}.png"))
}

fn ruta_info_capa(state: &AppState, huella: &str) -> std::path::PathBuf {
    state
        .carpeta_renders()
        .join("c")
        .join(format!("{huella}.json"))
}

/// Pinta (o saca de la caché) la capa de un diseño. `None` si la cara no la
/// tiene (compuestas, grandes) o no lleva ilustración.
pub fn capa(
    state: &AppState,
    d: &CardDesign,
    cara: u32,
    lang: DesignLang,
    ancho: u32,
) -> AppResult<Option<CapaInfo>> {
    let cols = colecciones_por_id(state)?;
    let mut e = efectivo_de(d, &cols);
    for f in &mut e.design.faces {
        if let Some(a) = f.art.as_mut() {
            a.framing = ArtFraming::default();
        }
    }
    let h = huella_de(state, &e, cara, lang)?;
    state
        .vistas
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?
        .meter(h.clone(), (e.clone(), cara, lang));
    let (ruta, ruta_info) = (ruta_capa(state, &h, ancho), ruta_info_capa(state, &h));
    if ruta.exists() {
        if let Some(i) = std::fs::read(&ruta_info)
            .ok()
            .and_then(|b| serde_json::from_slice::<CapaInfo>(&b).ok())
        {
            return Ok(Some(i));
        }
    }
    let Some(c) = pintar_capa(state, &e, cara, lang, ancho)? else {
        return Ok(None);
    };
    let tiene_arte = e.design.faces.get(c.cara).is_some_and(|f| f.art.is_some());
    if !tiene_arte {
        return Ok(None);
    }
    let info = CapaInfo {
        huella: h,
        arte: c.arte,
        extension: c.extension,
        radio: c.radio,
        cara: c.cara as u32,
    };
    let _ =
        crate::store::blobs::escribir_atomico(&ruta, &proxy_core::imaging::encode::png(&c.imagen)?);
    if let Ok(b) = serde_json::to_vec(&info) {
        let _ = crate::store::blobs::escribir_atomico(&ruta_info, &b);
    }
    Ok(Some(info))
}

/// La imagen de una capa ya pedida con [`capa`] (PNG), de la caché si está.
pub fn imagen_capa(state: &AppState, huella: &str, ancho: u32) -> AppResult<Vec<u8>> {
    let ruta = ruta_capa(state, huella, ancho);
    if let Ok(b) = std::fs::read(&ruta) {
        return Ok(b);
    }
    let (e, cara, lang) = state
        .vistas
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?
        .get(huella)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("diseno", huella))?;
    let c = pintar_capa(state, &e, cara, lang, ancho)?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("diseno", huella))?;
    let bytes = proxy_core::imaging::encode::png(&c.imagen)?;
    let _ = crate::store::blobs::escribir_atomico(&ruta, &bytes);
    Ok(bytes)
}

/// Una ilustración del almacén reducida a `lado` px (el lado más largo), para
/// moverla en la vista previa. JPEG, o PNG si tiene transparencias. Devuelve
/// los bytes y el tipo.
pub fn arte_reducido(
    state: &AppState,
    blob: &str,
    lado: u32,
) -> AppResult<(Vec<u8>, &'static str)> {
    use proxy_core::diseno::render::Imagenes;
    let dir = state.carpeta_renders().join("b");
    let (jpg, png) = (
        dir.join(format!("{blob}-{lado}.jpg")),
        dir.join(format!("{blob}-{lado}.png")),
    );
    if let Ok(b) = std::fs::read(&jpg) {
        return Ok((b, "image/jpeg"));
    }
    if let Ok(b) = std::fs::read(&png) {
        return Ok((b, "image/png"));
    }
    let imagenes = super::recursos::ImagenesDelAlmacen {
        db: &state.db,
        blobs: &state.blobs,
        cache: &state.imagenes,
    };
    let img = imagenes
        .imagen(blob)
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("blob", blob))?;
    let r = proxy_core::diseno::render::reducida(&img, lado);
    let (bytes, tipo, ruta) = if r.pixels().any(|p| p[3] < 255) {
        (proxy_core::imaging::encode::png(&r)?, "image/png", png)
    } else {
        (
            proxy_core::imaging::encode::jpeg(&r, 88)?,
            "image/jpeg",
            jpg,
        )
    };
    let _ = crate::store::blobs::escribir_atomico(&ruta, &bytes);
    Ok((bytes, tipo))
}

/// Los avisos de un diseño (texto que no cabe, símbolos que faltan…), sin
/// guardar nada.
pub fn avisos(state: &AppState, d: &CardDesign, lang: DesignLang) -> AppResult<Vec<Aviso>> {
    let cols = colecciones_por_id(state)?;
    let e = efectivo_de(d, &cols);
    let mut v = Vec::new();
    if e.coleccion_perdida {
        v.push(Aviso::ColeccionPerdida);
    }
    for cara in 0..d.faces.len().max(1) as u32 {
        for a in pintar(state, &e, cara, lang, CardSize::MTG, 300, Salida::Impresion)?.avisos {
            if !v.contains(&a) {
                v.push(a);
            }
        }
    }
    Ok(v)
}

fn guardar_miniatura(state: &AppState, img: &RgbaImage) -> AppResult<proxy_core::model::FaceImage> {
    let bytes = proxy_core::imaging::encode::jpeg(img, 92)?;
    state
        .blobs
        .put_bytes(&state.db, &bytes, ImageOrigin::Designed)
}

/// Aplica un diseño a una carta: pinta la miniatura de 1000 px (y la de la
/// otra cara en las DFC), la guarda en el almacén y devuelve la carta nueva.
/// No escribe el deck: lo aplica la pantalla (deshacer sigue funcionando).
pub fn aplicar(
    state: &AppState,
    card: &DeckCard,
    d: &CardDesign,
    lang: DesignLang,
) -> AppResult<DeckCard> {
    let cols = colecciones_por_id(state)?;
    let e = efectivo_de(d, &cols);
    let clave = huella_de(state, &e, 0, lang)?;
    let frente = pintar(
        state,
        &e,
        0,
        lang,
        CardSize::MTG,
        ANCHO_MINIATURA,
        Salida::Impresion,
    )?;
    let front = guardar_miniatura(state, &frente.imagen)?;
    let dos_caras = d.faces.len() >= 2 && maquetacion(&d.card).0.caras_impresas() == 2;
    let back = if dos_caras {
        let atras = pintar(
            state,
            &e,
            1,
            lang,
            CardSize::MTG,
            ANCHO_MINIATURA,
            Salida::Impresion,
        )?;
        BackSpec::Dfc {
            image: guardar_miniatura(state, &atras.imagen)?,
        }
    } else {
        // Si antes era una DFC de Scryfall y el diseño es de una sola cara,
        // el dorso vuelve al del deck.
        match &d.before.back {
            BackSpec::Dfc { .. } => BackSpec::DeckDefault,
            otro => otro.clone(),
        }
    };
    let mut nueva = card.clone();
    nueva.source = CardSource::Custom {
        scryfall_id: Some(d.card.scryfall_id.clone()),
        externa: None,
    };
    nueva.front = Some(front);
    nueva.back = back;
    nueva.edits = CardEdits::default();
    nueva.meta.is_dfc = dos_caras;
    let mut d = d.clone();
    d.thumb_key = Some(clave);
    nueva.design = Some(d);
    Ok(nueva)
}

/// Rehace las miniaturas desfasadas de un deck (cambió una colección, una
/// letra, el pack o los símbolos) y devuelve solo las cartas que cambian.
pub fn rehacer(
    state: &AppState,
    cartas: &[DeckCard],
    lang: DesignLang,
) -> AppResult<Vec<DeckCard>> {
    let cols = colecciones_por_id(state)?;
    let mut cambiadas = Vec::new();
    for c in cartas {
        let Some(d) = &c.design else { continue };
        let e = efectivo_de(d, &cols);
        if d.thumb_key.as_deref() == Some(huella_de(state, &e, 0, lang)?.as_str()) {
            continue;
        }
        cambiadas.push(aplicar(state, c, d, lang)?);
    }
    Ok(cambiadas)
}

/// Vuelve a numerar los diseños por el orden del deck (`0001…`; las fichas,
/// `T001…`). Las copias de una carta comparten número y los escritos a mano
/// no cambian. Devuelve solo las cartas que cambian, ya con su miniatura.
pub fn renumerar(
    state: &AppState,
    cartas: &[DeckCard],
    lang: DesignLang,
) -> AppResult<Vec<DeckCard>> {
    use proxy_core::diseno::borrador::numero;
    let mut asignados: HashMap<String, String> = HashMap::new();
    let (mut n, mut f) = (0u32, 0u32);
    let mut cambiadas = Vec::new();
    for c in cartas {
        let Some(d) = &c.design else { continue };
        if d.footer.number_manual {
            continue;
        }
        let clave = d.card.scryfall_id.clone();
        let num = match asignados.get(&clave) {
            Some(x) => x.clone(),
            None => {
                let x = if d.card.es_ficha() {
                    f += 1;
                    numero(f, true)
                } else {
                    n += 1;
                    numero(n, false)
                };
                asignados.insert(clave, x.clone());
                x
            }
        };
        if d.footer.number != num {
            let mut nuevo = d.clone();
            nuevo.footer.number = num;
            cambiadas.push(aplicar(state, c, &nuevo, lang)?);
        }
    }
    Ok(cambiadas)
}

/// Quita el diseño y deja la carta como estaba antes.
pub fn quitar(card: &DeckCard) -> DeckCard {
    let mut c = card.clone();
    if let Some(d) = c.design.take() {
        c.source = d.before.source;
        c.front = d.before.front;
        c.back = d.before.back;
        c.edits = d.before.edits;
    }
    c
}

/// Marca el diseño como listo o borrador.
pub fn con_estado(mut d: CardDesign, listo: bool) -> CardDesign {
    d.status = if listo {
        DesignStatus::Ready
    } else {
        DesignStatus::Draft
    };
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, Paths};
    use proxy_core::diseno::modelo::{ByLang, CardData, FaceData};
    use proxy_core::model::{CardMeta, FaceImage};

    fn estado() -> (tempfile::TempDir, AppState) {
        let dir = tempfile::tempdir().unwrap();
        let s = AppState::open(Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        (dir, s)
    }

    fn carta(s: &AppState) -> (DeckCard, CardDesign) {
        let img: FaceImage = s
            .blobs
            .put_bytes(
                &s.db,
                &crate::store::blobs::tests::png(63, 88, 4),
                ImageOrigin::Upload,
            )
            .unwrap();
        let c = DeckCard {
            id: "c".into(),
            source: CardSource::Scryfall {
                scryfall_id: "x".into(),
            },
            front: Some(img),
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
                mana_cost: "{1}".into(),
                type_line: ByLang {
                    es: "Artefacto".into(),
                    en: "Artifact".into(),
                },
                rules: ByLang {
                    es: "{T}: Agrega {C}{C}.".into(),
                    en: "{T}: Add {C}{C}.".into(),
                },
                ..FaceData::default()
            }],
            ..CardData::default()
        };
        let d = crate::diseno::borrador::montar(&c, datos, None, &[]).design;
        (c, d)
    }

    #[test]
    fn vista_previa_guardar_y_quitar() {
        let (_dir, s) = estado();
        let (c, d) = carta(&s);
        let h = registrar(&s, &d, 0, DesignLang::Es).unwrap();
        let png = imagen(&s, &h, 488).unwrap();
        let i = image::load_from_memory(&png).unwrap();
        assert_eq!(i.width(), 488);
        // De la caché, igual.
        assert_eq!(imagen(&s, &h, 488).unwrap(), png);
        // Una huella sin registrar no existe.
        assert_eq!(
            imagen(&s, &"f".repeat(64), 488).unwrap_err().code,
            ErrorCode::NotFound
        );
        // Sin símbolos de Scryfall descargados, se avisa.
        let av = avisos(&s, &d, DesignLang::Es).unwrap();
        assert!(av.iter().any(|a| matches!(a, Aviso::FaltanSimbolos { .. })));

        let nueva = aplicar(&s, &c, &d, DesignLang::Es).unwrap();
        let f = nueva.front.clone().unwrap();
        assert_eq!(f.origin, ImageOrigin::Designed);
        assert_eq!(f.width, ANCHO_MINIATURA);
        assert!(matches!(
            nueva.source,
            CardSource::Custom {
                scryfall_id: Some(_),
                ..
            }
        ));
        let vuelta = quitar(&nueva);
        assert_eq!(vuelta, c);

        // Al día: nada que rehacer. Con otra colección (otra huella), sí.
        assert!(rehacer(&s, std::slice::from_ref(&nueva), DesignLang::Es)
            .unwrap()
            .is_empty());
        let col = crate::store::colecciones::save(
            &s.db,
            &crate::store::colecciones::tests::nueva("Tensura 2"),
        )
        .unwrap();
        let mut otra = nueva.clone();
        if let Some(d) = otra.design.as_mut() {
            d.collection = Some(col.id.clone());
        }
        let r = rehacer(&s, &[otra], DesignLang::Es).unwrap();
        assert_eq!(r.len(), 1);
        assert_ne!(
            r[0].design.as_ref().unwrap().thumb_key,
            nueva.design.as_ref().unwrap().thumb_key
        );
    }

    #[test]
    fn capa_para_mover_la_ilustracion() {
        use proxy_core::diseno::modelo::{ArtRef, ArtSource};
        let (_dir, s) = estado();
        let (c, mut d) = carta(&s);
        // Sin ilustración no hay capa.
        assert!(capa(&s, &d, 0, DesignLang::Es, 488).unwrap().is_none());
        let img = s
            .blobs
            .put_bytes(
                &s.db,
                &crate::store::blobs::tests::png(1500, 900, 7),
                ImageOrigin::Upload,
            )
            .unwrap();
        d.faces[0].art = Some(ArtRef {
            image: img.clone(),
            source: ArtSource::Upload,
            framing: ArtFraming::default(),
        });
        let a = capa(&s, &d, 0, DesignLang::Es, 488).unwrap().unwrap();
        assert_eq!(a.cara, 0);
        assert!(a.arte[2] > 0.5 && a.arte[3] > 0.2);
        // Otro encuadre: la misma capa (y de la caché).
        if let Some(x) = d.faces[0].art.as_mut() {
            x.framing.zoom = 2.0;
            x.framing.offset_x = 0.5;
        }
        assert_eq!(capa(&s, &d, 0, DesignLang::Es, 488).unwrap().unwrap(), a);
        let png = imagen_capa(&s, &a.huella, 488).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 488);
        // A otro ancho la pinta al pedirla.
        let png = imagen_capa(&s, &a.huella, 745).unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 745);
        // La ilustración reducida: el lado largo, como mucho el pedido.
        let (b, tipo) = arte_reducido(&s, &img.blob, 1000).unwrap();
        assert_eq!(tipo, "image/jpeg");
        let r = image::load_from_memory(&b).unwrap();
        assert_eq!((r.width(), r.height()), (1000, 600));
        assert_eq!(arte_reducido(&s, &img.blob, 1000).unwrap().0, b);
        assert_eq!(
            arte_reducido(&s, &"f".repeat(64), 1000).unwrap_err().code,
            ErrorCode::NotFound
        );
        let _ = c;
    }

    /// Paso 6.1: 3 cartas y 2 fichas numeradas como en «Diseñar el deck»;
    /// mover no cambia nada hasta «Renumerar»; un número a mano sobrevive.
    #[test]
    fn numeros_de_un_deck_y_renumerar() {
        let (_dir, s) = estado();
        let (base, _) = carta(&s);
        let hacer = |id: &str, ficha: bool, usados: &[String]| {
            let mut c = base.clone();
            c.id = id.into();
            let datos = CardData {
                scryfall_id: format!("s-{id}"),
                layout: if ficha { "token" } else { "normal" }.into(),
                rarity: "common".into(),
                set: "tst".into(),
                collector_number: "1".into(),
                faces: vec![FaceData {
                    name: id.into(),
                    type_line: ByLang {
                        es: if ficha { "Criatura ficha" } else { "Artefacto" }.into(),
                        en: if ficha { "Token Creature" } else { "Artifact" }.into(),
                    },
                    ..FaceData::default()
                }],
                ..CardData::default()
            };
            let d = crate::diseno::borrador::montar(&c, datos, None, usados).design;
            aplicar(&s, &c, &d, DesignLang::Es).unwrap()
        };
        let mut usados = Vec::new();
        let mut deck = Vec::new();
        for (id, ficha) in [
            ("a", false),
            ("t1", true),
            ("b", false),
            ("t2", true),
            ("c", false),
        ] {
            let c = hacer(id, ficha, &usados);
            usados.push(c.design.as_ref().unwrap().footer.number.clone());
            deck.push(c);
        }
        let nums = |d: &[DeckCard]| -> Vec<String> {
            d.iter()
                .map(|c| c.design.as_ref().unwrap().footer.number.clone())
                .collect()
        };
        assert_eq!(nums(&deck), ["0001", "T001", "0002", "T002", "0003"]);
        // Mover la última al principio no cambia ningún número...
        let ultima = deck.pop().unwrap();
        deck.insert(0, ultima);
        assert_eq!(nums(&deck), ["0003", "0001", "T001", "0002", "T002"]);
        // ...hasta renumerar. Con un número a mano en «b», que se queda.
        if let Some(d) = deck[3].design.as_mut() {
            d.footer.number = "0099".into();
            d.footer.number_manual = true;
        }
        let cambiadas = renumerar(&s, &deck, DesignLang::Es).unwrap();
        for c in cambiadas {
            let i = deck.iter().position(|x| x.id == c.id).unwrap();
            deck[i] = c;
        }
        assert_eq!(nums(&deck), ["0001", "0002", "T001", "0099", "T002"]);
    }
}
