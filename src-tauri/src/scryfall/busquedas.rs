//! Las búsquedas recientes, en memoria: repetir una (volver a abrir un
//! diálogo con el mismo nombre, la misma carta en varias listas…) no la pide
//! otra vez a Scryfall.
//!
//! Vida corta: lo que trae una búsqueda puede acabar guardado en la caché de
//! cartas con la fecha de ahora (la resolución de listas lo hace), y así sus
//! precios no parecen más nuevos de lo que son. Con tope de cartas según la
//! memoria del equipo, las más antiguas fuera primero.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use tokio::time::Instant;

use super::models::ScryCard;

/// Lo que ocupa en memoria una carta de Scryfall, más o menos (medido: unos
/// 2,3 KB de texto más la estructura y lo que cuesta cada reserva).
const BYTES_POR_CARTA: u64 = 4 * 1024;
/// La parte de la memoria del equipo que pueden ocupar: un 0,25 % (8 GB →
/// ~20 MB, 64 GB → ~160 MB).
const PARTE_DE_LA_MEMORIA: u64 = 400;
/// Entre cuántas cartas (también si no se sabe la memoria: lo mínimo).
const MIN_CARTAS: usize = 5_000;
const MAX_CARTAS: usize = 50_000;

/// Cuántas cartas se recuerdan en este equipo.
pub fn cartas_segun_memoria() -> usize {
    let mut sistema = sysinfo::System::new();
    sistema.refresh_memory();
    cartas_para(sistema.total_memory())
}

fn cartas_para(memoria: u64) -> usize {
    let cartas = memoria / PARTE_DE_LA_MEMORIA / BYTES_POR_CARTA;
    usize::try_from(cartas)
        .unwrap_or(MAX_CARTAS)
        .clamp(MIN_CARTAS, MAX_CARTAS)
}

pub struct Busquedas {
    entradas: Mutex<VecDeque<Entrada>>,
    vida: Duration,
    max_cartas: usize,
}

struct Entrada {
    clave: String,
    cuando: Instant,
    cartas: Vec<ScryCard>,
}

impl Busquedas {
    pub fn new(vida: Duration, max_cartas: usize) -> Busquedas {
        Busquedas {
            entradas: Mutex::new(VecDeque::new()),
            vida,
            max_cartas,
        }
    }

    fn entradas(&self) -> std::sync::MutexGuard<'_, VecDeque<Entrada>> {
        // Si otro hilo se cayó con el candado, lo guardado sigue valiendo.
        self.entradas.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Lo guardado para esta búsqueda, si no ha caducado.
    pub fn leer(&self, clave: &str) -> Option<Vec<ScryCard>> {
        let mut entradas = self.entradas();
        let ahora = Instant::now();
        entradas.retain(|e| ahora.duration_since(e.cuando) < self.vida);
        entradas
            .iter()
            .find(|e| e.clave == clave)
            .map(|e| e.cartas.clone())
    }

    pub fn guardar(&self, clave: String, cartas: Vec<ScryCard>) {
        if cartas.len() > self.max_cartas {
            return;
        }
        let mut entradas = self.entradas();
        entradas.retain(|e| e.clave != clave);
        let mut total: usize = entradas.iter().map(|e| e.cartas.len()).sum();
        while total + cartas.len() > self.max_cartas {
            match entradas.pop_front() {
                Some(e) => total -= e.cartas.len(),
                None => break,
            }
        }
        entradas.push_back(Entrada {
            clave,
            cuando: Instant::now(),
            cartas,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cartas(n: usize) -> Vec<ScryCard> {
        (0..n)
            .map(|i| ScryCard {
                id: i.to_string(),
                ..ScryCard::default()
            })
            .collect()
    }

    #[test]
    fn el_tope_segun_la_memoria() {
        const GB: u64 = 1024 * 1024 * 1024;
        assert_eq!(cartas_para(0), MIN_CARTAS);
        assert_eq!(cartas_para(8 * GB), 5_242);
        assert_eq!(cartas_para(16 * GB), 10_485);
        assert_eq!(cartas_para(64 * GB), 41_943);
        assert_eq!(cartas_para(1024 * GB), MAX_CARTAS);
    }

    #[tokio::test(start_paused = true)]
    async fn caduca() {
        let b = Busquedas::new(Duration::from_secs(60), 100);
        b.guardar("waste".into(), cartas(3));
        assert_eq!(b.leer("waste").map(|c| c.len()), Some(3));
        assert!(b.leer("wastes").is_none());
        tokio::time::advance(Duration::from_secs(61)).await;
        assert!(b.leer("waste").is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn con_tope_salen_las_mas_antiguas() {
        let b = Busquedas::new(Duration::from_secs(60), 10);
        b.guardar("a".into(), cartas(4));
        b.guardar("b".into(), cartas(4));
        b.guardar("c".into(), cartas(4));
        assert!(b.leer("a").is_none());
        assert!(b.leer("b").is_some());
        assert!(b.leer("c").is_some());
        // Una que no cabe ni sola no se guarda (ni echa a las demás).
        b.guardar("d".into(), cartas(11));
        assert!(b.leer("d").is_none());
        assert!(b.leer("c").is_some());
    }
}
