//! **Azar determinista**: xoshiro256** sembrado con SplitMix64.
//!
//! Propio y no el crate `rand`: son pocas líneas con vectores de prueba
//! publicados, y lo que importa es que dé lo mismo en todos los equipos y en
//! todas las versiones. La entropía la pone la app (la semilla); el
//! resultado de cada tirada viaja dentro del evento, así nadie más necesita
//! el estado del generador.

/// SplitMix64: para sembrar xoshiro desde un solo `u64`.
#[derive(Debug, Clone)]
pub struct SplitMix64(u64);

impl SplitMix64 {
    pub fn new(semilla: u64) -> Self {
        SplitMix64(semilla)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}

/// xoshiro256**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Azar {
    s: [u64; 4],
}

impl Azar {
    pub fn new(semilla: u64) -> Self {
        let mut sm = SplitMix64::new(semilla);
        let s = [sm.next_u64(), sm.next_u64(), sm.next_u64(), sm.next_u64()];
        Azar { s }
    }

    /// Desde el estado tal cual (para los vectores de prueba).
    pub fn desde_estado(s: [u64; 4]) -> Self {
        Azar { s }
    }

    pub fn next_u64(&mut self) -> u64 {
        let resultado = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        resultado
    }

    /// Un número de `0..n` sin sesgo (por rechazo). `n == 0` da 0.
    pub fn menor_que(&mut self, n: u64) -> u64 {
        if n <= 1 {
            return 0;
        }
        // El mayor múltiplo de n que cabe: lo de encima se descarta.
        let zona = u64::MAX - (u64::MAX % n);
        loop {
            let x = self.next_u64();
            if x < zona {
                return x % n;
            }
        }
    }

    /// Un número de `1..=caras`.
    pub fn entre(&mut self, caras: u32) -> u32 {
        // `menor_que(caras)` es menor que `caras`, que cabe en u32.
        (self.menor_que(u64::from(caras)) as u32) + 1
    }

    pub fn moneda(&mut self) -> bool {
        self.next_u64() >> 63 == 1
    }

    /// Fisher–Yates.
    pub fn barajar<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.menor_que(i as u64 + 1) as usize;
            v.swap(i, j);
        }
    }

    /// `n` posiciones distintas de `0..total`, en el orden en que salieron.
    pub fn elegir(&mut self, total: usize, n: usize) -> Vec<usize> {
        let mut idx: Vec<usize> = (0..total).collect();
        self.barajar(&mut idx);
        idx.truncate(n.min(total));
        idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Vector de referencia de SplitMix64 (semilla 1234567, de la
    /// implementación de Vigna).
    #[test]
    fn splitmix64_vector_de_referencia() {
        let mut s = SplitMix64::new(1_234_567);
        let esperado = [
            6_457_827_717_110_365_317u64,
            3_203_168_211_198_807_973,
            9_817_491_932_198_370_423,
            4_593_380_528_125_082_431,
            16_408_922_859_458_223_821,
        ];
        for e in esperado {
            assert_eq!(s.next_u64(), e);
        }
    }

    /// Vector de referencia de xoshiro256** (estado 1, 2, 3, 4).
    #[test]
    fn xoshiro_vector_de_referencia() {
        let mut a = Azar::desde_estado([1, 2, 3, 4]);
        let esperado = [
            11_520u64,
            0,
            1_509_978_240,
            1_215_971_899_390_074_240,
            1_216_172_134_540_287_360,
            607_988_272_756_665_600,
            16_172_922_978_634_559_625,
            8_476_171_486_693_032_832,
            10_595_114_339_597_558_777,
            2_904_607_092_377_533_576,
        ];
        for e in esperado {
            assert_eq!(a.next_u64(), e);
        }
    }

    #[test]
    fn d20_sin_sesgo() {
        let mut a = Azar::new(42);
        let mut cuenta = [0u32; 20];
        let n = 100_000;
        for _ in 0..n {
            let x = a.entre(20);
            assert!((1..=20).contains(&x));
            cuenta[(x - 1) as usize] += 1;
        }
        for c in cuenta {
            let f = f64::from(c) / f64::from(n);
            assert!((0.047..=0.053).contains(&f), "frecuencia {f}");
        }
    }

    #[test]
    fn barajar_es_determinista() {
        let mut v1: Vec<u32> = (0..100).collect();
        let mut v2 = v1.clone();
        Azar::new(7).barajar(&mut v1);
        Azar::new(7).barajar(&mut v2);
        assert_eq!(v1, v2);
        assert_ne!(v1, (0..100).collect::<Vec<_>>());
        let mut v3: Vec<u32> = (0..100).collect();
        Azar::new(8).barajar(&mut v3);
        assert_ne!(v1, v3);
    }
}
