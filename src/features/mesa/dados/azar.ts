/**
 * **El azar de la animación**, a partir de la `semillaAnimacion` del evento
 * (la pone el motor): dónde entra cada dado, con qué empuje y qué giro. Sin
 * `Math.sin`/`Math.cos` ni `Math.random`: solo sumas y productos, para que
 * las mismas cifras salgan igual en todas las vistas web y Rapier (el build
 * determinista) dé la misma animación.
 */

/** mulberry32: un generador de 32 bits, pequeño y rápido. */
export function generador(semilla: number): () => number {
	let s = semilla >>> 0;
	return () => {
		s = (s + 0x6d2b79f5) >>> 0;
		let t = s;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Un número entre `a` y `b`. */
export const entre = (r: () => number, a: number, b: number) => a + (b - a) * r();

/**
 * Un giro al azar (cuaternión unitario `[x, y, z, w]`): cuatro números
 * entre −1 y 1 dentro de la bola unidad, normalizados (sin trigonometría).
 */
export function giroAlAzar(r: () => number): [number, number, number, number] {
	for (let i = 0; i < 64; i++) {
		const q = [r() * 2 - 1, r() * 2 - 1, r() * 2 - 1, r() * 2 - 1];
		const n = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
		if (n > 0.01 && n <= 1) {
			const k = 1 / Math.sqrt(n);
			return [q[0] * k, q[1] * k, q[2] * k, q[3] * k];
		}
	}
	return [0, 0, 0, 1];
}

/** Gira el vector `v` con el cuaternión `q` (`[x, y, z, w]`). */
export function girar(
	q: ArrayLike<number>,
	v: readonly [number, number, number],
): [number, number, number] {
	const [x, y, z, w] = [q[0], q[1], q[2], q[3]];
	// t = 2 · (q.xyz × v)
	const tx = 2 * (y * v[2] - z * v[1]);
	const ty = 2 * (z * v[0] - x * v[2]);
	const tz = 2 * (x * v[1] - y * v[0]);
	// v' = v + w·t + q.xyz × t
	return [
		v[0] + w * tx + (y * tz - z * ty),
		v[1] + w * ty + (z * tx - x * tz),
		v[2] + w * tz + (x * ty - y * tx),
	];
}
