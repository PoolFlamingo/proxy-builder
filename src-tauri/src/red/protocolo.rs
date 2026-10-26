//! **El protocolo de la partida en red** (`proxy-builder/mesa/1`).
//!
//! Un stream bidireccional fiable por jugador con el host, con tramas de
//! `u32` (big endian) de longitud + JSON. JSON y no binario: los mensajes
//! son pequeños y pocos, se depuran a simple vista y son las mismas formas
//! que los ejemplos de `fixtures/` (decisión D-E5 del plan).
//!
//! Topes (lo que pasa se corta y la conexión se cierra):
//! - 64 KiB por trama, en general;
//! - `Preparacion` (el deck con el que entra un jugador): 2 MiB y como mucho
//!   500 cartas;
//! - `Bienvenida` (con la partida entera) y `Evento` (un `Restaurar` lleva la
//!   partida entera): 4 MiB.
//!
//! Aparte, datagramas (no fiables, ≤ 1.200 bytes) para lo que no importa si
//! se pierde: latidos, el arrastre en vivo y las señales.
//!
//! 🛑 Todo lo que llega de otro equipo es entrada no confiable: tramas con
//! tope antes de leerlas, JSON tipado, decks validados (`validar_deck`) y
//! las intenciones, por `mesa::resolver` en el host (permisos y que existan
//! las cartas). El actor de una intención nunca viaja: lo pone el host según
//! la conexión.

use mesa::accion::{Accion, Evento};
use mesa::modelo::{Partida, RefImagen};
use mesa::preparar::DeckDeMesa;
use mesa::vista::{ParcheVista, VistaMesa};
use mesa::{IdCarta, IdJugador};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use ts_rs::TS;

/// La versión del protocolo (sube con cualquier cambio incompatible).
pub const PROTOCOLO: u32 = 1;
pub const TOPE_GENERAL: usize = 64 * 1024;
pub const TOPE_PREPARACION: usize = 2 * 1024 * 1024;
pub const TOPE_PARTIDA: usize = 4 * 1024 * 1024;
pub const TOPE_DATAGRAMA: usize = 1_200;
pub const MAX_CARTAS_DECK: usize = 500;
/// Lo más que puede leer el host de un `Hola` antes de admitir a alguien.
pub const TOPE_HOLA: usize = 4 * 1024;

pub fn version_app() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename = "RolEnSala")]
pub enum Rol {
    Jugador,
    Espectador,
}

/// Lo que manda un jugador (o un espectador) al host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "tipo",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[allow(clippy::large_enum_variant)]
pub enum AlHost {
    /// Lo primero, siempre. Si la versión de la app o del catálogo de
    /// acciones no es la del host, se rechaza: una acción nueva rompería a
    /// quien no la conoce.
    Hola {
        protocolo: u32,
        version_app: String,
        version_catalogo: u32,
        rol: Rol,
        apodo: String,
        /// El secreto del código directo (en hexadecimal), si se entra con uno.
        secreto: Option<String>,
        /// Al volver: el último evento que tiene y la época.
        ultimo_seq: Option<u64>,
        epoch: u32,
    },
    /// El deck con el que entra (en la sala de espera).
    Preparacion {
        deck: DeckDeMesa,
    },
    /// Una intención: el host pone el actor según la conexión.
    Intencion {
        id: u64,
        accion: Accion,
    },
    /// Le faltan eventos (un hueco): que se los mande desde `seq`.
    Pide {
        desde_seq: u64,
    },
    Adios,
}

/// Quien está en la sala de espera.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct EnSala {
    pub apodo: String,
    pub rol: Rol,
    /// El deck que trae (su nombre), si ya lo mandó.
    pub deck: Option<String>,
}

/// El nodo (clave pública) de cada jugador: cualquiera puede seguir de
/// anfitrión si el de ahora se va.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodoJugador {
    pub jugador: IdJugador,
    pub nodo: String,
}

/// Lo que manda el host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "tipo",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[allow(clippy::large_enum_variant)]
pub enum DelHost {
    /// Quién hay en la sala de espera (antes de empezar).
    Sala { jugadores: Vec<EnSala> },
    /// A un jugador: la partida entera (empieza o vuelve) y quién es en ella.
    Bienvenida {
        epoch: u32,
        yo: IdJugador,
        partida: Box<Partida>,
        nodos: Vec<NodoJugador>,
    },
    /// Un evento, en orden de `seq`. `confirma` es el `id` de la intención de
    /// quien lo recibe que lo trajo; `huella`, cada 50 eventos, la de la
    /// partida del host tras aplicarlo.
    Evento {
        epoch: u32,
        evento: Box<Evento>,
        confirma: Option<u64>,
        huella: Option<String>,
    },
    /// La intención `id` no se pudo hacer (o el `Hola`, sin `id`).
    Rechazo { id: Option<u64>, motivo: String },
    /// A un espectador: su vista (nunca la partida: lo oculto no viaja).
    VistaEspectador { epoch: u32, vista: Box<VistaMesa> },
    /// A un espectador: lo que cambió y, si se puede contar, el evento.
    ParcheEspectador {
        epoch: u32,
        parche: Box<ParcheVista>,
        evento: Option<Box<Evento>>,
    },
    /// El host cierra la partida.
    Cerrada,
}

impl DelHost {
    /// El tope de esta trama al escribirla (y lo que se le consiente al leer).
    fn tope(&self) -> usize {
        match self {
            DelHost::Bienvenida { .. }
            | DelHost::Evento { .. }
            | DelHost::VistaEspectador { .. } => TOPE_PARTIDA,
            DelHost::ParcheEspectador { .. } => TOPE_PARTIDA,
            _ => TOPE_GENERAL,
        }
    }
}

impl AlHost {
    fn tope(&self) -> usize {
        match self {
            AlHost::Preparacion { .. } => TOPE_PREPARACION,
            _ => TOPE_GENERAL,
        }
    }
}

/// Cómo va el enlace de un jugador con el anfitrión.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Enlace {
    pub jugador: IdJugador,
    /// Ida y vuelta, en milisegundos (0 en el propio anfitrión).
    pub rtt_ms: u32,
    /// Camino directo (`true`) o por el relé (`false`).
    pub directo: bool,
}

/// Las coordenadas de un `Fantasma` van de 0 a este valor (como `Pos`).
pub const COORD_MAX: u16 = 10_000;

/// Los datagramas: se tiran si llegan viejos (`t`, el reloj de quien los
/// manda: se compara solo con los anteriores del mismo jugador).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "tipo",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Datagrama {
    /// Jugador → host, cada 2 s.
    Latido { t: u64 },
    /// Host → jugador: el latido de vuelta, con la hora del host.
    Eco { t: u64, t_host: u64 },
    /// Por dónde va una carta que se arrastra (o el cursor, sin carta), en
    /// coordenadas del `tablero` sobre el que está (0–10.000). `jugador` es
    /// quien arrastra: lo pone el anfitrión según la conexión, nunca se fía
    /// de lo que dice quien lo manda.
    Fantasma {
        jugador: IdJugador,
        tablero: IdJugador,
        carta: Option<IdCarta>,
        x: u16,
        y: u16,
        t: u64,
    },
    /// Un aro sobre una carta o un jugador.
    Senal {
        jugador: IdJugador,
        carta: Option<IdCarta>,
        a: Option<IdJugador>,
        t: u64,
    },
    /// Anfitrión → jugador, cada 2 s: cómo va el enlace de cada uno con él.
    Enlaces { filas: Vec<Enlace>, t: u64 },
}

#[derive(Debug, thiserror::Error)]
pub enum ErrorTrama {
    #[error("la trama mide {0} bytes, más de lo permitido")]
    Grande(usize),
    #[error("JSON roto: {0}")]
    Json(#[from] serde_json::Error),
    #[error("conexión: {0}")]
    Io(#[from] std::io::Error),
    #[error("mensaje no válido: {0}")]
    Invalido(&'static str),
}

pub(crate) async fn escribir_trama<W: AsyncWrite + Unpin>(
    w: &mut W,
    bytes: &[u8],
    tope: usize,
) -> Result<(), ErrorTrama> {
    if bytes.len() > tope {
        return Err(ErrorTrama::Grande(bytes.len()));
    }
    let n = u32::try_from(bytes.len()).map_err(|_| ErrorTrama::Grande(bytes.len()))?;
    w.write_all(&n.to_be_bytes()).await?;
    w.write_all(bytes).await?;
    w.flush().await?;
    Ok(())
}

/// Lee una trama de como mucho `tope` bytes: si dice ser mayor, ni se lee.
pub(crate) async fn leer_trama<R: AsyncRead + Unpin>(
    r: &mut R,
    tope: usize,
) -> Result<Vec<u8>, ErrorTrama> {
    let mut largo = [0u8; 4];
    r.read_exact(&mut largo).await?;
    let n = u32::from_be_bytes(largo) as usize;
    if n > tope {
        return Err(ErrorTrama::Grande(n));
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn leer_json<T: DeserializeOwned, R: AsyncRead + Unpin>(
    r: &mut R,
    tope: usize,
) -> Result<(T, usize), ErrorTrama> {
    let b = leer_trama(r, tope).await?;
    Ok((serde_json::from_slice(&b)?, b.len()))
}

pub async fn escribir_al_host<W: AsyncWrite + Unpin>(
    w: &mut W,
    m: &AlHost,
) -> Result<(), ErrorTrama> {
    escribir_trama(w, &serde_json::to_vec(m)?, m.tope()).await
}

pub async fn escribir_del_host<W: AsyncWrite + Unpin>(
    w: &mut W,
    m: &DelHost,
) -> Result<(), ErrorTrama> {
    escribir_trama(w, &serde_json::to_vec(m)?, m.tope()).await
}

/// Lo que lee el host de un jugador: cada mensaje con su tope (una trama
/// de más de 64 KiB solo vale si es una `Preparacion`), y el deck validado.
pub async fn leer_al_host<R: AsyncRead + Unpin>(r: &mut R) -> Result<AlHost, ErrorTrama> {
    let (m, n): (AlHost, usize) = leer_json(r, TOPE_PREPARACION).await?;
    if n > m.tope() {
        return Err(ErrorTrama::Grande(n));
    }
    if let AlHost::Preparacion { deck } = &m {
        validar_deck(deck).map_err(ErrorTrama::Invalido)?;
    }
    Ok(m)
}

/// El primer mensaje, antes de admitir a nadie: como mucho `TOPE_HOLA`
/// bytes, y tiene que ser un `Hola`.
pub async fn leer_hola<R: AsyncRead + Unpin>(r: &mut R) -> Result<AlHost, ErrorTrama> {
    let (m, _): (AlHost, usize) = leer_json(r, TOPE_HOLA).await?;
    match m {
        AlHost::Hola { .. } => Ok(m),
        _ => Err(ErrorTrama::Invalido("se esperaba un Hola")),
    }
}

/// Lo que lee un jugador del host (con el tope de cada mensaje).
pub async fn leer_del_host<R: AsyncRead + Unpin>(r: &mut R) -> Result<DelHost, ErrorTrama> {
    let (m, n): (DelHost, usize) = leer_json(r, TOPE_PARTIDA).await?;
    if n > m.tope() {
        return Err(ErrorTrama::Grande(n));
    }
    Ok(m)
}

pub fn datagrama_a_bytes(d: &Datagrama) -> Option<Vec<u8>> {
    serde_json::to_vec(d)
        .ok()
        .filter(|b| b.len() <= TOPE_DATAGRAMA)
}

pub fn datagrama_de_bytes(b: &[u8]) -> Option<Datagrama> {
    if b.len() > TOPE_DATAGRAMA {
        return None;
    }
    serde_json::from_slice(b).ok()
}

fn texto_corto(s: &str, max: usize) -> bool {
    s.chars().count() <= max
}

fn imagen_valida(r: &RefImagen) -> bool {
    match r {
        RefImagen::Generica => true,
        RefImagen::Propia {
            sha256,
            ancho,
            alto,
        } => {
            sha256.len() == 64
                && sha256.chars().all(|c| c.is_ascii_hexdigit())
                && *ancho > 0
                && *alto > 0
                && *ancho <= 4096
                && *alto <= 4096
        }
        // Las de Scryfall las baja cada uno, y solo de cards.scryfall.io (E9).
        RefImagen::Scryfall { id, url, cara } => {
            texto_corto(id, 64)
                && *cara <= 1
                && url.starts_with("https://cards.scryfall.io/")
                && texto_corto(url, 300)
        }
    }
}

/// Lo que se le consiente al deck que trae un jugador.
pub fn validar_deck(d: &DeckDeMesa) -> Result<(), &'static str> {
    if d.cartas.len() > MAX_CARTAS_DECK {
        return Err("demasiadas cartas");
    }
    if !texto_corto(&d.nombre, 200) || !imagen_valida(&d.dorso) {
        return Err("deck");
    }
    if d.tapete.as_ref().is_some_and(|t| !imagen_valida(t)) {
        return Err("tapete");
    }
    for c in &d.cartas {
        let o = &c.origen;
        let info = &o.info;
        let ok = texto_corto(&o.nombre, 200)
            && texto_corto(&c.carta_deck, 100)
            && o.caras.len() <= 4
            && o.caras.iter().all(imagen_valida)
            && imagen_valida(&o.dorso)
            && info.tipo.as_deref().is_none_or(|t| texto_corto(t, 200))
            && info.coste.as_deref().is_none_or(|t| texto_corto(t, 100))
            && info.palabras_clave.len() <= 30
            && info.funciones.len() <= 30
            && info.colores.len() <= 6;
        if !ok {
            return Err("carta");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mesa::accion::{Actor, Cambio};
    use mesa::modelo::{InfoCarta, OrigenCarta};
    use mesa::preparar::{CartaDeMesa, Seccion};

    fn deck(n: usize) -> DeckDeMesa {
        DeckDeMesa {
            nombre: "Prueba".into(),
            cartas: (0..n)
                .map(|i| CartaDeMesa {
                    carta_deck: format!("c{i}"),
                    seccion: Seccion::Mazo,
                    origen: OrigenCarta {
                        carta_deck: format!("c{i}"),
                        nombre: format!("Carta {i}"),
                        caras: vec![RefImagen::Propia {
                            sha256: "a".repeat(64),
                            ancho: 745,
                            alto: 1040,
                        }],
                        dorso: RefImagen::Generica,
                        scryfall_id: None,
                        info: InfoCarta::default(),
                    },
                })
                .collect(),
            tapete: None,
            dorso: RefImagen::Generica,
        }
    }

    fn hola() -> AlHost {
        AlHost::Hola {
            protocolo: PROTOCOLO,
            version_app: version_app(),
            version_catalogo: mesa::VERSION_CATALOGO,
            rol: Rol::Jugador,
            apodo: "Pablo".into(),
            secreto: Some("00".repeat(16)),
            ultimo_seq: None,
            epoch: 1,
        }
    }

    /// Cada mensaje de ejemplo (`fixtures/`) se lee y se vuelve a escribir igual.
    #[tokio::test]
    async fn ida_y_vuelta_de_cada_mensaje() {
        let al_host = vec![
            hola(),
            AlHost::Preparacion { deck: deck(3) },
            AlHost::Intencion {
                id: 7,
                accion: Accion::Robar { n: 1 },
            },
            AlHost::Pide { desde_seq: 40 },
            AlHost::Adios,
        ];
        let evento = Evento {
            seq: 41,
            actor: Actor::Jugador(IdJugador(1)),
            t_ms: 5,
            cambio: Cambio::Accion {
                accion: Accion::Robar { n: 1 },
                azar: None,
            },
        };
        let del_host = vec![
            DelHost::Sala {
                jugadores: vec![EnSala {
                    apodo: "Pablo".into(),
                    rol: Rol::Jugador,
                    deck: Some("Prueba".into()),
                }],
            },
            DelHost::Evento {
                epoch: 1,
                evento: Box::new(evento.clone()),
                confirma: Some(7),
                huella: Some("abc".into()),
            },
            DelHost::Rechazo {
                id: Some(8),
                motivo: "ritmo".into(),
            },
            // Con cartas: sus ids son claves de un mapa dentro de un enum
            // etiquetado (llegan como texto).
            DelHost::Bienvenida {
                epoch: 1,
                yo: IdJugador(1),
                partida: Box::new(
                    mesa::preparar::iniciar(
                        "p".into(),
                        mesa::preparar::Config {
                            formato: mesa::formato::Formato::Constructed,
                            variantes: Default::default(),
                            opciones: mesa::formato::Opciones::de_fabrica(
                                mesa::formato::Formato::Constructed,
                                &Default::default(),
                                2,
                            ),
                            jugadores: (0..2)
                                .map(|i| mesa::preparar::JugadorNuevo {
                                    nombre: format!("J{i}"),
                                    color: "#fff".into(),
                                    equipo: None,
                                    mazo: deck(40),
                                })
                                .collect(),
                            equipos: vec![],
                            primero: None,
                        },
                        1,
                        0,
                    )
                    .unwrap(),
                ),
                nodos: vec![],
            },
            DelHost::Cerrada,
        ];
        let (mut a, mut b) = tokio::io::duplex(8 * 1024 * 1024);
        for m in &al_host {
            escribir_al_host(&mut a, m).await.unwrap();
            assert_eq!(&leer_al_host(&mut b).await.unwrap(), m);
        }
        for m in &del_host {
            escribir_del_host(&mut a, m).await.unwrap();
            assert_eq!(&leer_del_host(&mut b).await.unwrap(), m);
        }
        for d in [
            Datagrama::Latido { t: 1 },
            Datagrama::Eco { t: 1, t_host: 2 },
            Datagrama::Fantasma {
                jugador: IdJugador(1),
                tablero: IdJugador(0),
                carta: Some(IdCarta(3)),
                x: 5_000,
                y: 2_000,
                t: 9,
            },
            Datagrama::Senal {
                jugador: IdJugador(0),
                carta: None,
                a: Some(IdJugador(1)),
                t: 9,
            },
            Datagrama::Enlaces {
                filas: vec![Enlace {
                    jugador: IdJugador(1),
                    rtt_ms: 23,
                    directo: true,
                }],
                t: 9,
            },
        ] {
            let b = datagrama_a_bytes(&d).unwrap();
            assert_eq!(datagrama_de_bytes(&b), Some(d));
        }
    }

    /// Escribe los ejemplos de `fixtures/` (`PB_FIXTURES=1 cargo test --lib escribir_fixtures`).
    #[tokio::test]
    async fn escribir_fixtures() {
        if std::env::var("PB_FIXTURES").is_err() {
            return;
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/red/fixtures");
        let al = vec![
            hola(),
            AlHost::Preparacion { deck: deck(1) },
            AlHost::Intencion {
                id: 7,
                accion: Accion::Robar { n: 1 },
            },
            AlHost::Pide { desde_seq: 40 },
            AlHost::Adios,
        ];
        let del = vec![
            DelHost::Sala {
                jugadores: vec![EnSala {
                    apodo: "Pablo".into(),
                    rol: Rol::Jugador,
                    deck: Some("Prueba".into()),
                }],
            },
            DelHost::Evento {
                epoch: 1,
                evento: Box::new(Evento {
                    seq: 41,
                    actor: Actor::Jugador(IdJugador(1)),
                    t_ms: 5,
                    cambio: Cambio::Accion {
                        accion: Accion::Robar { n: 1 },
                        azar: None,
                    },
                }),
                confirma: Some(7),
                huella: None,
            },
            DelHost::Rechazo {
                id: Some(8),
                motivo: "ritmo".into(),
            },
            DelHost::Cerrada,
        ];
        let dg = vec![
            Datagrama::Latido { t: 1 },
            Datagrama::Eco { t: 1, t_host: 2 },
            Datagrama::Fantasma {
                jugador: IdJugador(1),
                tablero: IdJugador(0),
                carta: Some(IdCarta(3)),
                x: 5_000,
                y: 2_000,
                t: 9,
            },
            Datagrama::Senal {
                jugador: IdJugador(0),
                carta: None,
                a: Some(IdJugador(1)),
                t: 9,
            },
            Datagrama::Enlaces {
                filas: vec![
                    Enlace {
                        jugador: IdJugador(0),
                        rtt_ms: 0,
                        directo: true,
                    },
                    Enlace {
                        jugador: IdJugador(1),
                        rtt_ms: 42,
                        directo: false,
                    },
                ],
                t: 10,
            },
        ];
        let escribir = |n: &str, j: String| std::fs::write(dir.join(n), j + "\n").unwrap();
        escribir("al-host.json", serde_json::to_string_pretty(&al).unwrap());
        escribir("del-host.json", serde_json::to_string_pretty(&del).unwrap());
        escribir(
            "datagramas.json",
            serde_json::to_string_pretty(&dg).unwrap(),
        );
    }

    #[test]
    fn los_ejemplos_de_fixtures_se_leen() {
        let al: Vec<AlHost> = serde_json::from_str(include_str!("fixtures/al-host.json")).unwrap();
        assert!(matches!(al[0], AlHost::Hola { .. }));
        let del: Vec<DelHost> =
            serde_json::from_str(include_str!("fixtures/del-host.json")).unwrap();
        assert!(!del.is_empty());
        let dg: Vec<Datagrama> =
            serde_json::from_str(include_str!("fixtures/datagramas.json")).unwrap();
        assert_eq!(dg.len(), 5);
    }

    #[tokio::test]
    async fn una_trama_enorme_ni_se_lee() {
        let (mut a, mut b) = tokio::io::duplex(1024);
        // Dice medir 5 MiB.
        a.write_all(&(5u32 * 1024 * 1024).to_be_bytes())
            .await
            .unwrap();
        assert!(matches!(
            leer_al_host(&mut b).await,
            Err(ErrorTrama::Grande(_))
        ));
        let (mut a, mut b) = tokio::io::duplex(1024);
        a.write_all(&(5u32 * 1024 * 1024).to_be_bytes())
            .await
            .unwrap();
        assert!(matches!(
            leer_del_host(&mut b).await,
            Err(ErrorTrama::Grande(_))
        ));
    }

    #[tokio::test]
    async fn mas_de_64_kib_solo_si_es_una_preparacion() {
        let (mut a, mut b) = tokio::io::duplex(8 * 1024 * 1024);
        // Una intención inflada con un texto enorme.
        let gorda = AlHost::Intencion {
            id: 1,
            accion: Accion::Chat {
                texto: "x".repeat(100 * 1024),
            },
        };
        let bytes = serde_json::to_vec(&gorda).unwrap();
        a.write_all(&(bytes.len() as u32).to_be_bytes())
            .await
            .unwrap();
        a.write_all(&bytes).await.unwrap();
        assert!(matches!(
            leer_al_host(&mut b).await,
            Err(ErrorTrama::Grande(_))
        ));
        // Un deck de 300 cartas sí (y escribirlo la gorda, no).
        assert!(escribir_al_host(&mut a, &gorda).await.is_err());
        let prep = AlHost::Preparacion { deck: deck(300) };
        escribir_al_host(&mut a, &prep).await.unwrap();
        assert_eq!(leer_al_host(&mut b).await.unwrap(), prep);
    }

    #[tokio::test]
    async fn json_roto_y_decks_malos() {
        let (mut a, mut b) = tokio::io::duplex(1024 * 1024);
        a.write_all(&5u32.to_be_bytes()).await.unwrap();
        a.write_all(b"{nope").await.unwrap();
        assert!(matches!(
            leer_al_host(&mut b).await,
            Err(ErrorTrama::Json(_))
        ));
        // 501 cartas.
        assert_eq!(validar_deck(&deck(501)), Err("demasiadas cartas"));
        // Una imagen de Scryfall de otro sitio.
        let mut d = deck(1);
        d.cartas[0].origen.caras = vec![RefImagen::Scryfall {
            id: "x".into(),
            cara: 0,
            url: "https://malo.example/x.jpg".into(),
        }];
        assert_eq!(validar_deck(&d), Err("carta"));
        // Una huella que no lo es.
        let mut d = deck(1);
        d.cartas[0].origen.caras = vec![RefImagen::Propia {
            sha256: "../../etc".into(),
            ancho: 1,
            alto: 1,
        }];
        assert_eq!(validar_deck(&d), Err("carta"));
        assert_eq!(validar_deck(&deck(10)), Ok(()));
    }

    #[tokio::test]
    async fn el_hola_tiene_su_tope_y_va_primero() {
        let (mut a, mut b) = tokio::io::duplex(64 * 1024);
        escribir_al_host(&mut a, &hola()).await.unwrap();
        assert!(matches!(leer_hola(&mut b).await, Ok(AlHost::Hola { .. })));
        // Algo que no es un Hola.
        escribir_al_host(&mut a, &AlHost::Adios).await.unwrap();
        assert!(leer_hola(&mut b).await.is_err());
        // Un Hola de más de 4 KiB.
        let gordo = AlHost::Hola {
            protocolo: 1,
            version_app: "x".repeat(5_000),
            version_catalogo: 1,
            rol: Rol::Jugador,
            apodo: String::new(),
            secreto: None,
            ultimo_seq: None,
            epoch: 1,
        };
        escribir_al_host(&mut a, &gordo).await.unwrap();
        assert!(matches!(
            leer_hola(&mut b).await,
            Err(ErrorTrama::Grande(_))
        ));
    }

    #[test]
    fn un_datagrama_grande_o_roto_se_tira() {
        assert_eq!(datagrama_de_bytes(&[b'x'; 1_300]), None);
        assert_eq!(datagrama_de_bytes(b"{roto"), None);
    }
}
