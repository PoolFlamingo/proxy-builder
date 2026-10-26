//! **Lo que ve un espectador** (E11): no tiene la partida ni ejecuta el
//! motor. Guarda la vista de espectador que le manda el anfitrión
//! (`Mirador::Espectador`: de lo oculto, solo cuántas) y le aplica sus
//! parches; la pantalla la recibe por el mismo canal que una partida.

use std::collections::VecDeque;

use mesa::accion::Evento;
use mesa::vista::{aplicar_parche, ParcheVista, VistaMesa};
use tauri::ipc::Channel;

use crate::mesa::sesion::MensajeMesa;

/// Cuántos eventos se guardan para el registro de la pantalla.
const REGISTRO: usize = 500;

pub struct Mirada {
    pub epoch: u32,
    pub vista: VistaMesa,
    registro: VecDeque<Evento>,
    canal: Option<Channel<MensajeMesa>>,
}

/// Lo que pasó con un parche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visto {
    Aplicado,
    /// De una época vieja o ya visto: se tira.
    Tirado,
    /// No sigue a la vista que hay: hay que pedirla entera.
    Falta,
}

impl Mirada {
    pub fn nueva(epoch: u32, vista: VistaMesa) -> Self {
        Mirada {
            epoch,
            vista,
            registro: VecDeque::new(),
            canal: None,
        }
    }

    fn mandar(&self, m: MensajeMesa) {
        if let Some(c) = &self.canal {
            if let Err(e) = c.send(m) {
                tracing::warn!(error = %e, "no se pudo mandar a la mesa");
            }
        }
    }

    fn mensaje_vista(&self) -> MensajeMesa {
        MensajeMesa::Vista {
            vista: Box::new(self.vista.clone()),
            registro: self.registro.iter().cloned().collect(),
        }
    }

    pub fn suscribir(&mut self, canal: Channel<MensajeMesa>) {
        self.canal = Some(canal);
        self.mandar(self.mensaje_vista());
    }

    /// La vista entera (al entrar, al volver, tras un deshacer).
    pub fn vista_entera(&mut self, epoch: u32, vista: VistaMesa) {
        self.epoch = self.epoch.max(epoch);
        self.vista = vista;
        self.mandar(self.mensaje_vista());
    }

    /// Un parche del anfitrión. Sin el evento (no se puede contar), la
    /// pantalla recibe la vista entera.
    pub fn parche(&mut self, epoch: u32, parche: ParcheVista, evento: Option<Evento>) -> Visto {
        if epoch < self.epoch || parche.version <= self.vista.version {
            return Visto::Tirado;
        }
        if parche.version != self.vista.version + 1 {
            return Visto::Falta;
        }
        self.epoch = epoch;
        aplicar_parche(&mut self.vista, &parche);
        let Some(evento) = evento else {
            self.mandar(self.mensaje_vista());
            return Visto::Aplicado;
        };
        self.registro.push_back(evento.clone());
        while self.registro.len() > REGISTRO {
            self.registro.pop_front();
        }
        self.mandar(MensajeMesa::Parche {
            parche: Box::new(parche),
            evento: Box::new(evento),
        });
        Visto::Aplicado
    }

    pub fn cerrar(&mut self) {
        self.mandar(MensajeMesa::Cerrada);
        self.canal = None;
    }
}
