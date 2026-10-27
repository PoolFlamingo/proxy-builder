//! Los colores de los asientos en red (los mismos que `COLORES_JUGADOR` de
//! la pantalla, `features/mesa/colores.ts`).

const COLORES: [&str; 10] = [
    "#e11d48", "#2563eb", "#16a34a", "#ca8a04", "#9333ea", "#0891b2", "#ea580c", "#db2777",
    "#4f46e5", "#65a30d",
];

pub fn de_asiento(i: usize) -> String {
    COLORES[i % COLORES.len()].to_string()
}
