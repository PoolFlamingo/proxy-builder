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
/** Y lo mínimo, para que se vea cada cambio. */
export const ESPERA_MINIMA_MS = 150;

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
