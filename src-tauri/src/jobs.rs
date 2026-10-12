//! **Trabajos largos** (resolver una lista, exportar un PDF): cada uno tiene
//! un id y una bandera de cancelación. La pantalla los sigue por eventos.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Jobs {
    siguiente: AtomicU32,
    banderas: Mutex<HashMap<u32, Arc<AtomicBool>>>,
}

impl Jobs {
    /// Da de alta un trabajo y devuelve su id y su bandera de cancelación.
    pub fn alta(&self) -> (u32, Arc<AtomicBool>) {
        let id = self.siguiente.fetch_add(1, Ordering::SeqCst) + 1;
        let b = Arc::new(AtomicBool::new(false));
        if let Ok(mut m) = self.banderas.lock() {
            m.insert(id, b.clone());
        }
        (id, b)
    }

    /// Pide que se pare. Devuelve `false` si el trabajo ya no existe.
    pub fn cancelar(&self, id: u32) -> bool {
        self.banderas
            .lock()
            .ok()
            .and_then(|m| m.get(&id).map(|b| b.store(true, Ordering::SeqCst)))
            .is_some()
    }

    pub fn baja(&self, id: u32) {
        if let Ok(mut m) = self.banderas.lock() {
            m.remove(&id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alta_cancelar_y_baja() {
        let j = Jobs::default();
        let (a, ba) = j.alta();
        let (b, _) = j.alta();
        assert_ne!(a, b);
        assert!(j.cancelar(a));
        assert!(ba.load(Ordering::SeqCst));
        j.baja(a);
        assert!(!j.cancelar(a));
    }
}
