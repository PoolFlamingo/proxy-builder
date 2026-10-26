import { create } from "zustand";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";

/**
 * **Lo que los demás hacen ahora mismo** (E7): el cursor de cada jugador,
 * la carta que arrastra y los avisos con aro. Llega por datagramas, sin
 * garantías, y no se guarda ni toca la partida: si falta uno no pasa nada.
 * Cada dato lleva la hora local de cuando llegó, para que caduque.
 */
export type FantasmaVivo = {
	tablero: IdJugador;
	/** La carta que arrastra (`null`: solo el cursor). */
	carta: IdCarta | null;
	/** Posición del motor, 0–10.000. */
	x: number;
	y: number;
	/** `performance.now()` de cuando llegó. */
	visto: number;
};

export type SenalViva = {
	id: number;
	jugador: IdJugador;
	carta: IdCarta | null;
	a: IdJugador | null;
	/** `performance.now()` hasta el que se ve el aro. */
	hasta: number;
};

/** Cuánto se ve el aro de un aviso. */
export const MS_SENAL = 2000;
/** Cuánto se queda una carta arrastrada sin noticias suyas. */
export const MS_CARTA = 1000;
/** Y un cursor quieto. */
export const MS_CURSOR = 5000;

type VivoStore = {
	fantasmas: Record<number, FantasmaVivo>;
	senales: SenalViva[];
	fantasma: (jugador: IdJugador, f: Omit<FantasmaVivo, "visto">, ahora: number) => void;
	senal: (jugador: IdJugador, carta: IdCarta | null, a: IdJugador | null, ahora: number) => void;
	/** Quita lo que ya caducó (y no cambia nada si no hay nada). */
	purgar: (ahora: number) => void;
	limpiar: () => void;
};

/** Si un fantasma ya no vale a la hora `ahora`. */
export function caducado(f: FantasmaVivo, ahora: number): boolean {
	return ahora - f.visto > (f.carta === null ? MS_CURSOR : MS_CARTA);
}

let siguiente = 1;

export const useVivo = create<VivoStore>((set, get) => ({
	fantasmas: {},
	senales: [],
	fantasma: (jugador, f, ahora) =>
		set({ fantasmas: { ...get().fantasmas, [jugador]: { ...f, visto: ahora } } }),
	senal: (jugador, carta, a, ahora) =>
		set({
			senales: [
				...get().senales.filter((s) => s.hasta > ahora),
				{ id: siguiente++, jugador, carta, a, hasta: ahora + MS_SENAL },
			],
		}),
	purgar: (ahora) => {
		const { fantasmas, senales } = get();
		const vivos = Object.entries(fantasmas).filter(([, f]) => !caducado(f, ahora));
		const sigue = senales.filter((s) => s.hasta > ahora);
		if (vivos.length === Object.keys(fantasmas).length && sigue.length === senales.length) return;
		set({ fantasmas: Object.fromEntries(vivos), senales: sigue });
	},
	limpiar: () => set({ fantasmas: {}, senales: [] }),
}));
