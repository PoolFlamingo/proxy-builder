//! Pasar decks a otro equipo (`pasar/`), compartir y abrir el `.pbdeck` con
//! las mismas opciones: con o sin proxies, y qué hacer si el deck ya existe
//! aquí. Los comandos solo lanzan y consultan; lo que pasa lo cuenta el
//! estado (`pasar://receptor`, `pasar://emisor`, `pasar://cerca`).

use std::path::PathBuf;

use proxy_core::model::Deck;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::{AppError, AppResult, ErrorCode};
use crate::fichero;
use crate::package::{self, Opciones};
use crate::pasar::encontrar::Busqueda;
use crate::pasar::tipos::{
    Cerca, Decision, DeckOfrecido, Destino, Eleccion, EstadoEmisor, EstadoReceptor, ImagenesFin,
    ImagenesProgreso, InfoPaquete, PaqueteImportado, PaqueteLeido, Resolucion,
};
use crate::pasar::{self, cerrojo, Entorno};
use crate::state::AppState;
use crate::store::decks;

// ── Recibir ─────────────────────────────────────────────────────────────

/// Abre «Recibir»: el código, su QR y, si se puede, el anuncio en la red
/// local durante 5 minutos.
#[tauri::command]
pub async fn pasar_recibir_abrir(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<EstadoReceptor> {
    let ep = state.red.nodo().await?;
    let apodo = crate::red::identidad::leer(&state.db, &state.red.clave())
        .map(|i| i.apodo)
        .unwrap_or_default();
    let con_rele = crate::red::nodo::relay_de_entorno() != crate::red::nodo::Relay::Ninguno;
    pasar::recibir::abrir(&app, ep, &apodo, con_rele).await
}

#[tauri::command]
pub fn pasar_recibir_estado(app: AppHandle) -> Option<EstadoReceptor> {
    pasar::recibir::estado(&app)
}

/// Cierra «Recibir» (y corta lo que hubiera en curso).
#[tauri::command]
pub fn pasar_recibir_cerrar(app: AppHandle) {
    pasar::recibir::cerrar(&app);
}

/// Acepta o rechaza al equipo de la red local que quiere enviar (el código
/// de 6 cifras).
#[tauri::command]
pub fn pasar_peticion(app: AppHandle, aceptar: bool) {
    pasar::aceptar_peticion(&app, aceptar);
}

/// Lo que se elige de cada deck de la oferta.
#[tauri::command]
pub fn pasar_responder(app: AppHandle, decisiones: Vec<Decision>) -> AppResult<()> {
    pasar::responder(&app, decisiones)
}

/// Cancela lo que se esté recibiendo (se ve el resultado) o, si no había nada
/// en curso, cierra «Recibir».
#[tauri::command]
pub fn pasar_recibir_cancelar(app: AppHandle) {
    pasar::recibir::cancelar(&app);
}

// ── Enviar ──────────────────────────────────────────────────────────────

/// Empieza a buscar equipos cerca (los que tienen «Recibir» abierto en la
/// misma red). La lista llega por `pasar://cerca`.
#[tauri::command]
pub async fn pasar_cerca_buscar(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<Cerca>> {
    pasar_cerca_parar(app.clone(), state.clone());
    let ep = state.red.nodo().await?;
    app.multicast(crate::android::UsoMulticast::Buscar, true);
    let emisor = app.clone();
    let b = match Busqueda::nueva(&ep, move |l| emisor.emitir_cerca(&l)).await {
        Ok(b) => b,
        Err(e) => {
            app.multicast(crate::android::UsoMulticast::Buscar, false);
            return Err(e);
        }
    };
    let lista = b.lista();
    *cerrojo(&state.pasar.busqueda) = Some(b);
    Ok(lista)
}

#[tauri::command]
pub fn pasar_cerca_parar(app: AppHandle, state: State<'_, AppState>) {
    if cerrojo(&state.pasar.busqueda).take().is_some() {
        app.multicast(crate::android::UsoMulticast::Buscar, false);
    }
}

/// Comprueba un código pegado o leído de un QR antes de enviar: uno de una
/// partida (`PB1-`) da su error.
#[tauri::command]
pub fn pasar_codigo_comprobar(texto: String) -> AppResult<()> {
    crate::red::codigo::Codigo::de_texto_deck(&texto).map(|_| ())
}

/// Envía estos decks. Vuelve al momento; el avance llega por `pasar://emisor`.
#[tauri::command]
pub async fn pasar_enviar(
    app: AppHandle,
    state: State<'_, AppState>,
    destino: Destino,
    decks: Vec<String>,
    con_proxies: bool,
) -> AppResult<()> {
    let ep = state.red.nodo().await?;
    pasar::enviar::enviar(&app, ep, destino, decks, con_proxies).await
}

/// Cancela el envío en marcha.
#[tauri::command]
pub fn pasar_enviar_cancelar(app: AppHandle) {
    pasar::enviar::cancelar(&app);
}

#[tauri::command]
pub fn pasar_emisor_estado(app: AppHandle) -> Option<EstadoEmisor> {
    pasar::enviar::estado(&app)
}

/// Olvida un envío terminado (al empezar otro o cerrar la pantalla).
#[tauri::command]
pub fn pasar_emisor_olvidar(app: AppHandle) {
    pasar::enviar::olvidar(&app);
}

// ── Las imágenes que faltan ─────────────────────────────────────────────

/// Baja las imágenes que le faltan a un deck recibido «solo con las cartas
/// originales». Devuelve el id del trabajo; el avance llega por
/// `pasar://imagenes-progress` y el final por `pasar://imagenes-done`.
#[tauri::command]
pub fn pasar_completar(
    app: AppHandle,
    state: State<'_, AppState>,
    deck_id: String,
) -> AppResult<u32> {
    // El deck tiene que existir.
    decks::open(&state.db, &deck_id)?;
    let (job_id, cancelado) = state.jobs.alta();
    tauri::async_runtime::spawn(async move {
        let estado = app.state::<AppState>();
        let _marca = estado.trabajo();
        let emisor = app.clone();
        let r = pasar::imagenes::completar(
            estado.scryfall.as_ref(),
            &estado,
            &deck_id,
            &cancelado,
            |hechas, total| {
                let _ = emisor.emit(
                    "pasar://imagenes-progress",
                    ImagenesProgreso {
                        job_id,
                        hechas,
                        total,
                    },
                );
            },
        )
        .await;
        estado.jobs.baja(job_id);
        let fin = match r {
            Ok(b) => ImagenesFin {
                job_id,
                bajadas: b.bajadas,
                fallidas: b.fallidas,
                cancelado: b.cancelado,
                error: None,
            },
            Err(e) => ImagenesFin {
                job_id,
                bajadas: 0,
                fallidas: 0,
                cancelado: e.code == ErrorCode::Cancelled,
                error: Some(e),
            },
        };
        let _ = app.emit("pasar://imagenes-done", fin);
    });
    Ok(job_id)
}

// ── El fichero `.pbdeck` ────────────────────────────────────────────────

fn nombre_paquete(d: &Deck) -> String {
    fichero::nombre_seguro(&format!("{}.pbdeck", d.name))
}

/// Lo que pesa el paquete de un deck con y sin proxies y si lleva proxies
/// (para elegir al compartirlo).
#[tauri::command]
pub fn deck_paquete_info(state: State<'_, AppState>, id: String) -> AppResult<InfoPaquete> {
    let deck = decks::open(&state.db, &id)?;
    Ok(InfoPaquete {
        proxies: proxy_core::compartir::resumen_proxies(&deck).hay(),
        tamano: package::tamano_estimado(&deck, &state.blobs, &state.db, false),
        tamano_originales: package::tamano_estimado(&deck, &state.blobs, &state.db, true),
    })
}

/// Guarda el deck en un `.pbdeck` (una ruta o un `content://`), con o sin
/// proxies.
#[tauri::command]
pub async fn deck_paquete_guardar(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    ruta: String,
    sin_proxies: bool,
) -> AppResult<u64> {
    let deck = decks::open(&state.db, &id)?;
    if fichero::es_uri(&ruta) {
        let tmp = fichero::temporal(&app, &nombre_paquete(&deck))?;
        let bytes = package::export_con(&deck, tmp.ruta(), &state.blobs, &state.db, sin_proxies)?;
        fichero::copiar_a(&app, &ruta, tmp.ruta())?;
        return Ok(bytes);
    }
    package::export_con(
        &deck,
        &PathBuf::from(&ruta),
        &state.blobs,
        &state.db,
        sin_proxies,
    )
}

/// Comparte el deck con otra app del móvil (el selector del sistema), con o
/// sin proxies: se escribe en la caché y se ofrece desde allí.
#[tauri::command]
pub async fn deck_paquete_compartir(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    sin_proxies: bool,
) -> AppResult<()> {
    let deck = decks::open(&state.db, &id)?;
    let tmp = fichero::temporal(&app, &nombre_paquete(&deck))?;
    package::export_con(&deck, tmp.ruta(), &state.blobs, &state.db, sin_proxies)?;
    let ruta = tmp.conservar();
    crate::commands::fichero::compartir_fichero(app, ruta.to_string_lossy().into_owned()).await
}

/// Lee un `.pbdeck` sin importarlo: qué trae y si ya hay uno igual aquí.
#[tauri::command]
pub async fn paquete_leer(
    app: AppHandle,
    state: State<'_, AppState>,
    ruta: String,
) -> AppResult<PaqueteLeido> {
    use base64::Engine;
    let copia = fichero::leer(&app, &ruta, None)?;
    let tamano = std::fs::metadata(copia.ruta())?.len();
    let f = std::fs::File::open(copia.ruta())?;
    let r = package::resumen(std::io::BufReader::new(f))?;
    let deck = DeckOfrecido {
        nombre: r.nombre,
        cartas: r.cartas,
        proxies: r.proxies,
        tamano,
        tamano_originales: r.tamano_originales.min(tamano),
        linaje: r.linaje,
        actualizado: r.actualizado,
        portada: r
            .portada
            .map(|p| base64::engine::general_purpose::STANDARD.encode(p)),
    };
    let conflicto = pasar::recibir::conflicto_en(&state.db, &deck);
    Ok(PaqueteLeido { deck, conflicto })
}

/// Importa un `.pbdeck` con lo que eligió la persona.
#[tauri::command]
pub async fn paquete_importar(
    app: AppHandle,
    state: State<'_, AppState>,
    ruta: String,
    eleccion: Eleccion,
    resolucion: Option<Resolucion>,
) -> AppResult<PaqueteImportado> {
    if eleccion == Eleccion::No {
        return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "eleccion"));
    }
    let copia = fichero::leer(&app, &ruta, None)?;
    // El deck de aquí con el que coincide, mirado ahora y no fiado a la pantalla.
    let resumen = {
        let f = std::fs::File::open(copia.ruta())?;
        package::resumen(std::io::BufReader::new(f))?
    };
    let ofrecido = DeckOfrecido {
        nombre: resumen.nombre,
        cartas: resumen.cartas,
        proxies: resumen.proxies,
        tamano: 0,
        tamano_originales: 0,
        linaje: resumen.linaje,
        actualizado: resumen.actualizado,
        portada: None,
    };
    let destino = match (
        pasar::recibir::conflicto_en(&state.db, &ofrecido),
        resolucion,
    ) {
        (Some(c), Some(Resolucion::Reemplazar)) => package::Destino::Reemplazar(c.deck_id),
        (Some(_), Some(Resolucion::GuardarLosDos)) => package::Destino::GuardarLosDos,
        (Some(_), None) => {
            return Err(AppError::new(ErrorCode::InvalidInput).with("motivo", "falta_resolucion"))
        }
        (None, _) => package::Destino::Nuevo,
    };
    let reemplazado = matches!(destino, package::Destino::Reemplazar(_));
    let opciones = Opciones {
        solo_originales: eleccion == Eleccion::SoloOriginales,
        destino,
    };
    let f = std::fs::File::open(copia.ruta())?;
    let deck = package::import_con(
        std::io::BufReader::new(f),
        &state.blobs,
        &state.db,
        &opciones,
    )?;
    Ok(PaqueteImportado {
        deck_id: deck.id.clone(),
        nombre: deck.name.clone(),
        faltan_imagenes: proxy_core::compartir::pendientes(&deck).len() as u32,
        reemplazado,
    })
}
