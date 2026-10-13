import { create } from "zustand";
import type { ImagePolicy } from "@/bindings/ImagePolicy";
import type { AnchoMiniatura } from "@/lib/image-url";

/**
 * Estado de pantalla que no es del deck: zoom, selección, diálogos abiertos,
 * la última acción del pie y las preferencias de la lista (idioma, política
 * de imagen…), estas últimas recordadas en este equipo.
 */
type Preferencias = {
	lang: string;
	imagePolicy: ImagePolicy;
	addTokens: boolean;
	calidadVista: AnchoMiniatura;
	pedirConfirmacionAlCargar: boolean;
	zoom: number;
	infoCarta: boolean;
};

/** Los diálogos que se abren desde las cartas o las barras. */
export type Dialogo =
	| { tipo: "editar"; id: string }
	| { tipo: "variante"; id: string }
	| { tipo: "anadir"; en?: number }
	| { tipo: "moverA"; ids: string[] }
	| { tipo: "sets" }
	| { tipo: "estadisticas" };

export type Vista = "deck" | "biblioteca";

type UiStore = Preferencias & {
	vista: Vista;
	setVista: (v: Vista) => void;
	verReversos: boolean;
	seleccion: string[];
	/** Última carta pulsada, para seleccionar un tramo con Mayús. */
	ancla: string | null;
	dialogo: Dialogo | null;
	/** Lo último que ha pasado, para el pie («PDF guardado (6 páginas)»). */
	ultimaAccion: string | null;
	setZoom: (z: number) => void;
	setVerReversos: (b: boolean) => void;
	setInfoCarta: (b: boolean) => void;
	seleccionar: (ids: string[], ancla?: string | null) => void;
	abrir: (d: Dialogo | null) => void;
	anunciar: (texto: string | null) => void;
	setPreferencia: <K extends keyof Preferencias>(k: K, v: Preferencias[K]) => void;
};

const CLAVE = "proxybuilder.preferencias";
export const ZOOM_MIN = 0.4;
export const ZOOM_MAX = 2.5;

const PREDETERMINADAS: Preferencias = {
	lang: "en",
	imagePolicy: "withImage",
	addTokens: false,
	calidadVista: 488,
	pedirConfirmacionAlCargar: true,
	zoom: 0.8,
	infoCarta: false,
};

function leer(): Preferencias {
	try {
		const crudo = localStorage.getItem(CLAVE);
		if (!crudo) return PREDETERMINADAS;
		return { ...PREDETERMINADAS, ...(JSON.parse(crudo) as Partial<Preferencias>) };
	} catch {
		return PREDETERMINADAS;
	}
}

function guardar(s: Preferencias) {
	const p: Preferencias = {
		lang: s.lang,
		imagePolicy: s.imagePolicy,
		addTokens: s.addTokens,
		calidadVista: s.calidadVista,
		pedirConfirmacionAlCargar: s.pedirConfirmacionAlCargar,
		zoom: s.zoom,
		infoCarta: s.infoCarta,
	};
	try {
		localStorage.setItem(CLAVE, JSON.stringify(p));
	} catch {
		/* sin almacenamiento: solo esta sesión */
	}
}

export const useUi = create<UiStore>((set, get) => ({
	...leer(),
	vista: "deck",
	setVista: (vista) => set({ vista }),
	verReversos: false,
	seleccion: [],
	ancla: null,
	dialogo: null,
	ultimaAccion: null,
	setZoom: (z) => {
		set({ zoom: Math.round(Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, z)) * 100) / 100 });
		guardar(get());
	},
	setVerReversos: (verReversos) => set({ verReversos }),
	setInfoCarta: (infoCarta) => {
		set({ infoCarta });
		guardar(get());
	},
	seleccionar: (seleccion, ancla) =>
		set({ seleccion, ancla: ancla === undefined ? get().ancla : ancla }),
	abrir: (dialogo) => set({ dialogo }),
	anunciar: (ultimaAccion) => set({ ultimaAccion }),
	setPreferencia: (k, v) => {
		set({ [k]: v } as Partial<UiStore>);
		guardar(get());
	},
}));
