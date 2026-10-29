import type { Hito } from "@/bindings/Hito";

/**
 * **Cuentas de la línea de tiempo de la repetición** (H3): a dónde llevan
 * «turno anterior» y «siguiente» y cuánto se espera entre dos eventos al
 * reproducir. Qué partida hay en cada punto lo calcula Rust.
 */

/** Las velocidades de reproducción. */
export const VELOCIDADES = [0.5, 1, 2, 4, 8] as const;

/** Lo máximo que se espera entre dos eventos a velocidad 1, en ms. */
export const ESPERA_MAXIMA_MS = 1_500;
/** Y lo mínimo, para que dé tiempo a ver cómo se mueven las cartas. */
export const ESPERA_MINIMA_MS = 400;
/** Cuántos puntos como mucho se animan de una vez; un salto mayor se hace sin mover nada. */
export const SALTO_ANIMABLE = 3;

/**
 * Cuánto esperar antes del siguiente evento: el hueco real entre los dos,
 * acotado y dividido por la velocidad (sin horas en el primero).
 */
export function pausaEntre(
	antes: number | undefined,
	despues: number | undefined,
	velocidad: number,
): number {
	const hueco = antes === undefined || despues === undefined ? 0 : Math.max(0, despues - antes);
	const base = Math.min(ESPERA_MAXIMA_MS, Math.max(ESPERA_MINIMA_MS, hueco));
	return Math.round(base / Math.max(0.1, velocidad));
}

/** El principio del turno anterior al punto `pos` (0 si no hay). */
export function hitoAnterior(hitos: readonly Hito[], pos: number): number {
	for (let i = hitos.length - 1; i >= 0; i--) {
		const h = hitos[i];
		if (h && h.pos < pos) return h.pos;
	}
	return 0;
}

/** El principio del turno siguiente a `pos` (el final si no hay). */
export function hitoSiguiente(hitos: readonly Hito[], pos: number, total: number): number {
	return hitos.find((h) => h.pos > pos)?.pos ?? total;
}

/**
 * Cuánto dura el desplazamiento de las cartas al pasar de `desde` a `hasta`
 * (en ms; 0: sin animar). Rust solo manda la partida de cada punto, no el
 * movimiento de por medio: entre dos puntos seguidos las cartas se deslizan,
 * y es algo menos que la espera mínima a esa velocidad para que acabe antes
 * del siguiente.
 */
export function duracionMovimiento(desde: number, hasta: number, velocidad: number): number {
	const salto = Math.abs(hasta - desde);
	if (salto === 0 || salto > SALTO_ANIMABLE) return 0;
	return Math.max(40, Math.round((ESPERA_MINIMA_MS * 0.9) / Math.max(0.1, velocidad)));
}
