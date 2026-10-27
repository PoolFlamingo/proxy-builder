//! **Latencia, camino, reloj con el anfitrión y movimiento en vivo** (E7, E10).
//!
//! Todo va por datagramas de la conexión de la partida (`proxy-builder/mesa/1`):
//! no son fiables y no importa, porque se repiten solos.
//!
//! - **Latido y eco** (cada 2 s): el jugador manda `Latido { t }` y el
//!   anfitrión devuelve `Eco { t, t_host }`. De ahí sale el RTT de la propia
//!   app y el **desfase del reloj**: `t_host − (t + RTT/2)`, con la mediana
//!   de los últimos 10. El temporizador del turno cuenta desde la hora del
//!   anfitrión (`Turno.desde_ms` la pone él), así que la pantalla lee la
//!   hora como `Date.now() + desfase` y todos marcan lo mismo.
//! - **Enlaces** (cada 2 s): el anfitrión mide cada conexión (RTT de iroh y
//!   si el camino es directo o por relé) y manda la tabla a todos.
//! - **Fantasmas y señales** (E7): lo que se arrastra y el cursor de cada
//!   jugador, y los aros de aviso. El anfitrión **pone** quién los manda
//!   (nunca se fía del `jugador` que traen), los limita y los reparte; a un
//!   espectador solo le llegan si la carta es visible para él.
//!
//! Nada de esto se guarda, ni entra en el registro, ni en la repetición.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use iroh::endpoint::Connection;
use mesa::modelo::Partida;
use mesa::{IdCarta, IdJugador, Mirador};

use super::anfitrion::Anfitrion;
use super::mesa::{AvisoRed, Mesa};
use super::protocolo::{
    datagrama_a_bytes, datagrama_de_bytes, Datagrama, Enlace, Rol, COORD_MAX, MAX_MAS,
};
use crate::state::AppState;

/// Cada cuánto va el latido (y la tabla de enlaces).
pub const CADA_LATIDO: Duration = Duration::from_secs(2);
/// Cuántos desfases se promedian.
const MUESTRAS: usize = 10;
/// Fantasmas por segundo que el anfitrión admite de un jugador (la pantalla
/// manda 20 del arrastre y 10 del cursor, como mucho).
const FANTASMAS_POR_SEGUNDO: usize = 60;

/// La hora de esta app, en ms desde 1970.
pub fn ahora_ms() -> u64 {
    crate::mesa::sesion::ahora_ms()
}

/// La hora de un datagrama que sale de la pantalla: la de ahora, pero nunca
/// igual ni menor que la del anterior (el anfitrión tira lo que no es más
/// nuevo, y dos movimientos en el mismo milisegundo no deben pisarse).
fn marca() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static ULTIMA: AtomicU64 = AtomicU64::new(0);
    let ahora = ahora_ms();
    let mut previa = ULTIMA.load(Ordering::Relaxed);
    loop {
        let nueva = ahora.max(previa.saturating_add(1));
        match ULTIMA.compare_exchange_weak(previa, nueva, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return nueva,
            Err(p) => previa = p,
        }
    }
}

/// El reloj del anfitrión visto desde aquí.
#[derive(Debug, Default)]
pub struct Reloj {
    desfases: VecDeque<i64>,
}

impl Reloj {
    /// Llega el eco de un latido mandado en `t` (hora local), con la hora del
    /// anfitrión al recibirlo, y `ahora` es la hora local de ahora. Devuelve
    /// el RTT de la app en ms (`None` si el reloj fue hacia atrás).
    pub fn eco(&mut self, t: u64, t_host: u64, ahora: u64) -> Option<u32> {
        let rtt = ahora.checked_sub(t)?;
        // El eco tardó medio RTT en volver: en el instante `t + rtt/2` de
        // aquí, el anfitrión marcaba `t_host`.
        let medio = i64::try_from(rtt / 2).unwrap_or(i64::MAX);
        let desfase = i64::try_from(t_host)
            .unwrap_or(i64::MAX)
            .saturating_sub(i64::try_from(t).unwrap_or(i64::MAX).saturating_add(medio));
        self.desfases.push_back(desfase);
        while self.desfases.len() > MUESTRAS {
            self.desfases.pop_front();
        }
        Some(u32::try_from(rtt).unwrap_or(u32::MAX))
    }

    /// Lo que hay que sumar a la hora de aquí para tener la del anfitrión
    /// (la mediana de las últimas muestras; 0 si aún no hay).
    pub fn desfase_ms(&self) -> i32 {
        if self.desfases.is_empty() {
            return 0;
        }
        let mut v: Vec<i64> = self.desfases.iter().copied().collect();
        v.sort_unstable();
        let n = v.len();
        let mediana = if n % 2 == 1 {
            v[n / 2]
        } else {
            (v[n / 2 - 1] + v[n / 2]) / 2
        };
        i32::try_from(mediana.clamp(i64::from(i32::MIN), i64::from(i32::MAX))).unwrap_or(0)
    }
}

/// Descarta los datagramas viejos: de cada jugador y clase solo valen los
/// que traen una hora mayor que la del último.
#[derive(Debug, Default)]
pub struct Frescura {
    ultimos: BTreeMap<(IdJugador, u8), u64>,
}

impl Frescura {
    pub fn admite(&mut self, jugador: IdJugador, clase: u8, t: u64) -> bool {
        match self.ultimos.get(&(jugador, clase)) {
            Some(&u) if t <= u => false,
            _ => {
                self.ultimos.insert((jugador, clase), t);
                true
            }
        }
    }
}

const CLASE_FANTASMA: u8 = 0;
const CLASE_SENAL: u8 = 1;

/// Lo que se mide de una conexión: el RTT (ms) y si el camino es directo.
pub fn medir(conn: &Connection) -> Option<(u32, bool)> {
    let caminos = conn.paths();
    let p = caminos.iter().find(|p| p.is_selected())?;
    Some((
        u32::try_from(p.rtt().as_millis()).unwrap_or(u32::MAX),
        p.is_ip(),
    ))
}

pub(super) fn enviar(conn: &Connection, d: &Datagrama) {
    if let Some(b) = datagrama_a_bytes(d) {
        // Si no cabe o la conexión se cerró, se pierde: es lo que hay.
        let _ = conn.send_datagram(Bytes::from(b));
    }
}

/// Si un espectador vería esta carta tal como está ahora (si no, ni el
/// movimiento de lo que arrastra le llega).
pub fn visible_para_espectador(p: &Partida, carta: IdCarta) -> bool {
    let Some(c) = p.cartas.get(&carta) else {
        return false;
    };
    p.donde(carta)
        .is_some_and(|(zona, i)| mesa::visibilidad::ve(p, c, &zona, i, Mirador::Espectador))
}

fn limitar_coordenada(v: u16) -> u16 {
    v.min(COORD_MAX)
}

// ───────────────────────── anfitrión ─────────────────────────

/// Atiende los datagramas de un jugador (o espectador) conectado al
/// anfitrión, y cada `cada` le manda la tabla de enlaces.
pub async fn atender_host<M: Mesa>(
    conn: Connection,
    anf: Arc<Anfitrion>,
    mesa: M,
    conexion: u64,
    cada: Duration,
) {
    let mut tic = tokio::time::interval(cada);
    tic.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut fresca = Frescura::default();
    let mut ritmo: VecDeque<Instant> = VecDeque::new();
    loop {
        tokio::select! {
            d = conn.read_datagram() => {
                let Ok(bytes) = d else { return };
                let Some(dg) = datagrama_de_bytes(&bytes) else { continue };
                tratar_en_host(&conn, &anf, &mesa, conexion, dg, &mut fresca, &mut ritmo);
            }
            _ = tic.tick() => {
                if let Some((rtt, directo)) = medir(&conn) {
                    anf.poner_enlace(conexion, rtt, directo);
                }
                let filas = anf.enlaces();
                enviar(&conn, &Datagrama::Enlaces { filas: filas.clone(), ausentes: anf.ausentes(), t: ahora_ms() });
                mesa.avisar(AvisoRed::Enlaces { desfase_ms: 0, filas });
            }
        }
    }
}

fn tratar_en_host<M: Mesa>(
    conn: &Connection,
    anf: &Arc<Anfitrion>,
    mesa: &M,
    conexion: u64,
    dg: Datagrama,
    fresca: &mut Frescura,
    ritmo: &mut VecDeque<Instant>,
) {
    match dg {
        Datagrama::Latido { t } => enviar(
            conn,
            &Datagrama::Eco {
                t,
                t_host: ahora_ms(),
            },
        ),
        Datagrama::Fantasma {
            tablero,
            carta,
            mut mas,
            x,
            y,
            t,
            ..
        } => {
            let Some(quien) = anf.jugador_de(conexion) else {
                return;
            };
            if !pasa_el_ritmo(ritmo) || !fresca.admite(quien, CLASE_FANTASMA, t) {
                return;
            }
            mas.truncate(MAX_MAS);
            let cartas: Vec<IdCarta> = carta.into_iter().chain(mas.iter().copied()).collect();
            let d = Datagrama::Fantasma {
                jugador: quien,
                tablero,
                carta,
                mas,
                x: limitar_coordenada(x),
                y: limitar_coordenada(y),
                t,
            };
            repartir(anf, mesa, &d, Some(quien), &cartas);
        }
        Datagrama::Senal { carta, a, t, .. } => {
            let Some(quien) = anf.jugador_de(conexion) else {
                return;
            };
            if !pasa_el_ritmo(ritmo) || !fresca.admite(quien, CLASE_SENAL, t) {
                return;
            }
            let d = Datagrama::Senal {
                jugador: quien,
                carta,
                a,
                t,
            };
            repartir(anf, mesa, &d, Some(quien), carta.as_slice());
        }
        // Eco y Enlaces son del anfitrión hacia los demás: de un jugador no valen.
        Datagrama::Eco { .. } | Datagrama::Enlaces { .. } => {}
    }
}

fn pasa_el_ritmo(ritmo: &mut VecDeque<Instant>) -> bool {
    let ahora = Instant::now();
    while ritmo
        .front()
        .is_some_and(|t| ahora.duration_since(*t) > Duration::from_secs(1))
    {
        ritmo.pop_front();
    }
    if ritmo.len() >= FANTASMAS_POR_SEGUNDO {
        return false;
    }
    ritmo.push_back(ahora);
    true
}

/// Lo reparte a los demás y se lo cuenta a la pantalla del anfitrión.
fn repartir<M: Mesa>(
    anf: &Arc<Anfitrion>,
    mesa: &M,
    d: &Datagrama,
    excepto: Option<IdJugador>,
    cartas: &[IdCarta],
) {
    // Primero la sesión y luego la sala, nunca al revés (orden de cerrojos).
    // Al espectador solo le llega si ve todas las cartas que se arrastran.
    let visible = cartas.iter().all(|c| {
        mesa.con(|m| {
            m.as_ref()
                .is_some_and(|s| visible_para_espectador(s.partida(), *c))
        })
    });
    anf.repartir(d, excepto, visible);
    avisar_movimiento(mesa, d);
}

/// Lo que llega de otro, a la pantalla.
fn avisar_movimiento<M: Mesa>(mesa: &M, d: &Datagrama) {
    match d {
        Datagrama::Fantasma {
            jugador,
            tablero,
            carta,
            mas,
            x,
            y,
            ..
        } => mesa.avisar(AvisoRed::Fantasma {
            jugador: *jugador,
            tablero: *tablero,
            carta: *carta,
            mas: mas.clone(),
            x: *x,
            y: *y,
        }),
        Datagrama::Senal {
            jugador, carta, a, ..
        } => mesa.avisar(AvisoRed::Senal {
            jugador: *jugador,
            carta: *carta,
            a: *a,
        }),
        _ => {}
    }
}

// ───────────────────────── invitado ─────────────────────────

/// Atiende los datagramas del anfitrión y le manda el latido cada `cada`.
/// `reloj_local` es la hora de esta app (en las pruebas, una desfasada).
pub async fn atender_invitado<M: Mesa>(
    conn: Connection,
    mesa: M,
    reloj_local: Arc<dyn Fn() -> u64 + Send + Sync>,
    cada: Duration,
) {
    let mut tic = tokio::time::interval(cada);
    tic.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut reloj = Reloj::default();
    let mut fresca = Frescura::default();
    let mut filas: Vec<Enlace> = Vec::new();
    let mut rtt_app: Option<u32> = None;
    loop {
        tokio::select! {
            d = conn.read_datagram() => {
                let Ok(bytes) = d else { return };
                let Some(dg) = datagrama_de_bytes(&bytes) else { continue };
                match dg {
                    Datagrama::Eco { t, t_host } => {
                        if let Some(r) = reloj.eco(t, t_host, reloj_local()) {
                            rtt_app = Some(r);
                            avisar_enlaces(&mesa, &conn, &reloj, &filas, rtt_app);
                        }
                    }
                    Datagrama::Enlaces { filas: nuevas, ausentes, .. } => {
                        filas = nuevas;
                        mesa.avisar(AvisoRed::Ausentes { filas: ausentes });
                        avisar_enlaces(&mesa, &conn, &reloj, &filas, rtt_app);
                    }
                    Datagrama::Fantasma { jugador, t, .. } => {
                        if fresca.admite(jugador, CLASE_FANTASMA, t) {
                            avisar_movimiento(&mesa, &dg);
                        }
                    }
                    Datagrama::Senal { jugador, t, .. } => {
                        if fresca.admite(jugador, CLASE_SENAL, t) {
                            avisar_movimiento(&mesa, &dg);
                        }
                    }
                    Datagrama::Latido { .. } => {}
                }
            }
            _ = tic.tick() => enviar(&conn, &Datagrama::Latido { t: reloj_local() }),
        }
    }
}

/// La tabla que ve este jugador: el enlace de cada uno con el anfitrión y,
/// en la fila del propio anfitrión (el jugador 0), el de este con él.
fn avisar_enlaces<M: Mesa>(
    mesa: &M,
    conn: &Connection,
    reloj: &Reloj,
    filas: &[Enlace],
    rtt_app: Option<u32>,
) {
    let mut filas = filas.to_vec();
    if let Some((rtt, directo)) = medir(conn) {
        let mio = Enlace {
            jugador: IdJugador(0),
            rtt_ms: rtt_app.unwrap_or(rtt),
            directo,
        };
        match filas.iter_mut().find(|f| f.jugador == mio.jugador) {
            Some(f) => *f = mio,
            None => filas.insert(0, mio),
        }
    }
    mesa.avisar(AvisoRed::Enlaces {
        desfase_ms: reloj.desfase_ms(),
        filas,
    });
}

// ───────────────────────── desde la pantalla ─────────────────────────

/// Quién manda en esta app cuando la pantalla quiere mover un fantasma.
enum Salida {
    Anfitrion(Arc<Anfitrion>),
    Invitado(Connection),
}

fn salida(state: &AppState) -> Option<Salida> {
    state.red.con_sala(|s| match s {
        Some(super::Sala::Anfitrion { anf, .. }) => Some(Salida::Anfitrion(anf.clone())),
        Some(super::Sala::Invitado {
            conexion,
            rol: Rol::Jugador,
        }) => conexion.conexion().map(Salida::Invitado),
        _ => None,
    })
}

/// Quién soy en la partida abierta (el asiento que mira esta pantalla).
fn yo(state: &AppState) -> Option<IdJugador> {
    let g = match state.mesa.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    g.as_ref().map(|s| s.yo)
}

/// Si lo que se mueve es visible para un espectador (todas las cartas).
fn visible(state: &AppState, cartas: &[IdCarta]) -> bool {
    let g = match state.mesa.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    cartas.iter().all(|c| {
        g.as_ref()
            .is_some_and(|s| visible_para_espectador(s.partida(), *c))
    })
}

/// La pantalla mueve una carta (o su cursor, sin carta) por el `tablero`, y
/// con ella `mas` cartas si se arrastran varias.
/// Sin partida en red (o mirando), no hace nada.
pub fn mover_fantasma(
    state: &AppState,
    tablero: IdJugador,
    carta: Option<IdCarta>,
    mut mas: Vec<IdCarta>,
    x: u16,
    y: u16,
) {
    let (Some(salida), Some(yo)) = (salida(state), yo(state)) else {
        return;
    };
    mas.truncate(MAX_MAS);
    let cartas: Vec<IdCarta> = carta.into_iter().chain(mas.iter().copied()).collect();
    let d = Datagrama::Fantasma {
        jugador: yo,
        tablero,
        carta,
        mas,
        x: limitar_coordenada(x),
        y: limitar_coordenada(y),
        t: marca(),
    };
    mandar(state, salida, &d, yo, &cartas);
}

/// La pantalla señala una carta o un jugador (un aro de 2 s en todas).
pub fn senalar(state: &AppState, carta: Option<IdCarta>, a: Option<IdJugador>) {
    let (Some(salida), Some(yo)) = (salida(state), yo(state)) else {
        return;
    };
    let d = Datagrama::Senal {
        jugador: yo,
        carta,
        a,
        t: marca(),
    };
    mandar(state, salida, &d, yo, carta.as_slice());
}

fn mandar(state: &AppState, salida: Salida, d: &Datagrama, yo: IdJugador, cartas: &[IdCarta]) {
    match salida {
        Salida::Anfitrion(anf) => anf.repartir(d, Some(yo), visible(state, cartas)),
        Salida::Invitado(conn) => enviar(&conn, d),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_desfase_sale_aunque_el_reloj_vaya_cinco_segundos_atras() {
        // El reloj de aquí va 5 s por detrás del anfitrión; la red tarda
        // 40 ms de ida y 60 de vuelta (asimétrica).
        let mut r = Reloj::default();
        let mut local = 1_000_000u64;
        for _ in 0..12 {
            let t = local;
            let host = (t + 40) + 5_000; // el anfitrión lo recibe
            local = t + 100; // el eco vuelve
            assert_eq!(r.eco(t, host, local), Some(100));
            local += 2_000;
        }
        // Con la hora del anfitrión = hora local + desfase, a ±50 ms.
        let d = i64::from(r.desfase_ms());
        assert!((d - 5_000).abs() <= 50, "desfase {d}");
    }

    #[test]
    fn la_mediana_aguanta_un_eco_atrasado() {
        let mut r = Reloj::default();
        for i in 0..9u64 {
            let t = 10_000 + i * 2_000;
            r.eco(t, t + 25 + 800, t + 50);
        }
        // Un eco que tardó 3 s en volver da un desfase disparatado.
        r.eco(40_000, 40_000 + 25 + 800, 43_000);
        let d = r.desfase_ms();
        assert!((d - 800).abs() <= 5, "desfase {d}");
    }

    #[test]
    fn sin_muestras_no_hay_desfase_y_un_reloj_hacia_atras_se_tira() {
        let mut r = Reloj::default();
        assert_eq!(r.desfase_ms(), 0);
        assert_eq!(r.eco(500, 600, 400), None);
        assert_eq!(r.desfase_ms(), 0);
    }

    #[test]
    fn los_datagramas_viejos_se_tiran_por_jugador_y_clase() {
        let mut f = Frescura::default();
        let (a, b) = (IdJugador(0), IdJugador(1));
        assert!(f.admite(a, CLASE_FANTASMA, 10));
        assert!(!f.admite(a, CLASE_FANTASMA, 10));
        assert!(!f.admite(a, CLASE_FANTASMA, 9));
        assert!(f.admite(a, CLASE_FANTASMA, 11));
        // Otro jugador y otra clase llevan su cuenta.
        assert!(f.admite(b, CLASE_FANTASMA, 1));
        assert!(f.admite(a, CLASE_SENAL, 1));
    }

    #[test]
    fn el_ritmo_corta_a_partir_de_sesenta_por_segundo() {
        let mut r = VecDeque::new();
        let pasan = (0..100).filter(|_| pasa_el_ritmo(&mut r)).count();
        assert_eq!(pasan, FANTASMAS_POR_SEGUNDO);
    }

    #[test]
    fn la_marca_de_los_movimientos_siempre_crece() {
        let a: Vec<u64> = (0..1000).map(|_| marca()).collect();
        assert!(a.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn las_coordenadas_se_acotan() {
        assert_eq!(limitar_coordenada(65_000), COORD_MAX);
        assert_eq!(limitar_coordenada(4_000), 4_000);
    }
}
