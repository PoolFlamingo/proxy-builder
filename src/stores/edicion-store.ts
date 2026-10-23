import { create } from "zustand";

/**
 * **La carta que se está diseñando y tiene cambios sin guardar** (la vista
 * «Diseñar» la apunta y la quita). Salir de ella (cambiar de carta, de
 * pantalla o cerrar la app) pregunta, y con esto la pregunta puede ofrecer
 * «Guardar y continuar»: quien pregunta no sabe cómo se guarda, lo sabe ella.
 */
export type EdicionPendiente = {
	/** Guarda la carta; `false` si no se pudo (el aviso ya salió). */
	guardar: () => Promise<boolean>;
};

type EdicionStore = {
	pendiente: EdicionPendiente | null;
	poner: (p: EdicionPendiente | null) => void;
};

export const useEdicion = create<EdicionStore>((set) => ({
	pendiente: null,
	poner: (pendiente) => set({ pendiente }),
}));
