import { describe, expect, it } from "vitest";
import { comprobar, comprobarFuente } from "../idiomas.mjs";

const pedidas = new Map([
	["Hola", "a.tsx"],
	["{n} cartas", "a.tsx"],
	["Deck de {nombre}", "a.tsx"],
]);
const base = { areasEn: ["comun"], pedidas, en: {} };

describe("comprobar", () => {
	it("un diccionario completo pasa", () => {
		const r = comprobar({
			...base,
			codigo: "pl",
			locale: "pl-PL",
			areas: {
				comun: {
					Hola: "Cześć",
					"{n} cartas": {
						one: "{n} karta",
						few: "{n} karty",
						many: "{n} kart",
						other: "{n} karty",
					},
					"Deck de {nombre}": "Talia {nombre}",
				},
			},
		});
		expect(r.errores).toEqual([]);
	});

	it("dice lo que falta, el marcador cambiado y la forma de plural que falta", () => {
		const r = comprobar({
			...base,
			codigo: "pl",
			locale: "pl-PL",
			areas: {
				comun: {
					"{n} cartas": { one: "{n} karta", many: "{n} kart", other: "{n} karty" },
					"Deck de {nombre}": "Talia {m}",
				},
			},
		});
		expect(r.errores).toHaveLength(3);
		expect(r.errores.some((e) => e.startsWith('falta "Hola"'))).toBe(true);
		expect(r.errores.some((e) => e.includes("marcadores distintos") && e.includes("{m}"))).toBe(
			true,
		);
		expect(r.errores.some((e) => e.includes("le falta «few»"))).toBe(true);
	});

	it("no acepta plural sin {n} ni categorías que el idioma no tiene", () => {
		const r = comprobar({
			...base,
			codigo: "ja",
			locale: "ja-JP",
			areas: {
				comun: {
					Hola: { other: "こんにちは" },
					"{n} cartas": { one: "{n} 枚", other: "{n} 枚" },
					"Deck de {nombre}": "{nombre} のデッキ",
				},
			},
		});
		expect(r.errores).toHaveLength(2);
	});

	it("avisa de lo que sobra y de lo idéntico al inglés, sin fallar", () => {
		const r = comprobar({
			...base,
			codigo: "fr",
			locale: "fr-FR",
			en: { Hola: "Hello" },
			areas: {
				comun: {
					Hola: "Hello",
					"{n} cartas": { one: "{n} carte", many: "{n} de cartes", other: "{n} cartes" },
					"Deck de {nombre}": "Deck de {nombre}",
					Viejo: "Vieux",
				},
			},
		});
		expect(r.errores).toEqual([]);
		expect(r.avisos).toHaveLength(2);
	});

	it("falta un área que el inglés tiene", () => {
		const r = comprobar({
			...base,
			codigo: "fr",
			locale: "fr-FR",
			pedidas: new Map(),
			areas: { otra: {} },
		});
		expect(r.errores).toEqual(["falta el área comun.json"]);
	});

	it("lleva las cadenas repetidas dentro de un mismo fichero como fallo", () => {
		const r = comprobar({
			...base,
			codigo: "fr",
			locale: "fr-FR",
			repetidas: ['repetida en comun: "Hola"'],
			areas: {
				comun: {
					Hola: "Salut",
					"{n} cartas": { one: "{n} carte", many: "{n} de cartes", other: "{n} cartes" },
					"Deck de {nombre}": "Deck de {nombre}",
				},
			},
		});
		expect(r.errores).toEqual(['repetida en comun: "Hola"']);
	});
});

describe("comprobarFuente (español)", () => {
	it("cada cadena vale lo mismo que su clave: pasa", () => {
		const r = comprobarFuente({
			...base,
			areas: { comun: { Hola: "Hola", "{n} cartas": "{n} cartas" } },
			pedidas: new Map([
				["Hola", "a.tsx"],
				["{n} cartas", "a.tsx"],
			]),
		});
		expect(r.errores).toEqual([]);
	});

	it("una traducción distinta, una cadena que falta y un área que falta fallan", () => {
		const r = comprobarFuente({
			...base,
			areas: { otra: { Hola: "Hi" } },
			pedidas: new Map([["Adiós", "a.tsx"]]),
		});
		expect(r.errores).toHaveLength(3);
		expect(r.errores.some((e) => e.includes('el valor no es la fuente: "Hola"'))).toBe(true);
		expect(r.errores.some((e) => e.includes('falta "Adiós"'))).toBe(true);
		expect(r.errores.some((e) => e.includes("falta el área comun.json"))).toBe(true);
	});
});
