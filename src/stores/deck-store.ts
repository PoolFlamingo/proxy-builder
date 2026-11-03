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
	/**
	 * Cambia cartas sin apuntarlo en el historial: para lo que no es un cambio
	 * del usuario (miniaturas de diseño rehechas). Solo toca las cartas que
	 * siguen siendo el mismo objeto que `antes` (si el usuario las cambió
	 * entretanto, gana su cambio).
	 */
	refrescarCartas: (antes: DeckCard[], nuevas: DeckCard[]) => void;
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

	refrescarCartas: (antes, nuevas) => {
		const { deck, version } = get();
		if (!deck || nuevas.length === 0) return;
		const previas = new Map(antes.map((c) => [c.id, c]));
		const porId = new Map(nuevas.map((c) => [c.id, c]));
		let cambiadas = 0;
		const cards = deck.cards.map((c) => {
			const n = porId.get(c.id);
			if (!n || previas.get(c.id) !== c) return c;
			cambiadas++;
			return n;
		});
		if (cambiadas === 0) return;
		set({ deck: { ...deck, cards }, guardado: "dirty", version: version + 1 });
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
		// Rust está reescribiendo este deck: se espera a que lo devuelva (y
		// si al volver se recargó, ya no hay nada que guardar).
		if (prestamo?.id === deck.id) {
			programar();
			return;
		}
		if (useDeck.getState().guardado === "saved") return;
		enVuelo = true;
		_marcarGuardado("saving");
		let fin = () => {};
		guardadoEnVuelo = new Promise<void>((r) => {
			fin = r;
		});
		try {
			await call("deck_save", { deck });
			// Si hubo cambios mientras guardaba, sigue sucio.
			_marcarGuardado(useDeck.getState().version === version ? "saved" : "dirty");
		} catch (e) {
			_marcarGuardado("error", e);
		} finally {
			enVuelo = false;
			fin();
			guardadoEnVuelo = null;
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

/** Guarda ya lo que haya pendiente (antes de cambiar de deck). */
export async function guardarAhora(): Promise<void> {
	const { deck, guardado, version, _marcarGuardado } = useDeck.getState();
	if (!deck || guardado === "saved") return;
	if (prestamo?.id === deck.id) {
		// Mientras Rust lo reemplaza, lo de la pantalla ya no vale; mientras
		// le completa las imágenes, se guarda mezclado con lo que ya bajó.
		if (!prestamo.conservar) return;
		const enRust = await call("deck_open", { id: deck.id });
		const actual = useDeck.getState().deck ?? deck;
		await call("deck_save", {
			deck: { ...actual, cards: mezclarCartas(prestamo.cartas, actual.cards, enRust.cards) },
		});
		return;
	}
	await call("deck_save", { deck });
	if (useDeck.getState().version === version) _marcarGuardado("saved");
}

/**
 * **Prestar el deck abierto a Rust.** Cuando Rust escribe en la base un deck
 * con su id (reemplazarlo al importar o recibir otra versión, completar las
 * imágenes que faltan), el autoguardado de la pantalla lo pisaría con la
 * copia vieja. Mientras dura el préstamo no se guarda nada de la pantalla.
 */
type Prestamo = {
	id: string;
	/** Completar: lo que la persona cambie entretanto se conserva. */
	conservar: boolean;
	/** Las cartas al empezar: las que siguen siendo el mismo objeto no se han tocado. */
	cartas: DeckCard[];
	version: number;
	veces: number;
};
let prestamo: Prestamo | null = null;
let guardadoEnVuelo: Promise<void> | null = null;
/** La vuelta del último préstamo (volver a leer el deck), mientras dura. */
let devolviendo: Promise<void> | null = null;

/** Lo que devuelve el préstamo: vuelve a leer el deck de Rust. */
export type Devolver = () => Promise<void>;

/**
 * Las cartas tras el préstamo con `conservar`: la de Rust para las que la
 * persona no tocó (siguen siendo el mismo objeto que `antes`), la suya para
 * las demás.
 */
export function mezclarCartas(
	antes: DeckCard[],
	actuales: DeckCard[],
	deRust: DeckCard[],
): DeckCard[] {
	const previas = new Map(antes.map((c) => [c.id, c]));
	const porId = new Map(deRust.map((c) => [c.id, c]));
	return actuales.map((c) => {
		const r = porId.get(c.id);
		return r && previas.get(c.id) === c ? r : c;
	});
}

/**
 * Antes de que Rust escriba el deck `id`: si es el abierto, guarda lo
 * pendiente, para el autoguardado y devuelve la función que, al terminar
 * (bien o mal), lo vuelve a leer de Rust. Si no es el abierto, `null`.
 * `conservar: false` (reemplazar) tira lo de la pantalla y carga lo de Rust
 * sin historial; `true` (completar) mezcla con `mezclarCartas`.
 */
export async function prestarDeck(
	id: string,
	{ conservar }: { conservar: boolean },
): Promise<Devolver | null> {
	while (devolviendo) await devolviendo;
	if (useDeck.getState().deck?.id !== id) return null;
	if (guardadoEnVuelo) await guardadoEnVuelo;
	if (prestamo?.id === id && prestamo.veces > 0) {
		prestamo.veces += 1;
		prestamo.conservar &&= conservar;
	} else {
		await guardarAhora();
		const s = useDeck.getState();
		if (s.deck?.id !== id) return null;
		prestamo = { id, conservar, cartas: s.deck.cards, version: s.version, veces: 1 };
	}
	const p = prestamo;
	let devuelto = false;
	return async () => {
		if (devuelto || prestamo !== p) return;
		devuelto = true;
		p.veces -= 1;
		if (p.veces > 0) return;
		const vuelta = (async () => {
			try {
				if (useDeck.getState().deck?.id !== id) return;
				const enRust = await call("deck_open", { id });
				const ahora = useDeck.getState();
				if (ahora.deck?.id !== id) return;
				if (!p.conservar || ahora.version === p.version) ahora.cargar(enRust);
				else ahora.refrescarCartas(p.cartas, enRust.cards);
			} catch {
				// Si ya no existe o no se puede leer, se queda lo de la pantalla.
			} finally {
				prestamo = null;
			}
		})();
		devolviendo = vuelta;
		await vuelta;
		if (devolviendo === vuelta) devolviendo = null;
	};
}

/** Abre otro deck de la biblioteca (guardando antes el actual). */
export async function abrirDeck(id: string): Promise<Deck> {
	await guardarAhora();
	const deck = await call("deck_open", { id });
	await call("app_setting_set", { key: "last_deck_id", value: deck.id });
	useDeck.getState().cargar(deck);
	return deck;
}
