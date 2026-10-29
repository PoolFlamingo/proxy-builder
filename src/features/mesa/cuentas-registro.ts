import type { Evento } from "@/bindings/Evento";
import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";

/**
 * **Cuentas del registro** (H4): cuánto tardó cada turno y a qué líneas se les
 * ofrece «A la pila». El texto lo escribe `textos-eventos.ts`; aquí solo se
 * mira la lista de eventos.
 */

/** Las líneas de abajo de todo (las más recientes) a las que se ofrece «A la pila». */
export const ULTIMAS_PARA_APILAR = 8;

/**
 * Si el evento sale en el registro. De las notas solo salen las que se
 * apuntan o cambian **públicas**: una privada no se nombra (ni su texto ni
 * que existe) y borrar una nota no dice si lo era.
 */
export function visibleEnRegistro(ev: Evento): boolean {
	if (ev.cambio.tipo !== "accion") return true;
	const a = ev.cambio.accion;
	if (a.que === "quitarNota" || a.que === "atarNota") return false;
	if (a.que === "ponerNota" || a.que === "editarNota" || a.que === "notaEnCarta") return !a.privada;
	return true;
}

function pasaTurno(ev: Evento): boolean {
	return ev.cambio.tipo === "accion" && ev.cambio.accion.que === "pasarTurno";
}

function empiezaPartida(ev: Evento): boolean {
	return ev.cambio.tipo === "restaurar" && ev.cambio.deshecho === null;
}

/**
 * Cuánto tardó el turno que cierra el evento `i` (en ms): desde el turno
 * anterior que se pasó o desde que empezó la partida. `null` si no es un
 * «pasar turno» o si el principio ya no está en la lista (solo se guardan los
 * últimos eventos).
 */
export function duracionDeTurno(registro: readonly Evento[], i: number): number | null {
	const ev = registro[i];
	if (!ev || !pasaTurno(ev)) return null;
	for (let j = i - 1; j >= 0; j--) {
		const antes = registro[j];
		if (antes && (pasaTurno(antes) || empiezaPartida(antes))) {
			return Math.max(0, Number(ev.tMs) - Number(antes.tMs));
		}
	}
	const primero = registro[0];
	if (primero && Number(primero.seq) === 0 && i > 0) {
		return Math.max(0, Number(ev.tMs) - Number(primero.tMs));
	}
	return null;
}

/**
 * La carta que se puede mandar «a la pila» desde una línea del registro: la
 * de un `mover` de una sola carta al campo que sigue ahí (se puso en juego
 * sin pasar por la pila), solo entre las últimas líneas.
 */
export function cartaParaApilar(
	registro: readonly Evento[],
	i: number,
	cartas: Record<number, VistaCarta | undefined>,
): IdCarta | null {
	if (i < registro.length - ULTIMAS_PARA_APILAR) return null;
	const c = registro[i]?.cambio;
	if (c?.tipo !== "accion" || c.accion.que !== "mover") return null;
	const a = c.accion;
	if (a.a.zona.tipo !== "de" || a.a.zona.zona !== "campo" || a.cartas.length !== 1) return null;
	const id = a.cartas[0];
	if (id === undefined) return null;
	const z = cartas[id]?.zona;
	return z && z.tipo === "de" && z.zona === "campo" ? id : null;
}
