import { create } from "zustand";
import type { DatosIcono } from "@/bindings/DatosIcono";
import { call } from "@/lib/ipc";

/** Lo que se espera a juntar las peticiones de un mismo pintado, en ms. */
const JUNTAR_MS = 16;

type IconosStore = {
	/** Los iconos que no van dentro de la app y ya trajo Rust (`iconos_datos`). */
	datos: Record<string, DatosIcono>;
	/** Pide uno que falta. Se puede llamar al pintar: no cambia el estado al momento. */
	pedir: (id: string) => void;
	/** Mete los que llegaron por otro camino (el selector ya los tiene). */
	recibir: (datos: Record<string, DatosIcono>) => void;
};

const pedidos = new Set<string>();
let enCola: string[] = [];
let temporizador: number | null = null;

export const useIconos = create<IconosStore>((set, get) => ({
	datos: {},
	pedir: (id) => {
		if (pedidos.has(id) || get().datos[id]) return;
		pedidos.add(id);
		enCola.push(id);
		if (temporizador !== null) return;
		temporizador = window.setTimeout(() => {
			const ids = enCola;
			enCola = [];
			temporizador = null;
			call("iconos_datos", { ids })
				.then((d) => get().recibir(d))
				// Sin red ni copia guardada: se queda el hueco y se puede volver a pedir.
				.catch(() => {
					for (const i of ids) pedidos.delete(i);
				});
		}, JUNTAR_MS);
	},
	recibir: (datos) => set({ datos: { ...get().datos, ...datos } }),
}));
