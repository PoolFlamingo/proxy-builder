import { create } from "zustand";
import type { OriginalInfo } from "@/bindings/OriginalInfo";
import { call, on } from "@/lib/ipc";

/**
 * **Los marcos «Original»** (los de imagen de Card Conjurer): si están
 * descargados y la descarga en marcha. Rust los baja la primera vez que se
 * piden (`original_download`); `version` sube al terminar para que quien
 * lista los packs los vuelva a pedir.
 */
export const ID_ORIGINAL = "original";

type Estado = {
	info: OriginalInfo | null;
	progreso: { hechas: number; total: number } | null;
	version: number;
	cargar: () => Promise<void>;
	/** Descarga lo que falte. Rechaza con el error de Rust. */
	descargar: () => Promise<void>;
};

export const useMarcosOriginales = create<Estado>((set, get) => ({
	info: null,
	progreso: null,
	version: 0,
	cargar: async () => {
		try {
			set({ info: await call("original_info", {}) });
		} catch {
			/* sin datos: como no descargados */
		}
	},
	descargar: async () => {
		if (get().progreso) return;
		set({ progreso: { hechas: 0, total: get().info?.total ?? 0 } });
		const dejar = await on("original://progress", (p) =>
			set({ progreso: { hechas: p.done, total: p.total } }),
		);
		try {
			await call("original_download", {});
			set((s) => ({ version: s.version + 1 }));
		} finally {
			dejar();
			set({ progreso: null });
			await get().cargar();
		}
	},
}));
