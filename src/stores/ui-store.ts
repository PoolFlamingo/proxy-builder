import { create } from "zustand";
import type { ImagePolicy } from "@/bindings/ImagePolicy";
import type { Intensidad } from "@/bindings/Intensidad";
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
	/** Buscar y bajar las versiones nuevas sola (al arrancar y cada pocas horas). */
	actualizarSola: boolean;
	/** En Diseñar, las flechas pasan también por las copias de cada carta. */
	disenarCopias: boolean;
	/** Las pantallas cuyo tutorial ya ha salido (solo sale solo la primera vez). */
	tutorialesVistos: Vista[];
	/** Ajustes > Mesa. */
	mesa: PreferenciasMesa;
};

/**
 * Cómo se pintan los dados y las monedas: 3D (three.js y física), CSS 3D
 * (low poly, sin WebGL), 2D o lo mejor que aguante el equipo.
 */
export type NivelDados = "auto" | "3d" | "css" | "2d";

/** Los ajustes de la mesa de juego (Ajustes > Mesa). */
export type PreferenciasMesa = {
	/** Tamaño de las cartas del campo, en % (25–200). */
	tamanoCartas: number;
	/** Ancho de la imagen de las cartas del campo. */
	calidad: 146 | 244 | 488;
	/** Ancho de la vista grande al pasar el ratón, en px. */
	vistaGrande: number;
	/** El panel de estado junto a la vista grande. */
	panelEstado: boolean;
	reducirAnimaciones: boolean;
	sombras: boolean;
	/** Contador de fotogramas (depuración). */
	fps: boolean;
	/** Cómo se pintan las tiradas. */
	dados: NivelDados;
	/** Volumen de los sonidos de la mesa (0–100). */
	volumen: number;
	/** Sonido al tirar dados y lanzar monedas. */
	sonidoDados: boolean;
	/** La intensidad del sparring con la que empieza el solitario. */
	sparring: Intensidad;
	/** Si ya se enseñó la tarjeta de «Cómo funciona» del sparring. */
	sparringVisto: boolean;
	/** Mantener pulsado el botón izquierdo abre el menú de la carta. */
	clicSostenido: boolean;
	/** Al adjuntar un aura o equipo con un «+N/+N» fijo, ofrecer sumarlo. */
	ofrecerBono: boolean;
	/** La vida del fantasma en solitario. */
	fantasma: boolean;
	/** Los contadores de jugador que se enseñan junto a la vida. */
	trackers: string[];
};

export const MESA_DE_FABRICA: PreferenciasMesa = {
	tamanoCartas: 100,
	calidad: 244,
	vistaGrande: 320,
	panelEstado: true,
	reducirAnimaciones: false,
	sombras: true,
	fps: false,
	dados: "auto",
	volumen: 60,
	sonidoDados: true,
	sparring: "apagado",
	sparringVisto: false,
	clicSostenido: false,
	ofrecerBono: true,
	fantasma: true,
	trackers: [],
};

/** Los diálogos que se abren desde las cartas o las barras. */
export type Dialogo =
	| { tipo: "editar"; id: string }
	| { tipo: "variante"; id: string }
	| { tipo: "vincular"; id: string }
	/** Revisión en bloque: de las cartas `ids` o, sin ellas, de todo el deck. */
	/** `ids`: la selección, si la hay; `soloSeleccion`: empezar limitado a ella
	 *  (se puede cambiar a todo el deck en el diálogo). */
	| { tipo: "vincularLote"; ids?: string[]; soloSeleccion?: boolean }
	/** `en`: posición donde entra; `hueco`: casilla en blanco que la carta sustituye. */
	| { tipo: "anadir"; en?: number; hueco?: string; cambiar?: string }
	| { tipo: "moverA"; ids: string[] }
	| { tipo: "fichas" }
	| { tipo: "sets" }
	| { tipo: "estadisticas" }
	/** Editor de cartas: colecciones; letras, packs y símbolos; recortar arte. */
	| { tipo: "colecciones" }
	| { tipo: "recursosDiseno" }
	| { tipo: "disenarDeck" }
	| { tipo: "exportarPng" };

export type Vista = "deck" | "biblioteca" | "disenar" | "mesa";

type UiStore = Preferencias & {
	vista: Vista;
	/** El tutorial abierto (el de una pantalla), si lo hay. */
	tutorial: Vista | null;
	/** Abre el tutorial de una pantalla (y lo apunta como visto) o lo cierra. */
	abrirTutorial: (v: Vista | null) => void;
	setVista: (v: Vista) => void;
	/** La carta que se está diseñando (id de una copia) en la vista «Diseñar». */
	disenando: string | null;
	disenar: (id: string | null) => void;
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
	/** Enciende o apaga el editor; al apagarlo se sale de «Diseñar» y de sus diálogos. */
	seleccionar: (ids: string[], ancla?: string | null) => void;
	abrir: (d: Dialogo | null) => void;
	anunciar: (texto: string | null) => void;
	setPreferencia: <K extends keyof Preferencias>(k: K, v: Preferencias[K]) => void;
	setMesa: (cambio: Partial<PreferenciasMesa>) => void;
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
	actualizarSola: true,
	disenarCopias: false,
	tutorialesVistos: [],
	mesa: MESA_DE_FABRICA,
};

function leer(): Preferencias {
	try {
		const crudo = localStorage.getItem(CLAVE);
		if (!crudo) return PREDETERMINADAS;
		const leidas = JSON.parse(crudo) as Partial<Preferencias>;
		return {
			...PREDETERMINADAS,
			...leidas,
			mesa: { ...MESA_DE_FABRICA, ...(leidas.mesa ?? {}) },
		};
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
		actualizarSola: s.actualizarSola,
		disenarCopias: s.disenarCopias,
		tutorialesVistos: s.tutorialesVistos,
		mesa: s.mesa,
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
	tutorial: null,
	abrirTutorial: (tutorial) => {
		const vistos = get().tutorialesVistos;
		set({
			tutorial,
			...(tutorial && !vistos.includes(tutorial)
				? { tutorialesVistos: [...vistos, tutorial] }
				: {}),
		});
		guardar(get());
	},
	disenando: null,
	disenar: (id) => {
		set({ disenando: id, ...(id ? { vista: "disenar" as const } : {}) });
	},
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
	setMesa: (cambio) => {
		set({ mesa: { ...get().mesa, ...cambio } });
		guardar(get());
	},
}));
