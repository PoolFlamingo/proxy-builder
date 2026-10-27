import type { EnSala } from "@/bindings/EnSala";

/**
 * **Los asientos de la sala de espera en red.** El anfitrión (id 0) siempre
 * es el primer asiento; los invitados con deck se sientan por orden de llegada
 * y el anfitrión puede cambiarlos de sitio. Los que miran no se sientan, ni
 * los que aún no han mandado su deck (se quedan fuera al empezar).
 */

/** El id del anfitrión en la sala. */
export const ID_ANFITRION = 0;

/** Los que se sentarían ahora: el anfitrión y los jugadores con deck. */
export function sentados(sala: EnSala[]): number[] {
	return sala
		.filter((j) => j.id === ID_ANFITRION || (j.rol === "jugador" && j.deck !== null))
		.map((j) => j.id);
}

/**
 * El orden de asientos de antes, puesto al día con quienes hay ahora: se
 * quedan los que siguen (en su sitio), los nuevos van al final y el
 * anfitrión, el primero. `quitados` son las posiciones de antes de los que
 * se fueron, de mayor a menor (para ajustar «quien empieza» y el archienemigo).
 */
export function sincronizarAsientos(
	previo: number[],
	ahora: number[],
): { ids: number[]; quitados: number[] } {
	const hay = new Set(ahora);
	const quitados = previo
		.map((id, i) => (hay.has(id) ? -1 : i))
		.filter((i) => i >= 0)
		.reverse();
	const siguen = previo.filter((id) => hay.has(id));
	const nuevos = ahora.filter((id) => !siguen.includes(id));
	const ids = [...siguen, ...nuevos];
	// El anfitrión, delante.
	const sinAnfitrion = ids.filter((id) => id !== ID_ANFITRION);
	return {
		ids: ids.includes(ID_ANFITRION) ? [ID_ANFITRION, ...sinAnfitrion] : sinAnfitrion,
		quitados,
	};
}

/** ¿Es el mismo orden? (Para no volver a pintar sin cambios.) */
export function mismoOrden(a: number[], b: number[]): boolean {
	return a.length === b.length && a.every((x, i) => x === b[i]);
}
