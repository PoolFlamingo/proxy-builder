import { create } from "zustand";

/**
 * En el móvil la barra lateral de etiquetas no ocupa un lado: sale de abajo en
 * una hoja que abre el botón «Etiquetas» de la barra de herramientas. Este
 * estado los une sin que las dos piezas se conozcan (la de la mesa, «Jugar»,
 * también las usa).
 */
export const useLateralMovil = create<{
	abierto: boolean;
	poner: (abierto: boolean) => void;
}>((set) => ({
	abierto: false,
	poner: (abierto) => set({ abierto }),
}));
