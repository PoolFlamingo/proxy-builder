import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CardDesign } from "@/bindings/CardDesign";
import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";

vi.mock("@/lib/ipc", () => ({ call: vi.fn() }));
vi.mock("sonner", () => ({ toast: { error: vi.fn(), success: vi.fn() } }));

import { toast } from "sonner";
import { huecos } from "@/lib/deck-ops";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import { aplicarCambiadas, rehacerMiniaturas, renumerar } from "../renumerar";

const t = (es: string, p?: Record<string, string | number>) =>
	es.replace(/\{(\w+)\}/g, (_, k) => String(p?.[k]));
const mensaje = (e: unknown) => `error: ${(e as { code: string }).code}`;

/** Lo que responde Rust en cada prueba (por comando). */
function responder(f: (cmd: string, args: { cards: DeckCard[]; lang: string }) => unknown) {
	vi.mocked(call).mockImplementation(f as never);
}

function carta(id: string, numero?: string): DeckCard {
	const [b] = huecos(1);
	return {
		...b,
		id,
		source: { kind: "scryfall", scryfallId: `s-${id}` },
		...(numero !== undefined
			? { design: { status: "draft", footer: { number: numero } } as unknown as CardDesign }
			: {}),
	};
}

function cargar(cards: DeckCard[]) {
	const deck = {
		id: "d",
		name: "t",
		decklistText: "",
		cards,
		settings: { designLang: "es" } as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
	useDeck.getState().cargar(deck);
	useUi.getState().anunciar(null);
}
const cartas = () => useDeck.getState().deck?.cards ?? [];
const numero = (c: DeckCard) => c.design?.footer.number;

describe("aplicarCambiadas", () => {
	beforeEach(() => cargar([carta("a", "0001"), carta("b"), carta("c", "0003")]));

	it("cambia solo las cartas que llegan, por id, en un paso de historial", () => {
		aplicarCambiadas([carta("c", "0002"), carta("zz", "9999")]);
		expect(cartas().map((c) => c.id)).toEqual(["a", "b", "c"]);
		expect(numero(cartas()[2])).toBe("0002");
		expect(useDeck.getState().pasado).toHaveLength(1);
		useDeck.getState().deshacer();
		expect(numero(cartas()[2])).toBe("0003");
	});

	it("una lista vacía no apunta nada en el historial", () => {
		aplicarCambiadas([]);
		expect(useDeck.getState().pasado).toHaveLength(0);
		expect(useDeck.getState().guardado).toBe("saved");
	});
});

describe("renumerar", () => {
	beforeEach(() => cargar([carta("a", "0005"), carta("b", "0001")]));

	it("manda las cartas del deck y el idioma del diseño, y aplica lo que vuelve", async () => {
		responder((cmd, args) => {
			expect(cmd).toBe("design_renumber");
			expect(args.lang).toBe("es");
			expect(args.cards.map((c) => c.id)).toEqual(["a", "b"]);
			return [carta("a", "0001"), carta("b", "0002")];
		});
		await renumerar(t, mensaje);
		expect(cartas().map(numero)).toEqual(["0001", "0002"]);
		expect(useUi.getState().ultimaAccion).toBe("2 cartas renumeradas");
	});

	it("si no cambia nada, lo dice y no toca el historial", async () => {
		responder(() => []);
		await renumerar(t, mensaje);
		expect(useUi.getState().ultimaAccion).toBe("La numeración ya estaba al día");
		expect(useDeck.getState().pasado).toHaveLength(0);
	});

	it("un error de Rust sale en un aviso y el deck se queda igual", async () => {
		responder(() => Promise.reject({ code: "Io", params: {} }));
		await renumerar(t, mensaje);
		expect(toast.error).toHaveBeenCalledWith("error: Io");
		expect(cartas().map(numero)).toEqual(["0005", "0001"]);
		expect(useUi.getState().ultimaAccion).toBeNull();
	});

	it("sin deck abierto no llama a Rust", async () => {
		useDeck.setState({ deck: null });
		await renumerar(t, mensaje);
		expect(call).not.toHaveBeenCalled();
	});
});

describe("rehacerMiniaturas", () => {
	it("sin cartas diseñadas no llama a Rust", async () => {
		cargar([carta("a"), carta("b")]);
		await rehacerMiniaturas();
		expect(call).not.toHaveBeenCalled();
	});

	it("sin deck no hace nada", async () => {
		useDeck.setState({ deck: null });
		await rehacerMiniaturas();
		expect(call).not.toHaveBeenCalled();
	});

	it("pone las miniaturas nuevas sin apuntarlo en el historial", async () => {
		cargar([carta("a", "0001"), carta("b")]);
		responder((cmd, args) => {
			expect(cmd).toBe("design_rerender");
			expect(args.lang).toBe("es");
			return [{ ...args.cards[0], meta: { ...args.cards[0].meta, name: "nueva" } }];
		});
		await rehacerMiniaturas();
		expect(cartas()[0].meta.name).toBe("nueva");
		expect(useDeck.getState().pasado).toHaveLength(0);
		expect(useDeck.getState().guardado).toBe("dirty");
	});

	it("si el usuario cambió la carta mientras tanto, gana su cambio", async () => {
		cargar([carta("a", "0001")]);
		responder(async (_cmd, args) => {
			// Mientras Rust pinta, el usuario edita la carta.
			useDeck.getState().setCartas((cs) => cs.map((c) => ({ ...c, locked: true })));
			return [{ ...args.cards[0], meta: { ...args.cards[0].meta, name: "nueva" } }];
		});
		await rehacerMiniaturas();
		expect(cartas()[0].locked).toBe(true);
		expect(cartas()[0].meta.name).toBe("");
	});

	it("un error se calla (se rehará la próxima vez)", async () => {
		cargar([carta("a", "0001")]);
		responder(() => Promise.reject({ code: "Io", params: {} }));
		await expect(rehacerMiniaturas()).resolves.toBeUndefined();
		expect(toast.error).not.toHaveBeenCalled();
	});
});
