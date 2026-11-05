//! La pantalla **Buscar**: consultar cartas sin deck de por medio, como en una
//! app de colección. Monta la consulta de Scryfall con los filtros de la
//! pantalla, pide los resultados por páginas y da la ficha de una carta.

use serde::{Deserialize, Serialize};
use tauri::State;
use ts_rs::TS;

use crate::commands::cards::{registrar_miniatura, resumir, CardSummary};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::scryfall::client::CardApi;
use crate::scryfall::models::{CardFace, Prices, ScryCard};
use crate::state::AppState;

/// Qué significan los colores elegidos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModoColores {
    /// Lleva al menos estos colores.
    Incluye,
    /// Lleva exactamente estos colores.
    Exactos,
    /// Su identidad de color cabe en estos (para un comandante).
    Identidad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ComparadorCoste {
    Igual,
    Minimo,
    Maximo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CosteFiltro {
    pub comparador: ComparadorCoste,
    pub valor: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum OrdenCartas {
    Nombre,
    Fecha,
    Coste,
    Rareza,
    Fuerza,
    Eur,
    Usd,
}

impl OrdenCartas {
    fn de_scryfall(self) -> &'static str {
        match self {
            OrdenCartas::Nombre => "name",
            OrdenCartas::Fecha => "released",
            OrdenCartas::Coste => "cmc",
            OrdenCartas::Rareza => "rarity",
            OrdenCartas::Fuerza => "power",
            OrdenCartas::Eur => "eur",
            OrdenCartas::Usd => "usd",
        }
    }
}

/// Cuántas veces sale cada carta en los resultados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum UnicasCartas {
    Cartas,
    Arte,
    Impresiones,
}

impl UnicasCartas {
    fn de_scryfall(self) -> &'static str {
        match self {
            UnicasCartas::Cartas => "cards",
            UnicasCartas::Arte => "art",
            UnicasCartas::Impresiones => "prints",
        }
    }
}

/// Lo que se ha pedido en la pantalla.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FiltrosCartas {
    /// Un nombre o la sintaxis de Scryfall.
    pub texto: String,
    /// `texto` es el nombre exacto de una carta (elegido en las sugerencias).
    pub exacto: bool,
    /// De `w`, `u`, `b`, `r`, `g` y `c` (incolora).
    pub colores: Vec<String>,
    pub modo_colores: ModoColores,
    pub tipo: Option<String>,
    pub rarezas: Vec<String>,
    pub formato: Option<String>,
    pub coste: Option<CosteFiltro>,
    pub set: Option<String>,
    pub idioma: Option<String>,
    pub orden: OrdenCartas,
    pub descendente: bool,
    pub unicas: UnicasCartas,
}

/// Solo lo que puede ir en un filtro: que un valor no cuele otra consulta.
fn solo_alfanumerico(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).collect()
}

fn filtro_colores(colores: &[String], modo: ModoColores) -> Option<String> {
    let mut letras = String::new();
    for letra in ["w", "u", "b", "r", "g"] {
        if colores.iter().any(|c| c == letra) {
            letras.push_str(letra);
        }
    }
    // Incolora con colores no existe: mandan los colores.
    if letras.is_empty() {
        if !colores.iter().any(|c| c == "c") {
            return None;
        }
        letras.push('c');
    }
    Some(match modo {
        ModoColores::Incluye if letras == "c" => "c=c".to_string(),
        ModoColores::Incluye => format!("c>={letras}"),
        ModoColores::Exactos => format!("c={letras}"),
        ModoColores::Identidad => format!("id<={letras}"),
    })
}

/// La consulta de Scryfall. Vacía si no se ha pedido nada.
fn consulta(f: &FiltrosCartas) -> String {
    let mut partes: Vec<String> = Vec::new();
    let texto = f.texto.trim();
    if !texto.is_empty() {
        partes.push(if f.exacto {
            format!("!\"{}\"", texto.replace('"', ""))
        } else {
            texto.to_string()
        });
    }
    partes.extend(filtro_colores(&f.colores, f.modo_colores));
    if let Some(tipo) = f.tipo.as_deref().map(solo_alfanumerico) {
        if !tipo.is_empty() {
            partes.push(format!("t:{tipo}"));
        }
    }
    let rarezas: Vec<String> = f
        .rarezas
        .iter()
        .map(|r| solo_alfanumerico(r))
        .filter(|r| !r.is_empty())
        .map(|r| format!("r:{r}"))
        .collect();
    match rarezas.len() {
        0 => {}
        1 => partes.extend(rarezas),
        _ => partes.push(format!("({})", rarezas.join(" or "))),
    }
    if let Some(formato) = f.formato.as_deref().map(solo_alfanumerico) {
        if !formato.is_empty() {
            partes.push(format!("f:{formato}"));
        }
    }
    if let Some(coste) = f.coste {
        let signo = match coste.comparador {
            ComparadorCoste::Igual => "=",
            ComparadorCoste::Minimo => ">=",
            ComparadorCoste::Maximo => "<=",
        };
        partes.push(format!("cmc{signo}{}", coste.valor));
    }
    if let Some(set) = f.set.as_deref().map(solo_alfanumerico) {
        if !set.is_empty() {
            partes.push(format!("e:{set}"));
        }
    }
    if let Some(idioma) = f.idioma.as_deref().map(solo_alfanumerico) {
        if !idioma.is_empty() {
            partes.push(format!("lang:{idioma}"));
        }
    }
    partes.join(" ")
}

/// Los parámetros de `/cards/search` que no son la consulta.
///
/// `include_multilingual` es lo que hace que «rayo» encuentre el nombre impreso
/// en español; sin idioma, las cartas siguen saliendo en inglés. Con todas las
/// impresiones y sin idioma se quita: serían todas en todos los idiomas.
fn parametros(f: &FiltrosCartas) -> Vec<(&'static str, &'static str)> {
    let mut extra = vec![
        ("order", f.orden.de_scryfall()),
        ("dir", if f.descendente { "desc" } else { "asc" }),
        ("unique", f.unicas.de_scryfall()),
    ];
    if f.unicas != UnicasCartas::Impresiones || f.idioma.is_some() {
        extra.push(("include_multilingual", "true"));
    }
    extra
}

/// Una página de resultados.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PaginaCartas {
    pub cartas: Vec<CardSummary>,
    /// Cuántas hay en total, no solo en esta página.
    pub total: u32,
    pub hay_mas: bool,
}

/// Busca cartas; `pagina` empieza en 1 (Scryfall da hasta 175 por página). Sin
/// resultados no es un error.
#[tauri::command]
pub async fn cartas_buscar(
    state: State<'_, AppState>,
    filtros: FiltrosCartas,
    pagina: u32,
) -> AppResult<PaginaCartas> {
    let q = consulta(&filtros);
    if q.is_empty() {
        return Ok(PaginaCartas {
            cartas: Vec::new(),
            total: 0,
            hay_mas: false,
        });
    }
    let p = state
        .scryfall
        .buscar_pagina(&q, &parametros(&filtros), pagina.max(1))
        .await?;
    Ok(PaginaCartas {
        total: p.total_cards,
        hay_mas: p.has_more,
        cartas: resumir(&state, p.data),
    })
}

/// Una cara de la ficha. `imagen` es la huella del protocolo (`/t/<huella>`).
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CaraFicha {
    pub nombre: String,
    pub nombre_impreso: Option<String>,
    pub coste: Option<String>,
    pub tipo: Option<String>,
    pub tipo_impreso: Option<String>,
    pub texto: Option<String>,
    pub texto_impreso: Option<String>,
    pub ambientacion: Option<String>,
    pub fuerza: Option<String>,
    pub resistencia: Option<String>,
    pub lealtad: Option<String>,
    pub defensa: Option<String>,
    pub artista: Option<String>,
    pub imagen: Option<String>,
}

/// Si una carta es legal en un formato (`legal`, `not_legal`, `restricted`, `banned`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct LegalidadFicha {
    pub formato: String,
    pub estado: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PreciosFicha {
    pub eur: Option<String>,
    pub eur_foil: Option<String>,
    pub usd: Option<String>,
    pub usd_foil: Option<String>,
}

/// Todo lo que se enseña de una carta.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct FichaCarta {
    pub id: String,
    pub nombre: String,
    pub nombre_impreso: Option<String>,
    pub idioma: String,
    pub set: String,
    pub set_nombre: String,
    pub numero: String,
    pub rareza: Option<String>,
    pub fecha: Option<String>,
    /// Hay una imagen por cara y se le puede dar la vuelta.
    pub doble_cara: bool,
    pub caras: Vec<CaraFicha>,
    pub legalidades: Vec<LegalidadFicha>,
    pub precios: PreciosFicha,
}

/// Los formatos de la ficha, en este orden (Scryfall da muchos más).
const FORMATOS_FICHA: [&str; 10] = [
    "standard",
    "pioneer",
    "modern",
    "legacy",
    "vintage",
    "commander",
    "oathbreaker",
    "brawl",
    "historic",
    "pauper",
];

fn sin_vacio(s: Option<&String>) -> Option<String> {
    s.filter(|s| !s.is_empty()).cloned()
}

fn cara_de(
    f: &CardFace,
    imagen_comun: Option<&String>,
    registrar: &mut impl FnMut(&str) -> String,
) -> CaraFicha {
    let propia = f
        .image_uris
        .as_ref()
        .and_then(|u| u.normal.as_ref().or(u.small.as_ref()));
    CaraFicha {
        nombre: f.name.clone(),
        nombre_impreso: sin_vacio(f.printed_name.as_ref()),
        coste: sin_vacio(f.mana_cost.as_ref()),
        tipo: sin_vacio(f.type_line.as_ref()),
        tipo_impreso: sin_vacio(f.printed_type_line.as_ref()),
        texto: sin_vacio(f.oracle_text.as_ref()),
        texto_impreso: sin_vacio(f.printed_text.as_ref()),
        ambientacion: sin_vacio(f.flavor_text.as_ref()),
        fuerza: sin_vacio(f.power.as_ref()),
        resistencia: sin_vacio(f.toughness.as_ref()),
        lealtad: sin_vacio(f.loyalty.as_ref()),
        defensa: sin_vacio(f.defense.as_ref()),
        artista: sin_vacio(f.artist.as_ref()),
        imagen: propia.or(imagen_comun).map(|u| registrar(u)),
    }
}

/// La ficha de una carta. Las partidas y las de aventura llevan sus caras
/// (cada una con su texto) y una sola imagen; las de doble cara, una por cara.
fn ficha_de(c: ScryCard, registrar: &mut impl FnMut(&str) -> String) -> FichaCarta {
    let imagen_comun = c
        .image_uris
        .as_ref()
        .and_then(|u| u.normal.as_ref().or(u.small.as_ref()));
    let caras = if c.card_faces.is_empty() {
        vec![CaraFicha {
            nombre: c.name.clone(),
            nombre_impreso: sin_vacio(c.printed_name.as_ref()),
            coste: sin_vacio(c.mana_cost.as_ref()),
            tipo: sin_vacio(c.type_line.as_ref()),
            tipo_impreso: sin_vacio(c.printed_type_line.as_ref()),
            texto: sin_vacio(c.oracle_text.as_ref()),
            texto_impreso: sin_vacio(c.printed_text.as_ref()),
            ambientacion: sin_vacio(c.flavor_text.as_ref()),
            fuerza: sin_vacio(c.power.as_ref()),
            resistencia: sin_vacio(c.toughness.as_ref()),
            lealtad: sin_vacio(c.loyalty.as_ref()),
            defensa: sin_vacio(c.defense.as_ref()),
            artista: sin_vacio(c.artist.as_ref()),
            imagen: imagen_comun.map(|u| registrar(u)),
        }]
    } else {
        c.card_faces
            .iter()
            .map(|f| {
                let mut cara = cara_de(f, imagen_comun, registrar);
                // En una carta partida Scryfall suele dar el artista solo arriba.
                cara.artista = cara.artista.or_else(|| sin_vacio(c.artist.as_ref()));
                cara
            })
            .collect()
    };
    let legalidades = FORMATOS_FICHA
        .iter()
        .filter_map(|&formato| {
            c.legalities.get(formato).map(|estado| LegalidadFicha {
                formato: formato.to_string(),
                estado: estado.clone(),
            })
        })
        .collect();
    FichaCarta {
        doble_cara: c.is_dfc(),
        id: c.id,
        nombre: c.name,
        nombre_impreso: c.printed_name,
        idioma: c.lang,
        set: c.set,
        set_nombre: c.set_name,
        numero: c.collector_number,
        rareza: c.rarity,
        fecha: c.released_at,
        caras,
        legalidades,
        precios: PreciosFicha {
            eur: c.prices.eur,
            eur_foil: c.prices.eur_foil,
            usd: c.prices.usd,
            usd_foil: c.prices.usd_foil,
        },
    }
}

fn sin_precios(p: &Prices) -> bool {
    p.eur.is_none() && p.eur_foil.is_none() && p.usd.is_none() && p.usd_foil.is_none()
}

/// La ficha de una impresión. Scryfall no da precio a las que no están en
/// inglés: llevan el de la misma impresión en inglés, como en las estadísticas.
#[tauri::command]
pub async fn carta_ficha(state: State<'_, AppState>, id: String) -> AppResult<FichaCarta> {
    let mut c = state
        .scryfall
        .card(&id)
        .await?
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("carta", &id))?;
    if c.lang != "en" && sin_precios(&c.prices) {
        // Sin precio se puede ver igual la ficha: un fallo aquí no la tira.
        if let Ok(Some(inglesa)) = state
            .scryfall
            .card_lang(&c.set, &c.collector_number, "en")
            .await
        {
            c.prices = inglesa.prices;
        }
    }
    Ok(ficha_de(c, &mut |url| registrar_miniatura(&state, url)))
}

/// Una aclaración de reglas (Scryfall solo las tiene en inglés).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AclaracionCarta {
    pub fecha: String,
    pub texto: String,
}

/// Las aclaraciones de reglas de una carta.
#[tauri::command]
pub async fn carta_aclaraciones(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<Vec<AclaracionCarta>> {
    Ok(state
        .scryfall
        .aclaraciones(&id)
        .await?
        .into_iter()
        .map(|a| AclaracionCarta {
            fecha: a.published_at,
            texto: a.comment,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filtros() -> FiltrosCartas {
        FiltrosCartas {
            texto: String::new(),
            exacto: false,
            colores: Vec::new(),
            modo_colores: ModoColores::Incluye,
            tipo: None,
            rarezas: Vec::new(),
            formato: None,
            coste: None,
            set: None,
            idioma: None,
            orden: OrdenCartas::Nombre,
            descendente: false,
            unicas: UnicasCartas::Cartas,
        }
    }

    #[test]
    fn sin_nada_pedido_no_hay_consulta() {
        let mut f = filtros();
        f.texto = "   ".into();
        f.tipo = Some(String::new());
        assert_eq!(consulta(&f), "");
    }

    #[test]
    fn el_texto_va_tal_cual_y_el_exacto_entre_comillas() {
        let mut f = filtros();
        f.texto = " t:dragon c:r ".into();
        assert_eq!(consulta(&f), "t:dragon c:r");
        f.texto = "Sol \"Ring".into();
        f.exacto = true;
        assert_eq!(consulta(&f), "!\"Sol Ring\"");
    }

    #[test]
    fn los_colores_segun_el_modo() {
        let colores = vec!["r".to_string(), "w".to_string()];
        assert_eq!(
            filtro_colores(&colores, ModoColores::Incluye).as_deref(),
            Some("c>=wr")
        );
        assert_eq!(
            filtro_colores(&colores, ModoColores::Exactos).as_deref(),
            Some("c=wr")
        );
        assert_eq!(
            filtro_colores(&colores, ModoColores::Identidad).as_deref(),
            Some("id<=wr")
        );
        let incolora = vec!["c".to_string()];
        assert_eq!(
            filtro_colores(&incolora, ModoColores::Incluye).as_deref(),
            Some("c=c")
        );
        assert_eq!(
            filtro_colores(&incolora, ModoColores::Identidad).as_deref(),
            Some("id<=c")
        );
        // Incolora con colores: mandan los colores.
        let mezcla = vec!["c".to_string(), "g".to_string()];
        assert_eq!(
            filtro_colores(&mezcla, ModoColores::Exactos).as_deref(),
            Some("c=g")
        );
        assert_eq!(filtro_colores(&[], ModoColores::Exactos), None);
    }

    #[test]
    fn se_juntan_todos_los_filtros() {
        let f = FiltrosCartas {
            texto: "bolt".into(),
            colores: vec!["r".into()],
            tipo: Some("instant".into()),
            rarezas: vec!["common".into(), "rare".into()],
            formato: Some("modern".into()),
            coste: Some(CosteFiltro {
                comparador: ComparadorCoste::Maximo,
                valor: 2,
            }),
            set: Some("MH3".into()),
            idioma: Some("es".into()),
            ..filtros()
        };
        assert_eq!(
            consulta(&f),
            "bolt c>=r t:instant (r:common or r:rare) f:modern cmc<=2 e:MH3 lang:es"
        );
    }

    #[test]
    fn un_filtro_no_puede_colar_otra_consulta() {
        let f = FiltrosCartas {
            tipo: Some("land or t:goblin".into()),
            rarezas: vec!["rare)".into()],
            formato: Some("modern -f:legacy".into()),
            set: Some("mh3 c:r".into()),
            idioma: Some("es\"".into()),
            ..filtros()
        };
        assert_eq!(
            consulta(&f),
            "t:landortgoblin r:rare f:modernflegacy e:mh3cr lang:es"
        );
    }

    #[test]
    fn varios_idiomas_salvo_todas_las_impresiones_sin_idioma() {
        let mut f = filtros();
        assert!(parametros(&f).contains(&("include_multilingual", "true")));
        assert!(parametros(&f).contains(&("unique", "cards")));
        f.unicas = UnicasCartas::Impresiones;
        assert!(!parametros(&f).contains(&("include_multilingual", "true")));
        f.idioma = Some("es".into());
        assert!(parametros(&f).contains(&("include_multilingual", "true")));
        f.orden = OrdenCartas::Eur;
        f.descendente = true;
        assert!(parametros(&f).contains(&("order", "eur")));
        assert!(parametros(&f).contains(&("dir", "desc")));
    }

    fn ficha_json(v: serde_json::Value) -> FichaCarta {
        let c: ScryCard = serde_json::from_value(v).unwrap();
        ficha_de(c, &mut |url: &str| format!("h:{url}"))
    }

    #[test]
    fn una_carta_normal_tiene_una_cara() {
        let f = ficha_json(serde_json::json!({
            "id": "x", "name": "Sol Ring", "lang": "en", "rarity": "uncommon",
            "mana_cost": "{1}", "type_line": "Artifact", "oracle_text": "{T}: Add {C}{C}.",
            "artist": "Mike Bierek",
            "image_uris": { "normal": "https://i/n.jpg" },
            "legalities": { "commander": "legal", "modern": "not_legal", "penny": "legal", "vintage": "restricted" },
            "prices": { "eur": "1.50", "usd": null },
        }));
        assert!(!f.doble_cara);
        assert_eq!(f.caras.len(), 1);
        assert_eq!(f.caras[0].coste.as_deref(), Some("{1}"));
        assert_eq!(f.caras[0].artista.as_deref(), Some("Mike Bierek"));
        assert_eq!(f.caras[0].imagen.as_deref(), Some("h:https://i/n.jpg"));
        assert_eq!(f.precios.eur.as_deref(), Some("1.50"));
        assert_eq!(f.precios.usd, None);
        // En el orden fijo y sin los formatos que la ficha no enseña.
        let nombres: Vec<&str> = f.legalidades.iter().map(|l| l.formato.as_str()).collect();
        assert_eq!(nombres, ["modern", "vintage", "commander"]);
    }

    #[test]
    fn una_de_doble_cara_lleva_una_imagen_por_cara() {
        let f = ficha_json(serde_json::json!({
            "id": "x", "name": "A // B", "layout": "transform",
            "card_faces": [
                { "name": "A", "power": "1", "toughness": "2", "image_uris": { "normal": "https://i/a.jpg" } },
                { "name": "B", "loyalty": "3", "image_uris": { "normal": "https://i/b.jpg" } },
            ],
        }));
        assert!(f.doble_cara);
        assert_eq!(f.caras[0].imagen.as_deref(), Some("h:https://i/a.jpg"));
        assert_eq!(f.caras[0].fuerza.as_deref(), Some("1"));
        assert_eq!(f.caras[1].imagen.as_deref(), Some("h:https://i/b.jpg"));
        assert_eq!(f.caras[1].lealtad.as_deref(), Some("3"));
    }

    #[test]
    fn una_partida_comparte_la_imagen_y_lleva_sus_textos() {
        let c: ScryCard =
            serde_json::from_str(include_str!("../scryfall/fixtures/dusk-dawn-es.json")).unwrap();
        let f = ficha_de(c, &mut |url: &str| url.to_string());
        assert!(!f.doble_cara);
        assert_eq!(f.caras.len(), 2);
        assert_eq!(f.caras[1].nombre_impreso.as_deref(), Some("Alba"));
        assert_eq!(f.caras[1].tipo_impreso.as_deref(), Some("Conjuro"));
        assert_eq!(f.caras[1].artista.as_deref(), Some("Noah Bradley"));
        assert!(f.caras[0].imagen.is_some());
        assert_eq!(f.caras[0].imagen, f.caras[1].imagen);
    }
}
