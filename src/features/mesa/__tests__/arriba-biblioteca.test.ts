import { describe, expect, it } from "vitest";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { cartaDeArribaRevelada } from "../arriba-biblioteca";

const carta = (id: number, revelada: boolean) => ({ id, revelada }) as unknown as VistaCarta;

function mesa(visible: boolean, revelada: boolean, siempre = false) {
	return {
		jugadores: { 0: { deArribaRevelada: siempre } },
		zonas: { "0:biblioteca": { visibles: visible ? [[0, 7]] : [] } },
		cartas: { 7: carta(7, revelada) },
	} as never;
}

describe("la de arriba de la biblioteca", () => {
	it("se enseña si se acaba de revelar", () => {
		expect(cartaDeArribaRevelada(mesa(true, true), 0)?.id).toBe(7);
	});
	it("se enseña si se juega con la de arriba revelada", () => {
		expect(cartaDeArribaRevelada(mesa(true, false, true), 0)?.id).toBe(7);
	});
	it("mirarla no la enseña, y si no se conoce tampoco", () => {
		expect(cartaDeArribaRevelada(mesa(true, false), 0)).toBeNull();
		expect(cartaDeArribaRevelada(mesa(false, true), 0)).toBeNull();
	});
});
