import { describe, expect, it } from "vitest";
import { agrupar, ajustar, cambioValido, hayCambio, idsDe, SIN_CAMBIOS, total } from "../encuentro";

const carta = (id: string, nombre: string) => ({ id, nombre });

describe("agrupar", () => {
	it("junta las copias por nombre y guarda el orden", () => {
		const g = agrupar([carta("a", "Bosque"), carta("b", "Isla"), carta("c", "Bosque")]);
		expect(g).toEqual([
			{ nombre: "Bosque", ids: ["a", "c"] },
			{ nombre: "Isla", ids: ["b"] },
		]);
	});
	it("sin cartas, nada", () => {
		expect(agrupar([])).toEqual([]);
	});
});

describe("ajustar", () => {
	it("sube y baja sin salirse de 0 a max", () => {
		let c = ajustar({}, "Bosque", 1, 3);
		c = ajustar(c, "Bosque", 1, 3);
		expect(c).toEqual({ Bosque: 2 });
		expect(ajustar(c, "Bosque", 5, 3)).toEqual({ Bosque: 3 });
		expect(ajustar(c, "Bosque", -9, 3)).toEqual({});
	});
	it("no toca las demás", () => {
		expect(ajustar({ Isla: 1 }, "Bosque", 1, 4)).toEqual({ Isla: 1, Bosque: 1 });
	});
});

describe("idsDe", () => {
	const grupos = agrupar([carta("a", "Bosque"), carta("c", "Bosque"), carta("b", "Isla")]);
	it("coge las primeras copias de cada grupo", () => {
		expect(idsDe(grupos, { Bosque: 1, Isla: 1 })).toEqual(["a", "b"]);
		expect(idsDe(grupos, { Bosque: 2 })).toEqual(["a", "c"]);
	});
	it("no inventa copias que no hay", () => {
		expect(idsDe(grupos, { Isla: 5 })).toEqual(["b"]);
		expect(idsDe(grupos, { Otra: 2 })).toEqual([]);
	});
});

describe("cambioValido", () => {
	it("hacen falta las mismas cartas dentro que fuera", () => {
		expect(cambioValido(SIN_CAMBIOS)).toBe(true);
		expect(cambioValido({ sale: { A: 2 }, entra: { B: 1, C: 1 } })).toBe(true);
		expect(cambioValido({ sale: { A: 2 }, entra: { B: 1 } })).toBe(false);
	});
	it("dice si hay algo que cambiar", () => {
		expect(hayCambio(SIN_CAMBIOS)).toBe(false);
		expect(hayCambio({ sale: { A: 1 }, entra: {} })).toBe(true);
		expect(total({ A: 2, B: 3 })).toBe(5);
	});
});
