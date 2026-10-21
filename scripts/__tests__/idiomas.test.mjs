import { describe, expect, it } from "vitest";
import { comprobar } from "../idiomas.mjs";

const pedidas = new Map([
	["Hola", "a.tsx"],
	["{n} cartas", "a.tsx"],
	["Deck de {nombre}", "a.tsx"],
]);
const base = { areasEn: ["comun"], indice: 'import { COMUN } from "./comun";', pedidas, en: {} };

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

	it("falla si index.ts no funde un área", () => {
		const r = comprobar({
			...base,
			codigo: "fr",
			locale: "fr-FR",
			indice: "",
			areas: { comun: {} },
		});
		expect(r.errores.some((e) => e.includes("no funde"))).toBe(true);
	});
});
