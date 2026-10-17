import { create } from "zustand";
import { type Agrupar, FILTROS_VACIOS, type Filtros, type Orden } from "./organizar";

/** Cómo se ve la biblioteca: se recuerda en este equipo. */
export type PrefsBiblioteca = {
	vista: "cuadricula" | "lista";
	orden: Orden;
	ascendente: boolean;
	agrupar: Agrupar;
	/** Tamaño de las tarjetas en la cuadrícula. */
	tamano: "s" | "m" | "l";
	lateral: boolean;
	favoritosArriba: boolean;
};

const CLAVE = "proxybuilder.biblioteca";

export const PREFS_DE_FABRICA: PrefsBiblioteca = {
	vista: "cuadricula",
	orden: "modificado",
	ascendente: false,
	agrupar: "ninguno",
	tamano: "m",
	lateral: true,
	favoritosArriba: true,
};

function leer(): PrefsBiblioteca {
	try {
		const crudo = localStorage.getItem(CLAVE);
		return crudo
			? { ...PREFS_DE_FABRICA, ...(JSON.parse(crudo) as Partial<PrefsBiblioteca>) }
			: PREFS_DE_FABRICA;
	} catch {
		return PREFS_DE_FABRICA;
	}
}

type Store = PrefsBiblioteca & {
	poner: (c: Partial<PrefsBiblioteca>) => void;
};

export const usePrefsBiblioteca = create<Store>((set, get) => ({
	...leer(),
	poner: (c) => {
		set(c);
		const { poner: _, ...p } = get();
		try {
			localStorage.setItem(CLAVE, JSON.stringify(p));
		} catch {
			/* sin almacenamiento: solo esta sesión */
		}
	},
}));

/** El sentido natural de cada orden (al elegirlo). */
export function ascendentePorDefecto(o: Orden): boolean {
	return o === "nombre" || o === "colores" || o === "coste";
}

/** Los filtros de ahora (no se recuerdan entre sesiones). */
type StoreFiltros = {
	filtros: Filtros;
	ponerFiltros: (c: Partial<Filtros>) => void;
	limpiar: () => void;
};

export const useFiltrosBiblioteca = create<StoreFiltros>((set) => ({
	filtros: FILTROS_VACIOS,
	ponerFiltros: (c) => set((s) => ({ filtros: { ...s.filtros, ...c } })),
	limpiar: () => set({ filtros: FILTROS_VACIOS }),
}));
