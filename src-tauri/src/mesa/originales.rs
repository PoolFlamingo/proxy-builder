//! **Originales en mi idioma** (E9): en los ajustes de cada jugador de la
//! mesa se puede elegir ver sus cartas con la impresión de Scryfall en el
//! idioma de la interfaz (con la inglesa de respaldo) en vez de sus proxies.
//! Es solo de la pantalla de quien lo pide: no se manda a nadie.
//!
//! Cada carta con vínculo a Scryfall (`OrigenCarta.scryfall_id`) se busca por
//! su set y su número (`CardApi::card_lang`, una petición por carta al ritmo
//! del limitador) y sus caras se bajan a `app_cache_dir/mesa/s/`, donde ya
//! las sirve `pbimg://…/m/s/<id>/<cara>/<ancho>`. Las propias sin vínculo y
//! los tapetes no pasan por aquí.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use mesa::modelo::RefImagen;
use serde::Serialize;
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::scryfall::models::ScryCard;

use super::imagenes::{es_id_scryfall, guardar_scryfall_en, scryfall_en};

/// Lo que la pantalla recibe (`mesa://originales`) cuando una carta ya tiene
/// su impresión en el idioma pedido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct OriginalMesa {
    /// La carta de la mesa (su impresión de Scryfall).
    pub scryfall_id: String,
    /// El idioma pedido (código de Scryfall).
    pub idioma: String,
    /// Sus caras en el idioma pedido o, si no existe, en inglés. Vacío: no
    /// hay otra impresión y la carta se queda como está.
    pub caras: Vec<RefImagen>,
}

/// El código de Scryfall al que se pide cada idioma de la interfaz: los que
/// no tienen cartas impresas (rumano, polaco, neerlandés, ucraniano) piden
/// inglés. `None`: un código que no es de la app.
pub fn idioma_de_cartas(codigo: &str) -> Option<&'static str> {
    Some(match codigo {
        "es" => "es",
        "en" => "en",
        "fr" => "fr",
        "de" => "de",
        "it" => "it",
        "pt" => "pt",
        "ru" => "ru",
        "ja" => "ja",
        "ko" => "ko",
        "zhs" => "zhs",
        "zht" => "zht",
        "ro" | "pl" | "nl" | "uk" => "en",
        _ => return None,
    })
}

/// La impresión de `carta` en `idioma` o, si no existe, en inglés; con sus
/// caras ya bajadas. `None` si no hay ninguna de las dos (o es la misma
/// carta y no hay nada que cambiar).
pub async fn localizar<A: CardApi>(
    api: &A,
    carpeta: &Path,
    carta: &ScryCard,
    idioma: &str,
) -> AppResult<Option<Vec<RefImagen>>> {
    let mut objetivo = None;
    for lang in [idioma, "en"] {
        if carta.lang == lang {
            objetivo = Some(carta.clone());
            break;
        }
        if let Some(c) = api
            .card_lang(&carta.set, &carta.collector_number, lang)
            .await?
        {
            objetivo = Some(c);
            break;
        }
    }
    let Some(objetivo) = objetivo else {
        return Ok(None);
    };
    if !es_id_scryfall(&objetivo.id) {
        return Err(AppError::new(ErrorCode::InvalidInput));
    }
    let n = if objetivo.is_dfc() { 2 } else { 1 };
    let mut caras = Vec::new();
    for i in 0..n {
        let Some((url, _)) = objetivo.print_url(i) else {
            continue;
        };
        let cara = i as u8;
        if !scryfall_en(carpeta, &objetivo.id, cara).exists() {
            let bytes = api.download(&url).await?;
            let img = proxy_core::imaging::decode::decode(&bytes)?;
            guardar_scryfall_en(carpeta, &objetivo.id, cara, &img)?;
        }
        caras.push(RefImagen::Scryfall {
            id: objetivo.id.clone(),
            cara,
            url,
        });
    }
    Ok((!caras.is_empty()).then_some(caras))
}

/// Lo que ya se resolvió (o se está resolviendo) en esta sesión de la app,
/// por carta e idioma: pedir lo mismo dos veces no repite peticiones.
#[derive(Debug, Clone)]
enum Estado {
    EnCurso,
    Hecho(Vec<RefImagen>),
}

static MEMORIA: OnceLock<Mutex<BTreeMap<(String, String), Estado>>> = OnceLock::new();

fn memoria() -> std::sync::MutexGuard<'static, BTreeMap<(String, String), Estado>> {
    match MEMORIA.get_or_init(Mutex::default).lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// Qué hacer con una carta que se pide: ya está (se reenvía), se está
/// haciendo (se espera su aviso) o toca resolverla (y se marca en curso).
pub enum Toca {
    Reenviar(OriginalMesa),
    Resolver,
    Esperar,
}

pub fn reclamar(scryfall_id: &str, idioma: &str) -> Toca {
    let mut m = memoria();
    let clave = (scryfall_id.to_string(), idioma.to_string());
    match m.get(&clave) {
        Some(Estado::Hecho(c)) => Toca::Reenviar(OriginalMesa {
            scryfall_id: clave.0,
            idioma: clave.1,
            caras: c.clone(),
        }),
        Some(Estado::EnCurso) => Toca::Esperar,
        None => {
            m.insert(clave, Estado::EnCurso);
            Toca::Resolver
        }
    }
}

/// Anota el resultado; si falló, se quita la marca para poder reintentar.
pub fn terminar(scryfall_id: &str, idioma: &str, caras: Option<Vec<RefImagen>>) {
    let mut m = memoria();
    let clave = (scryfall_id.to_string(), idioma.to_string());
    match caras {
        Some(c) => {
            m.insert(clave, Estado::Hecho(c));
        }
        None => {
            m.remove(&clave);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scryfall::models::{Collection, Identifier};
    use serde_json::json;
    use std::sync::Mutex;

    const EN: &str = "11111111-1111-1111-1111-111111111111";
    const ES: &str = "22222222-2222-2222-2222-222222222222";
    const DFC_EN: &str = "33333333-3333-3333-3333-333333333333";

    fn carta(v: serde_json::Value) -> ScryCard {
        serde_json::from_value(v).unwrap()
    }

    /// Un Scryfall de mentira: la impresión inglesa y la española de una
    /// carta, y una de dos caras que solo existe en inglés.
    struct Falso {
        cartas: Vec<ScryCard>,
        descargas: Mutex<Vec<String>>,
        consultas: Mutex<Vec<String>>,
    }

    fn falso() -> Falso {
        let img = |id: &str| json!({"png": format!("https://cards.scryfall.io/{id}.png")});
        Falso {
            cartas: vec![
                carta(
                    json!({"id":EN,"name":"Sol Ring","lang":"en","set":"cmm","collector_number":"410","layout":"normal","image_uris":img("en")}),
                ),
                carta(
                    json!({"id":ES,"name":"Sol Ring","lang":"es","set":"cmm","collector_number":"410","layout":"normal","image_uris":img("es")}),
                ),
                carta(
                    json!({"id":DFC_EN,"name":"Delver of Secrets // Insectile Aberration","lang":"en","set":"isd","collector_number":"51","layout":"transform",
                    "card_faces":[{"name":"Delver of Secrets","image_uris":img("d0")},{"name":"Insectile Aberration","image_uris":img("d1")}]}),
                ),
            ],
            descargas: Mutex::default(),
            consultas: Mutex::default(),
        }
    }

    impl CardApi for Falso {
        async fn collection(&self, _: &[Identifier]) -> AppResult<Collection> {
            Ok(Collection {
                data: vec![],
                not_found: vec![],
            })
        }
        async fn card(&self, id: &str) -> AppResult<Option<ScryCard>> {
            Ok(self.cartas.iter().find(|c| c.id == id).cloned())
        }
        async fn card_lang(&self, set: &str, n: &str, lang: &str) -> AppResult<Option<ScryCard>> {
            self.consultas.lock().unwrap().push(lang.to_string());
            Ok(self
                .cartas
                .iter()
                .find(|c| c.set == set && c.collector_number == n && c.lang == lang)
                .cloned())
        }
        async fn search(&self, _: &str, _: bool, _: usize) -> AppResult<Vec<ScryCard>> {
            Ok(vec![])
        }
        async fn download(&self, url: &str) -> AppResult<Vec<u8>> {
            self.descargas.lock().unwrap().push(url.to_string());
            let mut png = std::io::Cursor::new(Vec::new());
            image::RgbaImage::from_pixel(8, 11, image::Rgba([200, 30, 30, 255]))
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            Ok(png.into_inner())
        }
    }

    fn temporal(nombre: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("pb-orig-{nombre}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    fn id_de(r: &RefImagen) -> &str {
        match r {
            RefImagen::Scryfall { id, .. } => id,
            _ => "",
        }
    }

    #[tokio::test]
    async fn en_espanol_sale_la_impresion_espanola_y_se_baja_una_vez() {
        let api = falso();
        let dir = temporal("es");
        let en = api.cartas[0].clone();
        let caras = localizar(&api, &dir, &en, "es").await.unwrap().unwrap();
        assert_eq!(caras.len(), 1);
        assert_eq!(id_de(&caras[0]), ES);
        assert!(scryfall_en(&dir, ES, 0).exists());
        // Otra vez: la imagen ya está, no se baja de nuevo.
        localizar(&api, &dir, &en, "es").await.unwrap();
        assert_eq!(api.descargas.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn donde_no_hay_en_su_idioma_sale_la_inglesa() {
        let api = falso();
        let dir = temporal("de");
        // El alemán no existe: la inglesa, que es la propia carta (no se pregunta por ella).
        let en = api.cartas[0].clone();
        let caras = localizar(&api, &dir, &en, "de").await.unwrap().unwrap();
        assert_eq!(id_de(&caras[0]), EN);
        assert_eq!(*api.consultas.lock().unwrap(), vec!["de".to_string()]);
    }

    #[tokio::test]
    async fn la_de_dos_caras_baja_las_dos() {
        let api = falso();
        let dir = temporal("dfc");
        let d = api.cartas[2].clone();
        let caras = localizar(&api, &dir, &d, "en").await.unwrap().unwrap();
        assert_eq!(caras.len(), 2);
        assert!(scryfall_en(&dir, DFC_EN, 1).exists());
    }

    #[tokio::test]
    async fn sin_ninguna_de_las_dos_la_carta_se_queda_como_esta() {
        let api = falso();
        let dir = temporal("nada");
        // Una carta en español que no tiene impresión inglesa ni japonesa.
        let sola = carta(
            json!({"id":ES,"name":"X","lang":"es","set":"zzz","collector_number":"1","layout":"normal"}),
        );
        assert_eq!(localizar(&api, &dir, &sola, "ja").await.unwrap(), None);
    }

    #[test]
    fn los_idiomas_sin_cartas_piden_ingles_y_un_codigo_raro_no_vale() {
        assert_eq!(idioma_de_cartas("ro"), Some("en"));
        assert_eq!(idioma_de_cartas("zhs"), Some("zhs"));
        assert_eq!(idioma_de_cartas("../x"), None);
    }

    #[test]
    fn lo_pedido_dos_veces_no_se_resuelve_dos_veces() {
        let id = "44444444-4444-4444-4444-444444444444";
        assert!(matches!(reclamar(id, "es"), Toca::Resolver));
        assert!(matches!(reclamar(id, "es"), Toca::Esperar));
        // Otro idioma es otra cosa.
        assert!(matches!(reclamar(id, "fr"), Toca::Resolver));
        terminar(id, "es", Some(vec![RefImagen::Generica]));
        assert!(matches!(reclamar(id, "es"), Toca::Reenviar(_)));
        // Si falló, se puede reintentar.
        terminar(id, "fr", None);
        assert!(matches!(reclamar(id, "fr"), Toca::Resolver));
    }
}
