import { create } from "zustand";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Lado } from "@/bindings/Lado";
import type { Zona } from "@/bindings/Zona";

/**
 * **El estado de pantalla de la mesa** que no es de la partida: el diálogo
 * abierto, el modo de elegir objetivo (adjuntar, flechas), el arrastre en
 * curso y la zona bajo el ratón (para los atajos de la biblioteca).
 */

export type DialogoMesa =
	| { tipo: "numero"; titulo: string; inicial: number; hacer: (n: number) => void }
	| { tipo: "texto"; titulo: string; max: number; hacer: (texto: string) => void }
	| { tipo: "contador"; cartas: IdCarta[]; temporal?: boolean }
	| { tipo: "editarContadores"; carta: IdCarta }
	| { tipo: "pt"; carta: IdCarta }
	| { tipo: "etiqueta"; carta: IdCarta }
	| { tipo: "mirar"; de: IdJugador; n: number; desde: Lado; modo: "mirar" | "escrutar" | "vigilar" }
	| { tipo: "buscar"; de: IdJugador; filtro: "todo" | "tierra" }
	| { tipo: "revelar" }
	| { tipo: "zona"; jugador: IdJugador; zona: Zona }
	| { tipo: "fichas" }
	| { tipo: "azar" }
	| { tipo: "terminar" }
	| { tipo: "danoComandante" }
	| { tipo: "atajos" }
	| { tipo: "tapete" };

/** El ancho del panel lateral, en px (la vista grande se aparta de él). */
export const ANCHO_PANEL = 320;

/** El panel de al lado de la mesa: el registro o el chat (`null`: cerrado). */
export type PanelMesa = "registro" | "chat";

/** Elegir con el siguiente clic la carta (o jugador) a la que se adjunta o apunta. */
export type ModoObjetivo =
	| { tipo: "adjuntar"; carta: IdCarta }
	| { tipo: "flecha"; de: IdCarta; flecha: "ataque" | "objetivo" };

export type Arrastre = {
	ids: IdCarta[];
	/** De dónde salen (para devolverlas a su sitio si no se sueltan en ninguno). */
	desde: "campo" | "mano" | "zona";
	/** El puntero ahora, relativo al tablero. */
	x: number;
	y: number;
	/** El puntero al empezar. */
	x0: number;
	y0: number;
	/**
	 * Ya se soltó: se sigue pintando donde se soltó hasta que llega la carta a su
	 * sitio (el movimiento va y vuelve por Rust), para que no desaparezca un
	 * momento ni aparezca en el de antes.
	 */
	soltado?: boolean;
	/** Se soltó en el campo (se pinta con el tamaño de una carta de campo). */
	alCampo?: boolean;
};

type Pantalla = {
	dialogo: DialogoMesa | null;
	modo: ModoObjetivo | null;
	arrastre: Arrastre | null;
	/** La zona bajo el ratón (biblioteca, cementerio…), para los atajos. */
	zonaSobre: Zona | null;
	/** Sube cada vez que se cambia el tapete, para que el tablero lo relea. */
	tapeteCambiado: number;
	/** El panel lateral abierto (registro o chat). */
	panel: PanelMesa | null;
	avisarTapete: () => void;
	abrir: (d: DialogoMesa | null) => void;
	setPanel: (p: PanelMesa | null) => void;
	setModo: (m: ModoObjetivo | null) => void;
	setArrastre: (a: Arrastre | null) => void;
	setZonaSobre: (z: Zona | null) => void;
};

export const usePantalla = create<Pantalla>((set) => ({
	dialogo: null,
	modo: null,
	arrastre: null,
	zonaSobre: null,
	tapeteCambiado: 0,
	panel: null,
	avisarTapete: () => set((s) => ({ tapeteCambiado: s.tapeteCambiado + 1 })),
	abrir: (dialogo) => set({ dialogo }),
	setPanel: (panel) => set({ panel }),
	setModo: (modo) => set({ modo }),
	setArrastre: (arrastre) => set({ arrastre }),
	setZonaSobre: (zonaSobre) => set({ zonaSobre }),
}));

/** Pide un número con un diálogo pequeño y hace algo con él. */
export function pedirNumero(titulo: string, inicial: number, hacer: (n: number) => void) {
	usePantalla.getState().abrir({ tipo: "numero", titulo, inicial, hacer });
}
