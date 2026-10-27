import { create } from "zustand";
import type { ActualizacionInfo } from "@/bindings/ActualizacionInfo";
import { call, on } from "@/lib/ipc";
import { guardarAhora } from "@/stores/deck-store";
import { useMesa } from "@/stores/mesa-store";

/**
 * **La actualización de la app**: en qué paso va. Rust hace el trabajo
 * (`src-tauri/src/actualizar.rs`); esto solo lo recuerda para Ajustes, el
 * aviso del arranque y el botón de la barra superior.
 */
type Paso = "inactiva" | "buscando" | "al-dia" | "disponible" | "bajando" | "lista" | "instalando";

type ActualizacionStore = {
	/** `null` hasta preguntarlo a Rust; `false` en desarrollo. */
	soportada: boolean | null;
	actual: string | null;
	paso: Paso;
	info: ActualizacionInfo | null;
	/** 0–100 mientras se baja; `null` si no se sabe el tamaño. */
	progreso: number | null;
	/** El último error (un `AppError`), hasta el siguiente intento. */
	error: unknown;
	/**
	 * Se pidió instalar con la mesa abierta: se instalará al salir de ella
	 * (`useAvisosActualizacion` lo lanza). Reiniciar a media partida la
	 * cortaría, y en red, para todos.
	 */
	alSalirDeLaMesa: boolean;
	/** Pregunta a Rust si se puede actualizar en este equipo (una vez). */
	iniciar: () => Promise<boolean>;
	/** Busca una versión nueva; devuelve la encontrada. */
	buscar: () => Promise<ActualizacionInfo | null>;
	/** Baja la encontrada; `true` si queda lista para instalar. */
	bajar: () => Promise<boolean>;
	/**
	 * Guarda el deck, instala y reinicia. Lanza el error si falla. Con la
	 * mesa abierta no instala: lo deja para cuando se salga de ella.
	 */
	instalar: () => Promise<void>;
};

const OCUPADA: Paso[] = ["buscando", "bajando", "instalando"];

export const useActualizacion = create<ActualizacionStore>((set, get) => ({
	soportada: null,
	actual: null,
	paso: "inactiva",
	info: null,
	progreso: null,
	error: null,
	alSalirDeLaMesa: false,

	iniciar: async () => {
		const { soportada } = get();
		if (soportada !== null) return soportada;
		try {
			const e = await call("update_status", {});
			set({ soportada: e.soportada, actual: e.actual });
			return e.soportada;
		} catch {
			set({ soportada: false });
			return false;
		}
	},

	buscar: async () => {
		const { paso, info } = get();
		if (OCUPADA.includes(paso)) return null;
		// Ya bajada: no se vuelve a bajar hasta reiniciar.
		if (paso === "lista") return info;
		set({ paso: "buscando", error: null });
		try {
			const nueva = await call("update_check", {});
			set({ paso: nueva ? "disponible" : "al-dia", info: nueva });
			return nueva;
		} catch (e) {
			set({ paso: info ? "disponible" : "inactiva", error: e });
			return null;
		}
	},

	bajar: async () => {
		const { paso, info } = get();
		if (paso !== "disponible" || !info) return paso === "lista";
		set({ paso: "bajando", progreso: null, error: null });
		const dejar = await on("update://progress", (p) => {
			if (p.total > 0) set({ progreso: Math.min(100, Math.round((p.hecho / p.total) * 100)) });
		});
		try {
			await call("update_download", { version: info.version });
			set({ paso: "lista" });
			return true;
		} catch (e) {
			set({ paso: "disponible", error: e });
			return false;
		} finally {
			dejar();
			set({ progreso: null });
		}
	},

	instalar: async () => {
		const { paso, info } = get();
		if (paso !== "lista" || !info) return;
		if (useMesa.getState().activa) {
			set({ alSalirDeLaMesa: true });
			return;
		}
		set({ paso: "instalando", error: null, alSalirDeLaMesa: false });
		try {
			await guardarAhora();
			// Si va bien, la app se reinicia y esto no vuelve.
			await call("update_install", { version: info.version });
		} catch (e) {
			// Rust también lo vigila: una partida que la pantalla no conocía.
			if (esperaALaPartida(e)) {
				set({ paso: "lista", alSalirDeLaMesa: true });
				return;
			}
			set({ paso: "lista", error: e });
			throw e;
		}
	},
}));

function esperaALaPartida(e: unknown): boolean {
	const x = e as { code?: string; params?: { motivo?: string } } | null;
	return x?.code === "Busy" && x.params?.motivo === "partida";
}
