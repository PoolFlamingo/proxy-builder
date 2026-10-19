import { create } from "zustand";
import type { DescargableInfo } from "@/bindings/DescargableInfo";
import { call, on } from "@/lib/ipc";

/**
 * **Los packs de marcos descargables** del repositorio de packs (Balatro):
 * cómo están y la descarga en marcha. `version` sube al terminar una descarga
 * para que quien lista los packs los vuelva a pedir.
 */
type Estado = {
	lista: DescargableInfo[] | null;
	progreso: { id: string; hechas: number; total: number } | null;
	version: number;
	cargar: () => Promise<void>;
	/** Descarga lo que falte de un pack. Rechaza con el error de Rust. */
	descargar: (id: string) => Promise<void>;
};

export const usePacksDescargables = create<Estado>((set, get) => ({
	lista: null,
	progreso: null,
	version: 0,
	cargar: async () => {
		try {
			set({ lista: await call("packs_downloadable_list", {}) });
		} catch {
			set({ lista: [] });
		}
	},
	descargar: async (id) => {
		if (get().progreso) return;
		set({ progreso: { id, hechas: 0, total: 0 } });
		const dejar = await on("packs://progress", (p) => {
			if (p.id === id) set({ progreso: { id, hechas: p.done, total: p.total } });
		});
		try {
			await call("pack_downloadable_download", { id });
			set((s) => ({ version: s.version + 1 }));
		} finally {
			dejar();
			set({ progreso: null });
			await get().cargar();
		}
	},
}));
