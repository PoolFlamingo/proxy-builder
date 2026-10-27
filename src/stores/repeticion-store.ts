import { create } from "zustand";
import type { VistaRepeticion } from "@/bindings/VistaRepeticion";

/**
 * Si se está viendo una repetición (H3) y dónde va. Es la respuesta de Rust
 * (`repeticion_*`); la pantalla de la mesa cambia su barra por la de la
 * repetición mientras haya una.
 */
type Estado = {
	activa: VistaRepeticion | null;
	poner: (v: VistaRepeticion | null) => void;
};

export const useRepeticion = create<Estado>((set) => ({
	activa: null,
	poner: (activa) => set({ activa }),
}));
