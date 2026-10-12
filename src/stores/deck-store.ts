import { create } from "zustand";
import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";
import type { PrintSettings } from "@/bindings/PrintSettings";
import { call } from "@/lib/ipc";

/**
 * **El deck abierto.** Un solo store para la pantalla del deck: las cartas,
 * los ajustes de impresión, el historial para deshacer y el estado del
 * autoguardado.
 *
 * 🛑 Todo cambio pasa por `aplicar()`, que apunta el estado anterior en el
 * historial y marca el deck como pendiente de guardar. El guardado lo hace
 * `iniciarAutoguardado()` (800 ms después del último cambio, uno en vuelo
 * cada vez), así que ninguna pantalla llama a `deck_save` por su cuenta.
 */

export type EstadoGuardado = "saved" | "saving" | "dirty" | "error";

/** Lo que se deshace: cartas, ajustes y reverso (no el texto de la lista). */
type Instantanea = Pick<Deck, "cards" | "settings" | "defaultBack">;

const MAX_HISTORIAL = 100;

type DeckStore = {
	deck: Deck | null;
	guardado: EstadoGuardado;
	errorGuardado: unknown;
	/** Versión que crece con cada cambio (el autoguardado la compara). */
	version: number;
	pasado: Instantanea[];
	futuro: Instantanea[];

	cargar: (deck: Deck) => void;
	aplicar: (cambio: (d: Deck) => Partial<Deck>) => void;
	setCartas: (f: (c: DeckCard[]) => DeckCard[]) => void;
	setAjustes: (f: (s: PrintSettings) => PrintSettings) => void;
	/** Cambios que no se deshacen (nombre, texto de la lista). */
	setSinHistorial: (cambio: Partial<Pick<Deck, "name" | "decklistText">>) => void;
	deshacer: () => void;
	rehacer: () => void;
	_marcarGuardado: (estado: EstadoGuardado, error?: unknown) => void;
};

const instantanea = (d: Deck): Instantanea => ({
	cards: d.cards,
	settings: d.settings,
	defaultBack: d.defaultBack,
});

export const useDeck = create<DeckStore>((set, get) => ({
	deck: null,
	guardado: "saved",
	errorGuardado: null,
	version: 0,
	pasado: [],
	futuro: [],

	cargar: (deck) =>
		set({ deck, guardado: "saved", errorGuardado: null, pasado: [], futuro: [], version: 0 }),

	aplicar: (cambio) => {
		const { deck, pasado, version } = get();
		if (!deck) return;
		const parche = cambio(deck);
		set({
			deck: { ...deck, ...parche },
			pasado: [...pasado, instantanea(deck)].slice(-MAX_HISTORIAL),
			futuro: [],
			guardado: "dirty",
			version: version + 1,
		});
	},

	setCartas: (f) => get().aplicar((d) => ({ cards: f(d.cards) })),
	setAjustes: (f) => get().aplicar((d) => ({ settings: f(d.settings) })),

	setSinHistorial: (cambio) => {
		const { deck, version } = get();
		if (!deck) return;
		set({ deck: { ...deck, ...cambio }, guardado: "dirty", version: version + 1 });
	},

	deshacer: () => {
		const { deck, pasado, futuro, version } = get();
		const anterior = pasado.at(-1);
		if (!deck || !anterior) return;
		set({
			deck: { ...deck, ...anterior },
			pasado: pasado.slice(0, -1),
			futuro: [instantanea(deck), ...futuro],
			guardado: "dirty",
			version: version + 1,
		});
	},

	rehacer: () => {
		const { deck, pasado, futuro, version } = get();
		const siguiente = futuro[0];
		if (!deck || !siguiente) return;
		set({
			deck: { ...deck, ...siguiente },
			pasado: [...pasado, instantanea(deck)],
			futuro: futuro.slice(1),
			guardado: "dirty",
			version: version + 1,
		});
	},

	_marcarGuardado: (guardado, error) => set({ guardado, errorGuardado: error ?? null }),
}));

/**
 * Guarda el deck 800 ms después del último cambio. Un solo guardado en vuelo:
 * si llega otro cambio mientras guarda, al terminar se encadena el siguiente
 * con la última versión. Devuelve la función que lo para.
 */
export function iniciarAutoguardado(retardoMs = 800): () => void {
	let temporizador: ReturnType<typeof setTimeout> | null = null;
	let enVuelo = false;
	let pendiente = false;

	const guardar = async () => {
		const { deck, version, _marcarGuardado } = useDeck.getState();
		if (!deck) return;
		if (enVuelo) {
			pendiente = true;
			return;
		}
		enVuelo = true;
		_marcarGuardado("saving");
		try {
			await call("deck_save", { deck });
			// Si hubo cambios mientras guardaba, sigue sucio.
			_marcarGuardado(useDeck.getState().version === version ? "saved" : "dirty");
		} catch (e) {
			_marcarGuardado("error", e);
		} finally {
			enVuelo = false;
			if (pendiente || useDeck.getState().guardado === "dirty") {
				pendiente = false;
				programar();
			}
		}
	};

	const programar = () => {
		if (temporizador) clearTimeout(temporizador);
		temporizador = setTimeout(() => {
			temporizador = null;
			void guardar();
		}, retardoMs);
	};

	const parar = useDeck.subscribe((s, antes) => {
		if (s.version !== antes.version && s.guardado === "dirty") programar();
	});

	return () => {
		parar();
		if (temporizador) clearTimeout(temporizador);
	};
}

/**
 * Abre el último deck usado o, si no hay (o ya no existe), crea uno nuevo.
 */
export async function abrirDeckInicial(nombreNuevo: string): Promise<Deck> {
	const ultimo = await call("app_setting_get", { key: "last_deck_id" });
	let deck: Deck | null = null;
	if (ultimo) {
		try {
			deck = await call("deck_open", { id: ultimo });
		} catch {
			deck = null;
		}
	}
	if (!deck) deck = await call("deck_create", { name: nombreNuevo });
	await call("app_setting_set", { key: "last_deck_id", value: deck.id });
	useDeck.getState().cargar(deck);
	return deck;
}
