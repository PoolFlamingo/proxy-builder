//! **La partida en curso, en un fichero**: `app_data_dir/partidas/actual.json.gz`
//! (el registro entero comprimido). Se guarda al salir y cada pocos
//! eventos, y al volver a «Jugar» se puede seguir. No va en la base de
//! datos: el historial de las terminadas es otra cosa (Fase H).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use mesa::preparar::DeckDeMesa;
use mesa::{Evento, ModoDeshacer, Partida, Registro, VERSION_CATALOGO};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::sesion::{ConfigMesa, Sesion};
use crate::error::{AppError, AppResult, ErrorCode};
use crate::store::blobs::escribir_atomico;

/// Cada cuántos eventos se guarda sola.
pub const CADA: u32 = 50;
/// O, con algo sin guardar, cada cuánto tiempo (ms): un cierre brusco no
/// pierde más que esto.
pub const CADA_MS: u64 = 20_000;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartidaGuardada {
    pub version_catalogo: u32,
    pub config: ConfigMesa,
    pub mazos: Vec<DeckDeMesa>,
    pub inicial: Partida,
    pub eventos: Vec<Evento>,
    pub guardada_ms: u64,
}

/// Lo que la pantalla enseña de la partida guardada antes de seguirla.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ResumenGuardada {
    pub mazos: Vec<String>,
    pub deck_ids: Vec<String>,
    pub turno: u32,
    pub ronda: u32,
    #[ts(type = "number")]
    pub guardada_ms: u64,
}

pub fn ruta(data: &Path) -> PathBuf {
    data.join("partidas").join("actual.json.gz")
}

pub fn guardar(data: &Path, s: &Sesion) -> AppResult<()> {
    let g = PartidaGuardada {
        version_catalogo: VERSION_CATALOGO,
        config: s.config.clone(),
        mazos: s.mazos.clone(),
        inicial: s.registro.inicial.clone(),
        eventos: s.registro.eventos.clone(),
        guardada_ms: super::sesion::ahora_ms(),
    };
    let json = serde_json::to_vec(&g)
        .map_err(|e| AppError::new(ErrorCode::Internal).with("detalle", e))?;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&json)?;
    escribir_atomico(&ruta(data), &gz.finish()?)
}

pub fn leer(data: &Path) -> AppResult<Option<PartidaGuardada>> {
    let Ok(bytes) = std::fs::read(ruta(data)) else {
        return Ok(None);
    };
    let mut json = Vec::new();
    GzDecoder::new(bytes.as_slice()).read_to_end(&mut json)?;
    let g: PartidaGuardada = serde_json::from_slice(&json)
        .map_err(|e| AppError::new(ErrorCode::InvalidInput).with("detalle", e))?;
    // Una partida de una versión más nueva del catálogo no se puede leer.
    if g.version_catalogo > VERSION_CATALOGO {
        return Err(AppError::new(ErrorCode::Unsupported).with("motivo", "version"));
    }
    Ok(Some(g))
}

pub fn borrar(data: &Path) {
    let _ = std::fs::remove_file(ruta(data));
}

pub fn resumen(g: &PartidaGuardada) -> AppResult<ResumenGuardada> {
    let r = Registro::reconstruir(
        g.inicial.clone(),
        g.eventos.clone(),
        ModoDeshacer::SinLimite,
    )?;
    let t = &r.partida().turno;
    Ok(ResumenGuardada {
        mazos: g.mazos.iter().map(|m| m.nombre.clone()).collect(),
        deck_ids: g
            .config
            .jugadores
            .iter()
            .map(|j| j.deck_id.clone())
            .collect(),
        turno: t.numero,
        ronda: t.ronda,
        guardada_ms: g.guardada_ms,
    })
}

/// La sesión de una partida guardada.
pub fn sesion(g: PartidaGuardada) -> AppResult<Sesion> {
    let r = Registro::reconstruir(g.inicial, g.eventos, ModoDeshacer::SinLimite)?;
    Ok(Sesion::con_registro(g.config, g.mazos, r))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesa::sesion::tests::{config, mazo};
    use mesa::accion::Accion;

    #[test]
    fn guardar_y_seguir_deja_la_misma_huella() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Sesion::nueva(config(), vec![mazo()]).unwrap();
        s.accion(Accion::QuedarseCon { al_fondo: vec![] }, None)
            .unwrap();
        for _ in 0..5 {
            s.accion(Accion::Robar { n: 1 }, None).unwrap();
            s.accion(Accion::PasarTurno, None).unwrap();
        }
        guardar(dir.path(), &s).unwrap();
        let g = leer(dir.path()).unwrap().unwrap();
        assert_eq!(resumen(&g).unwrap().mazos, vec!["Prueba"]);
        let otra = sesion(g).unwrap();
        assert_eq!(mesa::huella(otra.partida()), mesa::huella(s.partida()));
        borrar(dir.path());
        assert!(leer(dir.path()).unwrap().is_none());
    }
}
