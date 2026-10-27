import { create } from "zustand";
import type { Ausente } from "@/bindings/Ausente";
import type { Enlace } from "@/bindings/Enlace";
import type { EnSala } from "@/bindings/EnSala";
import type { PapelRed } from "@/bindings/PapelRed";

/** Cómo va la conexión con la partida en red (para el aviso de la mesa). */
export type EstadoConexion = "conectada" | "reconectando" | "perdida";

/** Un jugador que falta de la partida: desde hace `haceMs` cuando se supo, en `visto` (reloj de aquí). */
export type AusenteVisto = Ausente & { visto: number };

/**
 * **La partida en red** en pantalla: el papel de esta app, el código para
 * invitar (el anfitrión), quién hay en la sala y cómo va la conexión. Lo
 * llena `features/mesa/red/conexion-red.ts` con lo que dice Rust
 * (`red_estado` y los avisos `red://aviso`).
 */
type RedStore = {
	papel: PapelRed | null;
	codigo: string | null;
	sala: EnSala[];
	empezada: boolean;
	conexion: EstadoConexion;
	/** El intento de reconexión en curso (1–5). */
	intento: number;
	/**
	 * Lo que hay que sumar a la hora de aquí (`Date.now()`) para tener la del
	 * anfitrión (E10): el temporizador del turno cuenta desde la suya.
	 */
	desfaseMs: number;
	/** Cómo va el enlace de cada jugador con el anfitrión (RTT y camino). */
	enlaces: Enlace[];
	/** Los jugadores que no están conectados con el anfitrión (G8). */
	ausentes: AusenteVisto[];
	poner: (cambios: Partial<Omit<RedStore, "poner" | "olvidar">>) => void;
	olvidar: () => void;
};

const VACIA = {
	papel: null,
	codigo: null,
	sala: [],
	empezada: false,
	conexion: "conectada" as const,
	intento: 0,
	desfaseMs: 0,
	enlaces: [] as Enlace[],
	ausentes: [] as AusenteVisto[],
};

export const useRed = create<RedStore>((set) => ({
	...VACIA,
	poner: (cambios) => set(cambios),
	olvidar: () => set(VACIA),
}));
