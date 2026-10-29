import type { IdJugador } from "@/bindings/IdJugador";
import type { Nota } from "@/bindings/Nota";

/**
 * **Cuentas de las notas**: cómo se reparten en el panel. Quién ve qué lo
 * decide Rust (`VistaGeneral.notas` ya solo trae las que puede leer el que
 * mira); aquí solo se ordenan.
 */

/** Lo más largo que puede ser una nota (el mismo tope que pone Rust). */
export const MAX_NOTA = 200;

export type NotasDeAutor = { autor: IdJugador; notas: Nota[] };

/**
 * Las notas propias, por orden de creación, y las de los demás agrupadas por
 * jugador en el orden de los asientos.
 */
export function repartirNotas(
	notas: readonly Nota[],
	yo: IdJugador | null,
	asientos: readonly IdJugador[],
): { mias: Nota[]; ajenas: NotasDeAutor[] } {
	const porId = (a: Nota, b: Nota) => a.id - b.id;
	const mias = notas.filter((n) => n.autor === yo).sort(porId);
	const ajenas: NotasDeAutor[] = [];
	for (const autor of asientos) {
		if (autor === yo) continue;
		const delAutor = notas.filter((n) => n.autor === autor).sort(porId);
		if (delAutor.length > 0) ajenas.push({ autor, notas: delAutor });
	}
	return { mias, ajenas };
}
