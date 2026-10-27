import { describe, expect, it } from "vitest";
import { porcentajeVictorias } from "../partidas-deck";

describe("porcentajeVictorias", () => {
	it("redondea y no divide por cero", () => {
		expect(porcentajeVictorias(0, 0)).toBe(0);
		expect(porcentajeVictorias(3, 1)).toBe(33);
		expect(porcentajeVictorias(3, 2)).toBe(67);
		expect(porcentajeVictorias(4, 4)).toBe(100);
	});
});
