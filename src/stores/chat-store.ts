import { create } from "zustand";
import type { AvisoRapido } from "@/bindings/AvisoRapido";
import type { IdJugador } from "@/bindings/IdJugador";

/**
 * **Lo que la pantalla recuerda del chat** (H5): el bocadillo de cada
 * jugador, a quién he silenciado y cuántos mensajes no he leído. Los
 * mensajes en sí salen del registro de la partida; esto es solo de pantalla
 * y nunca viaja (silenciar a alguien lo oculta solo para mí).
 */
export type Bocadillo = {
	/** `seq` del evento: distingue un mensaje nuevo del anterior. */
	seq: number;
	texto: string | null;
	aviso: AvisoRapido | null;
};

type ChatStore = {
	bocadillos: Record<number, Bocadillo | undefined>;
	silenciados: IdJugador[];
	sinLeer: number;
	poner: (jugador: IdJugador, b: Bocadillo) => void;
	/** Quita el bocadillo si sigue siendo ese mensaje (no uno más nuevo). */
	quitar: (jugador: IdJugador, seq: number) => void;
	alternarSilencio: (jugador: IdJugador) => void;
	sumarSinLeer: () => void;
	marcarLeido: () => void;
	limpiar: () => void;
};

export const useChat = create<ChatStore>((set) => ({
	bocadillos: {},
	silenciados: [],
	sinLeer: 0,
	poner: (jugador, b) => set((s) => ({ bocadillos: { ...s.bocadillos, [jugador]: b } })),
	quitar: (jugador, seq) =>
		set((s) =>
			s.bocadillos[jugador]?.seq === seq
				? { bocadillos: { ...s.bocadillos, [jugador]: undefined } }
				: s,
		),
	alternarSilencio: (jugador) =>
		set((s) => ({
			silenciados: s.silenciados.includes(jugador)
				? s.silenciados.filter((x) => x !== jugador)
				: [...s.silenciados, jugador],
		})),
	sumarSinLeer: () => set((s) => ({ sinLeer: s.sinLeer + 1 })),
	marcarLeido: () => set((s) => (s.sinLeer === 0 ? s : { sinLeer: 0 })),
	limpiar: () => set({ bocadillos: {}, silenciados: [], sinLeer: 0 }),
}));
