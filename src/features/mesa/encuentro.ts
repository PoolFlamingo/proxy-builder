import { useEffect, useState } from "react";
import type { CartaBanquillo } from "@/bindings/CartaBanquillo";
import type { VistaEncuentro } from "@/bindings/VistaEncuentro";
import { call } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";

/**
 * **El encuentro** (al mejor de 1, 3 o 5) y el banquillo entre partidas. Las
 * cuentas las hace Rust (`mesa::encuentro`); aquí solo se agrupan las cartas
 * por nombre para elegir cuántas cambiar y se comprueba que salgan tantas
 * como entran antes de mandarlo.
 */

/** Las copias de una misma carta (mismo nombre), con el id de cada una. */
export type GrupoCartas = { nombre: string; ids: string[] };

/** Cuántas copias de cada carta (por nombre) se cambian. */
export type Cuantas = Record<string, number>;

/** Lo que un jugador cambia: lo que sale del mazo y lo que entra del banquillo. */
export type CambioDeJugador = { sale: Cuantas; entra: Cuantas };

export const SIN_CAMBIOS: CambioDeJugador = { sale: {}, entra: {} };

/** Junta las cartas de igual nombre, en el orden en que llegan. */
export function agrupar(cartas: CartaBanquillo[]): GrupoCartas[] {
	const por = new Map<string, GrupoCartas>();
	for (const c of cartas) {
		const g = por.get(c.nombre);
		if (g) g.ids.push(c.id);
		else por.set(c.nombre, { nombre: c.nombre, ids: [c.id] });
	}
	return [...por.values()];
}

/** Cambia en `delta` las copias de `nombre`, sin salirse de 0 a `max`. */
export function ajustar(cuantas: Cuantas, nombre: string, delta: number, max: number): Cuantas {
	const n = Math.max(0, Math.min(max, (cuantas[nombre] ?? 0) + delta));
	const { [nombre]: _, ...resto } = cuantas;
	return n === 0 ? resto : { ...resto, [nombre]: n };
}

export function total(cuantas: Cuantas): number {
	return Object.values(cuantas).reduce((a, n) => a + n, 0);
}

/** Los ids de las copias que se cambian: las primeras de cada grupo. */
export function idsDe(grupos: GrupoCartas[], cuantas: Cuantas): string[] {
	return grupos.flatMap((g) => g.ids.slice(0, cuantas[g.nombre] ?? 0));
}

/** Salen tantas cartas como entran (Rust vuelve a comprobarlo). */
export function cambioValido(c: CambioDeJugador): boolean {
	return total(c.sale) === total(c.entra);
}

export function hayCambio(c: CambioDeJugador): boolean {
	return total(c.sale) > 0 || total(c.entra) > 0;
}

/** Cómo va la partida y el encuentro, al día: cambia al terminar y al empezar la siguiente partida. */
export function useEncuentro(): VistaEncuentro | null {
	const fin = useMesa((s) => s.general?.fin ?? null);
	const [v, setV] = useState<VistaEncuentro | null>(null);
	useEffect(() => {
		let vivo = true;
		call("mesa_encuentro")
			.then((r) => vivo && setV(r))
			.catch(() => vivo && setV(null));
		return () => {
			vivo = false;
		};
	}, [fin]);
	return v;
}
