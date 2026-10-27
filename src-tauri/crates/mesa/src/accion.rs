//! **El catálogo de acciones**: cada cosa que se puede hacer en la mesa,
//! como un dato.
//!
//! - `Intencion` = lo que pide un jugador (`Accion`).
//! - `Evento` = lo ya resuelto por el host (`Cambio`): con el orden del
//!   barajado, el resultado del dado, las cartas que salieron… Es lo que se
//!   guarda en el registro, se manda a los demás y se repite.
//!
//! 🛑 Las variantes nunca se cambian ni se quitan: una variante nueva sube
//! `VERSION_CATALOGO` (un enum de serde no acepta una etiqueta que no
//! conoce; la red y las repeticiones guardadas comparan la versión).
//!
//! | Acción | De dónde sale (Playgroup) |
//! |---|---|
//! | `Mover` (con `Destino` y `ComoMover`) | jugar, «Move to…», exiliar (`capturas/pablo/12–13`), lanzar o poner el comandante (§4) |
//! | `Girar`, `BocaAbajo`, `CambiarCara` | «Tap», «Turn face down», transformar |
//! | `Contador`, `FijarContador`, `MultiplicarContadores` | «Add counter», «Set counter», «+X/+X counters» |
//! | `ContadorClave` | «Keyword counters» |
//! | `Pt`, `MasXMasY` | «Power / Toughness» (§2.2) |
//! | `Etiqueta` | «Add label» |
//! | `PonerBolita`, `MoverBolita`, `QuitarBolita` | «Place marker» (§2.2) |
//! | `Adjuntar` | «Attach to» |
//! | `FaseFuera`, `NoEndereza`, `Resaltar` | «Phase out», «Doesn't untap», «Highlight» |
//! | `CopiaFicha`, `DarControl`, `TomarControl` | «Create token copy», «Give to player», quitar el control (Control Magic, Threaten…; v2) |
//! | `HabilidadEnPila` | «Put ability on stack» |
//! | `Flecha`, `QuitarFlechas` | «Attack / Target» (`capturas/pablo/12`, `18`) |
//! | `Robar`, `Moler`, `Manifestar`, `ExiliarDeArriba` | menú de la biblioteca (`capturas/pablo/10–11`) |
//! | `Mirar` + `Ordenar` | «Look at top/bottom», «Scry», «Surveil» |
//! | `RevelarArriba`, `Excavar` + `ResolverExcavacion` | «Reveal», «Cascade», «Discover», «Reveal until» (§3) |
//! | `Buscar`, `DejarDeMirar` | «Tutor», «Fetch land», «View library» (`capturas/pablo/14–15`) |
//! | `DeArribaRevelada`, `Barajar`, `Rueda` | «Play with top revealed», «Shuffle», «Wheel» |
//! | `EnderezarTodo`, `Organizar`, `CrearFicha` | menú del campo (`capturas/pablo/18`) |
//! | `Mulligan`, `QuedarseCon` | `capturas/pablo/02` |
//! | `Vida`, `VidaATodos`, `Drenar`, `Extorsion`, `Veneno`, `DanoComandante` | vida (`capturas/pablo/05`) |
//! | `ContadorJugador`, `Designar`, `DiaNoche` | «Trackers» (`capturas/pablo/06–07`) |
//! | `Lanzamientos`, `SinCosteExtra` | «Cast count» del comandante (§4) |
//! | `RevelarMano`, `RevelarA`, `JugarBocaAbajo`, `Descartar` | mano (§2.3) |
//! | `Eliminar`, `Conceder` | muerte (§6) |
//! | `VidaFantasma`, `DanoComandanteFantasma`, `VerFantasma` | «Phantom life» (§6) |
//! | `Senal` | señales de prioridad (§7) |
//! | `PasarTurno`, `TurnoExtra`, `Pausar`, `SaltarTurno` | turno (`capturas/pablo/00`, `03`) |
//! | `Reiniciar`, `Deshacer`, `Rehacer` | los hace la sesión (`registro.rs`) |
//! | `TirarDados`, `Moneda`, `DadoPlanar` | azar (`capturas/pablo/01`) |
//! | `Planeswalk`, `PonerEnMarcha`, `AbandonarPlan` | Planechase y Archenemy |
//! | `Resolver`, `ApilarDespues` | panel de la pila, «Stack it» (§8) |
//! | `Amontonar`, `SacarDelMonton`, `CambiarMonton`, `Separar` | montones, «Pile these» (§2.5) |
//! | `Agrupar` | exilio agrupado (§2.6) |
//! | `AlAzarDe`, `TodoAlFondoAlAzar`, `BarajarTodoDentro`, `ExiliarTodo` | cementerio y exilio (§4) |
//! | `IntensidadSparring`, `DecidirSparring` | sparring (Fase D) |
//! | `Chat`, `Aviso` | Fase H |
//! | `Terminar` | «¿Quién ganó?» (`capturas/pablo/21`) |

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ids::{IdCarta, IdJugador};
use crate::modelo::{
    DiaNoche, Etiqueta, Extremo, OrigenCarta, Partida, Pos, Resultado, Senal, Tamano, TipoFlecha,
    Zona, ZonaComun,
};
use crate::sparring::{Decision, Intensidad, ReaccionSparring};

/// Sube con cada variante nueva o cambiada de `Accion` o `Cambio`.
pub const VERSION_CATALOGO: u32 = 2;

/// Lo que pide un jugador.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Intencion {
    pub actor: IdJugador,
    pub accion: Accion,
}

/// Quién genera un evento: un jugador o la propia mesa (la reacción del
/// sparring, las ayudas del turno).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "id", rename_all = "camelCase")]
#[ts(export)]
pub enum Actor {
    Jugador(IdJugador),
    Mesa,
}

impl Actor {
    pub fn jugador(&self) -> Option<IdJugador> {
        match self {
            Actor::Jugador(j) => Some(*j),
            Actor::Mesa => None,
        }
    }
}

/// Un cambio ya resuelto, con su número de orden.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Evento {
    #[ts(type = "number")]
    pub seq: u64,
    pub actor: Actor,
    /// La hora del host.
    #[ts(type = "number")]
    pub t_ms: u64,
    pub cambio: Cambio,
}

/// Una tirada de un dado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Tirada {
    pub caras: u8,
    pub valor: u8,
}

/// La cara del dado planar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum CaraPlanar {
    Planeswalk,
    Caos,
    Nada,
}

// Son mensajes que se crean, se mandan y se aplican una vez: no vale la
// pena meter en una caja las variantes grandes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "tipo",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum Cambio {
    /// Una acción del catálogo con lo que haya decidido el host.
    ///
    /// Si la acción necesita azar (barajar, al azar…), el host pone una
    /// semilla nueva en `azar` y `aplicar` saca de ella todo lo que haga
    /// falta, en orden: el mismo evento da lo mismo en cualquier equipo.
    Accion {
        accion: Accion,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional, type = "number")]
        azar: Option<u64>,
    },
    Dados {
        tiradas: Vec<Tirada>,
        semilla_animacion: u32,
    },
    /// Cada lanzamiento son las monedas lanzadas a la vez (con pulgares de
    /// Krark, varias y se elige); `true` = cara.
    Monedas {
        lanzamientos: Vec<Vec<bool>>,
        hasta_perder: bool,
        semilla_animacion: u32,
    },
    DadoPlanar {
        cara: CaraPlanar,
        /// Lo que costó esta tirada (las anteriores del turno).
        coste: u32,
        semilla_animacion: u32,
    },
    /// Deshacer: la partida entera tal como estaba antes de `deshecho`.
    Restaurar {
        #[ts(type = "number | null")]
        deshecho: Option<u64>,
        #[ts(skip)]
        partida: Box<Partida>,
    },
    /// La reacción del fantasma (Fase D).
    Sparring { reaccion: ReaccionSparring },
}

/// A dónde va una carta al moverla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Destino {
    pub zona: ZonaDestino,
    /// De quién es la zona. Solo cuenta en el campo (el nuevo
    /// controlador); el resto de zonas son siempre las del dueño.
    pub jugador: Option<IdJugador>,
    pub sitio: Sitio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "zona", rename_all = "camelCase")]
#[ts(export)]
pub enum ZonaDestino {
    De(Zona),
    Comun(ZonaComun),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "valor", rename_all = "camelCase")]
#[ts(export)]
pub enum Sitio {
    Arriba,
    Abajo,
    Indice(u32),
    /// La N-ésima desde arriba (1 = arriba).
    XDesdeArriba(u32),
    EnCampo(Pos),
    AlAzar,
    BarajarDentro,
}

/// Cómo sale el comandante del mando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ComoMover {
    /// Cuenta para el impuesto.
    Lanzar,
    /// No cuenta.
    Poner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ModoExilio {
    Normal,
    BocaAbajo,
    /// Boca abajo y sin que la mire ni el que la exilia.
    Oculta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Lado {
    Arriba,
    Abajo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum FilaPt {
    Otros,
    HastaFinDeTurno,
    Fijo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Comparacion {
    Menor,
    MenorOIgual,
    Igual,
    MayorOIgual,
    Mayor,
}

impl Comparacion {
    pub fn cumple(self, a: u16, b: u16) -> bool {
        match self {
            Comparacion::Menor => a < b,
            Comparacion::MenorOIgual => a <= b,
            Comparacion::Igual => a == b,
            Comparacion::MayorOIgual => a >= b,
            Comparacion::Mayor => a > b,
        }
    }
}

/// Un filtro de cartas (buscar, excavar, al azar del cementerio).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Filtro {
    pub no_tierra: bool,
    /// `Legendary`, `Basic`, `Snow`…
    pub supertipo: Option<String>,
    /// Tipo o subtipo (`Creature`, `Forest`, `Equipment`…).
    pub tipo: Option<String>,
    pub cmc: Option<(Comparacion, u8)>,
    /// Alguno de estos colores (`W`, `U`, `B`, `R`, `G`; `C` = incoloro).
    pub colores: Vec<char>,
    /// Que el nombre contenga esto.
    pub nombre: Option<String>,
}

impl Filtro {
    pub fn cumple(&self, o: &OrigenCarta) -> bool {
        let i = &o.info;
        if self.no_tierra && i.es_tierra {
            return false;
        }
        if let Some(s) = &self.supertipo {
            if !i.tiene_tipo(s) {
                return false;
            }
        }
        if let Some(t) = &self.tipo {
            if !i.tiene_tipo(t) {
                return false;
            }
        }
        if let Some((c, n)) = self.cmc {
            if !c.cumple(i.cmc(), u16::from(n)) {
                return false;
            }
        }
        if !self.colores.is_empty() {
            let incolora = i.colores.is_empty();
            let ok = self.colores.iter().any(|c| {
                (*c == 'C' && incolora) || i.colores.iter().any(|x| x.eq_ignore_ascii_case(c))
            });
            if !ok {
                return false;
            }
        }
        if let Some(n) = &self.nombre {
            if !o.nombre.to_lowercase().contains(&n.to_lowercase()) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "n", rename_all = "camelCase")]
#[ts(export)]
pub enum FormaExcavar {
    HastaQueCoincida,
    RevelarN(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum RestoExcavar {
    Abajo,
    AbajoAlAzar,
    Cementerio,
    Exilio,
    /// Se quedan arriba (revelar N sin más).
    Arriba,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ComoResolver {
    /// Permanentes al campo; instantáneos y conjuros al cementerio;
    /// habilidades, fuera.
    Auto,
    Permanente,
    Hechizo,
    /// La mitad aventura: al exilio.
    MitadAventura,
    /// La mitad omen: se baraja en la biblioteca.
    MitadOmen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "tipo", content = "valor", rename_all = "camelCase")]
#[ts(export)]
pub enum ModoDescarte {
    Todas,
    AlAzar(u32),
    Elegidas(Vec<IdCarta>),
}

/// De dónde sale una ficha nueva.
// Son mensajes que se crean, se mandan y se aplican una vez: no vale la
// pena meter en una caja las variantes grandes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(
    tag = "tipo",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export)]
pub enum OrigenFicha {
    /// Una de las fichas del deck del actor (por su sitio en la lista).
    DelDeck { indice: u32 },
    /// Una carta o ficha de Scryfall que la app ya ha preparado.
    Carta { origen: OrigenCarta },
    Libre {
        nombre: String,
        fuerza: Option<String>,
        resistencia: Option<String>,
        colores: Vec<char>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Designacion {
    Monarca,
    Iniciativa,
    Bendicion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AvisoRapido {
    Espera,
    Respondo,
    Ok,
    Paso,
    TuTurno,
    Pienso,
}

// Son mensajes que se crean, se mandan y se aplican una vez: no vale la
// pena meter en una caja las variantes grandes.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "que", rename_all = "camelCase", rename_all_fields = "camelCase")]
#[ts(export)]
pub enum Accion {
    // ── Mover ──
    Mover {
        cartas: Vec<IdCarta>,
        a: Destino,
        como: Option<ComoMover>,
    },
    // ── Carta ──
    Girar {
        cartas: Vec<IdCarta>,
        girada: bool,
    },
    BocaAbajo {
        cartas: Vec<IdCarta>,
        si: bool,
    },
    CambiarCara {
        carta: IdCarta,
    },
    Contador {
        cartas: Vec<IdCarta>,
        tipo: String,
        delta: i32,
    },
    FijarContador {
        carta: IdCarta,
        tipo: String,
        valor: i32,
    },
    MultiplicarContadores {
        carta: IdCarta,
        por: u8,
    },
    ContadorClave {
        cartas: Vec<IdCarta>,
        palabra: String,
        si: bool,
    },
    Pt {
        carta: IdCarta,
        fila: FilaPt,
        valor: Option<(i32, i32)>,
    },
    MasXMasY {
        cartas: Vec<IdCarta>,
        x: i32,
        y: i32,
        fila: FilaPt,
    },
    Etiqueta {
        carta: IdCarta,
        etiqueta: Etiqueta,
        poner: bool,
    },
    PonerBolita {
        campo_de: IdJugador,
        pos: Pos,
        color: u8,
        tamano: Tamano,
    },
    MoverBolita {
        id: u32,
        pos: Pos,
    },
    QuitarBolita {
        id: u32,
    },
    Adjuntar {
        carta: IdCarta,
        a: Option<IdCarta>,
    },
    FaseFuera {
        cartas: Vec<IdCarta>,
        si: bool,
    },
    NoEndereza {
        cartas: Vec<IdCarta>,
        si: bool,
    },
    Resaltar {
        cartas: Vec<IdCarta>,
        color: Option<u8>,
    },
    CopiaFicha {
        cartas: Vec<IdCarta>,
    },
    DarControl {
        cartas: Vec<IdCarta>,
        a: IdJugador,
    },
    /// Toma el control de permanentes que controla otro (Control Magic,
    /// Threaten, Treachery…), con lo suyo que lleven encima. Con
    /// `hasta_fin_de_turno` vuelven a quien las controlaba al pasar el turno.
    /// Catálogo v2.
    TomarControl {
        cartas: Vec<IdCarta>,
        hasta_fin_de_turno: bool,
    },
    HabilidadEnPila {
        carta: IdCarta,
        texto: String,
    },
    Flecha {
        de: Extremo,
        a: Extremo,
        tipo: TipoFlecha,
    },
    /// Quita mis flechas (o solo las que salen de `de`).
    QuitarFlechas {
        de: Option<Extremo>,
    },
    // ── Biblioteca ──
    Robar {
        n: u32,
    },
    Moler {
        n: u32,
    },
    Manifestar {
        n: u32,
    },
    ExiliarDeArriba {
        n: u32,
        modo: ModoExilio,
    },
    Mirar {
        de: IdJugador,
        n: u32,
        desde: Lado,
    },
    /// Después de `Mirar`: lo mirado, a su sitio (arriba en este orden,
    /// abajo, cementerio o exilio).
    Ordenar {
        de: IdJugador,
        arriba: Vec<IdCarta>,
        abajo: Vec<IdCarta>,
        cementerio: Vec<IdCarta>,
        exilio: Vec<IdCarta>,
    },
    RevelarArriba {
        n: u32,
    },
    Excavar {
        forma: FormaExcavar,
        filtro: Filtro,
        parar_tras: u8,
    },
    /// Después de `Excavar`: las elegidas a su sitio y el resto, todas juntas.
    ResolverExcavacion {
        elegidas: Vec<(IdCarta, Destino)>,
        resto: Vec<IdCarta>,
        a: RestoExcavar,
    },
    /// Enseña al actor la biblioteca (`de`) entera.
    Buscar {
        de: IdJugador,
    },
    /// Olvida lo mirado de una biblioteca sin barajarla.
    DejarDeMirar {
        de: IdJugador,
    },
    DeArribaRevelada {
        si: bool,
    },
    Barajar,
    /// La mano al fondo, barajar y robar las mismas (o `robar`).
    Rueda {
        robar: Option<u32>,
    },
    // ── Campo entero ──
    EnderezarTodo,
    Organizar,
    CrearFicha {
        de: OrigenFicha,
        cantidad: u32,
        /// Como un montón (una sola carta con `cantidad`).
        monton: bool,
        pos: Option<Pos>,
    },
    // ── Mulligan ──
    Mulligan,
    QuedarseCon {
        al_fondo: Vec<IdCarta>,
    },
    // ── Jugadores ──
    Vida {
        jugador: IdJugador,
        delta: i32,
    },
    VidaATodos {
        delta: i32,
        solo_oponentes: bool,
    },
    Drenar {
        n: i32,
    },
    Extorsion {
        n: i32,
    },
    Veneno {
        jugador: IdJugador,
        delta: i32,
    },
    DanoComandante {
        jugador: IdJugador,
        comandante: IdCarta,
        delta: i32,
    },
    ContadorJugador {
        jugador: IdJugador,
        tipo: String,
        delta: i32,
    },
    Designar {
        tipo: Designacion,
        a: Option<IdJugador>,
    },
    DiaNoche {
        valor: Option<DiaNoche>,
    },
    Lanzamientos {
        comandante: IdCarta,
        delta: i32,
    },
    SinCosteExtra {
        comandante: IdCarta,
        si: bool,
    },
    RevelarMano {
        si: bool,
    },
    RevelarA {
        cartas: Vec<IdCarta>,
        a: Vec<IdJugador>,
    },
    JugarBocaAbajo {
        carta: IdCarta,
        pos: Option<Pos>,
    },
    Descartar {
        modo: ModoDescarte,
    },
    Eliminar {
        jugador: IdJugador,
        por: Option<IdJugador>,
    },
    Conceder,
    VidaFantasma {
        delta: i32,
    },
    DanoComandanteFantasma {
        comandante: IdCarta,
        delta: i32,
    },
    VerFantasma {
        si: bool,
    },
    Senal {
        senal: Option<Senal>,
    },
    // ── Turno ──
    PasarTurno,
    TurnoExtra {
        jugador: IdJugador,
    },
    Pausar {
        si: bool,
    },
    SaltarTurno {
        jugador: IdJugador,
    },
    Reiniciar,
    // ── Azar ──
    TirarDados {
        /// (caras, cantidad)
        dados: Vec<(u8, u8)>,
    },
    Moneda {
        cuantas: u8,
        hasta_perder: bool,
        pulgares: u8,
    },
    DadoPlanar,
    // ── Planechase / Archenemy ──
    Planeswalk,
    PonerEnMarcha,
    AbandonarPlan {
        carta: IdCarta,
    },
    // ── Pila ──
    Resolver {
        carta: IdCarta,
        como: ComoResolver,
    },
    ApilarDespues {
        carta: IdCarta,
    },
    // ── Montones ──
    Amontonar {
        cartas: Vec<IdCarta>,
    },
    SacarDelMonton {
        monton: IdCarta,
        n: u32,
    },
    CambiarMonton {
        monton: IdCarta,
        delta: i32,
    },
    Separar {
        monton: IdCarta,
    },
    // ── Exilio, cementerio ──
    Agrupar {
        cartas: Vec<IdCarta>,
        bajo: Option<IdCarta>,
    },
    AlAzarDe {
        zona: Zona,
        filtro: Filtro,
        a: Zona,
    },
    TodoAlFondoAlAzar {
        zona: Zona,
    },
    BarajarTodoDentro {
        zona: Zona,
    },
    ExiliarTodo {
        zona: Zona,
    },
    // ── Sparring ──
    IntensidadSparring {
        intensidad: Intensidad,
    },
    DecidirSparring {
        #[ts(type = "number")]
        reaccion: u64,
        decision: Decision,
        /// En un sacrificio, la que elige el jugador.
        eleccion: Option<IdCarta>,
    },
    // ── Comunicación ──
    Chat {
        texto: String,
    },
    Aviso {
        tipo: AvisoRapido,
    },
    // ── Fin ──
    Terminar {
        resultado: Resultado,
    },
    // ── Deshacer ──
    Deshacer {
        #[ts(type = "number | null")]
        hasta: Option<u64>,
    },
    Rehacer,
}

impl Accion {
    /// Lo que no cambia la partida ni se deshace (va al registro igual).
    pub fn es_comunicacion(&self) -> bool {
        matches!(self, Accion::Chat { .. } | Accion::Aviso { .. })
    }
}
