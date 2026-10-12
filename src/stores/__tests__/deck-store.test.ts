import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Deck } from "@/bindings/Deck";

const llamadas: { cmd: string; args: unknown }[] = [];
let fallar = false;
vi.mock("@/lib/ipc", () => ({
	call: vi.fn(async (cmd: string, args: unknown) => {
		llamadas.push({ cmd, args });
		if (fallar) throw { code: "Db", params: {} };
		return { updatedAt: 1 };
	}),
}));

import { huecos } from "@/lib/deck-ops";
import { iniciarAutoguardado, useDeck } from "@/stores/deck-store";

function deckDePrueba(): Deck {
	return {
		id: "d1",
		name: "Prueba",
		decklistText: "",
		cards: [],
		settings: {} as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
}

describe("store del deck", () => {
	let parar: () => void;
	beforeEach(() => {
		vi.useFakeTimers();
		llamadas.length = 0;
		fallar = false;
		useDeck.getState().cargar(deckDePrueba());
		parar = iniciarAutoguardado(800);
	});
	afterEach(() => {
		parar();
		vi.useRealTimers();
	});

	it("cinco cambios rápidos se guardan una sola vez", async () => {
		for (let i = 0; i < 5; i++) useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		expect(useDeck.getState().guardado).toBe("dirty");
		await vi.advanceTimersByTimeAsync(799);
		expect(llamadas).toHaveLength(0);
		await vi.advanceTimersByTimeAsync(1);
		expect(llamadas).toHaveLength(1);
		expect((llamadas[0].args as { deck: Deck }).deck.cards).toHaveLength(5);
		expect(useDeck.getState().guardado).toBe("saved");
	});

	it("deshacer tres veces vuelve al estado exacto y rehacer lo repone", () => {
		const s = useDeck.getState;
		s().setCartas((c) => [...c, ...huecos(1)]);
		const tras1 = s().deck?.cards;
		s().setCartas((c) => [...c, ...huecos(1)]);
		s().setCartas((c) => [...c, ...huecos(1)]);
		s().setCartas((c) => [...c, ...huecos(1)]);
		s().deshacer();
		s().deshacer();
		s().deshacer();
		expect(s().deck?.cards).toBe(tras1);
		s().rehacer();
		expect(s().deck?.cards).toHaveLength(2);
		// Un cambio nuevo borra lo que se podía rehacer.
		s().setCartas((c) => c.slice(0, 1));
		expect(s().futuro).toHaveLength(0);
	});

	it("si falla el guardado queda en error y reintenta con el siguiente cambio", async () => {
		fallar = true;
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(800);
		expect(useDeck.getState().guardado).toBe("error");
		fallar = false;
		useDeck.getState().setCartas((c) => [...c, ...huecos(1)]);
		await vi.advanceTimersByTimeAsync(800);
		expect(useDeck.getState().guardado).toBe("saved");
		expect(llamadas).toHaveLength(2);
	});

	it("los cambios sin historial se guardan pero no se deshacen", async () => {
		useDeck.getState().setSinHistorial({ name: "Nuevo nombre" });
		expect(useDeck.getState().pasado).toHaveLength(0);
		await vi.advanceTimersByTimeAsync(800);
		expect((llamadas[0].args as { deck: Deck }).deck.name).toBe("Nuevo nombre");
	});
});
