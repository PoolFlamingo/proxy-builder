import { describe, expect, it } from "vitest";
import {
	caraLeida,
	centro,
	conResultado,
	dadosDeTirada,
	esquinasD4,
	geometria,
	punto,
	resta,
	type TipoDado,
	type V3,
} from "../geometria";

const TIPOS: TipoDado[] = [4, 6, 8, 10, 12, 20];
const LADOS: Record<TipoDado, number> = { 4: 3, 6: 4, 8: 3, 10: 4, 12: 5, 20: 3 };

describe("geometría de los dados", () => {
	it.each(TIPOS)("d%i: caras, lados, normales hacia fuera y planas", (tipo) => {
		const g = geometria(tipo);
		expect(g.caras).toHaveLength(tipo);
		expect(g.normales).toHaveLength(tipo);
		for (const [i, cara] of g.caras.entries()) {
			expect(cara).toHaveLength(LADOS[tipo]);
			const n = g.normales[i];
			expect(punto(n, n)).toBeCloseTo(1, 6);
			// Hacia fuera: el centro de la cara está del lado de la normal.
			expect(punto(centro(g, i), n)).toBeGreaterThan(0.1);
			// Plana: todos los vértices a la misma distancia del plano.
			const d0 = punto(g.vertices[cara[0]], n);
			for (const v of cara) expect(punto(g.vertices[v], n)).toBeCloseTo(d0, 5);
		}
		// Convexo: ningún vértice por fuera de ninguna cara.
		for (const [i, cara] of g.caras.entries()) {
			const d0 = punto(g.vertices[cara[0]], g.normales[i]);
			for (const v of g.vertices) expect(punto(v, g.normales[i])).toBeLessThanOrEqual(d0 + 1e-6);
		}
	});

	it.each([
		6, 8, 12, 20,
	] as TipoDado[])("d%i: números del 1 al n y opuestas que suman n + 1", (tipo) => {
		const g = geometria(tipo);
		expect([...g.valores].sort((a, b) => a - b)).toEqual(
			Array.from({ length: tipo }, (_, i) => i + 1),
		);
		for (const [i, n] of g.normales.entries()) {
			const op = g.normales.findIndex((m) => punto(m, n) < -0.999);
			expect(g.valores[i] + g.valores[op]).toBe(tipo + 1);
		}
	});

	it("d10: del 0 al 9, opuestas que suman 9", () => {
		const g = geometria(10);
		expect([...g.valores].sort((a, b) => a - b)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
		for (const [i, n] of g.normales.entries()) {
			const op = g.normales.findIndex((m) => punto(m, n) < -0.999);
			expect(g.valores[i] + g.valores[op]).toBe(9);
		}
	});

	it("la cara leída y el cambio de números", () => {
		const g = geometria(20);
		const quieto = (v: V3) => v;
		const { cara, plana } = caraLeida(g, quieto);
		expect(plana).toBeGreaterThan(0.7);
		const valores = conResultado(g, cara, 17);
		expect(valores[cara]).toBe(17);
		expect([...valores].sort((a, b) => a - b)).toEqual([...g.valores].sort((a, b) => a - b));
	});

	it("el d4 se lee por la cara de abajo y sus esquinas dicen ese número", () => {
		const g = geometria(4);
		// Girado para que el vértice 0 apunte arriba: la cara de abajo es la que no lo tiene.
		const v0 = g.vertices[0];
		const n = Math.sqrt(punto(v0, v0));
		const eje = resta([0, 1, 0], [v0[0] / n, v0[1] / n, v0[2] / n]);
		// Reflejo de Householder que lleva v0 a +Y (vale para leer una cara).
		const k = punto(eje, eje);
		const reflejar = (v: V3): V3 => {
			const d = (2 * punto(v, eje)) / k;
			return [v[0] - eje[0] * d, v[1] - eje[1] * d, v[2] - eje[2] * d];
		};
		const { cara } = caraLeida(g, reflejar);
		expect(g.caras[cara]).not.toContain(0);
		const valores = conResultado(g, cara, 3);
		for (const [i, c] of g.caras.entries()) {
			if (i === cara) continue;
			const esquinas = esquinasD4(g, valores, i);
			expect(esquinas[c.indexOf(0)]).toBe(3);
		}
	});

	it("el d100 son dos d10", () => {
		expect(dadosDeTirada(100, 37)).toEqual([
			{ tipo: 10, valor: 3, decenas: true },
			{ tipo: 10, valor: 7 },
		]);
		expect(dadosDeTirada(100, 100)).toEqual([
			{ tipo: 10, valor: 0, decenas: true },
			{ tipo: 10, valor: 0 },
		]);
		expect(dadosDeTirada(10, 10)).toEqual([{ tipo: 10, valor: 0 }]);
		expect(dadosDeTirada(20, 20)).toEqual([{ tipo: 20, valor: 20 }]);
	});
});

describe("el plano de cada cara", () => {
	it("no sale en espejo: los vértices giran en sentido antihorario", async () => {
		const { planoCara } = await import("../geometria");
		for (const tipo of TIPOS) {
			const g = geometria(tipo);
			for (let i = 0; i < g.caras.length; i++) {
				const { puntos } = planoCara(g, i);
				// Área con signo (fórmula del cordón): positiva = antihorario con y hacia arriba.
				let area = 0;
				puntos.forEach(([x, y], k) => {
					const [x2, y2] = puntos[(k + 1) % puntos.length];
					area += x * y2 - x2 * y;
				});
				expect(area).toBeGreaterThan(0);
			}
		}
	});
});
