//! **La repetición de una partida** (H3): la mesa en modo «solo mirar» que
//! va a cualquier momento de una partida terminada. Es la misma sesión (el
//! mismo canal, las mismas imágenes) con otra partida delante: la que
//! calcula `Registro::en` para el punto que se mira, y la vista de quien se
//! elija (un jugador o todo, porque la partida ya terminó).
//!
//! Se entra de dos maneras: en el sitio, al acabar una partida (`entrar`), o
//! abriendo una guardada del historial (`desde_guardada`, en una sesión que
//! solo existe para verla).

use mesa::{aplicar, Cambio, IdJugador, Partida};
use serde::Serialize;
use ts_rs::TS;

use super::guardada::{self, PartidaGuardada};
use super::sesion::Sesion;
use crate::error::{AppError, AppResult, ErrorCode};

/// El estado de la repetición dentro de la sesión.
pub struct Repeticion {
    /// La partida en el punto que se mira.
    pub punto: Partida,
    /// Cuántos eventos vigentes se han aplicado (0 = antes del primero).
    pub pos: u32,
    /// Quién mira: un jugador, o todo (`None`).
    pub como: Option<IdJugador>,
    /// Los `seq` de los eventos vigentes, en orden.
    seqs: Vec<u64>,
    /// Dónde empieza cada turno.
    hitos: Vec<Hito>,
    /// La sesión se abrió solo para verla (del historial): al salir se cierra.
    pub desde_historial: bool,
}

/// Dónde empieza un turno en la línea de tiempo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Hito {
    /// Posición (eventos aplicados) justo después de empezar el turno.
    pub pos: u32,
    pub turno: u32,
    pub ronda: u32,
    pub activo: IdJugador,
    #[ts(type = "number")]
    pub t_ms: u64,
}

/// Lo que la pantalla necesita para pintar la línea de tiempo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct VistaRepeticion {
    /// Eventos que cuentan.
    pub total: u32,
    pub pos: u32,
    pub como: Option<IdJugador>,
    pub hitos: Vec<Hito>,
    /// La hora (del anfitrión) de cada evento, para el ritmo de la reproducción.
    #[ts(type = "Array<number>")]
    pub t_ms: Vec<u64>,
    pub desde_historial: bool,
}

impl Sesion {
    /// Una sesión solo para ver una partida guardada, ya en su último punto.
    pub fn desde_guardada(g: PartidaGuardada) -> AppResult<Sesion> {
        let mut s = guardada::sesion(g)?;
        s.entrar_repeticion(true)?;
        Ok(s)
    }

    /// Pasa esta sesión a repetición (si no lo estaba), en el último punto.
    pub fn entrar_repeticion(&mut self, desde_historial: bool) -> AppResult<VistaRepeticion> {
        if self.repeticion.is_none() {
            let (seqs, hitos) = indice(self)?;
            let pos = seqs.len() as u32;
            let punto = self.registro.partida().clone();
            self.repeticion = Some(Repeticion {
                punto,
                pos,
                como: None,
                seqs,
                hitos,
                desde_historial,
            });
        }
        self.mandar(self.mensaje_vista());
        self.vista_repeticion()
    }

    /// Va al punto `pos` (eventos aplicados) viendo la partida como `como`.
    pub fn repeticion_ir(
        &mut self,
        pos: u32,
        como: Option<IdJugador>,
    ) -> AppResult<VistaRepeticion> {
        let fuera = || AppError::new(ErrorCode::Rejected).with("motivo", "no_es_repeticion");
        let r = self.repeticion.as_ref().ok_or_else(fuera)?;
        let pos = pos.min(r.seqs.len() as u32);
        if let Some(c) = como {
            if self.registro.partida().jugador(c).is_none() {
                return Err(AppError::new(ErrorCode::Rejected).with("motivo", "no_existe"));
            }
        }
        let seq = match pos {
            0 => r.seqs.first().map_or(0, |s| s.saturating_sub(1)),
            n => r.seqs[n as usize - 1],
        };
        let punto = self
            .registro
            .en(seq.max(self.registro.inicial.seq))
            .map_err(|e| AppError::new(ErrorCode::Rejected).with("motivo", e.to_string()))?;
        if let Some(r) = self.repeticion.as_mut() {
            r.punto = punto;
            r.pos = pos;
            r.como = como;
        }
        self.mandar(self.mensaje_vista());
        self.vista_repeticion()
    }

    /// Deja la repetición y vuelve a la partida tal como terminó.
    pub fn salir_de_repeticion(&mut self) {
        if self.repeticion.take().is_some() {
            self.mandar(self.mensaje_vista());
        }
    }

    pub fn vista_repeticion(&self) -> AppResult<VistaRepeticion> {
        let r = self
            .repeticion
            .as_ref()
            .ok_or_else(|| AppError::new(ErrorCode::Rejected).with("motivo", "no_es_repeticion"))?;
        let vigentes = self.registro.vigentes();
        Ok(VistaRepeticion {
            total: r.seqs.len() as u32,
            pos: r.pos,
            como: r.como,
            hitos: r.hitos.clone(),
            t_ms: vigentes.iter().map(|e| e.t_ms).collect(),
            desde_historial: r.desde_historial,
        })
    }
}

/// Los eventos que cuentan y dónde empieza cada turno: se recorre el
/// registro entero una vez, aplicando también lo deshecho para que cada
/// estado sea el de verdad.
fn indice(s: &Sesion) -> AppResult<(Vec<u64>, Vec<Hito>)> {
    let vigentes = s.registro.vigentes();
    let seqs: Vec<u64> = vigentes.iter().map(|e| e.seq).collect();
    let mut p = s.registro.inicial.clone();
    let mut ultimo = p.turno.numero;
    let mut hitos = Vec::new();
    let mut pos = 0u32;
    let mut siguiente = 0usize;
    for ev in &s.registro.eventos {
        aplicar(&mut p, ev).map_err(|e| {
            AppError::new(ErrorCode::Internal).with("detalle", format!("repetición: {e}"))
        })?;
        if siguiente < seqs.len() && seqs[siguiente] == ev.seq {
            siguiente += 1;
            pos += 1;
            if !matches!(ev.cambio, Cambio::Restaurar { .. }) && p.turno.numero != ultimo {
                hitos.push(Hito {
                    pos,
                    turno: p.turno.numero,
                    ronda: p.turno.ronda,
                    activo: p.turno.activo,
                    t_ms: ev.t_ms,
                });
            }
        }
        ultimo = p.turno.numero;
    }
    Ok((seqs, hitos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesa::sesion::tests::{config, mazo};
    use mesa::Accion;

    fn jugada() -> Sesion {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        for _ in 0..6 {
            s.accion(Accion::Robar { n: 1 }, None).unwrap();
            s.accion(Accion::PasarTurno, None).unwrap();
        }
        s
    }

    #[test]
    fn ir_a_cualquier_punto_da_la_huella_de_ese_momento() {
        let mut s = jugada();
        let total = s.registro.vigentes().len() as u32;
        let v = s.entrar_repeticion(false).unwrap();
        assert_eq!(v.total, total);
        assert_eq!(v.pos, total);
        let final_ = mesa::huella(s.partida());
        for pos in [0, 1, 5, total / 2, total - 1, total] {
            s.repeticion_ir(pos, None).unwrap();
            let seq = if pos == 0 {
                s.registro.vigentes()[0].seq - 1
            } else {
                s.registro.vigentes()[pos as usize - 1].seq
            };
            let esperada = s.registro.en(seq).unwrap();
            assert_eq!(
                mesa::huella(s.partida()),
                mesa::huella(&esperada),
                "pos {pos}"
            );
        }
        s.repeticion_ir(total, None).unwrap();
        assert_eq!(mesa::huella(s.partida()), final_);
    }

    #[test]
    fn hay_un_hito_por_turno_y_no_se_juega_en_repeticion() {
        let mut s = jugada();
        let v = s.entrar_repeticion(false).unwrap();
        assert!(v.hitos.len() >= 6, "{:?}", v.hitos);
        assert!(v.hitos.windows(2).all(|w| w[0].pos < w[1].pos));
        let e = s.accion(Accion::Robar { n: 1 }, None).unwrap_err();
        assert_eq!(e.params["motivo"], "repeticion");
        assert!(!s.guardable());
        // Salir deja la partida como terminó.
        s.repeticion_ir(0, None).unwrap();
        s.salir_de_repeticion();
        assert!(s.repeticion.is_none());
        assert_eq!(s.partida().turno.numero, s.registro.partida().turno.numero);
    }

    #[test]
    fn ver_como_un_jugador_o_todo_y_uno_que_no_existe() {
        let mut s = jugada();
        s.entrar_repeticion(false).unwrap();
        s.repeticion_ir(3, Some(IdJugador(0))).unwrap();
        assert_eq!(s.vista().yo, Some(IdJugador(0)));
        s.repeticion_ir(3, None).unwrap();
        assert_eq!(s.vista().yo, None);
        assert!(s.repeticion_ir(3, Some(IdJugador(9))).is_err());
    }

    #[test]
    fn una_guardada_se_abre_en_su_ultimo_punto() {
        let s = jugada();
        let g = guardada::de_sesion(&s);
        let r = Sesion::desde_guardada(g).unwrap();
        assert!(r.repeticion.as_ref().is_some_and(|x| x.desde_historial));
        assert_eq!(mesa::huella(r.partida()), mesa::huella(s.partida()));
    }

    #[test]
    fn cada_punto_coincide_con_la_huella_que_tenia_al_jugarse() {
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        let mut huellas = vec![mesa::huella(s.partida())];
        let mut jugar = |s: &mut Sesion, a: Accion| {
            s.accion(a, None).unwrap();
            huellas.push(mesa::huella(s.partida()));
        };
        jugar(&mut s, Accion::QuedarseCon { al_fondo: vec![] });
        for _ in 0..8 {
            jugar(&mut s, Accion::Robar { n: 1 });
            jugar(&mut s, Accion::Barajar);
            jugar(&mut s, Accion::PasarTurno);
        }
        s.entrar_repeticion(false).unwrap();
        assert_eq!(huellas.len(), s.repeticion.as_ref().unwrap().seqs.len() + 1);
        for (pos, esperada) in huellas.iter().enumerate() {
            s.repeticion_ir(pos as u32, None).unwrap();
            assert_eq!(&mesa::huella(s.partida()), esperada, "pos {pos}");
        }
    }
}
