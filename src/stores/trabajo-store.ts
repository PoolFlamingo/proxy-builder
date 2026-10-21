import { create } from "zustand";

/**
 * El trabajo largo en marcha (uno a la vez): resolver una lista, exportar un
 * PDF o completar un idioma de diseño. Lo pintan el pie del panel izquierdo y el botón de exportar, y desde
 * aquí se cancela.
 */
export type TipoTrabajo = "resolver" | "exportar" | "idioma";

export type EstadoTrabajo = {
	tipo: TipoTrabajo;
	jobId: number | null;
	/** Texto ya traducido de la fase («Buscando cartas…»). */
	fase: string;
	hechas: number;
	total: number;
};

type TrabajoStore = {
	actual: EstadoTrabajo | null;
	empezar: (tipo: TipoTrabajo, fase: string) => void;
	setId: (jobId: number) => void;
	avanzar: (fase: string, hechas: number, total: number) => void;
	terminar: () => void;
};

export const useTrabajo = create<TrabajoStore>((set, get) => ({
	actual: null,
	empezar: (tipo, fase) => set({ actual: { tipo, jobId: null, fase, hechas: 0, total: 0 } }),
	setId: (jobId) => {
		const a = get().actual;
		if (a) set({ actual: { ...a, jobId } });
	},
	avanzar: (fase, hechas, total) => {
		const a = get().actual;
		if (a) set({ actual: { ...a, fase, hechas, total } });
	},
	terminar: () => set({ actual: null }),
}));
