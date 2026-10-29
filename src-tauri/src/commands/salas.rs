//! Comandos del servidor de salas (Fase F): conectar, playgroups,
//! invitaciones y salas. La pantalla recibe además los avisos `salas://…`
//! (`red/servidor/sala.rs`).

use std::sync::Arc;

use mesa::formato::Formato;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::mesa::{preparar_mazo, MesaDoneEvent, Progreso};
use crate::commands::mesa::lanzar_trabajo;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::red::servidor::cliente::Cliente;
use crate::red::servidor::mensajes::{
    AlServidor, Como, DelServidor, EstadoSala, Plataforma, Playgroup, Rol,
};
use crate::red::servidor::sala::{self, AvisosApp, DeckListo};
use crate::red::servidor::url;
use crate::state::AppState;

/// Cómo está la app con el servidor, para la pantalla.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoSalas {
    /// Si hay servidor configurado (en producción, aún no).
    pub hay_servidor: bool,
    pub conectado: bool,
    /// La clave de esta instalación (su nodo de iroh).
    pub yo: String,
    pub sala: Option<EstadoSala>,
    pub como: Option<Como>,
    /// El deck preparado para la sala.
    pub deck: Option<String>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Invitacion {
    pub codigo: String,
    /// Cuándo caduca (ms); las de sala, cuando la sala termine.
    #[ts(type = "number | null")]
    pub caduca: Option<i64>,
}

/// Lo que da usar una invitación: los playgroups o la sala en la que se entró.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum Usada {
    Playgroups { playgroups: Vec<Playgroup> },
    Sala { estado: EstadoSala },
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SalaNueva {
    pub playgroup: Option<String>,
    pub formato: Formato,
    pub max_jugadores: u8,
    pub espectadores: bool,
}

fn inesperado() -> AppError {
    AppError::new(ErrorCode::Internal).with("motivo", "respuesta")
}

fn ocupada() -> AppError {
    AppError::new(ErrorCode::Busy).with("motivo", "partida")
}

fn cliente(state: &AppState) -> AppResult<Arc<Cliente>> {
    state
        .red
        .con_servidor(|c| c.clone())
        .ok_or_else(|| AppError::new(ErrorCode::Network).with("motivo", "sin_servidor"))
}

/// Arranca el cliente del servidor (si no lo estaba) y dice cómo está.
#[tauri::command]
pub async fn salas_conectar(
    app: AppHandle,
    state: State<'_, AppState>,
    apodo: String,
) -> AppResult<EstadoSalas> {
    if let Some(url) = url() {
        if state.red.con_servidor(|c| c.is_none()) {
            let apodo = apodo.trim();
            let apodo = if apodo.is_empty() { "?" } else { apodo };
            let c = Cliente::arrancar(
                url,
                state.red.clave(),
                apodo.chars().take(32).collect(),
                Plataforma::Escritorio,
                AvisosApp(app.clone()),
            );
            state.red.con_servidor(|s| *s = Some(Arc::new(c)));
            // Un momento para que conecte antes de contestar.
            for _ in 0..40 {
                if state
                    .red
                    .con_servidor(|c| c.as_ref().is_some_and(|c| c.conectado()))
                {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }
    }
    Ok(salas_estado(state))
}

#[tauri::command]
pub fn salas_estado(state: State<'_, AppState>) -> EstadoSalas {
    let salon = state.red.con_salon(|s| s.clone());
    EstadoSalas {
        hay_servidor: url().is_some(),
        conectado: state
            .red
            .con_servidor(|c| c.as_ref().is_some_and(|c| c.conectado())),
        yo: state.red.clave().public().to_string(),
        sala: salon.as_ref().and_then(|s| s.estado()),
        como: salon.as_ref().map(|s| s.como),
        deck: salon.as_ref().and_then(|s| {
            s.deck
                .lock()
                .ok()
                .and_then(|d| d.as_ref().map(|d| d.nombre.clone()))
        }),
    }
}

async fn playgroups(state: &AppState, m: AlServidor) -> AppResult<Vec<Playgroup>> {
    match cliente(state)?.pedir(m).await? {
        DelServidor::PgLista { playgroups } => Ok(playgroups),
        _ => Err(inesperado()),
    }
}

#[tauri::command]
pub async fn salas_playgroups(state: State<'_, AppState>) -> AppResult<Vec<Playgroup>> {
    playgroups(&state, AlServidor::PgListar).await
}

#[tauri::command]
pub async fn salas_pg_crear(
    state: State<'_, AppState>,
    nombre: String,
) -> AppResult<Vec<Playgroup>> {
    playgroups(&state, AlServidor::PgCrear { nombre }).await
}

#[tauri::command]
pub async fn salas_pg_salir(
    state: State<'_, AppState>,
    playgroup: String,
) -> AppResult<Vec<Playgroup>> {
    playgroups(&state, AlServidor::PgSalir { playgroup }).await
}

#[tauri::command]
pub async fn salas_pg_expulsar(
    state: State<'_, AppState>,
    playgroup: String,
    clave: String,
) -> AppResult<Vec<Playgroup>> {
    playgroups(&state, AlServidor::PgExpulsar { playgroup, clave }).await
}

#[tauri::command]
pub async fn salas_pg_rol(
    state: State<'_, AppState>,
    playgroup: String,
    clave: String,
    rol: Rol,
) -> AppResult<Vec<Playgroup>> {
    playgroups(
        &state,
        AlServidor::PgRol {
            playgroup,
            clave,
            rol,
        },
    )
    .await
}

/// Una invitación a un playgroup o a la sala en la que se está.
#[tauri::command]
pub async fn salas_invitar(
    state: State<'_, AppState>,
    playgroup: Option<String>,
) -> AppResult<Invitacion> {
    let sala = match &playgroup {
        Some(_) => None,
        None => Some(
            state
                .red
                .con_salon(|s| s.as_ref().map(|s| s.id.clone()))
                .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_sala"))?,
        ),
    };
    match cliente(&state)?
        .pedir(AlServidor::InvCrear {
            playgroup,
            sala,
            caduca_horas: None,
            usos_max: None,
        })
        .await?
    {
        DelServidor::Inv { codigo, caduca, .. } => Ok(Invitacion { codigo, caduca }),
        _ => Err(inesperado()),
    }
}

#[tauri::command]
pub async fn salas_revocar(state: State<'_, AppState>, codigo: String) -> AppResult<()> {
    cliente(&state)?
        .pedir(AlServidor::InvRevocar { codigo })
        .await?;
    Ok(())
}

/// Con una partida abierta (o una partida por código directo) no se entra en otra sala.
fn libre(state: &AppState) -> AppResult<()> {
    let directo = state.red.con_sala(|s| s.is_some()) && state.red.con_salon(|s| s.is_none());
    if state.hay_partida() || state.hay_mirada() || directo {
        return Err(ocupada());
    }
    Ok(())
}

/// Entra en la sala que dice el servidor (crearla, unirse, invitación).
async fn entrar(
    app: &AppHandle,
    state: &AppState,
    m: AlServidor,
    como: Como,
    apodo: String,
) -> AppResult<EstadoSala> {
    let c = cliente(state)?;
    let DelServidor::SalaEstado(e) = c.pedir(m).await? else {
        return Err(inesperado());
    };
    let ep = state.red.nodo().await?;
    sala::entrar(app, c, ep, e.clone(), como, apodo);
    Ok(e)
}

#[tauri::command]
pub async fn salas_usar(
    app: AppHandle,
    state: State<'_, AppState>,
    codigo: String,
    como: Option<Como>,
    apodo: String,
) -> AppResult<Usada> {
    let c = cliente(&state)?;
    // Primero se mira qué es sin entrar: si es de sala, hay que estar libre.
    let m = AlServidor::InvUsar { codigo, como };
    if state.hay_partida() || state.hay_mirada() {
        // Un playgroup sí se puede añadir con la partida abierta.
        return match c.pedir(m).await? {
            DelServidor::PgLista { playgroups } => Ok(Usada::Playgroups { playgroups }),
            DelServidor::SalaEstado(_) => {
                c.mandar(AlServidor::SalaSalir);
                Err(ocupada())
            }
            _ => Err(inesperado()),
        };
    }
    libre(&state)?;
    match c.pedir(m).await? {
        DelServidor::PgLista { playgroups } => Ok(Usada::Playgroups { playgroups }),
        DelServidor::SalaEstado(e) => {
            let ep = state.red.nodo().await?;
            sala::entrar(&app, c, ep, e.clone(), como.unwrap_or(Como::Jugador), apodo);
            Ok(Usada::Sala { estado: e })
        }
        _ => Err(inesperado()),
    }
}

#[tauri::command]
pub async fn salas_sala_crear(
    app: AppHandle,
    state: State<'_, AppState>,
    nueva: SalaNueva,
    apodo: String,
) -> AppResult<EstadoSala> {
    libre(&state)?;
    let formato = serde_json::to_value(nueva.formato)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| "commander".into());
    entrar(
        &app,
        &state,
        AlServidor::SalaCrear {
            playgroup: nueva.playgroup,
            formato,
            max_jugadores: nueva.max_jugadores.clamp(2, 10),
            espectadores: nueva.espectadores,
        },
        Como::Jugador,
        apodo,
    )
    .await
}

#[tauri::command]
pub async fn salas_sala_unirse(
    app: AppHandle,
    state: State<'_, AppState>,
    sala: String,
    como: Como,
    apodo: String,
) -> AppResult<EstadoSala> {
    libre(&state)?;
    entrar(
        &app,
        &state,
        AlServidor::SalaUnirse {
            sala,
            como,
            registro: None,
        },
        como,
        apodo,
    )
    .await
}

/// Quien creó la sala expulsa a otro miembro por su clave (en espera o en juego).
#[tauri::command]
pub fn salas_sala_expulsar(state: State<'_, AppState>, clave: String) -> AppResult<()> {
    cliente(&state)?.mandar(AlServidor::SalaExpulsar { clave });
    Ok(())
}

/// Lo mismo, pero por el jugador de la partida (sale de su nodo).
#[tauri::command]
pub fn salas_sala_expulsar_jugador(
    app: AppHandle,
    state: State<'_, AppState>,
    jugador: u8,
) -> AppResult<()> {
    let yo = state.red.con_salon(|s| s.as_ref().map(|s| s.yo));
    let nodo = {
        let g = match state.mesa.lock() {
            Ok(g) => g,
            Err(e) => e.into_inner(),
        };
        match g.as_ref().and_then(|s| s.red.as_ref()) {
            Some(crate::mesa::sesion::RedSesion::Invitado(i)) => i
                .nodos
                .iter()
                .find(|n| n.jugador == mesa::IdJugador(jugador))
                .map(|n| n.nodo.clone()),
            Some(crate::mesa::sesion::RedSesion::Anfitrion(anf)) => yo.and_then(|yo| {
                anf.nodos(yo)
                    .into_iter()
                    .find(|n| n.jugador == mesa::IdJugador(jugador))
                    .map(|n| n.nodo)
            }),
            None => None,
        }
    };
    let clave = nodo.ok_or_else(|| AppError::new(ErrorCode::NotFound))?;
    match cliente(&state) {
        // En una sala del servidor, lo hace quien la creó (que puede no ser el host).
        Ok(c) if state.red.con_salon(|s| s.is_some()) => {
            c.mandar(AlServidor::SalaExpulsar { clave })
        }
        // Con un código, el anfitrión lo hace directamente.
        _ => {
            let nodo = clave
                .parse::<iroh::EndpointId>()
                .map_err(|_| AppError::new(ErrorCode::NotFound))?;
            if !sala::expulsar_en_host(&app, nodo) {
                return Err(AppError::new(ErrorCode::Rejected).with("motivo", "no_anfitrion"));
            }
        }
    }
    Ok(())
}

/// Sale de la sala (y de su partida, si la hay).
#[tauri::command]
pub async fn salas_sala_salir(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if let Ok(c) = cliente(&state) {
        c.mandar(AlServidor::SalaSalir);
    }
    sala::salir(&app);
    if let Some(s) = state.red.quitar_sala() {
        s.cerrar().await;
    }
    Ok(())
}

/// Prepara el deck con el que se juega en la sala y lo dice al servidor. Es
/// un trabajo (`mesa://progress`, `mesa://done`).
#[tauri::command]
pub fn salas_deck(
    app: AppHandle,
    state: State<'_, AppState>,
    deck_id: String,
    lista: Option<String>,
    formato: Formato,
) -> AppResult<u32> {
    let salon = state
        .red
        .con_salon(|s| s.clone())
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_sala"))?;
    let (job_id, _) = state.jobs.alta();
    lanzar_trabajo(app.clone(), job_id, async move {
        let estado = app.state::<AppState>();
        let r = async {
            let _marca = estado.trabajo();
            let p = Progreso {
                job_id,
                mazo: 0,
                mazos: 1,
            };
            let mazo = preparar_mazo(&app, &estado, p, &deck_id, lista.as_deref(), formato).await?;
            let nombre = crate::store::decks::open(&estado.db, &deck_id)
                .map(|d| d.name)
                .unwrap_or_else(|_| mazo.nombre.clone());
            crate::red::recordar_mi_deck(Some(crate::red::MiDeck {
                deck_id: deck_id.clone(),
                lista: lista.clone(),
                nombre: nombre.clone(),
            }));
            if let Ok(mut d) = salon.deck.lock() {
                *d = Some(DeckListo {
                    deck_id,
                    lista,
                    nombre: nombre.clone(),
                    mazo,
                });
            }
            salon
                .cliente
                .mandar(AlServidor::SalaDeck { deck: Some(nombre) });
            Ok(())
        }
        .await;
        estado.jobs.baja(job_id);
        let _ = app.emit(
            "mesa://done",
            MesaDoneEvent {
                job_id,
                error: r.err(),
            },
        );
    });
    Ok(job_id)
}

#[tauri::command]
pub async fn salas_empezar(state: State<'_, AppState>) -> AppResult<()> {
    cliente(&state)?.pedir(AlServidor::SalaEmpezar).await?;
    Ok(())
}

#[tauri::command]
pub async fn salas_terminar(state: State<'_, AppState>) -> AppResult<()> {
    cliente(&state)?.pedir(AlServidor::SalaTerminada).await?;
    Ok(())
}
