import { create } from "zustand";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Lado } from "@/bindings/Lado";
import type { Nota } from "@/bindings/Nota";
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
	| { tipo: "revelar"; preset?: "descubrir" | "cascada" }
	| { tipo: "zona"; jugador: IdJugador; zona: Zona }
	| { tipo: "fichas"; para?: IdJugador }
	| { tipo: "azar" }
	| { tipo: "terminar" }
	| { tipo: "danoComandante" }
	| { tipo: "atajos" }
	/** Escribir una nota (suelta o atada a `carta`) o cambiar `nota`. */
	| { tipo: "nota"; carta: IdCarta | null; nota?: Nota }
	| { tipo: "tapete" };

/** El ancho del panel lateral, en px (la vista grande se aparta de él). */
export const ANCHO_PANEL = 320;

/** El panel de al lado de la mesa: el registro, el chat o las notas (`null`: cerrado). */
export type PanelMesa = "registro" | "chat" | "notas";

/** Elegir con el siguiente clic la carta (o jugador) a la que se adjunta o apunta. */
export type ModoObjetivo =
	| { tipo: "adjuntar"; carta: IdCarta }
	| { tipo: "flecha"; de: IdCarta; flecha: "ataque" | "objetivo" }
	/** Atar esta nota a la carta del campo que se pulse. */
	| { tipo: "nota"; nota: number };

export type Arrastre = {
	ids: IdCarta[];
	/** Lo que va con ellas sin llevarlo el puntero: sus adjuntas y amontonadas. */
	con: IdCarta[];
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
	/** El panel lateral abierto (registro, chat o notas). */
	panel: PanelMesa | null;
	/**
	 * En una partida local, el jugador cuya mano se ha destapado con la cortina
	 * (el que responde fuera de su turno). Cambiar de jugador la vuelve a tapar.
	 */
	manoMostrada: IdJugador | null;
	/** La carta del fantasma (por nombre) que se señala, para verla en grande. */
	fantasma: string | null;
	/**
	 * El tablero está cambiando de tamaño (la ventana, el reparto entre mesas,
	 * un panel): las cartas van a su sitio nuevo sin deslizarse, o se quedan
	 * detrás de la mesa y parece que tiemblan.
	 */
	redimensionando: boolean;
	setRedimensionando: (si: boolean) => void;
	avisarTapete: () => void;
	setFantasma: (nombre: string | null) => void;
	abrir: (d: DialogoMesa | null) => void;
	setPanel: (p: PanelMesa | null) => void;
	mostrarMano: (j: IdJugador | null) => void;
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
	manoMostrada: null,
	fantasma: null,
	redimensionando: false,
	setRedimensionando: (redimensionando) => set({ redimensionando }),
	setFantasma: (fantasma) => set({ fantasma }),
	avisarTapete: () => set((s) => ({ tapeteCambiado: s.tapeteCambiado + 1 })),
	abrir: (dialogo) => set({ dialogo }),
	setPanel: (panel) => set({ panel }),
	mostrarMano: (manoMostrada) => set({ manoMostrada }),
	setModo: (modo) => set({ modo }),
	setArrastre: (arrastre) => set({ arrastre }),
	setZonaSobre: (zonaSobre) => set({ zonaSobre }),
}));

/** Pide un número con un diálogo pequeño y hace algo con él. */
export function pedirNumero(titulo: string, inicial: number, hacer: (n: number) => void) {
	usePantalla.getState().abrir({ tipo: "numero", titulo, inicial, hacer });
}
