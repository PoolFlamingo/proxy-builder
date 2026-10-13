import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Deck } from "@/bindings/Deck";

vi.mock("@/lib/ipc", () => ({
	call: vi.fn(async (cmd: string, args: { cards?: { id: string }[] }) => {
		// «Auto ordenar» de mentira: al revés.
		if (cmd === "arrange_cards") return (args.cards ?? []).map((c) => c.id).reverse();
		return null;
	}),
}));

import { huecos } from "@/lib/deck-ops";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import {
	autoOrdenar,
	moverA,
	moverUno,
	objetivo,
	seleccionarConClic,
	soltarSobre,
} from "../acciones-carta";

let n = 0;
function cargar(k: number) {
	n = 0;
	const cards = huecos(k, () => `c${n++}`);
	const deck = {
		id: "d",
		name: "t",
		decklistText: "",
		cards,
		settings: {} as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
	useDeck.getState().cargar(deck);
	useUi.getState().seleccionar([], null);
}
const ids = () => useDeck.getState().deck?.cards.map((c) => c.id);

describe("acciones sobre cartas", () => {
	beforeEach(() => cargar(12));

	it("arrastrar la carta 1 a la 12 la pasa a la segunda página", () => {
		soltarSobre(["c0"], "c11");
		expect(ids()?.at(-1)).toBe("c0");
		expect(ids()?.[0]).toBe("c1");
	});

	it("arrastrar hacia atrás la deja delante del destino", () => {
		soltarSobre(["c5"], "c1");
		expect(ids()?.slice(0, 3)).toEqual(["c0", "c5", "c1"]);
	});

	it("soltar en una casilla vacía la manda al final", () => {
		soltarSobre(["c2", "c3"], null);
		expect(ids()?.slice(-2)).toEqual(["c2", "c3"]);
	});

	it("mover antes / después y a una posición", () => {
		moverUno(["c3"], -1);
		expect(ids()?.slice(2, 4)).toEqual(["c3", "c2"]);
		moverA(["c0"], 5);
		expect(ids()?.[4]).toBe("c0");
		moverUno(["c11"], 1);
		expect(ids()?.at(-1)).toBe("c11");
	});

	it("se deshace con el historial del deck", () => {
		soltarSobre(["c0"], "c11");
		useDeck.getState().deshacer();
		expect(ids()?.[0]).toBe("c0");
	});

	it("selección con Ctrl y con Mayús; el clic derecho actúa sobre la selección", () => {
		const sin = { ctrlKey: false, metaKey: false, shiftKey: false };
		seleccionarConClic("c2", sin);
		seleccionarConClic("c5", { ...sin, shiftKey: true });
		expect(useUi.getState().seleccion).toEqual(["c2", "c3", "c4", "c5"]);
		seleccionarConClic("c3", { ...sin, ctrlKey: true });
		expect(useUi.getState().seleccion).toEqual(["c2", "c4", "c5"]);
		expect(objetivo("c4")).toEqual(["c2", "c4", "c5"]);
		expect(objetivo("c9")).toEqual(["c9"]);
	});

	it("auto ordenar aplica el orden que devuelve Rust", async () => {
		await autoOrdenar((s) => s);
		expect(ids()?.[0]).toBe("c11");
	});
});
