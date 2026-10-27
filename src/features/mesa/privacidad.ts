import type { IdJugador } from "@/bindings/IdJugador";
import type { Mesa } from "@/stores/mesa-store";

/**
 * 🔒 En una partida **local** (varios jugadores en el mismo equipo) todos
 * miran la misma pantalla: la mano de quien no tiene el turno no se enseña
 * (salen sus reversos), para que nadie vea lo que no debe. Con el mulligan en
 * marcha sí, porque cada uno tiene que ver su mano para decidir. En red cada
 * uno solo ve la suya, así que no hace falta.
 */
export function manoOcultaPorTurno(
	m: Pick<Mesa, "general" | "asientos">,
	tablero: IdJugador,
	enRed: boolean,
): boolean {
	if (enRed || m.asientos.length < 2) return false;
	if (m.general?.mulligan != null) return false;
	const activo = m.general?.turno.activo;
	return activo !== undefined && activo !== tablero;
}

/**
 * El deck cuyo tapete se lee de los ajustes para el tablero de `jugador`: en
 * una partida local, el de su asiento; en red, solo el del anfitrión (el único
 * que sé cuál es) cuando se mira su tablero. Sin él, vale el tapete que la
 * partida preparó para ese jugador, no el mío.
 */
export function deckDelTablero(
	deckIds: string[],
	asientos: IdJugador[],
	jugador: IdJugador,
	enRed: boolean,
	esAnfitrion: boolean,
): string | null {
	const i = asientos.indexOf(jugador);
	if (i < 0) return null;
	if (!enRed) return deckIds[i] ?? null;
	return esAnfitrion && i === 0 ? (deckIds[0] ?? null) : null;
}
