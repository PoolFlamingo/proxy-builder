import { create } from "zustand";

/**
 * Qué diálogo de pasar decks está abierto. Uno solo a la vez; lo monta
 * `DialogosPasar` (en el marco de la app) y lo abren el botón «Enviar /
 * Recibir» de la biblioteca, «Compartir…» del menú de un deck y un `.pbdeck`
 * que se importa.
 */
export type DialogoPasar =
	| { tipo: "pasar"; pestana: "enviar" | "recibir"; ids: string[] }
	| { tipo: "compartir"; ids: string[] }
	| { tipo: "importar"; ruta: string };

type Store = {
	dialogo: DialogoPasar | null;
	/** «Enviar / Recibir», con estos decks ya elegidos (ninguno: se eligen dentro). */
	abrirPasar: (opciones?: { pestana?: "enviar" | "recibir"; ids?: string[] }) => void;
	/** «Compartir…» de uno o varios decks. */
	abrirCompartir: (ids: string[]) => void;
	/** Un `.pbdeck` (una ruta o un `content://`) que se va a importar. */
	abrirImportar: (ruta: string) => void;
	cerrar: () => void;
};

export const usePasarUi = create<Store>((set) => ({
	dialogo: null,
	abrirPasar: (o) =>
		set({ dialogo: { tipo: "pasar", pestana: o?.pestana ?? "enviar", ids: o?.ids ?? [] } }),
	abrirCompartir: (ids) => set({ dialogo: { tipo: "compartir", ids } }),
	abrirImportar: (ruta) => set({ dialogo: { tipo: "importar", ruta } }),
	cerrar: () => set({ dialogo: null }),
}));
