import { create } from "zustand";
import type { ImagePolicy } from "@/bindings/ImagePolicy";
import type { AnchoMiniatura } from "@/lib/image-url";

/**
 * Estado de pantalla que no es del deck: zoom, selección, vistas y las
 * preferencias de la lista (idioma, política de imagen…), estas últimas
 * recordadas en este equipo.
 */
type Preferencias = {
	lang: string;
	imagePolicy: ImagePolicy;
	addTokens: boolean;
	calidadVista: AnchoMiniatura;
	pedirConfirmacionAlCargar: boolean;
};

type UiStore = Preferencias & {
	zoom: number;
	verReversos: boolean;
	infoCarta: boolean;
	seleccion: string[];
	/** Última selección «ancla» para seleccionar con Mayús. */
	ancla: string | null;
	setZoom: (z: number) => void;
	setVerReversos: (b: boolean) => void;
	setInfoCarta: (b: boolean) => void;
	seleccionar: (ids: string[], ancla?: string | null) => void;
	setPreferencia: <K extends keyof Preferencias>(k: K, v: Preferencias[K]) => void;
};

const CLAVE = "proxybuilder.preferencias";

const PREDETERMINADAS: Preferencias = {
	lang: "en",
	imagePolicy: "withImage",
	addTokens: false,
	calidadVista: 488,
	pedirConfirmacionAlCargar: true,
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

export const useUi = create<UiStore>((set, get) => ({
	...leer(),
	zoom: 1,
	verReversos: false,
	infoCarta: false,
	seleccion: [],
	ancla: null,
	setZoom: (zoom) => set({ zoom: Math.max(0.4, Math.min(2.5, zoom)) }),
	setVerReversos: (verReversos) => set({ verReversos }),
	setInfoCarta: (infoCarta) => set({ infoCarta }),
	seleccionar: (seleccion, ancla) =>
		set({ seleccion, ancla: ancla === undefined ? get().ancla : ancla }),
	setPreferencia: (k, v) => {
		set({ [k]: v } as Partial<UiStore>);
		const s = get();
		const guardar: Preferencias = {
			lang: s.lang,
			imagePolicy: s.imagePolicy,
			addTokens: s.addTokens,
			calidadVista: s.calidadVista,
			pedirConfirmacionAlCargar: s.pedirConfirmacionAlCargar,
		};
		try {
			localStorage.setItem(CLAVE, JSON.stringify(guardar));
		} catch {
			/* sin almacenamiento: solo esta sesión */
		}
	},
}));
