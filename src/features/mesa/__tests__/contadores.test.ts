import { describe, expect, it } from "vitest";
import { CONTADORES_OTROS, PALABRAS_CLAVE } from "../colores";
import {
	conTemporal,
	esTemporal,
	manaDe,
	nombreContador,
	nombreEnRegistro,
	sinTemporal,
} from "../contadores";

const t = (s: string) => s;

describe("contadores temporales", () => {
	it("el prefijo marca y quita lo temporal", () => {
		expect(esTemporal("~+1/+1")).toBe(true);
		expect(esTemporal("+1/+1")).toBe(false);
		expect(sinTemporal("~Flying")).toBe("Flying");
		expect(conTemporal("oil", true)).toBe("~oil");
		expect(conTemporal("~oil", true)).toBe("~oil");
		expect(conTemporal("~oil", false)).toBe("oil");
	});
	it("el maná se reconoce, temporal o no", () => {
		expect(manaDe("mana-R")).toBe("R");
		expect(manaDe("~mana-G")).toBe("G");
		expect(manaDe("oil")).toBeNull();
	});
	it("el nombre sale sin prefijo, traducido si es del catálogo", () => {
		expect(nombreContador("~oil", t)).toBe("Aceite");
		expect(nombreContador("~Flying", t)).toBe("Volar");
		expect(nombreContador("age", t)).toBe("Edad");
		expect(nombreContador("lo-que-escriba-alguien", t)).toBe("lo-que-escriba-alguien");
		expect(nombreEnRegistro("~oil", t)).toBe("Aceite (temporal)");
	});
	it("el catálogo no repite tipos ni nombres", () => {
		for (const lista of [CONTADORES_OTROS, PALABRAS_CLAVE]) {
			expect(new Set(lista.map((c) => c.tipo)).size).toBe(lista.length);
			expect(new Set(lista.map((c) => c.nombre)).size).toBe(lista.length);
		}
	});
});
