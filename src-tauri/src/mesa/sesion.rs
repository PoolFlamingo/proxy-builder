//! **La sesión de partida**: la partida viva de la app, con su host local.
//!
//! La sesión guarda el registro del motor (`mesa::Registro`), el generador
//! sembrado con entropía del sistema y el canal por el que la pantalla
//! recibe la vista y sus parches (`tauri::ipc::Channel`, en orden). Cada
//! intención se resuelve, se aplica y se apunta en el acto (son
//! milisegundos); la Fase E pone delante el transporte de la red sin tocar
//! esto.

use mesa::accion::{Accion, Evento, Intencion};
use mesa::encuentro::Encuentro;
use mesa::formato::{Formato, Opciones, Variantes};
use mesa::modelo::{Equipo, Partida};
use mesa::preparar::{iniciar, Config, DeckDeMesa, JugadorNuevo};
use mesa::sparring::Intensidad;
use mesa::vista::{parche, vista, ParcheVista, VistaMesa};
use mesa::{Azar, Cambio, IdEquipo, IdJugador, Mirador, ModoDeshacer, Rechazo, Registro};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use ts_rs::TS;

use crate::error::{AppError, AppResult, ErrorCode};

/// Un jugador de la partida que se empieza.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct JugadorMesa {
    pub deck_id: String,
    pub nombre: String,
    /// Color CSS.
    pub color: String,
    /// La lista para jugar del deck (su id), si se juega solo una parte.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub lista: Option<String>,
    /// Su equipo (de los de `ConfigMesa.equipos`), si se juega por equipos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub equipo: Option<IdEquipo>,
}

/// Lo que elige la pantalla al empezar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ConfigMesa {
    pub jugadores: Vec<JugadorMesa>,
    pub formato: Formato,
    pub variantes: Variantes,
    /// `None`: las de fábrica del formato.
    pub opciones: Option<Opciones>,
    pub intensidad: Intensidad,
    /// Los equipos de la partida (Fase G); los de 2HG los monta el motor solo.
    #[serde(default)]
    pub equipos: Vec<Equipo>,
    /// «Al mejor de» 1, 3 o 5 partidas, con banquillo entre ellas (Fase G);
    /// `None`, una partida suelta.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub al_mejor_de: Option<u8>,
    /// Quién empieza la primera partida (por asiento); `None`, se sortea.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub primero: Option<IdJugador>,
}

/// Lo que llega a la pantalla por el canal de la mesa.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "tipo", rename_all = "camelCase")]
#[ts(export)]
pub enum MensajeMesa {
    /// La vista entera (al suscribirse, al deshacer, al reiniciar), con el
    /// registro que sigue contando (los últimos `REGISTRO_EN_VISTA`): así la
    /// pantalla deja de enseñar lo deshecho y lo de la partida anterior.
    Vista {
        vista: Box<VistaMesa>,
        registro: Vec<Evento>,
    },
    /// Lo que cambió con un evento, y el evento (para el registro).
    Parche {
        parche: Box<ParcheVista>,
        evento: Box<Evento>,
    },
    /// La partida se ha cerrado.
    Cerrada,
    /// En red: el anfitrión no hizo una intención de esta pantalla (llega
    /// después, no como respuesta de `mesa_accion`).
    Rechazo { motivo: String },
}

/// El papel de la sesión en una partida en red.
pub enum RedSesion {
    /// Esta app hace de anfitrión: reparte cada evento a los demás.
    Anfitrion(std::sync::Arc<crate::red::anfitrion::Anfitrion>),
    /// Esta app es un invitado: manda sus intenciones al anfitrión y aplica
    /// los eventos que le llegan.
    Invitado(crate::red::invitado::Invitado),
}

/// Lo que pasó con un evento que llega del anfitrión.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recibido {
    Aplicado,
    /// Ya lo tenía (repetido) o es de una época vieja: se tira.
    Tirado,
    /// Falta alguno antes: hay que pedirlos desde este `seq`.
    Hueco(u64),
    /// No encaja con la copia (o la huella no coincide): hay que pedir la
    /// partida entera.
    Desfase,
}

impl From<Rechazo> for AppError {
    fn from(r: Rechazo) -> Self {
        AppError::new(ErrorCode::Rejected).with("motivo", format!("{r:?}"))
    }
}

/// Cuántos eventos del registro van con la vista entera (los que guarda la
/// pantalla, `MAX_REGISTRO` en `mesa-store.ts`).
const REGISTRO_EN_VISTA: usize = 500;

/// Una semilla de la entropía del sistema.
pub fn semilla() -> u64 {
    let mut b = [0u8; 8];
    if getrandom::fill(&mut b).is_err() {
        // Sin entropía (no pasa en la práctica): el reloj.
        return crate::store::ahora_ms() as u64;
    }
    u64::from_le_bytes(b)
}

pub fn ahora_ms() -> u64 {
    crate::store::ahora_ms().max(0) as u64
}

pub struct Sesion {
    pub config: ConfigMesa,
    /// Los decks preparados, para reiniciar con la misma lista.
    pub mazos: Vec<DeckDeMesa>,
    pub registro: Registro,
    azar: Azar,
    canal: Option<Channel<MensajeMesa>>,
    /// Quién mira en esta pantalla.
    pub yo: IdJugador,
    /// Eventos desde la última vez que se guardó.
    pub sin_guardar: u32,
    /// Cuándo se guardó por última vez (ms de reloj), para no dejar
    /// jugadas sin guardar mucho rato aunque sean pocas.
    pub guardada_ms: u64,
    /// En red, el papel de esta app; `None`, solitario.
    pub red: Option<RedSesion>,
    /// El marcador si se juega al mejor de varias (`mesa/encuentro.rs`).
    pub encuentro: Option<Encuentro>,
    /// Si se está viendo la repetición de una partida (`mesa/repeticion.rs`).
    pub repeticion: Option<super::repeticion::Repeticion>,
}

/// La partida del motor para esta configuración.
pub(super) fn partida(
    config: &ConfigMesa,
    mazos: &[DeckDeMesa],
    semilla: u64,
    primero: Option<IdJugador>,
) -> AppResult<Partida> {
    if config.jugadores.len() != mazos.len() {
        return Err(AppError::new(ErrorCode::InvalidInput));
    }
    let opciones = config.opciones.clone().unwrap_or_else(|| {
        Opciones::de_fabrica(config.formato, &config.variantes, config.jugadores.len())
    });
    let cfg = Config {
        formato: config.formato,
        variantes: config.variantes.clone(),
        opciones,
        jugadores: config
            .jugadores
            .iter()
            .zip(mazos)
            .map(|(j, m)| JugadorNuevo {
                nombre: j.nombre.clone(),
                color: j.color.clone(),
                equipo: j.equipo,
                mazo: m.clone(),
            })
            .collect(),
        equipos: config.equipos.clone(),
        primero,
    };
    let id = format!("{semilla:016x}");
    let mut p = iniciar(id, cfg, semilla, ahora_ms())?;
    p.sparring.intensidad = config.intensidad;
    Ok(p)
}

impl Sesion {
    pub fn nueva(config: ConfigMesa, mazos: Vec<DeckDeMesa>) -> AppResult<Sesion> {
        let s = semilla();
        let p = partida(&config, &mazos, s, config.primero)?;
        Ok(Sesion::con_registro(
            config,
            mazos,
            Registro::nuevo(p, ModoDeshacer::SinLimite),
        ))
    }

    /// Una sesión con un registro ya hecho (la partida guardada).
    pub fn con_registro(config: ConfigMesa, mazos: Vec<DeckDeMesa>, registro: Registro) -> Sesion {
        let encuentro = config.al_mejor_de.and_then(|n| Encuentro::nuevo(n).ok());
        Sesion {
            encuentro,
            config,
            mazos,
            registro,
            azar: Azar::new(semilla()),
            canal: None,
            yo: IdJugador(0),
            sin_guardar: 0,
            guardada_ms: ahora_ms(),
            red: None,
            repeticion: None,
        }
    }

    /// La partida en red que empieza el anfitrión (esta app es el jugador 0).
    /// Deshacer, solo la última acción propia y si nadie ha hecho nada después.
    pub fn en_red(
        config: ConfigMesa,
        mazos: Vec<DeckDeMesa>,
        anfitrion: std::sync::Arc<crate::red::anfitrion::Anfitrion>,
    ) -> AppResult<Sesion> {
        let p = partida(&config, &mazos, semilla(), config.primero)?;
        let mut s = Sesion::con_registro(
            config,
            mazos,
            Registro::nuevo(p, ModoDeshacer::UltimaPropia),
        );
        s.red = Some(RedSesion::Anfitrion(anfitrion));
        Ok(s)
    }

    /// La partida que recibe un invitado (o el anfitrión nuevo, tras migrar).
    pub fn invitada(
        partida: Partida,
        yo: IdJugador,
        inv: crate::red::invitado::Invitado,
    ) -> Sesion {
        let config = ConfigMesa {
            jugadores: partida
                .jugadores
                .iter()
                .map(|j| JugadorMesa {
                    deck_id: String::new(),
                    nombre: j.nombre.clone(),
                    color: j.color.clone(),
                    lista: None,
                    equipo: j.equipo,
                })
                .collect(),
            formato: partida.formato,
            variantes: partida.variantes.clone(),
            opciones: Some(partida.opciones.clone()),
            intensidad: Intensidad::Apagado,
            equipos: partida.equipos.clone(),
            al_mejor_de: None,
            primero: None,
        };
        let mut s = Sesion::con_registro(
            config,
            Vec::new(),
            Registro::nuevo(partida, ModoDeshacer::UltimaPropia),
        );
        s.yo = yo;
        s.red = Some(RedSesion::Invitado(inv));
        s
    }

    /// Si la partida se guarda en disco (la del invitado la tiene el anfitrión).
    pub fn guardable(&self) -> bool {
        self.repeticion.is_none() && !matches!(self.red, Some(RedSesion::Invitado(_)))
    }

    pub fn anfitrion(&self) -> Option<&std::sync::Arc<crate::red::anfitrion::Anfitrion>> {
        match &self.red {
            Some(RedSesion::Anfitrion(a)) => Some(a),
            _ => None,
        }
    }

    /// La partida que se ve: la de ahora o, en una repetición, la del punto
    /// que se está mirando.
    pub fn partida(&self) -> &Partida {
        match &self.repeticion {
            Some(r) => &r.punto,
            None => self.registro.partida(),
        }
    }

    pub fn mirador(&self) -> Mirador {
        match &self.repeticion {
            Some(r) => r.como.map_or(Mirador::Todo, Mirador::Jugador),
            None => Mirador::Jugador(self.yo),
        }
    }

    pub fn vista(&self) -> VistaMesa {
        vista(self.partida(), self.mirador())
    }

    pub(super) fn mandar(&self, m: MensajeMesa) {
        if let Some(c) = &self.canal {
            if let Err(e) = c.send(m) {
                tracing::warn!(error = %e, "no se pudo mandar a la mesa");
            }
        }
    }

    /// La pantalla se suscribe: recibe la vista entera y, desde ahí, los
    /// parches.
    /// La vista entera con el registro vigente.
    pub(super) fn mensaje_vista(&self) -> MensajeMesa {
        let mut vigentes = self.registro.vigentes();
        if let Some(r) = &self.repeticion {
            vigentes.truncate(r.pos as usize);
        }
        let desde = vigentes.len().saturating_sub(REGISTRO_EN_VISTA);
        MensajeMesa::Vista {
            vista: Box::new(self.vista()),
            registro: vigentes[desde..].iter().map(|e| (*e).clone()).collect(),
        }
    }

    pub fn suscribir(&mut self, canal: Channel<MensajeMesa>) {
        self.canal = Some(canal);
        self.mandar(self.mensaje_vista());
    }

    pub fn cerrar(&mut self) {
        self.mandar(MensajeMesa::Cerrada);
        self.canal = None;
    }

    /// Una acción de la pantalla. `como`: a nombre de otro jugador de esta
    /// sesión local (partida local de varios, Fase G).
    pub fn accion(&mut self, accion: Accion, como: Option<IdJugador>) -> AppResult<()> {
        if self.repeticion.is_some() {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "repeticion"));
        }
        // El invitado no aplica nada: se lo pide al anfitrión (sin predecir,
        // decisión D-E2 del plan) y espera el evento.
        if let Some(RedSesion::Invitado(inv)) = &mut self.red {
            return inv.enviar(accion);
        }
        // Actuar a nombre de otro solo tiene sentido en una partida local
        // (quien manda el portátil); en red cada uno es quien es.
        if self.red.is_some() && como.is_some_and(|c| c != self.yo) {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "como"));
        }
        let actor = como.unwrap_or(self.yo);
        self.hacer(actor, accion, None)
    }

    /// Pasa a mirar (y a jugar) como otro jugador de la partida local: la
    /// vista se vuelve a calcular con lo que ese jugador puede ver, su mano
    /// incluida, y se manda entera. En red no se puede.
    pub fn cambiar_yo(&mut self, jugador: IdJugador) -> AppResult<()> {
        if self.red.is_some() {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "red"));
        }
        if self.partida().jugador(jugador).is_none() {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "no_existe"));
        }
        if jugador != self.yo {
            self.yo = jugador;
            self.mandar(self.mensaje_vista());
        }
        Ok(())
    }

    /// Una intención de un jugador remoto (en el anfitrión). `id` es el suyo,
    /// para confirmársela con el evento.
    pub fn accion_remota(&mut self, jugador: IdJugador, id: u64, accion: Accion) -> AppResult<()> {
        self.hacer(jugador, accion, Some((jugador, id)))
    }

    /// El anfitrión hace algo en nombre de otro jugador: darlo por eliminado
    /// al expulsarlo o quedarse con su mano si se fue durante el mulligan
    /// (G8). Solo vale en el anfitrión de una partida en red.
    pub fn suplir(&mut self, actor: IdJugador, accion: Accion) -> AppResult<()> {
        if !matches!(self.red, Some(RedSesion::Anfitrion(_))) {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "no_anfitrion"));
        }
        self.hacer(actor, accion, None)
    }

    fn hacer(
        &mut self,
        actor: IdJugador,
        accion: Accion,
        origen: Option<(IdJugador, u64)>,
    ) -> AppResult<()> {
        if self.repeticion.is_some() && !accion.es_comunicacion() {
            return Err(AppError::new(ErrorCode::Rejected).with("motivo", "repeticion"));
        }
        let ahora = ahora_ms();
        // El fantasma mira la partida de antes y la de después (solo si está
        // encendido: la copia no sale gratis).
        let antes = (self.partida().sparring.intensidad != Intensidad::Apagado)
            .then(|| self.partida().clone());
        let hecho = if accion == Accion::Reiniciar {
            let primero = self.primero();
            let p = partida(&self.config, &self.mazos, semilla(), primero)?;
            self.registro.reemplazar(actor, p, ahora)?
        } else {
            self.registro
                .procesar(&Intencion { actor, accion }, &mut self.azar, ahora)?
        };
        let reaccion = antes.and_then(|a| {
            mesa::sparring::reaccionar(&a, self.registro.partida(), &hecho.evento, &mut self.azar)
        });
        self.publicar(hecho, origen);
        // La reacción, como un evento más de la mesa justo detrás.
        if let Some(reaccion) = reaccion {
            let h = self
                .registro
                .de_la_mesa(Cambio::Sparring { reaccion }, ahora)?;
            self.publicar(h, None);
        }
        Ok(())
    }

    /// En el invitado: un evento del anfitrión, en orden de `seq`.
    pub fn recibir(&mut self, ev: Evento, huella: Option<&str>) -> Recibido {
        let ultimo = self.partida().seq;
        if ev.seq <= ultimo {
            return Recibido::Tirado;
        }
        if ev.seq > ultimo + 1 {
            return Recibido::Hueco(ultimo + 1);
        }
        let Ok(tocado) = self.registro.recibir(ev.clone()) else {
            return Recibido::Desfase;
        };
        if huella.is_some_and(|h| h != mesa::huella(self.partida())) {
            tracing::warn!(seq = ev.seq, "la partida no coincide con la del anfitrión");
            return Recibido::Desfase;
        }
        self.publicar(mesa::registro::Hecho { evento: ev, tocado }, None);
        Recibido::Aplicado
    }

    /// En el invitado: la partida entera (al volver o si se desfasó).
    pub fn recibir_partida(&mut self, partida: Partida, yo: IdJugador) {
        self.registro = Registro::nuevo(partida, ModoDeshacer::UltimaPropia);
        self.yo = yo;
        self.mandar(self.mensaje_vista());
    }

    /// En el invitado: el anfitrión no hizo la intención `id`.
    pub fn rechazada(&mut self, id: Option<u64>, motivo: String) {
        if let (Some(RedSesion::Invitado(inv)), Some(id)) = (&mut self.red, id) {
            inv.confirmada(id);
        }
        self.mandar(MensajeMesa::Rechazo { motivo });
    }

    /// En el invitado: el evento que confirma la intención `id`.
    pub fn confirmada(&mut self, id: u64) {
        if let Some(RedSesion::Invitado(inv)) = &mut self.red {
            inv.confirmada(id);
        }
    }

    /// Apunta un evento hecho, se lo manda a la pantalla y, en el anfitrión,
    /// a los demás (`origen`: de qué intención remota viene).
    pub(super) fn publicar(
        &mut self,
        hecho: mesa::registro::Hecho,
        origen: Option<(IdJugador, u64)>,
    ) {
        if let Some(RedSesion::Anfitrion(a)) = &self.red {
            a.difundir(self.registro.partida(), &hecho, origen);
        }
        self.sin_guardar += 1;
        // Al terminar la partida, al historial (H2).
        if matches!(
            &hecho.evento.cambio,
            Cambio::Accion {
                accion: Accion::Terminar { .. },
                ..
            }
        ) {
            if let Some(n) = super::historial::de_sesion(self) {
                super::historial::avisar(n);
            }
        }
        // Mientras se repite, la pantalla ve otro momento: lo nuevo espera.
        if self.repeticion.is_some() {
            return;
        }
        if matches!(hecho.evento.cambio, Cambio::Restaurar { .. }) {
            self.mandar(self.mensaje_vista());
        } else {
            let pa = parche(self.partida(), &hecho.tocado, self.mirador());
            self.mandar(MensajeMesa::Parche {
                parche: Box::new(pa),
                evento: Box::new(hecho.evento),
            });
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mesa::modelo::{InfoCarta, OrigenCarta, RefImagen};
    use mesa::preparar::{CartaDeMesa, Seccion};

    pub fn mazo() -> DeckDeMesa {
        let carta = |i: usize, tipo: &str| CartaDeMesa {
            carta_deck: format!("c{i}"),
            seccion: Seccion::Mazo,
            origen: OrigenCarta {
                carta_deck: format!("c{i}"),
                nombre: format!("Carta {i}"),
                caras: vec![RefImagen::Generica],
                dorso: RefImagen::Generica,
                scryfall_id: None,
                info: InfoCarta {
                    tipo: Some(tipo.into()),
                    es_tierra: tipo.contains("Land"),
                    ..InfoCarta::default()
                },
            },
        };
        DeckDeMesa {
            nombre: "Prueba".into(),
            cartas: (0..60)
                .map(|i| carta(i, if i % 3 == 0 { "Land" } else { "Creature" }))
                .collect(),
            tapete: None,
            dorso: RefImagen::Generica,
        }
    }

    pub fn config() -> ConfigMesa {
        ConfigMesa {
            jugadores: vec![JugadorMesa {
                deck_id: "d".into(),
                nombre: "Yo".into(),
                color: "#e11d48".into(),
                lista: None,
                equipo: None,
            }],
            formato: Formato::Constructed,
            variantes: Variantes::default(),
            opciones: None,
            intensidad: Intensidad::Apagado,
            equipos: Vec::new(),
            al_mejor_de: None,
            primero: None,
        }
    }

    #[test]
    fn cincuenta_acciones_dan_la_misma_vista_por_parches() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        // La vista de la pantalla al suscribirse, y los parches que le irían
        // llegando por el canal.
        let mut v = s.vista();
        let mut azar = Azar::new(5);
        let mut hechas = 0;
        while hechas < 50 {
            let accion = match azar.menor_que(4) {
                0 => Accion::Robar { n: 1 },
                1 => Accion::Moler { n: 1 },
                2 => Accion::PasarTurno,
                _ => Accion::Vida {
                    jugador: IdJugador(0),
                    delta: -1,
                },
            };
            let antes = s.registro.eventos.len();
            if s.accion(accion, None).is_ok() {
                hechas += 1;
                let ev = s.registro.eventos[antes].clone();
                let mut p = s.registro.en(ev.seq - 1).unwrap();
                let t = mesa::aplicar(&mut p, &ev).unwrap();
                mesa::vista::aplicar_parche(&mut v, &parche(s.partida(), &t, s.mirador()));
            }
        }
        assert_eq!(v, s.vista());
    }

    /// Con una partida abierta la app está ocupada: la actualización
    /// (`update_install`) devuelve `Busy` y no reinicia a media partida.
    #[test]
    fn con_partida_la_app_esta_ocupada() {
        let dir = tempfile::tempdir().unwrap();
        let st = crate::state::AppState::open(crate::state::Paths {
            data: dir.path().join("data"),
            cache: dir.path().join("cache"),
        })
        .unwrap();
        assert!(!st.ocupado());
        *st.mesa.lock().unwrap() = Some(Sesion::nueva(config(), vec![mazo()]).unwrap());
        assert!(st.ocupado());
    }

    #[test]
    fn un_rechazo_llega_como_error_con_su_motivo() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        let e = s.accion(Accion::PasarTurno, None).unwrap_err();
        assert_eq!(e.code, ErrorCode::Rejected);
        assert_eq!(e.params["motivo"], "NoSePuede");
    }

    /// Con «Despiadado», jugar criaturas en la ronda 6 acaba trayendo una
    /// reacción; «Resolverla» quita la carta y deshacer la devuelve.
    #[test]
    fn el_fantasma_reacciona_y_deshacer_lo_devuelve_todo() {
        use mesa::accion::{Destino, Sitio, ZonaDestino};
        use mesa::modelo::{Pos, RefZona, Zona};
        use mesa::sparring::{Decision, TipoReaccion};
        let mut c = config();
        c.intensidad = Intensidad::Despiadado;
        let mut s = Sesion::nueva(c, vec![mazo()]).unwrap();
        s.azar = Azar::new(42);
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        for _ in 0..5 {
            s.accion(Accion::PasarTurno, None).unwrap();
        }
        assert_eq!(s.partida().turno.ronda, 6);
        let yo = s.yo;
        let campo = RefZona::de(yo, Zona::Campo);
        let biblioteca = RefZona::de(yo, Zona::Biblioteca);
        for _ in 0..30 {
            // Una criatura de la biblioteca, directa al campo.
            let Some(id) =
                s.partida().zona(&biblioteca).iter().copied().find(|c| {
                    s.partida().cartas[c].origen.info.tipo.as_deref() == Some("Creature")
                })
            else {
                break;
            };
            s.accion(
                Accion::Mover {
                    cartas: vec![id],
                    a: Destino {
                        zona: ZonaDestino::De(Zona::Campo),
                        jugador: Some(yo),
                        sitio: Sitio::EnCampo(Pos::nueva(5_000, 5_000)),
                    },
                    como: None,
                },
                None,
            )
            .unwrap();
            let Some(r) = s.partida().sparring.reacciones.last().cloned() else {
                continue;
            };
            // Llegó como evento de la mesa, justo detrás.
            let ultimo = s.registro.eventos.last().unwrap();
            assert_eq!(ultimo.actor, mesa::accion::Actor::Mesa);
            assert_eq!(ultimo.seq, r.reaccion.id);
            let antes = s.partida().clone();
            let eleccion = (r.reaccion.tipo == TipoReaccion::Sacrificio).then_some(id);
            s.accion(
                Accion::DecidirSparring {
                    reaccion: r.reaccion.id,
                    decision: Decision::Aceptar,
                    eleccion,
                },
                None,
            )
            .unwrap();
            let quitada = r.reaccion.objetivo.first().copied().unwrap_or(id);
            if r.reaccion.tipo != TipoReaccion::Barrer {
                assert!(!s.partida().zona(&campo).contains(&quitada));
            }
            // Deshacer la decisión lo devuelve todo.
            s.accion(Accion::Deshacer { hasta: None }, None).unwrap();
            assert_eq!(s.partida().cartas, antes.cartas);
            assert_eq!(s.partida().zonas, antes.zonas);
            return;
        }
        panic!("el fantasma no reaccionó");
    }

    /// Dos jugadores en el mismo equipo (partida local de la Fase G).
    fn local_de_dos() -> Sesion {
        let mut c = config();
        c.jugadores.push(JugadorMesa {
            deck_id: "d2".into(),
            nombre: "Rival".into(),
            color: "#2563eb".into(),
            lista: None,
            equipo: None,
        });
        Sesion::nueva(c, vec![mazo(), mazo()]).unwrap()
    }

    fn mano_visible(s: &Sesion, de: u8) -> Option<Vec<mesa::IdCarta>> {
        let z = mesa::modelo::RefZona::de(IdJugador(de), mesa::modelo::Zona::Mano);
        s.vista()
            .zonas
            .into_iter()
            .find(|v| v.zona == z)
            .and_then(|v| v.cartas)
    }

    #[test]
    fn en_local_mirar_como_otro_enseña_su_mano_y_oculta_la_del_anterior() {
        let mut s = local_de_dos();
        assert!(mano_visible(&s, 0).is_some_and(|m| m.len() == 7));
        assert!(mano_visible(&s, 1).is_none());
        s.cambiar_yo(IdJugador(1)).unwrap();
        assert_eq!(s.vista().yo, Some(IdJugador(1)));
        assert!(mano_visible(&s, 1).is_some_and(|m| m.len() == 7));
        assert!(mano_visible(&s, 0).is_none());
        // Un jugador que no existe no cambia nada.
        assert!(s.cambiar_yo(IdJugador(7)).is_err());
        assert_eq!(s.yo, IdJugador(1));
        // Y las acciones salen a nombre del que mira.
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        assert!(s
            .partida()
            .mulligan
            .as_ref()
            .is_some_and(|m| m.decididos == vec![IdJugador(1)]));
    }

    #[test]
    fn en_red_no_se_puede_mirar_ni_actuar_como_otro() {
        let mut s = local_de_dos();
        s.red = Some(RedSesion::Anfitrion(
            crate::red::anfitrion::Anfitrion::nuevo("A".into(), "s".into()),
        ));
        let e = s.cambiar_yo(IdJugador(1)).unwrap_err();
        assert_eq!(e.code, ErrorCode::Rejected);
        let e = s
            .accion(Accion::QuedarseCon { al_fondo: vec![] }, Some(IdJugador(1)))
            .unwrap_err();
        assert_eq!(e.code, ErrorCode::Rejected);
    }

    #[test]
    fn reiniciar_empieza_otra_con_la_misma_lista() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        s.accion(Accion::Reiniciar, None).unwrap();
        assert!(s.partida().mulligan.is_some());
        assert_eq!(s.partida().cartas.len(), 60);
    }

    #[test]
    fn empieza_quien_se_eligio_y_reiniciar_lo_respeta() {
        let mut s = local_de_dos();
        s.config.primero = Some(IdJugador(1));
        s.accion(Accion::Reiniciar, None).unwrap();
        assert_eq!(s.partida().turno.primero, IdJugador(1));
        assert_eq!(s.partida().turno.activo, IdJugador(1));
        // Una sesión nueva con la elección hecha empieza igual.
        let mut c = config();
        c.jugadores.push(s.config.jugadores[1].clone());
        c.primero = Some(IdJugador(1));
        let n = Sesion::nueva(c, vec![mazo(), mazo()]).unwrap();
        assert_eq!(n.partida().turno.primero, IdJugador(1));
    }
}
