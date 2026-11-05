import { describe, expect, it } from "vitest";
import {
	claveDeFiltros,
	cuantosFiltros,
	FILTROS_DE_FABRICA as F,
	hayCriterio,
	partirSimbolos,
} from "../consulta";

describe("hayCriterio y cuantosFiltros", () => {
	it("sin texto ni filtros no hay nada que buscar", () => {
		expect(hayCriterio(F)).toBe(false);
		expect(hayCriterio({ ...F, texto: "   " })).toBe(false);
		expect(hayCriterio({ ...F, set: " " })).toBe(false);
	});

	it("vale un texto o cualquier filtro, pero no el orden", () => {
		expect(hayCriterio({ ...F, texto: "bolt" })).toBe(true);
		expect(hayCriterio({ ...F, colores: ["r"] })).toBe(true);
		expect(hayCriterio({ ...F, coste: { comparador: "igual", valor: 0 } })).toBe(true);
		expect(hayCriterio({ ...F, orden: "eur", descendente: true })).toBe(false);
	});

	it("cuenta cada filtro una vez", () => {
		expect(
			cuantosFiltros({ ...F, colores: ["r", "g"], rarezas: ["rare"], formato: "modern" }),
		).toBe(3);
	});
});

describe("claveDeFiltros", () => {
	it("escribir no cambia la clave; un filtro o el orden, sí", () => {
		const base = claveDeFiltros(F);
		expect(claveDeFiltros({ ...F, texto: "bolt", exacto: true })).toBe(base);
		expect(claveDeFiltros({ ...F, tipo: "land" })).not.toBe(base);
		expect(claveDeFiltros({ ...F, orden: "fecha" })).not.toBe(base);
	});
});

describe("partirSimbolos", () => {
	it("separa las letras de los símbolos", () => {
		expect(partirSimbolos("{T}: Add {W/U}.")).toEqual([
			{ simbolo: true, texto: "T" },
			{ simbolo: false, texto: ": Add " },
			{ simbolo: true, texto: "W/U" },
			{ simbolo: false, texto: "." },
		]);
	});

	it("un texto sin símbolos o vacío", () => {
		expect(partirSimbolos("Vuela")).toEqual([{ simbolo: false, texto: "Vuela" }]);
		expect(partirSimbolos("")).toEqual([]);
	});
});
