import { describe, expect, it } from "vitest";
import { comandantesDe, danoMaximo, deltaSeguro, esLetal, impuestoDe } from "../dano-comandante";

const carta = (id: number, dueno: number, nombre: string, comandante: boolean) =>
	({
		id,
		dueno,
		origen: { nombre, info: { esComandante: comandante } },
	}) as never;

describe("comandantesDe", () => {
	it("solo los comandantes, por dueño y luego por carta", () => {
		const cs = comandantesDe({
			5: carta(5, 1, "B", true),
			2: carta(2, 1, "Socio", true),
			3: carta(3, 0, "A", true),
			4: carta(4, 0, "Isla", false),
			7: { id: 7, dueno: 0, origen: null } as never,
		});
		expect(cs.map((c) => c.nombre)).toEqual(["A", "Socio", "B"]);
	});
});

describe("deltaSeguro", () => {
	it("no deja quitar más de lo que hay", () => {
		expect(deltaSeguro(3, -5)).toBe(-3);
		expect(deltaSeguro(3, -1)).toBe(-1);
		expect(deltaSeguro(0, -1)).toBe(0);
	});
	it("sumar no tiene tope", () => {
		expect(deltaSeguro(20, 5)).toBe(5);
	});
});

describe("daño letal", () => {
	it("cuenta el mayor de un solo comandante, no la suma", () => {
		const j = { danoComandante: { 1: 12, 2: 12 } };
		expect(danoMaximo(j)).toBe(12);
		expect(esLetal(j, 21)).toBe(false);
		expect(esLetal({ danoComandante: { 1: 21 } }, 21)).toBe(true);
		expect(danoMaximo({ danoComandante: {} })).toBe(0);
	});
});

describe("impuestoDe", () => {
	it("2 por cada lanzamiento", () => {
		expect(impuestoDe({ lanzamientos: { 9: 3 }, sinCosteExtra: [] }, 9)).toBe(6);
		expect(impuestoDe({ lanzamientos: {}, sinCosteExtra: [] }, 9)).toBe(0);
	});
	it("sin impuesto si el comandante no lo lleva", () => {
		expect(impuestoDe({ lanzamientos: { 9: 3 }, sinCosteExtra: [9] }, 9)).toBe(0);
	});
	it("sin dueño, nada", () => {
		expect(impuestoDe(undefined, 9)).toBe(0);
	});
});
