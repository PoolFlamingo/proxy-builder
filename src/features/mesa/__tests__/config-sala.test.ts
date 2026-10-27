import { describe, expect, it } from "vitest";
import type { EnSala } from "@/bindings/EnSala";
import { mismoOrden, sentados, sincronizarAsientos } from "../config-sala";

const fila = (id: number, rol: "jugador" | "espectador", deck: string | null): EnSala => ({
	id,
	apodo: `J${id}`,
	rol,
	deck,
});

describe("sentados", () => {
	it("son el anfitrión y los jugadores con deck, no los que miran ni los sin deck", () => {
		const sala = [
			fila(0, "jugador", null),
			fila(3, "jugador", "Mazo"),
			fila(4, "espectador", null),
			fila(5, "jugador", null),
			fila(6, "jugador", "Otro"),
		];
		// El anfitrión se sienta aunque aún no haya elegido nombre de deck.
		expect(sentados(sala)).toEqual([0, 3, 6]);
	});
});

describe("sincronizarAsientos", () => {
	it("sin cambios deja el mismo orden", () => {
		expect(sincronizarAsientos([0, 3, 2, 1], [0, 1, 2, 3])).toEqual({
			ids: [0, 3, 2, 1],
			quitados: [],
		});
	});

	it("los que llegan se sientan al final", () => {
		expect(sincronizarAsientos([0, 2, 1], [0, 1, 2, 4])).toEqual({
			ids: [0, 2, 1, 4],
			quitados: [],
		});
	});

	it("los que se van se quitan y se dice dónde estaban, de mayor a menor", () => {
		expect(sincronizarAsientos([0, 3, 2, 1], [0, 1, 3])).toEqual({
			ids: [0, 3, 1],
			quitados: [2],
		});
		expect(sincronizarAsientos([0, 3, 2, 1], [0, 2])).toEqual({
			ids: [0, 2],
			quitados: [3, 1],
		});
	});

	it("el anfitrión siempre va el primero", () => {
		expect(sincronizarAsientos([], [0, 1, 2]).ids).toEqual([0, 1, 2]);
		expect(sincronizarAsientos([2, 0, 1], [0, 1, 2]).ids).toEqual([0, 2, 1]);
	});
});

describe("mismoOrden", () => {
	it("compara listas", () => {
		expect(mismoOrden([0, 1], [0, 1])).toBe(true);
		expect(mismoOrden([0, 1], [1, 0])).toBe(false);
		expect(mismoOrden([0], [0, 1])).toBe(false);
	});
});
