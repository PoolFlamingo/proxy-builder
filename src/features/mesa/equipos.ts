import type { IdJugador } from "@/bindings/IdJugador";
import type { Variantes } from "@/bindings/Variantes";
import type { VistaJugador } from "@/bindings/VistaJugador";

/**
 * Lo poco de equipos que decide la pantalla: avisar de un ataque a un
 * compañero. Quién es oponente de quién lo decide `crates/mesa`.
 */

/** ¿Tienen el mismo equipo? Sin equipo, nunca. */
export function sonCompaneros(
	jugadores: Record<number, Pick<VistaJugador, "equipo"> | undefined>,
	a: IdJugador,
	b: IdJugador,
): boolean {
	if (a === b) return false;
	const ea = jugadores[a]?.equipo ?? null;
	return ea !== null && ea === (jugadores[b]?.equipo ?? null);
}

/** Si la partida deja atacarse entre compañeros (sin equipos, todos son rivales). */
export function hayFuegoAmigo(variantes: Variantes | null): boolean {
	const e = variantes?.equipos;
	if (!e || e.tipo === "ninguno") return true;
	return e.tipo === "equipos" && e.fuegoAmigo;
}

/** ¿Atacar de `a` a `b` merece un aviso? Es un compañero y el fuego amigo está apagado. */
export function ataqueAUnCompanero(
	jugadores: Record<number, Pick<VistaJugador, "equipo"> | undefined>,
	variantes: Variantes | null,
	a: IdJugador,
	b: IdJugador,
): boolean {
	return !hayFuegoAmigo(variantes) && sonCompaneros(jugadores, a, b);
}
