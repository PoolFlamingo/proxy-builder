//! **Símbolos de maná** de un coste (`{2}{W}{U/B}{G/P}{X}`): cuántos de cada
//! color pide.

/// Los colores en el orden de siempre (WUBRG) y el incoloro.
pub const COLORES: [&str; 6] = ["W", "U", "B", "R", "G", "C"];

/// Lo que pide un coste.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Coste {
    /// Símbolos de cada color, en el orden de `COLORES`. Un híbrido cuenta
    /// para los dos colores; uno pirexiano, para el suyo.
    pub pips: [u32; 6],
    /// Lleva `{X}`.
    pub x: bool,
}

pub fn leer(coste: &str) -> Coste {
    let mut c = Coste::default();
    let mut resto = coste;
    while let Some(i) = resto.find('{') {
        let Some(j) = resto[i..].find('}') else {
            break;
        };
        let simbolo = &resto[i + 1..i + j];
        for parte in simbolo.split('/') {
            let parte = parte.trim().to_ascii_uppercase();
            if parte == "X" {
                c.x = true;
            } else if let Some(k) = COLORES.iter().position(|col| *col == parte) {
                c.pips[k] += 1;
            }
        }
        resto = &resto[i + j + 1..];
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_los_simbolos() {
        assert_eq!(leer("{2}{W}{W}").pips, [2, 0, 0, 0, 0, 0]);
        let h = leer("{U/B}{G/P}{2/R}{C}");
        assert_eq!(h.pips, [0, 1, 1, 1, 1, 1]);
        assert!(!h.x);
        let x = leer("{X}{X}{R}");
        assert!(x.x);
        assert_eq!(x.pips[3], 1);
        // Las cartas partidas traen los dos costes.
        assert_eq!(leer("{1}{R} // {1}{U}").pips, [0, 1, 0, 1, 0, 0]);
        assert_eq!(leer("").pips, [0; 6]);
    }
}
