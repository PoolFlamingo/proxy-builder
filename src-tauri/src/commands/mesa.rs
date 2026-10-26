//! Los comandos de la mesa de juego: empezar o seguir una partida, mandarle
//! acciones, suscribirse a su vista y el tapete.

use mesa::accion::Accion;
use mesa::formato::Formato;
use mesa::lista::{comprobar, AvisoLista, CartaDeLista, Construccion, ListaDeJuego};
use mesa::modelo::{OrigenCarta, RefImagen};
use mesa::preparar::{secciones_de_mesa, Seccion};
use mesa::vista::VistaMesa;
use mesa::{IdCarta, IdJugador};
use proxy_core::model::Deck;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::mesa::guardada::{self, ResumenGuardada};
use crate::mesa::imagenes;
use crate::mesa::preparar::{cartas_de_lista, deck_de_mesa};
use crate::mesa::sesion::{ConfigMesa, MensajeMesa, Sesion};
use crate::mesa::tapete::{self, OrigenTapete, Tapete};
use crate::scryfall::client::CardApi;
use crate::state::AppState;
use crate::store::listas;

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MesaProgressEvent {
    pub job_id: u32,
    /// Qué deck se prepara (de `mazos`).
    pub mazo: u32,
    pub mazos: u32,
    /// Cartas preparadas de ese deck.
    pub hechas: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MesaDoneEvent {
    pub job_id: u32,
    pub error: Option<AppError>,
}

/// Si hay una partida abierta y, si no, si hay una guardada para seguir.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoMesa {
    pub abierta: bool,
    /// Los decks de la partida abierta (para «Editar este deck»).
    pub deck_ids: Vec<String>,
    pub guardada: Option<ResumenGuardada>,
}

/// El texto de reglas de una carta de la mesa («Inspeccionar»).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ReglasCarta {
    pub nombre: String,
    pub caras: Vec<CaraReglas>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CaraReglas {
    pub nombre: String,
    pub coste: Option<String>,
    pub tipo: Option<String>,
    pub texto: Option<String>,
    pub fuerza: Option<String>,
    pub resistencia: Option<String>,
    pub lealtad: Option<String>,
}

fn sin_partida() -> AppError {
    AppError::new(ErrorCode::NotFound).with("motivo", "sin_partida")
}

fn con_sesion<T>(state: &AppState, f: impl FnOnce(&mut Sesion) -> AppResult<T>) -> AppResult<T> {
    let mut m = state
        .mesa
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
    let s = m.as_mut().ok_or_else(sin_partida)?;
    f(s)
}

/// Guarda la partida en su fichero (sin bloquear por un fallo de disco).
fn guardar_si_toca(state: &AppState, forzar: bool) {
    let Ok(mut m) = state.mesa.lock() else { return };
    let Some(s) = m.as_mut() else { return };
    let ahora = crate::mesa::sesion::ahora_ms();
    let viejo = s.sin_guardar > 0 && ahora.saturating_sub(s.guardada_ms) >= guardada::CADA_MS;
    if forzar || viejo || s.sin_guardar >= guardada::CADA {
        match guardada::guardar(&state.paths.data, s) {
            Ok(()) => {
                s.sin_guardar = 0;
                s.guardada_ms = ahora;
            }
            Err(e) => tracing::warn!(codigo = ?e.code, "no se pudo guardar la partida"),
        }
    }
}

/// La lista para jugar de un jugador, comprobada. Sin lista, el deck entero,
/// salvo que tenga más cartas de las que caben en el formato: entonces hay que
/// elegir una (`motivo: falta_lista`). Una lista que no vale para el formato
/// no deja empezar (`motivo: lista`).
async fn lista_para_jugar(
    state: &AppState,
    deck: &Deck,
    id: Option<&str>,
    formato: Formato,
) -> AppResult<Option<ListaDeJuego>> {
    let mal = |motivo: &str| AppError::new(ErrorCode::InvalidInput).with("motivo", motivo);
    let Some(id) = id else {
        let cuentan = secciones_de_mesa(deck, None)
            .iter()
            .filter(|(_, s)| matches!(s, Seccion::Mazo | Seccion::Comandante | Seccion::Companero))
            .count() as u32;
        if Construccion::de(formato).is_some_and(|c| c.pide_lista(cuentan)) {
            return Err(mal("falta_lista"));
        }
        return Ok(None);
    };
    let lista = listas::una(&state.db, &deck.id, id)?.ok_or_else(|| mal("lista"))?;
    let cartas = cartas_de_lista(state, state.scryfall.as_ref(), deck).await?;
    if comprobar(&cartas, &lista, formato)
        .iter()
        .any(AvisoLista::impide)
    {
        return Err(mal("lista"));
    }
    Ok(Some(lista))
}

/// Prepara los decks y abre la partida (un trabajo largo con progreso:
/// pintar las cartas diseñadas puede tardar la primera vez).
async fn abrir(
    app: &AppHandle,
    state: &AppState,
    job_id: u32,
    config: ConfigMesa,
) -> AppResult<()> {
    let mut mazos = Vec::new();
    let n = config.jugadores.len() as u32;
    for (i, j) in config.jugadores.iter().enumerate() {
        let deck = crate::store::decks::open(&state.db, &j.deck_id)?;
        let app2 = app.clone();
        let progreso = move |hechas: u32, total: u32| {
            let _ = app2.emit(
                "mesa://progress",
                MesaProgressEvent {
                    job_id,
                    mazo: i as u32,
                    mazos: n,
                    hechas,
                    total,
                },
            );
        };
        let lista = lista_para_jugar(state, &deck, j.lista.as_deref(), config.formato).await?;
        let mut m = deck_de_mesa(
            state,
            state.scryfall.as_ref(),
            &deck,
            lista.as_ref(),
            &progreso,
        )
        .await?;
        if let Some(t) = tapete::de_deck(state, &j.deck_id)? {
            m.tapete = tapete::para_mesa(state, &t.blob).ok();
        }
        mazos.push(m);
    }
    let s = Sesion::nueva(config, mazos)?;
    guardada::guardar(&state.paths.data, &s)?;
    let mut m = state
        .mesa
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
    *m = Some(s);
    Ok(())
}

/// Empieza una partida. Devuelve el id del trabajo (`mesa://progress`,
/// `mesa://done`).
#[tauri::command]
pub fn mesa_empezar(app: AppHandle, state: State<'_, AppState>, config: ConfigMesa) -> u32 {
    let (job_id, _) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let r = {
            let _marca = estado.trabajo();
            abrir(&app, &estado, job_id, config).await
        };
        estado.jobs.baja(job_id);
        let _ = app.emit(
            "mesa://done",
            MesaDoneEvent {
                job_id,
                error: r.err(),
            },
        );
    });
    job_id
}

/// Las imágenes de una partida guardada que ya no están en la caché.
fn faltan_imagenes(state: &AppState, g: &guardada::PartidaGuardada) -> bool {
    let falta = |r: &RefImagen| match r {
        RefImagen::Propia { sha256, .. } => !imagenes::ruta_propia(state, sha256).exists(),
        RefImagen::Scryfall { id, cara, .. } => !imagenes::ruta_scryfall(state, id, *cara).exists(),
        RefImagen::Generica => false,
    };
    g.mazos.iter().any(|m| {
        falta(&m.dorso)
            || m.cartas
                .iter()
                .any(|c| c.origen.caras.iter().any(falta) || falta(&c.origen.dorso))
    })
}

async fn retomar(app: &AppHandle, state: &AppState, job_id: u32) -> AppResult<()> {
    let g = guardada::leer(&state.paths.data)?.ok_or_else(sin_partida)?;
    // Si se borró la caché, se vuelven a preparar (salen las mismas).
    if faltan_imagenes(state, &g) {
        let n = g.config.jugadores.len() as u32;
        for (i, j) in g.config.jugadores.iter().enumerate() {
            let Ok(deck) = crate::store::decks::open(&state.db, &j.deck_id) else {
                continue;
            };
            let app2 = app.clone();
            let progreso = move |hechas: u32, total: u32| {
                let _ = app2.emit(
                    "mesa://progress",
                    MesaProgressEvent {
                        job_id,
                        mazo: i as u32,
                        mazos: n,
                        hechas,
                        total,
                    },
                );
            };
            // La lista pudo borrarse después: solo cambia qué imágenes se preparan.
            let lista = match &j.lista {
                Some(id) => listas::una(&state.db, &j.deck_id, id)?,
                None => None,
            };
            deck_de_mesa(
                state,
                state.scryfall.as_ref(),
                &deck,
                lista.as_ref(),
                &progreso,
            )
            .await?;
        }
    }
    let s = guardada::sesion(g)?;
    let mut m = state
        .mesa
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
    *m = Some(s);
    Ok(())
}

/// Sigue la partida guardada (un trabajo, por si hay que volver a preparar
/// las imágenes).
#[tauri::command]
pub fn mesa_retomar(app: AppHandle, state: State<'_, AppState>) -> u32 {
    let (job_id, _) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let r = {
            let _marca = estado.trabajo();
            retomar(&app, &estado, job_id).await
        };
        estado.jobs.baja(job_id);
        let _ = app.emit(
            "mesa://done",
            MesaDoneEvent {
                job_id,
                error: r.err(),
            },
        );
    });
    job_id
}

#[tauri::command]
pub fn mesa_estado(state: State<'_, AppState>) -> AppResult<EstadoMesa> {
    let abierta = state.hay_partida();
    let deck_ids = state
        .mesa
        .lock()
        .ok()
        .and_then(|m| {
            m.as_ref().map(|s| {
                s.config
                    .jugadores
                    .iter()
                    .map(|j| j.deck_id.clone())
                    .collect()
            })
        })
        .unwrap_or_default();
    let guardada = if abierta {
        None
    } else {
        match guardada::leer(&state.paths.data) {
            Ok(Some(g)) => guardada::resumen(&g).ok(),
            Ok(None) => None,
            // Una guardada que no se entiende no impide empezar otra.
            Err(e) => {
                tracing::warn!(codigo = ?e.code, "partida guardada ilegible");
                None
            }
        }
    };
    Ok(EstadoMesa {
        abierta,
        deck_ids,
        guardada,
    })
}

/// La pantalla se suscribe a la mesa: recibe la vista y luego los parches.
#[tauri::command]
pub fn mesa_suscribir(state: State<'_, AppState>, canal: Channel<MensajeMesa>) -> AppResult<()> {
    con_sesion(&state, |s| {
        s.suscribir(canal);
        Ok(())
    })
}

#[tauri::command]
pub fn mesa_vista(state: State<'_, AppState>) -> AppResult<VistaMesa> {
    con_sesion(&state, |s| Ok(s.vista()))
}

#[tauri::command]
pub fn mesa_accion(
    state: State<'_, AppState>,
    accion: Accion,
    como: Option<IdJugador>,
) -> AppResult<()> {
    con_sesion(&state, |s| s.accion(accion, como))?;
    guardar_si_toca(&state, false);
    Ok(())
}

/// Guarda la partida ahora (al cerrar la app o salir de la pestaña).
#[tauri::command]
pub fn mesa_guardar(state: State<'_, AppState>) -> AppResult<()> {
    guardar_si_toca(&state, true);
    Ok(())
}

/// Sale de la mesa. Con `guardar`, la partida se queda guardada para
/// seguirla luego; sin él, se termina y se olvida.
#[tauri::command]
pub fn mesa_salir(state: State<'_, AppState>, guardar: bool) -> AppResult<()> {
    if guardar {
        guardar_si_toca(&state, true);
    }
    let mut m = state
        .mesa
        .lock()
        .map_err(|_| AppError::new(ErrorCode::Internal))?;
    if let Some(s) = m.as_mut() {
        s.cerrar();
    }
    *m = None;
    if !guardar {
        guardada::borrar(&state.paths.data);
    }
    Ok(())
}

#[tauri::command]
pub async fn mesa_info_carta(state: State<'_, AppState>, carta: IdCarta) -> AppResult<ReglasCarta> {
    let origen = con_sesion(&state, |s| {
        let p = s.partida();
        let c = p.cartas.get(&carta).ok_or_else(sin_partida)?;
        let (z, i) = p.donde(carta).ok_or_else(sin_partida)?;
        if !mesa::visibilidad::ve(p, c, &z, i, s.mirador()) {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "SinPermiso"));
        }
        Ok(c.origen.clone())
    })?;
    let datos = match &origen.scryfall_id {
        Some(id) => crate::scryfall::cache::leer(&state.db, id)?.map(|(c, _)| c),
        None => None,
    };
    let caras = match &datos {
        Some(d) if !d.card_faces.is_empty() => d
            .card_faces
            .iter()
            .map(|f| CaraReglas {
                nombre: f.name.clone(),
                coste: f.mana_cost.clone(),
                tipo: f.type_line.clone(),
                texto: f.oracle_text.clone(),
                fuerza: f.power.clone(),
                resistencia: f.toughness.clone(),
                lealtad: None,
            })
            .collect(),
        Some(d) => vec![CaraReglas {
            nombre: d.name.clone(),
            coste: d.coste(),
            tipo: d.type_line.clone(),
            texto: d.oracle_text.clone(),
            fuerza: d.power.clone(),
            resistencia: d.toughness.clone(),
            lealtad: d.loyalty.clone(),
        }],
        None => vec![CaraReglas {
            nombre: origen.nombre.clone(),
            coste: origen.info.coste.clone(),
            tipo: origen.info.tipo.clone(),
            texto: None,
            fuerza: origen.info.fuerza.clone(),
            resistencia: origen.info.resistencia.clone(),
            lealtad: origen.info.lealtad.clone(),
        }],
    };
    Ok(ReglasCarta {
        nombre: origen.nombre,
        caras,
    })
}

/// Una carta o ficha de Scryfall lista para crearla en la mesa (fichas que
/// no están en el deck, copiar cartas reales).
#[tauri::command]
pub async fn mesa_origen_scryfall(
    state: State<'_, AppState>,
    scryfall_id: String,
) -> AppResult<OrigenCarta> {
    if !imagenes::es_id_scryfall(&scryfall_id) {
        return Err(AppError::new(ErrorCode::InvalidInput));
    }
    let api = state.scryfall.as_ref();
    let carta = match crate::scryfall::cache::leer(&state.db, &scryfall_id)? {
        Some((c, _)) => c,
        None => {
            let c = api
                .card(&scryfall_id)
                .await?
                .ok_or_else(|| AppError::new(ErrorCode::NotFound))?;
            crate::scryfall::cache::guardar(&state.db, &c)?;
            c
        }
    };
    let mut caras = Vec::new();
    let n = if carta.is_dfc() { 2 } else { 1 };
    for i in 0..n {
        let Some((url, _)) = carta.print_url(i) else {
            continue;
        };
        let cara = i as u8;
        let ruta = imagenes::ruta_scryfall(&state, &scryfall_id, cara);
        if !ruta.exists() {
            let bytes = api.download(&url).await?;
            let img = proxy_core::imaging::decode::decode(&bytes)?;
            imagenes::guardar_scryfall(&state, &scryfall_id, cara, &img)?;
        }
        caras.push(RefImagen::Scryfall {
            id: scryfall_id.clone(),
            cara,
            url,
        });
    }
    if caras.is_empty() {
        caras.push(RefImagen::Generica);
    }
    let card = proxy_core::model::DeckCard {
        id: String::new(),
        source: proxy_core::model::CardSource::Token {
            scryfall_id: scryfall_id.clone(),
        },
        front: None,
        back: proxy_core::model::BackSpec::DeckDefault,
        edits: Default::default(),
        locked: false,
        meta: Default::default(),
        design: None,
        marca: None,
    };
    Ok(OrigenCarta {
        carta_deck: String::new(),
        nombre: carta.name.clone(),
        caras,
        dorso: RefImagen::Generica,
        scryfall_id: Some(scryfall_id),
        info: crate::mesa::preparar::info_de(&card, Some(&carta)),
    })
}

/// El tapete propio de un deck (`deck: null`: el general).
#[tauri::command]
pub fn mesa_tapete_get(
    state: State<'_, AppState>,
    deck: Option<String>,
) -> AppResult<Option<Tapete>> {
    tapete::propio(&state, deck.as_deref())
}

/// Pone (o quita, con `origen: null`) el tapete de un deck o el general.
#[tauri::command]
pub async fn mesa_tapete_set(
    state: State<'_, AppState>,
    deck: Option<String>,
    origen: Option<OrigenTapete>,
    velo: u8,
    desenfoque: u8,
) -> AppResult<Option<Tapete>> {
    let t = match origen {
        Some(o) => Some(Tapete {
            blob: tapete::importar(&state, state.scryfall.as_ref(), &o).await?,
            velo: velo.min(90),
            desenfoque: desenfoque.min(20),
        }),
        None => None,
    };
    tapete::guardar(&state, deck.as_deref(), t.as_ref())?;
    Ok(t)
}

/// Cambia solo el velo y el desenfoque del tapete que ya hay.
#[tauri::command]
pub fn mesa_tapete_ajustes(
    state: State<'_, AppState>,
    deck: Option<String>,
    velo: u8,
    desenfoque: u8,
) -> AppResult<()> {
    if let Some(mut t) = tapete::propio(&state, deck.as_deref())? {
        t.velo = velo.min(90);
        t.desenfoque = desenfoque.min(20);
        tapete::guardar(&state, deck.as_deref(), Some(&t))?;
    }
    Ok(())
}

/// Las listas para jugar de un deck y cuántas cartas suyas cuentan (para
/// saber si hace falta elegir una: más de las que caben en el formato).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ListasDeDeck {
    pub listas: Vec<ListaDeJuego>,
    pub cuentan: u32,
}

#[tauri::command]
pub fn mesa_listas(state: State<'_, AppState>, deck: String) -> AppResult<ListasDeDeck> {
    let d = crate::store::decks::open(&state.db, &deck)?;
    let cuentan = secciones_de_mesa(&d, None)
        .iter()
        .filter(|(_, s)| matches!(s, Seccion::Mazo | Seccion::Comandante | Seccion::Companero))
        .count() as u32;
    Ok(ListasDeDeck {
        listas: listas::de_deck(&state.db, &deck)?,
        cuentan,
    })
}

/// Las copias elegibles de un deck con sus datos (de Scryfall o del diseño),
/// para el editor de listas.
#[tauri::command]
pub async fn mesa_lista_cartas(
    state: State<'_, AppState>,
    deck: String,
) -> AppResult<Vec<CartaDeLista>> {
    let d = crate::store::decks::open(&state.db, &deck)?;
    cartas_de_lista(&state, state.scryfall.as_ref(), &d).await
}

/// Lo que está mal (o solo avisa) de una lista para un formato.
#[tauri::command]
pub async fn mesa_lista_comprobar(
    state: State<'_, AppState>,
    deck: String,
    formato: Formato,
    lista: ListaDeJuego,
) -> AppResult<Vec<AvisoLista>> {
    let d = crate::store::decks::open(&state.db, &deck)?;
    let cartas = cartas_de_lista(&state, state.scryfall.as_ref(), &d).await?;
    Ok(comprobar(&cartas, &lista, formato))
}

/// Guarda (crea o reemplaza) una lista; devuelve la guardada, con su id.
#[tauri::command]
pub fn mesa_lista_guardar(
    state: State<'_, AppState>,
    deck: String,
    lista: ListaDeJuego,
) -> AppResult<ListaDeJuego> {
    listas::guardar(&state.db, &deck, &lista)
}

#[tauri::command]
pub fn mesa_lista_borrar(state: State<'_, AppState>, deck: String, id: String) -> AppResult<()> {
    listas::borrar(&state.db, &deck, &id)
}
