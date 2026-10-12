//! El ritmo de Scryfall: 50–100 ms entre peticiones a `api.scryfall.com`
//! (~10 por segundo). Un solo limitador para toda la app.

use std::time::Duration;

use tokio::sync::Mutex;
use tokio::time::Instant;

pub struct Limitador {
    ultima: Mutex<Option<Instant>>,
    espacio: Duration,
}

impl Limitador {
    pub fn new(espacio: Duration) -> Limitador {
        Limitador {
            ultima: Mutex::new(None),
            espacio,
        }
    }

    /// Espera lo que haga falta para respetar el espacio con la anterior.
    pub async fn turno(&self) {
        let mut ultima = self.ultima.lock().await;
        if let Some(t) = *ultima {
            let listo = t + self.espacio;
            let ahora = Instant::now();
            if listo > ahora {
                tokio::time::sleep(listo - ahora).await;
            }
        }
        *ultima = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn veinte_turnos_tardan_al_menos_diecinueve_espacios() {
        let l = Limitador::new(Duration::from_millis(100));
        let inicio = Instant::now();
        for _ in 0..20 {
            l.turno().await;
        }
        assert!(inicio.elapsed() >= Duration::from_millis(1900));
    }
}
