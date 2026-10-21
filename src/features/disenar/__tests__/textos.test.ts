import { describe, expect, it } from "vitest";
import type { Aviso } from "@/bindings/Aviso";
import { esGrave, MAQUETACIONES, textoAviso } from "../textos";

/** Un `t()` de mentira: sin traducir, solo rellena las variables (y marca las que falten). */
const t = (es: string, p?: Record<string, string | number>) =>
	es.replace(/\{(\w+)\}/g, (_, k) => (p?.[k] != null ? String(p[k]) : `FALTA:${k}`));

/** Uno de cada tipo: si Rust añade otro, el `switch` deja de compilar. */
const TODOS: Aviso[] = [
	{ kind: "sinTextoOficial" },
	{ kind: "textoNoCabe", hueco: "reglas", sobra: 2.345 },
	{ kind: "tituloReducido", hueco: "titulo" },
	{ kind: "reglasEditadas", cara: 0, lang: "es" },
	{ kind: "faltaNombre", cara: 1, lang: "en" },
	{ kind: "letraNoEncontrada", familia: "Beleren" },
	{ kind: "faltanSimbolos", simbolos: ["{W/P}", "{H}"] },
	{ kind: "packSinLayout", layout: "saga" },
	{ kind: "layoutDesconocido", layout: "reversible_card" },
	{ kind: "coleccionPerdida" },
	{ kind: "sinCinta" },
	{ kind: "faltanLetras", lang: "ja" },
];

describe("textoAviso", () => {
	it("cada tipo de aviso tiene su texto, sin variables sin rellenar", () => {
		for (const a of TODOS) {
			const s = textoAviso(a, t);
			expect(s, a.kind).toBeTruthy();
			expect(s, a.kind).not.toContain("FALTA:");
		}
	});

	it("textoNoCabe: los milímetros que sobran, redondeados a un decimal", () => {
		expect(textoAviso({ kind: "textoNoCabe", hueco: "reglas", sobra: 2.345 }, t)).toContain(
			"sobran 2.3 mm",
		);
		expect(textoAviso({ kind: "textoNoCabe", hueco: "reglas", sobra: 0 }, t)).toContain(
			"sobran 0 mm",
		);
	});

	it("tituloReducido: distingue la línea de tipo del título", () => {
		expect(textoAviso({ kind: "tituloReducido", hueco: "tipo" }, t)).toMatch(/^La línea de tipo/);
		expect(textoAviso({ kind: "tituloReducido", hueco: "titulo" }, t)).toMatch(/^El título/);
		// Cualquier otro hueco cuenta como título.
		expect(textoAviso({ kind: "tituloReducido", hueco: "otro" }, t)).toMatch(/^El título/);
	});

	it("reglasEditadas, faltaNombre y faltanLetras: el idioma por su nombre propio", () => {
		expect(textoAviso({ kind: "reglasEditadas", cara: 0, lang: "es" }, t)).toBe(
			"Reglas escritas a mano (Español).",
		);
		expect(textoAviso({ kind: "faltaNombre", cara: 0, lang: "ja" }, t)).toBe(
			"Falta el nombre temático (日本語): se usa el original.",
		);
		expect(textoAviso({ kind: "faltanLetras", lang: "zht" }, t)).toContain("繁體中文");
		expect(esGrave({ kind: "faltanLetras", lang: "ko" })).toBe(true);
	});

	it("letraNoEncontrada, packSinLayout y layoutDesconocido nombran lo que falta", () => {
		expect(textoAviso({ kind: "letraNoEncontrada", familia: "Beleren" }, t)).toContain("«Beleren»");
		expect(textoAviso({ kind: "packSinLayout", layout: "saga" }, t)).toContain("«saga»");
		expect(textoAviso({ kind: "layoutDesconocido", layout: "raro" }, t)).toContain("(raro)");
	});

	it("faltanSimbolos: la lista separada por comas", () => {
		expect(textoAviso({ kind: "faltanSimbolos", simbolos: ["{W/P}", "{H}"] }, t)).toContain(
			"({W/P}, {H})",
		);
		expect(textoAviso({ kind: "faltanSimbolos", simbolos: ["{C}"] }, t)).toContain("({C})");
	});

	it("los textos fijos salen tal cual", () => {
		expect(textoAviso({ kind: "sinTextoOficial" }, t)).toMatch(
			/^No hay impresión de esta carta en el idioma de diseño/,
		);
		expect(textoAviso({ kind: "coleccionPerdida" }, t)).toBe(
			"La colección de esta carta ya no existe: se usan los valores de fábrica.",
		);
	});
});

describe("esGrave", () => {
	it("solo el texto que no cabe y los símbolos o letras que faltan son graves", () => {
		const graves = TODOS.filter(esGrave).map((a) => a.kind);
		expect(graves).toEqual(["textoNoCabe", "faltanSimbolos", "faltanLetras"]);
	});
});

describe("MAQUETACIONES", () => {
	it("todas tienen nombre y no se repite ninguno", () => {
		const nombres = Object.values(MAQUETACIONES);
		expect(nombres.every((n) => n.length > 0)).toBe(true);
		expect(new Set(nombres).size).toBe(nombres.length);
	});
});
