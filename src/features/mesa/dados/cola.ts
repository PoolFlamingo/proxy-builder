import { create } from "zustand";
import type { Cambio } from "@/bindings/Cambio";
import type { CaraPlanar } from "@/bindings/CaraPlanar";
import type { Evento } from "@/bindings/Evento";
import { dadosDeTirada, type TipoDado } from "./geometria";

/**
 * **Las tiradas que se están enseñando** (dados, monedas, dado planar), en
 * cola: cada evento de azar que llega en un parche se pinta en la capa de
 * los dados (`capa.tsx`), una tirada tras otra.
 */

/** Como mucho se pintan estas piezas; el resultado sale entero igualmente. */
export const MAX_PIEZAS = 20;

export type PiezaTirada =
	| { forma: "dado"; tipo: TipoDado; valor: number; decenas?: boolean }
	| { forma: "moneda"; cara: boolean }
	| { forma: "planar"; cara: CaraPlanar };

export type Tirada = {
	/** El `seq` del evento. */
	id: number;
	/** Quién tiró (para el color); `null`, la mesa. */
	actor: number | null;
	semilla: number;
	piezas: PiezaTirada[];
	/** El cambio del motor: lo que se lee al final, en grande (`ResultadoAzar`). */
	cambio: CambioAzar;
};

export type CambioAzar = Extract<
	Cambio,
	{ tipo: "dados" } | { tipo: "monedas" } | { tipo: "dadoPlanar" }
>;

/** La tirada que hay que pintar de un evento, o `null` si no es de azar. */
export function tiradaDe(ev: Evento): Tirada | null {
	const c = ev.cambio;
	const actor = ev.actor.tipo === "jugador" ? ev.actor.id : null;
	switch (c.tipo) {
		case "dados":
			return {
				id: ev.seq,
				actor,
				semilla: c.semillaAnimacion,
				piezas: c.tiradas
					.flatMap((t) =>
						dadosDeTirada(t.caras, t.valor).map((d) => ({ forma: "dado" as const, ...d })),
					)
					.slice(0, MAX_PIEZAS),
				cambio: c,
			};
		case "monedas":
			return {
				id: ev.seq,
				actor,
				semilla: c.semillaAnimacion,
				// Se pinta la última tanda (con «hasta perder», la que perdió).
				piezas: (c.lanzamientos.at(-1) ?? [])
					.slice(0, MAX_PIEZAS)
					.map((cara) => ({ forma: "moneda" as const, cara })),
				cambio: c,
			};
		case "dadoPlanar":
			return {
				id: ev.seq,
				actor,
				semilla: c.semillaAnimacion,
				piezas: [{ forma: "planar", cara: c.cara }],
				cambio: c,
			};
		default:
			return null;
	}
}

type Cola = {
	tiradas: Tirada[];
	encolar: (ev: Evento) => void;
	/** Quita la primera (ya enseñada). */
	siguiente: () => void;
	vaciar: () => void;
};

export const useDados = create<Cola>((set) => ({
	tiradas: [],
	encolar: (ev) => {
		const t = tiradaDe(ev);
		// Como mucho tres en espera: si se tira muy deprisa, las de en medio sobran.
		if (t)
			set((s) => ({
				tiradas: [...s.tiradas.slice(0, 1), ...[...s.tiradas.slice(1), t].slice(-2)],
			}));
	},
	siguiente: () => set((s) => ({ tiradas: s.tiradas.slice(1) })),
	vaciar: () => set({ tiradas: [] }),
}));
