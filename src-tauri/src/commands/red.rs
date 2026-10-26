//! Comandos de la partida en red: la identidad de la instalación y la
//! partida por código directo (crear, unirse, empezar y salir).

use std::time::Duration;

use mesa::formato::Formato;
use mesa::sparring::Intensidad;
use mesa::{IdCarta, IdJugador};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use ts_rs::TS;

use super::mesa::{preparar_mazo, MesaDoneEvent, Progreso};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::mesa::sesion::ConfigMesa;
use crate::red::anfitrion::{atender, empezar_partida, Anfitrion};
use crate::red::codigo::{secreto_nuevo, Codigo};
use crate::red::identidad::{self, Identidad};
use crate::red::invitado::{unirse, Union};
use crate::red::mesa::MesaDeApp;
use crate::red::nodo::{relay_de_entorno, Relay};
use crate::red::protocolo::{EnSala, Rol};
use crate::red::Sala;
use crate::state::AppState;

/// La identidad con la que se juega en red (clave pública, apodo, avatar).
#[tauri::command]
pub fn red_identidad(state: State<'_, AppState>) -> AppResult<Identidad> {
    identidad::leer(&state.db, &state.red.clave())
}

/// Cambia el apodo y el avatar (una huella de blob o ninguno).
#[tauri::command]
pub fn red_identidad_set(
    state: State<'_, AppState>,
    apodo: String,
    avatar: Option<String>,
) -> AppResult<Identidad> {
    identidad::guardar(&state.db, &apodo, avatar.as_deref())?;
    identidad::leer(&state.db, &state.red.clave())
}

/// Guarda la identidad de la instalación en `ruta` (un fichero que permite
/// jugar como esta instalación: la pantalla avisa de que no se comparta).
#[tauri::command]
pub fn red_identidad_exportar(state: State<'_, AppState>, ruta: String) -> AppResult<()> {
    identidad::exportar(&state.red.clave(), std::path::Path::new(&ruta))
}

/// Cambia la identidad de esta instalación por la de un fichero exportado.
/// Con una partida, una sala o un salón abiertos no se puede (`Busy`).
#[tauri::command]
pub async fn red_identidad_importar(
    state: State<'_, AppState>,
    ruta: String,
) -> AppResult<Identidad> {
    if state.red.ocupada() || state.hay_partida() || state.hay_mirada() {
        return Err(AppError::new(ErrorCode::Busy).with("motivo", "partida"));
    }
    let nueva = identidad::importar(&state.paths.data, std::path::Path::new(&ruta))?;
    state.red.cambiar_clave(nueva).await;
    identidad::leer(&state.db, &state.red.clave())
}

/// Qué hace esta app en la partida en red.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PapelRed {
    Anfitrion,
    Jugador,
    Espectador,
}

/// La partida en red de esta app, para la pantalla.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EstadoRed {
    pub papel: Option<PapelRed>,
    /// El código para invitar (solo el anfitrión).
    pub codigo: Option<String>,
    /// Quién hay en la sala (solo el anfitrión; el invitado lo recibe por
    /// `red://aviso`).
    pub sala: Vec<EnSala>,
    pub empezada: bool,
}

fn ocupada() -> AppError {
    AppError::new(ErrorCode::Busy).with("motivo", "partida")
}

/// En una partida, o en una sala del servidor, no se abre otra por código.
fn ocupado(state: &AppState) -> bool {
    state.hay_partida() || state.hay_mirada() || state.red.con_salon(|s| s.is_some())
}

#[tauri::command]
pub fn red_estado(state: State<'_, AppState>) -> EstadoRed {
    state.red.con_sala(|s| match s {
        None => EstadoRed {
            papel: None,
            codigo: None,
            sala: Vec::new(),
            empezada: false,
        },
        Some(Sala::Anfitrion { anf, codigo, .. }) => EstadoRed {
            papel: Some(PapelRed::Anfitrion),
            codigo: Some(codigo.clone()),
            sala: anf.vista_sala(),
            empezada: anf.empezada(),
        },
        Some(Sala::Invitado { rol, .. }) => EstadoRed {
            papel: Some(match rol {
                Rol::Jugador => PapelRed::Jugador,
                Rol::Espectador => PapelRed::Espectador,
            }),
            codigo: None,
            sala: Vec::new(),
            empezada: state.hay_partida() || state.hay_mirada(),
        },
    })
}

/// Abre una partida en red: levanta el nodo, empieza a atender a quien
/// llegue con el código y lo devuelve (`PB1-…`). `deck` es el nombre del
/// deck del anfitrión, para la sala.
#[tauri::command]
pub async fn red_crear(
    app: AppHandle,
    state: State<'_, AppState>,
    apodo: String,
    deck: Option<String>,
) -> AppResult<String> {
    if ocupado(&state) {
        return Err(ocupada());
    }
    if let Some(vieja) = state.red.quitar_sala() {
        vieja.cerrar().await;
    }
    let ep = state.red.nodo().await?;
    // Con relé, el código lleva el suyo: se espera (un poco) a tenerlo.
    if relay_de_entorno() != Relay::Ninguno {
        let _ = tokio::time::timeout(Duration::from_secs(5), ep.online()).await;
    }
    let anf = Anfitrion::nuevo(apodo, secreto_nuevo());
    if let Ok(mut d) = anf.deck.lock() {
        *d = deck;
    }
    let codigo = Codigo {
        addr: ep.addr(),
        secreto: anf.secreto().to_string(),
    }
    .a_texto()?;
    let tarea = tokio::spawn(atender(ep, anf.clone(), MesaDeApp(app)));
    state.red.con_sala(|s| {
        *s = Some(Sala::Anfitrion {
            anf,
            tarea: tarea.abort_handle(),
            codigo: codigo.clone(),
        })
    });
    Ok(codigo)
}

fn terminar(app: &AppHandle, job_id: u32, r: AppResult<()>) {
    let estado = app.state::<AppState>();
    estado.jobs.baja(job_id);
    let _ = app.emit(
        "mesa://done",
        MesaDoneEvent {
            job_id,
            error: r.err(),
        },
    );
}

/// Se une a la partida de un código: prepara el deck (si juega) y conecta.
/// Es un trabajo (`mesa://progress`, `mesa://done`); un rechazo del
/// anfitrión llega como error `Rejected` con su `motivo`.
/// Con qué se une la pantalla a una partida.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PeticionUnion {
    /// El código del anfitrión (`PB1-…`).
    pub codigo: String,
    pub apodo: String,
    pub rol: Rol,
    /// El deck con el que juega (los espectadores no traen).
    pub deck_id: Option<String>,
    pub lista: Option<String>,
    /// El formato con el que se comprueba la lista.
    pub formato: Formato,
}

#[tauri::command]
pub fn red_unirse(
    app: AppHandle,
    state: State<'_, AppState>,
    peticion: PeticionUnion,
) -> AppResult<u32> {
    let PeticionUnion {
        codigo,
        apodo,
        rol,
        deck_id,
        lista,
        formato,
    } = peticion;
    if ocupado(&state) {
        return Err(ocupada());
    }
    let codigo = Codigo::de_texto(&codigo)?;
    let deck_id = match rol {
        Rol::Jugador => Some(
            deck_id.ok_or_else(|| AppError::new(ErrorCode::InvalidInput).with("motivo", "deck"))?,
        ),
        Rol::Espectador => None,
    };
    let (job_id, _) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let r = async {
            let _marca = estado.trabajo();
            if let Some(vieja) = estado.red.quitar_sala() {
                vieja.cerrar().await;
            }
            let deck = match &deck_id {
                Some(id) => {
                    let p = Progreso {
                        job_id,
                        mazo: 0,
                        mazos: 1,
                    };
                    Some(preparar_mazo(&app, &estado, p, id, lista.as_deref(), formato).await?)
                }
                None => None,
            };
            let ep = estado.red.nodo().await?;
            let conexion = unirse(
                ep,
                Union {
                    codigo,
                    apodo,
                    rol,
                    deck,
                },
                MesaDeApp(app.clone()),
            )
            .await?;
            estado
                .red
                .con_sala(|s| *s = Some(Sala::Invitado { conexion, rol }));
            Ok(())
        }
        .await;
        terminar(&app, job_id, r);
    });
    Ok(job_id)
}

/// El anfitrión empieza la partida con los de la sala que trajeron deck.
/// `config` trae solo al anfitrión (el jugador 0) y su deck; en red no hay
/// sparring. Es un trabajo, como `mesa_empezar`.
#[tauri::command]
pub fn red_empezar(
    app: AppHandle,
    state: State<'_, AppState>,
    mut config: ConfigMesa,
) -> AppResult<u32> {
    let anf = state
        .red
        .con_sala(|s| match s {
            Some(Sala::Anfitrion { anf, .. }) if !anf.empezada() => Some(anf.clone()),
            _ => None,
        })
        .ok_or_else(|| AppError::new(ErrorCode::NotFound).with("motivo", "sin_sala"))?;
    if config.jugadores.len() != 1 {
        return Err(AppError::new(ErrorCode::InvalidInput));
    }
    config.intensidad = Intensidad::Apagado;
    let (job_id, _) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let r = async {
            let _marca = estado.trabajo();
            let j = &config.jugadores[0];
            let p = Progreso {
                job_id,
                mazo: 0,
                mazos: 1,
            };
            let deck = preparar_mazo(
                &app,
                &estado,
                p,
                &j.deck_id,
                j.lista.as_deref(),
                config.formato,
            )
            .await?;
            let ep = estado.red.nodo().await?;
            empezar_partida(&anf, &MesaDeApp(app.clone()), ep.id(), config, deck)
        }
        .await;
        terminar(&app, job_id, r);
    });
    Ok(job_id)
}

/// Movimiento en vivo (E7): la pantalla arrastra una carta (o mueve su
/// cursor, sin carta) por el tablero de `tablero`, en coordenadas 0–10.000.
/// Va por datagrama al anfitrión, que lo reparte; sin partida en red o
/// siendo espectador, no hace nada. Nada de esto se guarda.
#[tauri::command]
pub fn mesa_fantasma(
    state: State<'_, AppState>,
    tablero: IdJugador,
    carta: Option<IdCarta>,
    x: u16,
    y: u16,
) {
    crate::red::latencia::mover_fantasma(&state, tablero, carta, x, y);
}

/// Señala una carta o a un jugador: un aro de 2 s en todas las pantallas.
#[tauri::command]
pub fn mesa_senal(state: State<'_, AppState>, carta: Option<IdCarta>, a: Option<IdJugador>) {
    crate::red::latencia::senalar(&state, carta, a);
}

/// Sale de la partida en red (en la sala o empezada): el anfitrión la
/// cierra para todos, el invitado se despide. La mesa la cierra `mesa_salir`.
#[tauri::command]
pub async fn red_salir(state: State<'_, AppState>) -> AppResult<()> {
    if let Some(s) = state.red.quitar_sala() {
        s.cerrar().await;
    }
    Ok(())
}
