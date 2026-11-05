import { create } from "zustand";
import type { CardSummary } from "@/bindings/CardSummary";
import type { FiltrosCartas } from "@/bindings/FiltrosCartas";
import { FILTROS_DE_FABRICA, hayCriterio } from "@/features/buscar/consulta";
import { call } from "@/lib/ipc";

/**
 * La pantalla «Buscar». Aquí y no en el componente porque cambiar de pestaña
 * la desmonta y lo buscado tiene que seguir ahí al volver.
 */
type Estado = {
	filtros: FiltrosCartas;
	/** `null` = todavía no se ha buscado nada. */
	resultados: CardSummary[] | null;
	total: number;
	pagina: number;
	hayMas: boolean;
	buscando: boolean;
	/** Falló al pedir la página siguiente: ya no se pide sola al bajar. */
	falloAlSeguir: boolean;
	/** La carta cuya ficha está abierta. */
	ficha: string | null;
	poner: (cambio: Partial<FiltrosCartas>) => void;
	quitarFiltros: () => void;
	/** Busca desde el principio; si falla, se lanza el error para enseñarlo. */
	buscar: () => Promise<void>;
	/** Pide la página siguiente; si falla, se lanza el error para enseñarlo. */
	seguir: () => Promise<void>;
	abrirFicha: (id: string | null) => void;
};

/** La última búsqueda pedida: lo que llegue de una anterior se tira. */
let turno = 0;

export const useBusqueda = create<Estado>((set, get) => ({
	filtros: FILTROS_DE_FABRICA,
	resultados: null,
	total: 0,
	pagina: 1,
	hayMas: false,
	buscando: false,
	falloAlSeguir: false,
	ficha: null,
	poner: (cambio) => set((s) => ({ filtros: { ...s.filtros, ...cambio } })),
	// El texto y el orden no son filtros: se quedan.
	quitarFiltros: () =>
		set(({ filtros: { texto, exacto, orden, descendente } }) => ({
			filtros: { ...FILTROS_DE_FABRICA, texto, exacto, orden, descendente },
		})),
	buscar: async () => {
		const { filtros } = get();
		const yo = ++turno;
		if (!hayCriterio(filtros)) {
			set({ resultados: null, total: 0, hayMas: false, buscando: false, falloAlSeguir: false });
			return;
		}
		set({ buscando: true, falloAlSeguir: false });
		try {
			const r = await call("cartas_buscar", { filtros, pagina: 1 });
			if (yo !== turno) return;
			set({ resultados: r.cartas, total: r.total, hayMas: r.hayMas, pagina: 1, buscando: false });
		} catch (e) {
			if (yo !== turno) return;
			set({ resultados: [], total: 0, hayMas: false, buscando: false });
			throw e;
		}
	},
	seguir: async () => {
		const { filtros, resultados, pagina, hayMas, buscando } = get();
		if (!resultados || !hayMas || buscando) return;
		const yo = ++turno;
		set({ buscando: true, falloAlSeguir: false });
		try {
			const r = await call("cartas_buscar", { filtros, pagina: pagina + 1 });
			if (yo !== turno) return;
			set({
				// Scryfall puede repetir una carta entre páginas si su índice cambia.
				resultados: [
					...resultados,
					...r.cartas.filter((c) => !resultados.some((x) => x.id === c.id)),
				],
				hayMas: r.hayMas,
				pagina: pagina + 1,
				buscando: false,
			});
		} catch (e) {
			if (yo !== turno) return;
			set({ buscando: false, falloAlSeguir: true });
			throw e;
		}
	},
	abrirFicha: (ficha) => set({ ficha }),
}));
