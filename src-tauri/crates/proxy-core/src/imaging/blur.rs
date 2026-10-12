//! Desenfoque gaussiano aproximado con tres pasadas de caja (coste
//! constante por píxel, sea cual sea el radio). Para nitidez, claridad y
//! reducción de ruido.

use rayon::prelude::*;

/// Radios de las tres cajas que aproximan un gaussiano de σ (Wells, 1986).
fn radios(sigma: f32) -> [usize; 3] {
    let n = 3.0f32;
    let ideal = ((12.0 * sigma * sigma / n) + 1.0).sqrt();
    let mut wl = ideal.floor() as i32;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m = ((12.0 * sigma * sigma - n * (wl * wl) as f32 - 4.0 * n * wl as f32 - 3.0 * n)
        / (-4.0 * wl as f32 - 4.0))
        .round() as i32;
    let r = |i: i32| (((if i < m { wl } else { wu }) - 1) / 2).max(0) as usize;
    [r(0), r(1), r(2)]
}

fn caja_h(src: &[f32], dst: &mut [f32], w: usize, r: usize) {
    if r == 0 {
        dst.copy_from_slice(src);
        return;
    }
    dst.par_chunks_mut(w)
        .zip(src.par_chunks(w))
        .for_each(|(fila_d, fila_s)| {
            let n = (2 * r + 1) as f32;
            let at = |i: isize| fila_s[i.clamp(0, w as isize - 1) as usize];
            let mut suma: f32 = (-(r as isize)..=r as isize).map(at).sum();
            for (x, d) in fila_d.iter_mut().enumerate() {
                *d = suma / n;
                suma += at(x as isize + r as isize + 1) - at(x as isize - r as isize);
            }
        });
}

fn transponer(src: &[f32], w: usize, h: usize) -> Vec<f32> {
    let mut out = vec![0.0; w * h];
    out.par_chunks_mut(h).enumerate().for_each(|(x, col)| {
        for (y, v) in col.iter_mut().enumerate() {
            *v = src[y * w + x];
        }
    });
    out
}

/// Desenfoca un plano (`w × h`, una muestra por píxel) con σ en píxeles.
pub fn gauss(plano: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    if sigma < 0.3 || w == 0 || h == 0 {
        return plano.to_vec();
    }
    let rs = radios(sigma);
    let mut a = plano.to_vec();
    let mut b = vec![0.0; a.len()];
    for r in rs {
        caja_h(&a, &mut b, w, r);
        std::mem::swap(&mut a, &mut b);
    }
    let mut t = transponer(&a, w, h);
    let mut b = vec![0.0; t.len()];
    for r in rs {
        caja_h(&t, &mut b, h, r);
        std::mem::swap(&mut t, &mut b);
    }
    transponer(&t, h, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_plano_uniforme_no_cambia() {
        let p = vec![0.5f32; 40 * 30];
        let r = gauss(&p, 40, 30, 4.0);
        assert!(r.iter().all(|v| (v - 0.5).abs() < 1e-5));
    }

    #[test]
    fn un_punto_se_reparte_y_conserva_la_masa() {
        let (w, h) = (51, 51);
        let mut p = vec![0.0f32; w * h];
        p[25 * w + 25] = 1.0;
        let r = gauss(&p, w, h, 3.0);
        let masa: f32 = r.iter().sum();
        assert!((masa - 1.0).abs() < 1e-3, "{masa}");
        assert!(r[25 * w + 25] < 0.1 && r[25 * w + 25] > r[25 * w + 30]);
    }
}
