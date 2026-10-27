import { createContext, useContext } from "react";
import { create, type StoreApi, type UseBoundStore } from "zustand";
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
const CLAVE_JUGAR = "proxybuilder.jugar";

export const PREFS_DE_FABRICA: PrefsBiblioteca = {
	vista: "cuadricula",
	orden: "modificado",
	ascendente: false,
	agrupar: "ninguno",
	tamano: "m",
	lateral: true,
	favoritosArriba: true,
};

function leer(clave: string): PrefsBiblioteca {
	try {
		const crudo = localStorage.getItem(clave);
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

function crearPrefs(clave: string) {
	return create<Store>((set, get) => ({
		...leer(clave),
		poner: (c) => {
			set(c);
			const { poner: _, ...p } = get();
			try {
				localStorage.setItem(clave, JSON.stringify(p));
			} catch {
				/* sin almacenamiento: solo esta sesión */
			}
		},
	}));
}

export const usePrefsBiblioteca = crearPrefs(CLAVE);
/** Las de la pestaña «Jugar»: aparte, para que cada pantalla se acuerde de la suya. */
export const usePrefsJugar = crearPrefs(CLAVE_JUGAR);

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

function crearFiltros() {
	return create<StoreFiltros>((set) => ({
		filtros: FILTROS_VACIOS,
		ponerFiltros: (c) => set((s) => ({ filtros: { ...s.filtros, ...c } })),
		limpiar: () => set({ filtros: FILTROS_VACIOS }),
	}));
}

export const useFiltrosBiblioteca = crearFiltros();
export const useFiltrosJugar = crearFiltros();

/**
 * Qué preferencias y filtros usan las piezas compartidas (barra, lateral,
 * cuadrícula, lista): las de la biblioteca salvo que una pantalla ponga las
 * suyas.
 */
export type Organizador = {
	prefs: UseBoundStore<StoreApi<Store>>;
	filtros: UseBoundStore<StoreApi<StoreFiltros>>;
	/** Prefijo de las anclas del tutorial (`<x>-barra`…); sin él, no hay anclas. */
	ancla: string | null;
};

export const ORGANIZADOR_BIBLIOTECA: Organizador = {
	prefs: usePrefsBiblioteca,
	filtros: useFiltrosBiblioteca,
	ancla: "biblioteca",
};

export const ORGANIZADOR_JUGAR: Organizador = {
	prefs: usePrefsJugar,
	filtros: useFiltrosJugar,
	ancla: null,
};

export const OrganizadorContexto = createContext<Organizador>(ORGANIZADOR_BIBLIOTECA);

export function usePrefs(): Store {
	return useContext(OrganizadorContexto).prefs();
}

export function usePrefsSel<T>(f: (s: Store) => T): T {
	return useContext(OrganizadorContexto).prefs(f);
}

export function useFiltros(): StoreFiltros {
	return useContext(OrganizadorContexto).filtros();
}

export function useAncla(nombre: string): string | undefined {
	const a = useContext(OrganizadorContexto).ancla;
	return a ? `${a}-${nombre}` : undefined;
}
