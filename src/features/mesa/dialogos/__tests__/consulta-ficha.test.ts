import { describe, expect, it } from "vitest";
import { consultaDeFicha } from "../consulta-ficha";

describe("consultaDeFicha", () => {
	it("el color y la fuerza se vuelven filtros, el resto es el nombre", () => {
		expect(consultaDeFicha("Green Beast")).toBe("Beast c:g");
		expect(consultaDeFicha("3/3 beast")).toBe("beast pow=3 tou=3");
		expect(consultaDeFicha("bestia verde 2/2")).toBe("bestia c:g pow=2 tou=2");
		expect(consultaDeFicha("white blue spirit")).toBe("spirit c:wu");
		expect(consultaDeFicha("*/4 wall")).toBe("wall tou=4");
	});
	it("una palabra sola o la sintaxis de Scryfall se dejan como están", () => {
		expect(consultaDeFicha("  Treasure ")).toBe("Treasure");
		expect(consultaDeFicha("t:goblin c:r")).toBe("t:goblin c:r");
		expect(consultaDeFicha('"Eldrazi Spawn"')).toBe('"Eldrazi Spawn"');
	});
});
