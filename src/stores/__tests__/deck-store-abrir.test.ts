import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";

const llamadas: { cmd: string; args: Record<string, unknown> }[] = [];
let ultimo: string | null = null;
const existentes = new Map<string, Deck>();
vi.mock("@/lib/ipc", () => ({
	call: vi.fn(async (cmd: string, args: Record<string, unknown>) => {
		llamadas.push({ cmd, args });
		switch (cmd) {
			case "app_setting_get":
				return ultimo;
			case "app_setting_set":
				ultimo = args.value as string;
				return null;
			case "deck_open": {
				const d = existentes.get(args.id as string);
				if (!d) throw { code: "NotFound", params: {} };
				return d;
			}
			case "deck_create":
				return deck("nuevo", args.name as string);
			case "deck_save":
				return { updatedAt: 1 };
		}
		throw new Error(`orden inesperada ${cmd}`);
	}),
}));

import { huecos } from "@/lib/deck-ops";
import {
	abrirDeck,
	abrirDeckInicial,
	guardarAhora,
	iniciarAutoguardado,
	mezclarCartas,
	prestarDeck,
	useDeck,
} from "@/stores/deck-store";

function deck(id: string, name = "Deck"): Deck {
	return {
		id,
		name,
		decklistText: "",
		cards: [],
		settings: {} as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
}

const ordenes = () => llamadas.map((l) => l.cmd);

describe("abrir y guardar decks", () => {
	beforeEach(() => {
		llamadas.length = 0;
		ultimo = null;
		existentes.clear();
		useDeck.setState({ deck: null, guardado: "saved", version: 0, pasado: [], futuro: [] });
	});

	it("abre el último deck usado", async () => {
		existentes.set("d1", deck("d1", "Elfos"));
		ultimo = "d1";
		const d = await abrirDeckInicial("Nuevo deck");
		expect(d.name).toBe("Elfos");
		expect(useDeck.getState().deck?.id).toBe("d1");
		expect(ordenes()).not.toContain("deck_create");
	});

	it("sin último deck crea uno con el nombre dado y lo recuerda", async () => {
		const d = await abrirDeckInicial("Nuevo deck");
		expect(d.name).toBe("Nuevo deck");
		expect(ultimo).toBe("nuevo");
		expect(useDeck.getState().deck?.id).toBe("nuevo");
	});

	it("si el último deck ya no existe, crea uno nuevo", async () => {
		ultimo = "borrado";
		const d = await abrirDeckInicial("Nuevo deck");
		expect(d.id).toBe("nuevo");
		expect(ordenes()).toEqual(["app_setting_get", "deck_open", "deck_create", "app_setting_set"]);
	});

	it("guardarAhora no hace nada si está guardado o no hay deck", async () => {
		await guardarAhora();
		useDeck.getState().cargar(deck("d1"));
		await guardarAhora();
		expect(ordenes()).toEqual([]);
	});

	it("guardarAhora guarda lo pendiente y lo marca guardado", async () => {
		useDeck.getState().cargar(deck("d1"));
		useDeck.getState().setCartas((c) => [...c, ...huecos(2)]);
		await guardarAhora();
		expect(ordenes()).toEqual(["deck_save"]);
		expect((llamadas[0].args.deck as Deck).cards).toHaveLength(2);
		expect(useDeck.getState().guardado).toBe("saved");
	});

	it("abrir otro deck guarda antes el actual, lo carga sin historial y lo recuerda", async () => {
		existentes.set("d2", deck("d2", "Goblins"));
		useDeck.getState().cargar(deck("d1"));
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		const d = await abrirDeck("d2");
		expect(ordenes()).toEqual(["deck_save", "deck_open", "app_setting_set"]);
		expect(d.name).toBe("Goblins");
		expect(useDeck.getState().pasado).toHaveLength(0);
		expect(useDeck.getState().guardado).toBe("saved");
		expect(ultimo).toBe("d2");
	});

	it("si abrir falla, el deck actual sigue abierto", async () => {
		useDeck.getState().cargar(deck("d1"));
		await expect(abrirDeck("no-existe")).rejects.toEqual({ code: "NotFound", params: {} });
		expect(useDeck.getState().deck?.id).toBe("d1");
	});
});

describe("refrescarCartas", () => {
	const carta = (id: string, extra: Partial<DeckCard> = {}) =>
		({ ...huecos(1)[0], id, ...extra }) as DeckCard;

	beforeEach(() => {
		useDeck.getState().cargar({ ...deck("d1"), cards: [carta("a"), carta("b"), carta("c")] });
	});

	it("cambia las cartas que siguen igual, sin entrar en el historial", () => {
		const antes = useDeck.getState().deck?.cards ?? [];
		const nueva = carta("b", { locked: true });
		useDeck.getState().refrescarCartas(antes, [nueva]);
		const s = useDeck.getState();
		expect(s.deck?.cards[1]).toBe(nueva);
		expect(s.deck?.cards[0]).toBe(antes[0]);
		expect(s.pasado).toHaveLength(0);
		expect(s.guardado).toBe("dirty");
		expect(s.version).toBe(1);
	});

	it("si el usuario cambió la carta entretanto, gana su cambio", () => {
		const antes = useDeck.getState().deck?.cards ?? [];
		useDeck
			.getState()
			.setCartas((cs) => cs.map((c) => (c.id === "b" ? { ...c, locked: true } : c)));
		const delUsuario = useDeck.getState().deck?.cards[1];
		const version = useDeck.getState().version;
		useDeck.getState().refrescarCartas(antes, [carta("b", { meta: {} as DeckCard["meta"] })]);
		expect(useDeck.getState().deck?.cards[1]).toBe(delUsuario);
		// Nada cambió: ni versión ni guardado.
		expect(useDeck.getState().version).toBe(version);
	});

	it("ignora las cartas que ya no están en el deck y las listas vacías", () => {
		const antes = useDeck.getState().deck?.cards ?? [];
		useDeck.getState().refrescarCartas(antes, [carta("z")]);
		useDeck.getState().refrescarCartas(antes, []);
		expect(useDeck.getState().version).toBe(0);
		expect(useDeck.getState().guardado).toBe("saved");
	});

	it("sin deck no hace nada", () => {
		useDeck.setState({ deck: null });
		expect(() => useDeck.getState().refrescarCartas([], [carta("a")])).not.toThrow();
	});
});

describe("historial", () => {
	it("deshacer y rehacer sin nada que hacer no cambian nada", () => {
		useDeck.getState().cargar(deck("d1"));
		useDeck.getState().deshacer();
		useDeck.getState().rehacer();
		expect(useDeck.getState().version).toBe(0);
		expect(useDeck.getState().guardado).toBe("saved");
	});

	it("los cambios sin deck se ignoran", () => {
		useDeck.setState({ deck: null, version: 0 });
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		useDeck.getState().setSinHistorial({ name: "x" });
		expect(useDeck.getState().deck).toBeNull();
		expect(useDeck.getState().version).toBe(0);
	});

	it("el historial no crece sin límite", () => {
		useDeck.getState().cargar(deck("d1"));
		for (let i = 0; i < 500; i++) useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		const n = useDeck.getState().pasado.length;
		expect(n).toBeGreaterThan(0);
		expect(n).toBeLessThan(500);
	});

	it("deshacer también devuelve los ajustes y el dorso", () => {
		useDeck.getState().cargar(deck("d1"));
		const ajustes = useDeck.getState().deck?.settings;
		useDeck.getState().setAjustes((s) => ({ ...s, fillCorners: true }));
		useDeck.getState().aplicar(() => ({ defaultBack: { blob: "x" } as Deck["defaultBack"] }));
		useDeck.getState().deshacer();
		useDeck.getState().deshacer();
		expect(useDeck.getState().deck?.settings).toBe(ajustes);
		expect(useDeck.getState().deck?.defaultBack).toBeNull();
	});
});

describe("prestar el deck abierto a Rust", () => {
	const carta = (id: string, extra: Partial<DeckCard> = {}) =>
		({ ...huecos(1)[0], id, ...extra }) as DeckCard;

	beforeEach(() => {
		llamadas.length = 0;
		existentes.clear();
		vi.useRealTimers();
	});

	it("si no es el deck abierto, no hace nada", async () => {
		useDeck.getState().cargar(deck("d1"));
		expect(await prestarDeck("d2", { conservar: false })).toBeNull();
		expect(ordenes()).toEqual([]);
	});

	it("reemplazar: guarda lo pendiente, no guarda durante y recarga lo de Rust sin historial", async () => {
		useDeck.getState().cargar(deck("d1"));
		useDeck.getState().setCartas((c) => [...c, carta("vieja")]);
		const devolver = await prestarDeck("d1", { conservar: false });
		expect(ordenes()).toEqual(["deck_save"]);
		// Mientras Rust lo reemplaza, ni guardarAhora ni el autoguardado lo pisan.
		useDeck.getState().setCartas((c) => [...c, carta("otra")]);
		await guardarAhora();
		expect(ordenes()).toEqual(["deck_save"]);
		existentes.set("d1", { ...deck("d1"), cards: [carta("nueva")] });
		await devolver?.();
		const s = useDeck.getState();
		expect(s.deck?.cards.map((c) => c.id)).toEqual(["nueva"]);
		expect(s.pasado).toHaveLength(0);
		expect(s.guardado).toBe("saved");
		// Devolver dos veces no hace nada más.
		await devolver?.();
		expect(ordenes()).toEqual(["deck_save", "deck_open"]);
	});

	it("el autoguardado espera mientras está prestado", async () => {
		vi.useFakeTimers();
		const parar = iniciarAutoguardado(10);
		useDeck.getState().cargar(deck("d1"));
		const devolver = await prestarDeck("d1", { conservar: false });
		useDeck.getState().setCartas((c) => [...c, carta("x")]);
		await vi.advanceTimersByTimeAsync(50);
		expect(ordenes()).not.toContain("deck_save");
		existentes.set("d1", deck("d1"));
		await devolver?.();
		await vi.advanceTimersByTimeAsync(50);
		expect(ordenes()).toEqual(["deck_open"]);
		parar();
		vi.useRealTimers();
	});

	it("completar: conserva lo que la persona cambió y toma lo de Rust para lo demás", async () => {
		useDeck.getState().cargar({ ...deck("d1"), cards: [carta("a"), carta("b")] });
		const devolver = await prestarDeck("d1", { conservar: true });
		useDeck
			.getState()
			.setCartas((cs) => cs.map((c) => (c.id === "b" ? { ...c, locked: true } : c)));
		existentes.set("d1", {
			...deck("d1"),
			cards: [carta("a", { meta: { lang: "fr" } as DeckCard["meta"] }), carta("b")],
		});
		// Guardar entretanto mezcla con lo que ya está en Rust.
		await guardarAhora();
		const guardado = llamadas.find((l) => l.cmd === "deck_save")?.args.deck as Deck;
		expect(guardado.cards[0].meta).toEqual({ lang: "fr" });
		expect(guardado.cards[1].locked).toBe(true);
		await devolver?.();
		const cs = useDeck.getState().deck?.cards ?? [];
		expect(cs[0].meta).toEqual({ lang: "fr" });
		expect(cs[1].locked).toBe(true);
	});

	it("mezclarCartas", () => {
		const a = carta("a");
		const b = carta("b");
		const b2 = { ...b, locked: true };
		const deRust = [carta("a", { locked: true }), carta("b")];
		const r = mezclarCartas([a, b], [a, b2, carta("c")], deRust);
		expect(r[0]).toBe(deRust[0]);
		expect(r[1]).toBe(b2);
		expect(r[2].id).toBe("c");
	});
});
