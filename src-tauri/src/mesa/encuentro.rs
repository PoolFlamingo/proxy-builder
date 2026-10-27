//! **El encuentro en la sesión**: el marcador de «al mejor de 1, 3 o 5», el
//! cambio de cartas con el banquillo entre partidas y el paso a la
//! siguiente. Las cuentas son puras y viven en `mesa::encuentro` (el motor);
//! aquí solo se unen a la sesión. No cambian la `Partida` (así `Accion` y
//! `Cambio` siguen igual): la siguiente se abre con el mismo reemplazo que
//! «Otra partida», pero con los mazos ya cambiados y el marcador al día.

use mesa::encuentro::{cambiar, Encuentro, EstadoEncuentro};
use mesa::preparar::Seccion;
use mesa::IdJugador;
use serde::Serialize;
use ts_rs::TS;

use super::sesion::{ahora_ms, partida, semilla, RedSesion, Sesion};
use crate::error::{AppError, AppResult, ErrorCode};

/// Una carta del mazo o del banquillo, para el diálogo de cambios.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct CartaBanquillo {
    /// El id de la carta en el deck (lo que se manda para cambiarla).
    pub id: String,
    pub nombre: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaBanquillo {
    pub mazo: Vec<CartaBanquillo>,
    pub banquillo: Vec<CartaBanquillo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaEncuentro {
    pub estado: EstadoEncuentro,
    /// Quién empieza la siguiente (el que perdió), si se sabe.
    #[ts(optional)]
    pub primero: Option<IdJugador>,
    /// Quién empieza la partida de ahora por haber perdido la anterior
    /// (`None`: la primera, o tras unas tablas, que se sortea).
    #[ts(optional)]
    pub empieza: Option<IdJugador>,
}

fn rechazo(motivo: &str) -> AppError {
    AppError::new(ErrorCode::Rejected).with("motivo", motivo)
}

impl Sesion {
    /// Quién ganó la partida de ahora, si ya terminó (vacío = tablas).
    fn ganadores_de_ahora(&self) -> Option<Vec<IdJugador>> {
        self.partida().fin.as_ref().map(|f| f.ganadores.clone())
    }

    /// Cómo va el encuentro; `None` si se juega una partida suelta.
    pub fn estado_encuentro(&self) -> Option<VistaEncuentro> {
        let e = self.encuentro.as_ref()?;
        let n = self.partida().jugadores.len();
        let actual = self.ganadores_de_ahora();
        Some(VistaEncuentro {
            estado: e.estado(n, actual.as_deref()),
            primero: actual
                .as_deref()
                .and_then(|g| Encuentro::primero_siguiente(n, g)),
            empieza: self.primero(),
        })
    }

    /// Quién empieza la partida que se abre ahora: el que perdió la última
    /// cerrada. `None` en la primera (o con tablas): lo decide el motor.
    pub(super) fn primero(&self) -> Option<IdJugador> {
        let ultima = self.encuentro.as_ref()?.cerradas.last()?;
        Encuentro::primero_siguiente(self.partida().jugadores.len(), ultima)
    }

    /// Lo que hay que cumplir para tocar el banquillo o seguir: ser quien
    /// lleva la partida, que haya encuentro y que la partida ya haya
    /// terminado. Devuelve quién ganó.
    fn entre_partidas(&self) -> AppResult<Vec<IdJugador>> {
        // En red, quien lleva los mazos de todos es el anfitrión.
        if matches!(self.red, Some(RedSesion::Invitado(_))) {
            return Err(rechazo("invitado"));
        }
        if self.encuentro.is_none() {
            return Err(rechazo("sin_encuentro"));
        }
        self.ganadores_de_ahora()
            .ok_or_else(|| rechazo("sin_terminar"))
    }

    /// El mazo y el banquillo de un jugador, para elegir qué cambiar.
    pub fn banquillo(&self, jugador: IdJugador) -> AppResult<VistaBanquillo> {
        self.entre_partidas()?;
        let deck = self
            .mazos
            .get(usize::from(jugador.0))
            .ok_or_else(|| rechazo("no_existe"))?;
        let de = |s: Seccion| {
            let mut v: Vec<CartaBanquillo> = deck
                .cartas
                .iter()
                .filter(|c| c.seccion == s)
                .map(|c| CartaBanquillo {
                    id: c.carta_deck.clone(),
                    nombre: c.origen.nombre.clone(),
                })
                .collect();
            v.sort_by(|a, b| a.nombre.cmp(&b.nombre).then_with(|| a.id.cmp(&b.id)));
            v
        };
        Ok(VistaBanquillo {
            mazo: de(Seccion::Mazo),
            banquillo: de(Seccion::Banquillo),
        })
    }

    /// Cambia cartas entre el mazo y el banquillo de un jugador para la
    /// partida siguiente (el tamaño del mazo no cambia).
    pub fn cambiar_banquillo(
        &mut self,
        jugador: IdJugador,
        sale: Vec<String>,
        entra: Vec<String>,
    ) -> AppResult<()> {
        self.entre_partidas()?;
        let deck = self
            .mazos
            .get_mut(usize::from(jugador.0))
            .ok_or_else(|| rechazo("no_existe"))?;
        cambiar(deck, &sale, &entra).map_err(|e| rechazo("cambio").with("detalle", e.to_string()))
    }

    /// Cierra la partida de ahora, suma su resultado al marcador y abre la
    /// siguiente con los mazos como estén. Empieza el que perdió.
    pub fn siguiente_partida(&mut self) -> AppResult<()> {
        let ganadores = self.entre_partidas()?;
        let n = self.partida().jugadores.len();
        let terminado = self
            .encuentro
            .as_ref()
            .is_some_and(|e| e.estado(n, Some(&ganadores)).terminado);
        if terminado {
            return Err(rechazo("encuentro_terminado"));
        }
        let primero = Encuentro::primero_siguiente(n, &ganadores);
        let p = partida(&self.config, &self.mazos, semilla(), primero)?;
        let hecho = self.registro.reemplazar(self.yo, p, ahora_ms())?;
        // Hasta aquí nada ha cambiado si algo falló: ahora sí, el marcador.
        if let Some(e) = &mut self.encuentro {
            e.cerrar(ganadores);
        }
        self.publicar(hecho, None);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesa::sesion::tests::{config, mazo};
    use crate::mesa::sesion::JugadorMesa;
    use mesa::accion::Accion;
    use mesa::modelo::{ComoGano, RefZona, Resultado, Zona};
    use mesa::preparar::CartaDeMesa;

    /// Dos jugadores al mejor de tres, con 60 cartas y 3 de banquillo cada uno.
    fn encuentro_de_dos() -> Sesion {
        let mut c = config();
        c.al_mejor_de = Some(3);
        c.jugadores.push(JugadorMesa {
            deck_id: "d2".into(),
            nombre: "Rival".into(),
            color: "#2563eb".into(),
            lista: None,
            equipo: None,
        });
        let con_banquillo = || {
            let mut m = mazo();
            for i in 0..3 {
                let mut extra: CartaDeMesa = m.cartas[i].clone();
                extra.carta_deck = format!("banq{i}");
                extra.origen.carta_deck = format!("banq{i}");
                extra.origen.nombre = format!("Del banquillo {i}");
                extra.seccion = Seccion::Banquillo;
                m.cartas.push(extra);
            }
            m
        };
        let mut s = Sesion::nueva(c, vec![con_banquillo(), con_banquillo()]).unwrap();
        for j in 0..2 {
            s.accion(Accion::QuedarseCon { al_fondo: vec![] }, Some(IdJugador(j)))
                .unwrap();
        }
        s
    }

    fn gana(s: &mut Sesion, quien: u8) {
        s.accion(
            Accion::Terminar {
                resultado: Resultado {
                    ganadores: vec![IdJugador(quien)],
                    como: ComoGano::Combate,
                    nota: String::new(),
                },
            },
            None,
        )
        .unwrap();
    }

    fn en_juego(s: &Sesion, j: u8) -> Vec<String> {
        let p = s.partida();
        [Zona::Biblioteca, Zona::Mano]
            .into_iter()
            .flat_map(|z| p.zona(&RefZona::de(IdJugador(j), z)).to_vec())
            .map(|c| p.cartas[&c].origen.carta_deck.clone())
            .collect()
    }

    #[test]
    fn sin_terminar_no_se_toca_el_banquillo_ni_se_sigue() {
        let mut s = encuentro_de_dos();
        assert_eq!(
            s.banquillo(IdJugador(0)).unwrap_err().params["motivo"],
            "sin_terminar"
        );
        assert_eq!(
            s.siguiente_partida().unwrap_err().params["motivo"],
            "sin_terminar"
        );
    }

    #[test]
    fn una_partida_suelta_no_tiene_encuentro() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        assert!(s.estado_encuentro().is_none());
        assert_eq!(
            s.siguiente_partida().unwrap_err().params["motivo"],
            "sin_encuentro"
        );
    }

    #[test]
    fn al_mejor_de_tres_acaba_2_a_1_con_los_cambios_en_la_segunda() {
        let mut s = encuentro_de_dos();
        assert_eq!(s.estado_encuentro().unwrap().estado.marcador, vec![0, 0]);

        // Partida 1: gana el 0. Ya cuenta aunque no se haya cerrado.
        gana(&mut s, 0);
        let v = s.estado_encuentro().unwrap();
        assert_eq!(v.estado.marcador, vec![1, 0]);
        assert!(!v.estado.terminado);
        // Empieza el que perdió.
        assert_eq!(v.primero, Some(IdJugador(1)));

        // El 0 cambia dos cartas del mazo por dos del banquillo.
        let b = s.banquillo(IdJugador(0)).unwrap();
        assert_eq!((b.mazo.len(), b.banquillo.len()), (60, 3));
        let sale: Vec<String> = b.mazo.iter().take(2).map(|c| c.id.clone()).collect();
        let entra: Vec<String> = b.banquillo.iter().take(2).map(|c| c.id.clone()).collect();
        s.cambiar_banquillo(IdJugador(0), sale.clone(), entra.clone())
            .unwrap();

        s.siguiente_partida().unwrap();
        // La partida 2 empieza con el marcador, mano nueva y el que perdió primero.
        assert_eq!(s.estado_encuentro().unwrap().estado.marcador, vec![1, 0]);
        assert!(s.partida().fin.is_none());
        assert_eq!(s.partida().turno.activo, IdJugador(1));
        assert_eq!(s.estado_encuentro().unwrap().empieza, Some(IdJugador(1)));
        let en = en_juego(&s, 0);
        assert_eq!(en.len(), 60);
        for id in &entra {
            assert!(en.contains(id), "{id} debería estar en la biblioteca");
        }
        for id in &sale {
            assert!(!en.contains(id), "{id} debería haber salido");
        }
        // El otro no tocó nada.
        assert!(!en_juego(&s, 1).iter().any(|c| c.starts_with("banq")));

        // Partida 2: gana el 1 (1–1) y partida 3: gana el 0 → 2–1.
        for j in 0..2 {
            s.accion(Accion::QuedarseCon { al_fondo: vec![] }, Some(IdJugador(j)))
                .unwrap();
        }
        gana(&mut s, 1);
        assert_eq!(s.estado_encuentro().unwrap().estado.marcador, vec![1, 1]);
        s.siguiente_partida().unwrap();
        for j in 0..2 {
            s.accion(Accion::QuedarseCon { al_fondo: vec![] }, Some(IdJugador(j)))
                .unwrap();
        }
        gana(&mut s, 0);
        let fin = s.estado_encuentro().unwrap().estado;
        assert_eq!(fin.marcador, vec![2, 1]);
        assert!(fin.terminado);
        assert_eq!(fin.ganadores, vec![IdJugador(0)]);
        // Y ya no hay otra.
        assert_eq!(
            s.siguiente_partida().unwrap_err().params["motivo"],
            "encuentro_terminado"
        );
    }

    #[test]
    fn deshacer_el_terminar_no_deja_puntos() {
        let mut s = encuentro_de_dos();
        gana(&mut s, 0);
        assert_eq!(s.estado_encuentro().unwrap().estado.marcador, vec![1, 0]);
        s.accion(Accion::Deshacer { hasta: None }, None).unwrap();
        assert!(s.partida().fin.is_none());
        assert_eq!(s.estado_encuentro().unwrap().estado.marcador, vec![0, 0]);
    }

    #[test]
    fn un_cambio_que_no_encaja_no_toca_nada() {
        let mut s = encuentro_de_dos();
        gana(&mut s, 0);
        let antes = s.banquillo(IdJugador(0)).unwrap();
        let e = s
            .cambiar_banquillo(
                IdJugador(0),
                vec![antes.mazo[0].id.clone()],
                vec![antes.mazo[1].id.clone()],
            )
            .unwrap_err();
        assert_eq!(e.params["motivo"], "cambio");
        assert_eq!(s.banquillo(IdJugador(0)).unwrap(), antes);
    }

    #[test]
    fn el_marcador_se_guarda_con_la_partida() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = encuentro_de_dos();
        gana(&mut s, 0);
        s.siguiente_partida().unwrap();
        crate::mesa::guardada::guardar(dir.path(), &s).unwrap();
        let g = crate::mesa::guardada::leer(dir.path()).unwrap().unwrap();
        let otra = crate::mesa::guardada::sesion(g).unwrap();
        assert_eq!(otra.encuentro, s.encuentro);
        assert_eq!(otra.estado_encuentro().unwrap().estado.marcador, vec![1, 0]);
    }
}
