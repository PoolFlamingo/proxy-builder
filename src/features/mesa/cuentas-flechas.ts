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

type Punto = { x: number; y: number };

/**
 * Dónde entra en el campo (`w`×`h`) una flecha que viene de fuera, hacia el
 * punto `hacia` (el centro de la otra mesa, en las coordenadas de este
 * tablero): el punto del borde en esa dirección desde el centro, a `margen`
 * del borde. Sin `hacia`, arriba en el centro (el rival imaginado).
 */
export function entradaDesde(w: number, h: number, hacia: Punto | undefined, margen = 24): Punto {
	const cx = w / 2;
	const cy = h / 2;
	if (!hacia) return { x: cx, y: Math.min(30, h / 2) };
	const dx = hacia.x - cx;
	const dy = hacia.y - cy;
	if (dx === 0 && dy === 0) return { x: cx, y: Math.min(30, h / 2) };
	const mx = Math.max(0, cx - margen);
	const my = Math.max(0, cy - margen);
	// Lo que hay que estirar la dirección para tocar el borde más cercano.
	const k = Math.min(
		dx === 0 ? Infinity : mx / Math.abs(dx),
		dy === 0 ? Infinity : my / Math.abs(dy),
	);
	return { x: cx + dx * k, y: cy + dy * k };
}
