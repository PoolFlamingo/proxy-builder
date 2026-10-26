import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import { call } from "@/lib/ipc";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { crearLimitador } from "./limitador";

/**
 * **Lo que esta pantalla cuenta en vivo a los demás** (E7): el cursor y la
 * carta que se arrastra (con el tablero en el que se mueve, porque se puede
 * estar mirando el de otro), y los avisos con aro. Va por `mesa_fantasma` y
 * `mesa_senal`; Rust lo manda por datagramas y no pasa por el motor.
 *
 * Solo en red y solo quien juega; el espectador no manda nada. Se puede
 * apagar en Ajustes > Mesa (entonces tampoco se ve lo de los demás).
 */
const limArrastre = crearLimitador(50); // 20 por segundo
const limCursor = crearLimitador(100); // 10 por segundo

/** Si esta pantalla manda movimiento en vivo ahora. */
export function mandaEnVivo(): boolean {
	const papel = useRed.getState().papel;
	return (papel === "anfitrion" || papel === "jugador") && useUi.getState().mesa.movimientoEnVivo;
}

function mandar(tablero: IdJugador, carta: IdCarta | null, x: number, y: number) {
	void call("mesa_fantasma", { tablero, carta, x, y }).catch(() => {});
}

/** Una carta arrastrada pasa por `pos` (0–10.000). `fuerza`: sin esperar al ritmo. */
export function arrastre(
	tablero: IdJugador,
	carta: IdCarta,
	pos: { x: number; y: number },
	fuerza = false,
) {
	if (!mandaEnVivo() || !limArrastre(fuerza)) return;
	mandar(tablero, carta, pos.x, pos.y);
}

/** El cursor está sobre `pos`; sin carta (`fuerza` al soltar lo arrastrado). */
export function cursor(tablero: IdJugador, pos: { x: number; y: number }, fuerza = false) {
	if (!mandaEnVivo() || !limCursor(fuerza)) return;
	mandar(tablero, null, pos.x, pos.y);
}

/** Avisa a todos con un aro sobre una carta o un jugador (2 s). */
export function senalar(carta: IdCarta | null, a: IdJugador | null) {
	if (useRed.getState().papel === null || useRed.getState().papel === "espectador") return;
	void call("mesa_senal", { carta, a }).catch(() => {});
}
