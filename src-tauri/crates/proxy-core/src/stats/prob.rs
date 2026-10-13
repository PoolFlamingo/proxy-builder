//! **Probabilidades de robo** (distribución hipergeométrica): cuántas tierras
//! salen en la mano inicial, si se llega a las caídas de tierra de cada turno
//! y con qué probabilidad se ha visto ya una carta (o una función) en el
//! turno N.

/// Cartas de la mano inicial.
pub const MANO: u32 = 7;
/// Hasta cuántas cartas vistas de más se calcula: la mano y 12 robos (turno
/// 12 en la draw, 13 en la play).
pub const ROBOS: u32 = 12;

/// `ln C(n, k)`, sumando logaritmos (los mazos tienen como mucho unos
/// cientos de cartas: no hace falta una gamma).
fn ln_comb(n: u32, k: u32) -> f64 {
    if k > n {
        return f64::NEG_INFINITY;
    }
    let k = k.min(n - k);
    (1..=k)
        .map(|i| f64::from(n - k + i).ln() - f64::from(i).ln())
        .sum()
}

/// P(X = k): `k` éxitos al sacar `n` cartas de un mazo de `total` con `exitos`.
pub fn exacta(total: u32, exitos: u32, n: u32, k: u32) -> f64 {
    let n = n.min(total);
    if k > exitos || k > n || n - k > total - exitos.min(total) {
        return 0.0;
    }
    (ln_comb(exitos, k) + ln_comb(total - exitos, n - k) - ln_comb(total, n)).exp()
}

/// P(X ≥ k).
pub fn al_menos(total: u32, exitos: u32, n: u32, k: u32) -> f64 {
    if k == 0 {
        return 1.0;
    }
    let menos: f64 = (0..k).map(|j| exacta(total, exitos, n, j)).sum();
    (1.0 - menos).clamp(0.0, 1.0)
}

/// P(al menos una) con 7, 8, … 7 + `ROBOS` cartas vistas.
pub fn por_vistas(total: u32, copias: u32) -> Vec<f64> {
    (0..=ROBOS)
        .map(|i| redondear(al_menos(total, copias, MANO + i, 1)))
        .collect()
}

/// Cuatro decimales: lo que se enseña es un porcentaje con uno.
pub fn redondear(p: f64) -> f64 {
    (p * 10_000.0).round() / 10_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cerca(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn la_hipergeometrica() {
        // 60 cartas, 24 tierras, mano de 7: P(3 tierras) ≈ 30,9 %.
        assert!(
            cerca(exacta(60, 24, 7, 3), 0.3087),
            "{}",
            exacta(60, 24, 7, 3)
        );
        // Las probabilidades de 0 a 7 suman 1.
        let suma: f64 = (0..=7).map(|k| exacta(60, 24, 7, k)).sum();
        assert!(cerca(suma, 1.0));
        // Un 4-of en la mano inicial de 60: ≈ 39,9 %.
        assert!(
            cerca(al_menos(60, 4, 7, 1), 0.3995),
            "{}",
            al_menos(60, 4, 7, 1)
        );
        // Una sola copia en 100: 7 %.
        assert!(cerca(al_menos(100, 1, 7, 1), 0.07));
        // Sin copias, nunca; con todas, siempre.
        assert_eq!(al_menos(60, 0, 7, 1), 0.0);
        assert!(cerca(al_menos(60, 60, 7, 1), 1.0));
    }

    #[test]
    fn por_vistas_crece() {
        let v = por_vistas(99, 1);
        assert_eq!(v.len(), (ROBOS + 1) as usize);
        assert!(v.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn mas_cartas_de_las_que_hay() {
        assert!(cerca(al_menos(5, 1, 7, 1), 1.0));
        assert_eq!(exacta(5, 2, 7, 3), 0.0);
    }
}
