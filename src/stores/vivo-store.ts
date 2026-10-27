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
	/** Las demás cartas que arrastra con ella (en pila). */
	mas: IdCarta[];
	/** Posición del motor, 0–10.000. */
	x: number;
	y: number;
	/** `performance.now()` de cuando llegó. */
	visto: number;
	/** Dónde estaba cada carta arrastrada cuando se empezó a ver el arrastre. */
	origenes: Record<number, Origen>;
};

/** Dónde está una carta en la mesa (zona y posición), para saber si ya se movió. */
export type Origen = { zona: string; x: number | null; y: number | null };

/** El sitio de una carta de la vista. */
export function origenDe(c: { zona: unknown; pos: { x: number; y: number } | null }): Origen {
	return { zona: JSON.stringify(c.zona), x: c.pos?.x ?? null, y: c.pos?.y ?? null };
}

function mismoOrigen(a: Origen, b: Origen): boolean {
	return a.zona === b.zona && a.x === b.x && a.y === b.y;
}

/** Cuánto se sigue ocultando una carta recién soltada si aún no se ha movido (llega el evento). */
export const MS_SOLTADA = 800;

/**
 * Una carta que acaba de dejar de arrastrarse y aún se ve en su sitio de antes.
 * Si la soltó otro, también dónde: se sigue pintando ahí hasta que llega el
 * movimiento (`tablero` y `x`, `y` como un fantasma).
 */
export type Soltada = {
	hasta: number;
	origen: Origen;
	donde?: { tablero: IdJugador; x: number; y: number };
};

/** Si una carta soltada sigue en el sitio de antes (aún no ha llegado su movimiento). */
export function soltadaSigue(
	s: Soltada,
	c: { zona: unknown; pos: { x: number; y: number } | null },
) {
	return mismoOrigen(s.origen, origenDe(c));
}

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
	soltadas: Record<number, Soltada>;
	senales: SenalViva[];
	/**
	 * Lo que hace `jugador`. `origenes` dice dónde está ahora cada carta que
	 * arrastra (solo se usa la primera vez que se la ve arrastrar).
	 */
	fantasma: (
		jugador: IdJugador,
		f: Omit<FantasmaVivo, "visto" | "mas" | "origenes"> & { mas?: IdCarta[] },
		ahora: number,
		origenes?: Record<number, Origen>,
	) => void;
	/** Mis propias cartas que acabo de soltar: se ocultan en su sitio de antes hasta que se muevan. */
	soltar: (origenes: Record<number, Origen>, ahora: number) => void;
	senal: (jugador: IdJugador, carta: IdCarta | null, a: IdJugador | null, ahora: number) => void;
	/** Quita lo que ya caducó (y no cambia nada si no hay nada). */
	purgar: (ahora: number) => void;
	limpiar: () => void;
};

/** Si un fantasma ya no vale a la hora `ahora`. */
export function caducado(f: FantasmaVivo, ahora: number): boolean {
	return ahora - f.visto > (f.carta === null ? MS_CURSOR : MS_CARTA);
}

/** Las cartas que arrastra un fantasma. */
export function cartasDeFantasma(f: Pick<FantasmaVivo, "carta" | "mas">): IdCarta[] {
	return f.carta === null ? [] : [f.carta, ...f.mas];
}

/**
 * Si una carta se está arrastrando (por otro) o se acaba de soltar y sigue
 * en su sitio de antes: no se pinta ahí, que se ve el arrastre y no su
 * posición vieja. `actual` es el sitio de la carta ahora.
 */
export function enVuelo(
	s: Pick<VivoStore, "fantasmas" | "soltadas">,
	id: IdCarta,
	actual: () => Origen,
): boolean {
	for (const f of Object.values(s.fantasmas)) {
		if (f.carta === id || f.mas.includes(id)) return true;
	}
	const soltada = s.soltadas[id];
	return soltada !== undefined && mismoOrigen(soltada.origen, actual());
}

let siguiente = 1;

export const useVivo = create<VivoStore>((set, get) => ({
	fantasmas: {},
	soltadas: {},
	senales: [],
	fantasma: (jugador, f, ahora, origenes = {}) => {
		const previo = get().fantasmas[jugador];
		const nuevo = { ...f, mas: f.mas ?? [] };
		const cartas = cartasDeFantasma(nuevo);
		const sitios: Record<number, Origen> = {};
		for (const id of cartas) {
			const o = previo?.origenes[id] ?? origenes[id];
			if (o) sitios[id] = o;
		}
		// Lo que dejó de arrastrar (soltó, y el movimiento llega enseguida) sigue
		// oculto en su sitio de antes un momento.
		const soltadas = { ...get().soltadas };
		for (const id of previo ? cartasDeFantasma(previo) : []) {
			const o = previo?.origenes[id];
			if (o && previo && !cartas.includes(id)) {
				soltadas[id] = {
					hasta: ahora + MS_SOLTADA,
					origen: o,
					donde: { tablero: previo.tablero, x: previo.x, y: previo.y },
				};
			}
		}
		for (const id of cartas) delete soltadas[id];
		set({
			fantasmas: { ...get().fantasmas, [jugador]: { ...nuevo, visto: ahora, origenes: sitios } },
			soltadas,
		});
	},
	soltar: (origenes, ahora) => {
		const soltadas = { ...get().soltadas };
		for (const [id, origen] of Object.entries(origenes)) {
			soltadas[Number(id)] = { hasta: ahora + MS_SOLTADA, origen };
		}
		set({ soltadas });
	},
	senal: (jugador, carta, a, ahora) =>
		set({
			senales: [
				...get().senales.filter((s) => s.hasta > ahora),
				{ id: siguiente++, jugador, carta, a, hasta: ahora + MS_SENAL },
			],
		}),
	purgar: (ahora) => {
		const { fantasmas, senales, soltadas } = get();
		const vivos = Object.entries(fantasmas).filter(([, f]) => !caducado(f, ahora));
		const sigue = senales.filter((s) => s.hasta > ahora);
		const siguen = Object.entries(soltadas).filter(([, x]) => x.hasta > ahora);
		if (
			vivos.length === Object.keys(fantasmas).length &&
			sigue.length === senales.length &&
			siguen.length === Object.keys(soltadas).length
		) {
			return;
		}
		set({
			fantasmas: Object.fromEntries(vivos),
			senales: sigue,
			soltadas: Object.fromEntries(siguen),
		});
	},
	limpiar: () => set({ fantasmas: {}, senales: [], soltadas: {} }),
}));
