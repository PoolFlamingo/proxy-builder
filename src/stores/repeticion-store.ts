import { create } from "zustand";
import type { VistaRepeticion } from "@/bindings/VistaRepeticion";

/**
 * Si se está viendo una repetición (H3) y dónde va. Es la respuesta de Rust
 * (`repeticion_*`); la pantalla de la mesa cambia su barra por la de la
 * repetición mientras haya una.
 */
type Estado = {
	activa: VistaRepeticion | null;
	/**
	 * Cuánto dura (ms) el desplazamiento de las cartas al pasar al siguiente
	 * punto. Rust manda la partida entera de cada punto, sin el movimiento de
	 * por medio: lo suave lo pone la pantalla. 0: sin animar (un salto grande).
	 */
	animarMs: number;
	/** Cuántos saltos sin animar van: las cartas vuelven a apuntar dónde están. */
	saltos: number;
	poner: (v: VistaRepeticion | null) => void;
	animar: (ms: number) => void;
};

export const useRepeticion = create<Estado>((set) => ({
	activa: null,
	animarMs: 0,
	saltos: 0,
	poner: (activa) => set(activa ? { activa } : { activa, animarMs: 0 }),
	animar: (animarMs) =>
		set((s) => ({ animarMs, saltos: animarMs === 0 ? s.saltos + 1 : s.saltos })),
}));
