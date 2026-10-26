//! **La sesión de partida**: la partida viva de la app, con su host local.
//!
//! La sesión guarda el registro del motor (`mesa::Registro`), el generador
//! sembrado con entropía del sistema y el canal por el que la pantalla
//! recibe la vista y sus parches (`tauri::ipc::Channel`, en orden). Cada
//! intención se resuelve, se aplica y se apunta en el acto (son
//! milisegundos); la Fase E pone delante el transporte de la red sin tocar
//! esto.

use mesa::accion::{Accion, Evento, Intencion};
use mesa::formato::{Formato, Opciones, Variantes};
use mesa::modelo::Partida;
use mesa::preparar::{iniciar, Config, DeckDeMesa, JugadorNuevo};
use mesa::sparring::Intensidad;
use mesa::vista::{parche, vista, ParcheVista, VistaMesa};
use mesa::{Azar, Cambio, IdJugador, Mirador, ModoDeshacer, Rechazo, Registro};
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
}

/// La partida del motor para esta configuración.
fn partida(config: &ConfigMesa, mazos: &[DeckDeMesa], semilla: u64) -> AppResult<Partida> {
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
                equipo: None,
                mazo: m.clone(),
            })
            .collect(),
        equipos: Vec::new(),
        primero: None,
    };
    let id = format!("{semilla:016x}");
    let mut p = iniciar(id, cfg, semilla, ahora_ms())?;
    p.sparring.intensidad = config.intensidad;
    Ok(p)
}

impl Sesion {
    pub fn nueva(config: ConfigMesa, mazos: Vec<DeckDeMesa>) -> AppResult<Sesion> {
        let s = semilla();
        let p = partida(&config, &mazos, s)?;
        Ok(Sesion::con_registro(
            config,
            mazos,
            Registro::nuevo(p, ModoDeshacer::SinLimite),
        ))
    }

    /// Una sesión con un registro ya hecho (la partida guardada).
    pub fn con_registro(config: ConfigMesa, mazos: Vec<DeckDeMesa>, registro: Registro) -> Sesion {
        Sesion {
            config,
            mazos,
            registro,
            azar: Azar::new(semilla()),
            canal: None,
            yo: IdJugador(0),
            sin_guardar: 0,
            guardada_ms: ahora_ms(),
        }
    }

    pub fn partida(&self) -> &Partida {
        self.registro.partida()
    }

    pub fn mirador(&self) -> Mirador {
        Mirador::Jugador(self.yo)
    }

    pub fn vista(&self) -> VistaMesa {
        vista(self.partida(), self.mirador())
    }

    fn mandar(&self, m: MensajeMesa) {
        if let Some(c) = &self.canal {
            if let Err(e) = c.send(m) {
                tracing::warn!(error = %e, "no se pudo mandar a la mesa");
            }
        }
    }

    /// La pantalla se suscribe: recibe la vista entera y, desde ahí, los
    /// parches.
    /// La vista entera con el registro vigente.
    fn mensaje_vista(&self) -> MensajeMesa {
        let vigentes = self.registro.vigentes();
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
        let actor = como.unwrap_or(self.yo);
        let ahora = ahora_ms();
        // El fantasma mira la partida de antes y la de después (solo si está
        // encendido: la copia no sale gratis).
        let antes = (self.partida().sparring.intensidad != Intensidad::Apagado)
            .then(|| self.partida().clone());
        let hecho = if accion == Accion::Reiniciar {
            let p = partida(&self.config, &self.mazos, semilla())?;
            self.registro.reemplazar(actor, p, ahora)?
        } else {
            self.registro
                .procesar(&Intencion { actor, accion }, &mut self.azar, ahora)?
        };
        let reaccion = antes.and_then(|a| {
            mesa::sparring::reaccionar(&a, self.registro.partida(), &hecho.evento, &mut self.azar)
        });
        self.publicar(hecho);
        // La reacción, como un evento más de la mesa justo detrás.
        if let Some(reaccion) = reaccion {
            let h = self
                .registro
                .de_la_mesa(Cambio::Sparring { reaccion }, ahora)?;
            self.publicar(h);
        }
        Ok(())
    }

    /// Apunta un evento hecho y se lo manda a la pantalla.
    fn publicar(&mut self, hecho: mesa::registro::Hecho) {
        self.sin_guardar += 1;
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
            }],
            formato: Formato::Constructed,
            variantes: Variantes::default(),
            opciones: None,
            intensidad: Intensidad::Apagado,
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

    #[test]
    fn reiniciar_empieza_otra_con_la_misma_lista() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        s.accion(Accion::Reiniciar, None).unwrap();
        assert!(s.partida().mulligan.is_some());
        assert_eq!(s.partida().cartas.len(), 60);
    }
}
