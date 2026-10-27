import type { Causa } from "@/bindings/Causa";
import type { IdJugador } from "@/bindings/IdJugador";

/**
 * **Muertes y «de quién fue el daño»** (G8). La cuenta de quién está fuera la
 * hace Rust (`mesa::fuera_de_juego`, que llega en `VistaJugador`); aquí solo
 * lo que decide qué preguntar a quién y qué se apunta por defecto. Nada de
 * reglas.
 */

/** Lo que se espera a que el afectado confirme antes de que otro pueda apuntarlo, en ms. */
export const ESPERA_APUNTAR_MS = 15_000;
/** Daño a partir del cual se pregunta de quién fue. */
export const DANO_MINIMO = 5;
/** Lo que espera la pregunta a que llegue más daño antes de hacerse, en ms. */
export const REPOSO_DANO_MS = 1_200;
/** Lo que dura la pregunta sin contestar, en ms. */
export const DURA_PREGUNTA_MS = 12_000;

/** Una clave que cambia cuando cambia el motivo (para volver a preguntar). */
export function claveCausa(c: Causa): string {
	return c.tipo === "comandante" ? `comandante:${c.carta}` : c.tipo;
}

/**
 * A quién se apunta como autor de la muerte si nadie dice otra cosa: el dueño
 * del comandante (si fue por comandante), quien se dijo que hizo el último
 * daño y, si no, el jugador del turno. `null`: nadie en concreto.
 */
export function eliminadoPorDefecto(o: {
	causa: Causa;
	jugador: IdJugador;
	activo: IdJugador;
	ultimoDano: IdJugador | undefined;
	vivos: readonly IdJugador[];
}): IdJugador | null {
	const { causa, jugador, activo, ultimoDano, vivos } = o;
	if (causa.tipo === "comandante" && causa.de !== jugador) return causa.de;
	if (ultimoDano !== undefined && ultimoDano !== jugador) return ultimoDano;
	if (activo !== jugador && vivos.includes(activo)) return activo;
	return null;
}

/**
 * Si hay que preguntar de quién fue el daño: mucho (`DANO_MINIMO` o más) a
 * alguien que se puede contestar por él, en el turno de otro que sigue en
 * la partida, y con alguien más a quien culpar.
 */
export function debePreguntarDano(o: {
	total: number;
	jugador: IdJugador;
	activo: IdJugador;
	vivos: readonly IdJugador[];
	controlado: boolean;
	terminada: boolean;
}): boolean {
	const { total, jugador, activo, vivos, controlado, terminada } = o;
	return (
		!terminada &&
		controlado &&
		total >= DANO_MINIMO &&
		activo !== jugador &&
		vivos.includes(jugador) &&
		vivos.includes(activo) &&
		vivos.length >= 2
	);
}

/** Lo que se ha perdido de vida entre dos lecturas (0 si subió). */
export function vidaPerdida(antes: number, ahora: number): number {
	return Math.max(0, antes - ahora);
}

/** Lo que se espera a que vuelva quien juega el turno antes de poder saltarlo, en ms. */
export const ESPERA_SALTAR_MS = 30_000;

/** El siguiente asiento que sigue en la partida después de `activo` (el que puede saltar su turno). */
export function siguienteVivo(
	asientos: readonly IdJugador[],
	vivos: readonly IdJugador[],
	activo: IdJugador,
): IdJugador | null {
	const i = asientos.indexOf(activo);
	if (i < 0) return null;
	for (let k = 1; k < asientos.length; k++) {
		const id = asientos[(i + k) % asientos.length];
		if (id !== undefined && id !== activo && vivos.includes(id)) return id;
	}
	return null;
}

/** Los segundos que lleva ausente quien se supo ausente hace `haceMs` en `visto`. */
export function segundosAusente(haceMs: number, visto: number, ahora: number): number {
	return Math.max(0, Math.floor((haceMs + Math.max(0, ahora - visto)) / 1000));
}
