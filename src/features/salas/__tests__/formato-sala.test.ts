import { describe, expect, it } from "vitest";
import { formatoDeSala } from "../sala-espera";

describe("formatoDeSala", () => {
	it("la lista se comprueba con el formato de la sala", () => {
		expect(formatoDeSala("commander", "limited")).toBe("commander");
		expect(formatoDeSala("libre", "commander")).toBe("libre");
	});
	it("un formato que esta versión no conoce cae en el de «Jugar»", () => {
		expect(formatoDeSala("oathbreaker", "limited")).toBe("limited");
	});
});
