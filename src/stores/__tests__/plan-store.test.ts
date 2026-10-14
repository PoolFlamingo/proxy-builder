import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Deck } from "@/bindings/Deck";

type Pendiente = { deck: Deck; resolver: (p: unknown) => void; rechazar: (e: unknown) => void };
const pendientes: Pendiente[] = [];
vi.mock("@/lib/ipc", () => ({
	call: vi.fn(
		(_cmd: string, args: { deck: Deck }) =>
			new Promise((resolver, rechazar) => pendientes.push({ deck: args.deck, resolver, rechazar })),
	),
}));

import { huecos } from "@/lib/deck-ops";
import { useDeck } from "@/stores/deck-store";
import { iniciarPlan, usePlan } from "@/stores/plan-store";

function deck(id = "d1"): Deck {
	return {
		id,
		name: "Prueba",
		decklistText: "",
		cards: [],
		settings: {} as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
}

const plan = (n: number) => ({ pages: n }) as never;

describe("plan de páginas", () => {
	let parar: () => void;
	beforeEach(() => {
		vi.useFakeTimers();
		pendientes.length = 0;
		useDeck.setState({ deck: null });
		usePlan.setState({ plan: null, error: null, deDeck: null });
	});
	afterEach(() => {
		parar?.();
		vi.useRealTimers();
	});

	it("al arrancar pide el plan enseguida", async () => {
		useDeck.getState().cargar(deck());
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		expect(pendientes).toHaveLength(1);
		pendientes[0].resolver(plan(1));
		await vi.advanceTimersByTimeAsync(0);
		expect(usePlan.getState().plan).toEqual(plan(1));
		expect(usePlan.getState().deDeck).toBe(pendientes[0].deck);
	});

	it("sin deck, deja el plan vacío y no pide nada", async () => {
		usePlan.setState({ plan: plan(9) });
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		expect(pendientes).toHaveLength(0);
		expect(usePlan.getState().plan).toBeNull();
	});

	it("muchos cambios seguidos piden una sola vez, 100 ms después del último", async () => {
		useDeck.getState().cargar(deck());
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		pendientes.length = 0;
		for (let i = 0; i < 5; i++) {
			useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
			await vi.advanceTimersByTimeAsync(50);
		}
		expect(pendientes).toHaveLength(0);
		await vi.advanceTimersByTimeAsync(50);
		expect(pendientes).toHaveLength(1);
		expect(pendientes[0].deck.cards).toHaveLength(5);
	});

	it("un deck distinto se pide sin esperar", async () => {
		useDeck.getState().cargar(deck("a"));
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		pendientes.length = 0;
		useDeck.getState().cargar(deck("b"));
		await vi.advanceTimersByTimeAsync(0);
		expect(pendientes).toHaveLength(1);
		expect(pendientes[0].deck.id).toBe("b");
	});

	it("cambiar solo el nombre no recalcula", async () => {
		useDeck.getState().cargar(deck());
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		pendientes.length = 0;
		useDeck.getState().setSinHistorial({ name: "Otro" });
		await vi.advanceTimersByTimeAsync(500);
		expect(pendientes).toHaveLength(0);
	});

	it("si las respuestas llegan desordenadas gana la última petición", async () => {
		useDeck.getState().cargar(deck());
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(100);
		expect(pendientes).toHaveLength(2);
		pendientes[1].resolver(plan(2));
		await vi.advanceTimersByTimeAsync(0);
		pendientes[0].resolver(plan(1));
		await vi.advanceTimersByTimeAsync(0);
		expect(usePlan.getState().plan).toEqual(plan(2));
		// Un error viejo tampoco pisa el plan bueno.
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(100);
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(100);
		pendientes[3].resolver(plan(4));
		pendientes[2].rechazar({ code: "Internal", params: {} });
		await vi.advanceTimersByTimeAsync(0);
		expect(usePlan.getState().plan).toEqual(plan(4));
		expect(usePlan.getState().error).toBeNull();
	});

	it("un error deja el último plan bueno y guarda el error", async () => {
		useDeck.getState().cargar(deck());
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		pendientes[0].resolver(plan(1));
		await vi.advanceTimersByTimeAsync(0);
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(100);
		pendientes[1].rechazar({ code: "Internal", params: {} });
		await vi.advanceTimersByTimeAsync(0);
		expect(usePlan.getState().plan).toEqual(plan(1));
		expect(usePlan.getState().error).toEqual({ code: "Internal", params: {} });
	});

	it("parar cancela lo programado y deja de escuchar", async () => {
		useDeck.getState().cargar(deck());
		parar = iniciarPlan(100);
		await vi.advanceTimersByTimeAsync(0);
		pendientes.length = 0;
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		parar();
		await vi.advanceTimersByTimeAsync(500);
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(500);
		expect(pendientes).toHaveLength(0);
	});
});
