import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { Mesa } from "@/stores/mesa-store";

/**
 * 🔒 En una partida **local** (varios jugadores en el mismo equipo) todos
 * miran la misma pantalla: la mano de quien no tiene el turno no se enseña
 * (salen sus reversos), para que nadie vea lo que no debe. Con el mulligan en
 * marcha sí, porque cada uno tiene que ver su mano para decidir. En red cada
 * uno solo ve la suya, así que no hace falta. Quien responde fuera de su
 * turno (un instantáneo) la destapa con la cortina (`mostrada`).
 */
export function manoOcultaPorTurno(
	m: Pick<Mesa, "general" | "asientos">,
	tablero: IdJugador,
	enRed: boolean,
	mostrada: IdJugador | null = null,
): boolean {
	if (enRed || m.asientos.length < 2) return false;
	if (m.general?.mulligan != null) return false;
	const activo = m.general?.turno.activo;
	return activo !== undefined && activo !== tablero && mostrada !== tablero;
}

/**
 * Quién puede responder ahora en una partida local: los que no tienen el
 * turno y siguen en juego, en el orden de la mesa a partir del que juega.
 */
export function quienResponde(m: Pick<Mesa, "general" | "asientos" | "jugadores">): IdJugador[] {
	const activo = m.general?.turno.activo;
	const desde = activo === undefined ? -1 : m.asientos.indexOf(activo);
	if (desde < 0) return [];
	const n = m.asientos.length;
	const orden: IdJugador[] = [];
	for (let k = 1; k < n; k++) {
		const j = m.asientos[(desde + k) % n];
		if (j !== undefined && m.jugadores[j]?.eliminado === null) orden.push(j);
	}
	return orden;
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

/**
 * A quién pasa a mirar el atajo de responder: desde el del turno, al primero que
 * puede responder; de uno que responde, al siguiente y, tras el último, de
 * vuelta al del turno. Así con varios jugadores se llega a cualquiera.
 */
export function siguienteAlResponder(
	m: Pick<Mesa, "general" | "asientos" | "jugadores" | "yo">,
): IdJugador | null {
	const activo = m.general?.turno.activo;
	const quienes = quienResponde(m);
	if (m.yo === null || m.yo === activo) return quienes[0] ?? null;
	return quienes[quienes.indexOf(m.yo) + 1] ?? activo ?? null;
}

/**
 * Una carta del campo que no controlo, con el campo de los demás protegido
 * (`Opciones.campoAjenoProtegido`, en red): Rust no deja tocarla, así que la
 * pantalla ni la arrastra ni ofrece lo que se rechazaría. Solo se mira o se
 * toma el control.
 */
export function campoAjeno(
	m: Pick<Mesa, "opciones" | "yo">,
	c: Pick<VistaCarta, "zona" | "controlador">,
): boolean {
	return (
		m.opciones?.campoAjenoProtegido === true &&
		c.zona.tipo === "de" &&
		c.zona.zona === "campo" &&
		c.controlador !== m.yo
	);
}
