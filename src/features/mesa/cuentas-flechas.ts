import type { Extremo } from "@/bindings/Extremo";
import type { Flecha } from "@/bindings/Flecha";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";

/**
 * De quién es el extremo de una flecha: el jugador al que se apunta, o el que
 * controla la carta (una flecha a una criatura de otro va, en el fondo, a ese
 * jugador: es lo que dice de a quién se ataca).
 */
export function duenoDeExtremo(
	e: Extremo,
	cartas: Record<IdCarta, Pick<VistaCarta, "controlador"> | undefined>,
): IdJugador | null {
	if (e.tipo === "jugador") return e.id;
	return cartas[e.id]?.controlador ?? null;
}

/** Cuántas flechas de **ataque** (de cualquiera) apuntan a este jugador o a sus cartas. */
export function atacantesDe(
	flechas: readonly Flecha[],
	cartas: Record<IdCarta, Pick<VistaCarta, "controlador"> | undefined>,
	jugador: IdJugador,
): number {
	let n = 0;
	for (const f of flechas) {
		if (f.tipo === "ataque" && duenoDeExtremo(f.a, cartas) === jugador) n++;
	}
	return n;
}

/** Un id válido para un `<marker>` de SVG que sale de un color (cada color, su punta). */
export function idDePunta(color: string): string {
	return `punta-${color.replace(/[^a-zA-Z0-9]/g, "")}`;
}
