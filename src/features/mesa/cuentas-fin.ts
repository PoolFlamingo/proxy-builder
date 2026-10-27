import type { IdEquipo } from "@/bindings/IdEquipo";
import type { IdJugador } from "@/bindings/IdJugador";

/**
 * **Cuentas del final de partida** (H1). Quién queda en pie, el orden de
 * los puestos y la duración los calcula Rust (`mesa::resumir`); aquí solo lo
 * que decide qué proponer en la pantalla.
 */

/** Lo mínimo de cada jugador que hace falta para saber si queda un ganador. */
export type EstadoDeJugador = { eliminado: boolean; equipo: IdEquipo | null };

/**
 * Los que ganarían si se terminara ahora: el único que sigue en pie o, con
 * equipos, todos los del único equipo que sigue en pie. Vacío si hay más de
 * uno (o de un equipo) en pie, si no queda nadie o si se juega solo.
 */
export function ganadoresEnPie(
	asientos: readonly IdJugador[],
	estado: (id: IdJugador) => EstadoDeJugador | undefined,
): IdJugador[] {
	if (asientos.length < 2) return [];
	const vivos = asientos.filter((id) => estado(id)?.eliminado === false);
	if (vivos.length === 0) return [];
	if (vivos.length === 1) return vivos;
	const primero = estado(vivos[0] ?? 0)?.equipo ?? null;
	if (primero === null) return [];
	return vivos.every((id) => estado(id)?.equipo === primero) ? vivos : [];
}

/** `1:05` o `1:02:03`: el tiempo de una partida. */
export function formatoDuracion(ms: number): string {
	const total = Math.max(0, Math.floor(ms / 1000));
	const h = Math.floor(total / 3600);
	const m = Math.floor((total % 3600) / 60);
	const s = total % 60;
	const dos = (n: number) => String(n).padStart(2, "0");
	return h > 0 ? `${h}:${dos(m)}:${dos(s)}` : `${m}:${dos(s)}`;
}
