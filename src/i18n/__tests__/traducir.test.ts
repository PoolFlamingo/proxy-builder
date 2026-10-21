import { describe, expect, it } from "vitest";
import { cargarDiccionario, diccionarioCargado } from "../cargar";
import type { Diccionario } from "../tipos";
import { interpolar, traducir } from "../traducir";

const EN: Diccionario = {
	Hola: "Hello",
	"{n} cartas": { one: "{n} card", other: "{n} cards" },
	"Solo en inglés": "English only",
};
const RU: Diccionario = {
	Hola: "Привет",
	"{n} cartas": { one: "{n} карта", few: "{n} карты", many: "{n} карт", other: "{n} карты" },
};

describe("traducir", () => {
	it("cae del idioma al inglés y del inglés al español", () => {
		expect(traducir("Hola", "ru", undefined, RU, EN)).toBe("Привет");
		expect(traducir("Solo en inglés", "ru", undefined, RU, EN)).toBe("English only");
		expect(traducir("Sin traducir", "ru", undefined, RU, EN)).toBe("Sin traducir");
		expect(traducir("Hola", "pl", undefined, undefined, EN)).toBe("Hello");
		expect(traducir("Hola", "es", undefined, undefined, EN)).toBe("Hola");
	});

	it("elige la forma del plural ruso por n", () => {
		const de = (n: number) => traducir("{n} cartas", "ru", { n }, RU, EN);
		expect(de(1)).toBe("1 карта");
		expect(de(3)).toBe("3 карты");
		expect(de(5)).toBe("5 карт");
		expect(de(21)).toBe("21 карта");
	});

	it("un n que no es número usa other y no rompe", () => {
		expect(traducir("{n} cartas", "ru", { n: "muchas" }, RU, EN)).toBe("muchas карты");
		expect(traducir("{n} cartas", "en", undefined, undefined, EN)).toBe("{n} cards");
	});

	it("escribe los números con el formato del idioma", () => {
		// El francés separa los miles con un espacio fino de no separación (U+202F).
		expect(interpolar("{n}", { n: 1234.5 }, "fr-FR")).toBe("1\u202f234,5");
		expect(interpolar("{n}", { n: 1234.5 }, "en-GB")).toBe("1,234.5");
		expect(traducir("{n} cartas", "en", { n: 1500 }, undefined, EN)).toBe("1,500 cards");
		expect(interpolar("{x} {falta}", { x: "a" }, "es-ES")).toBe("a {falta}");
	});
});

describe("cargarDiccionario", () => {
	it("el inglés ya está, el español no tiene y uno sin diccionario no rompe", async () => {
		expect(diccionarioCargado("en")?.Ajustes).toBe("Settings");
		expect(await cargarDiccionario("es")).toBeUndefined();
		// Mientras no haya diccionario de rumano, se queda sin él (cae al inglés).
		const ro = await cargarDiccionario("ro");
		expect(ro === undefined || typeof ro.Ajustes === "string").toBe(true);
	});
});
