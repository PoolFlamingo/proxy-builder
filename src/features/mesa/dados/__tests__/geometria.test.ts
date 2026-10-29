import { describe, expect, it } from "vitest";
import {
	angulosEsquinas,
	caraLeida,
	centro,
	conResultado,
	dadosDeTirada,
	esquinasD4,
	geometria,
	giroDeCara,
	type M3,
	planoCara,
	punto,
	resta,
	type TipoDado,
	textoCara,
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

describe("d4: las esquinas", () => {
	it("cada número mira hacia su vértice (las esquinas van en sentido antihorario)", () => {
		const g = geometria(4);
		const vuelta = 2 * Math.PI;
		const modulo = (a: number) => ((a % vuelta) + vuelta) % vuelta;
		for (let i = 0; i < 4; i++) {
			const { puntos } = planoCara(g, i);
			const angulos = angulosEsquinas(puntos);
			expect(angulos[0]).toBe(0);
			// La segunda esquina está 120° antes que la primera, no después.
			expect(modulo(angulos[1])).toBeCloseTo(modulo(-vuelta / 3), 5);
			expect(modulo(angulos[2])).toBeCloseTo(modulo(vuelta / 3), 5);
			const [x0, y0] = puntos[0];
			puntos.forEach(([x, y], k) => {
				expect(modulo(Math.atan2(x0, y0) + angulos[k])).toBeCloseTo(modulo(Math.atan2(x, y)), 5);
			});
		}
	});
});

const aplicar = (m: M3, v: V3): V3 => [
	m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
	m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
	m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
];

describe("la pose final de los dados en CSS 3D", () => {
	it.each(TIPOS)("d%i: la cara se ve de frente, derecha y sin espejo", (tipo) => {
		const g = geometria(tipo);
		const signo = tipo === 4 ? -1 : 1;
		g.caras.forEach((_, i) => {
			const m = giroDeCara(g, i, 0);
			// Es un giro (no un espejo): determinante 1.
			const det =
				m[0] * (m[4] * m[8] - m[5] * m[7]) -
				m[1] * (m[3] * m[8] - m[5] * m[6]) +
				m[2] * (m[3] * m[7] - m[4] * m[6]);
			expect(det).toBeCloseTo(1, 5);
			// La normal de la cara mira a quien mira (al revés en el d4, que se lee por abajo)…
			const n = aplicar(m, g.normales[i]);
			expect(n[0]).toBeCloseTo(0, 5);
			expect(n[1]).toBeCloseTo(0, 5);
			expect(n[2]).toBeCloseTo(signo, 5);
			// …y su «arriba» queda hacia arriba (los números salen derechos).
			const { u } = planoCara(g, i);
			const arriba = aplicar(m, u);
			expect(arriba[0]).toBeCloseTo(0, 5);
			expect(arriba[1]).toBeCloseTo(1, 5);
		});
	});

	it("d4: quien mira ve la punta contraria a la cara del resultado, con el número del resultado", () => {
		const g = geometria(4);
		for (let cara = 0; cara < 4; cara++) {
			const m = giroDeCara(g, cara, 0);
			// El vértice que mira a quien mira (el de más z) es el que no toca a esa cara.
			const z = g.vertices.map((v) => aplicar(m, v)[2]);
			const punta = z.indexOf(Math.max(...z));
			expect(g.caras[cara]).not.toContain(punta);
			// Y en las tres caras que lo rodean lleva el número de la cara de abajo.
			const resultado = g.valores[cara];
			g.caras.forEach((vs, i) => {
				if (!vs.includes(punta)) return;
				expect(esquinasD4(g, g.valores, i)[vs.indexOf(punta)]).toBe(resultado);
			});
		}
	});
});

describe("los números de los dados", () => {
	it("el d100 son dos d10 (decenas y unidades) y el 100 es «00» y «0»", () => {
		expect(dadosDeTirada(100, 100)).toEqual([
			{ tipo: 10, valor: 0, decenas: true },
			{ tipo: 10, valor: 0 },
		]);
		expect(dadosDeTirada(100, 47)).toEqual([
			{ tipo: 10, valor: 4, decenas: true },
			{ tipo: 10, valor: 7 },
		]);
		expect(dadosDeTirada(10, 10)).toEqual([{ tipo: 10, valor: 0 }]);
	});

	it.each(
		TIPOS,
	)("d%i: cada número sale una vez y el resultado se lee en la cara elegida", (tipo) => {
		const g = geometria(tipo);
		const esperados =
			tipo === 10 ? [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] : Array.from({ length: tipo }, (_, i) => i + 1);
		expect([...g.valores].sort((a, b) => a - b)).toEqual(esperados);
		for (const valor of esperados) {
			for (let cara = 0; cara < tipo; cara++) {
				const v = conResultado(g, cara, valor);
				expect(v[cara]).toBe(valor);
				expect([...v].sort((a, b) => a - b)).toEqual(esperados);
			}
		}
	});
});

describe("textoCara", () => {
	it("el 6 y el 9 llevan punto desde el d8, y también el 60 y el 90 de las decenas", () => {
		expect(textoCara(6, 6)).toBe("6");
		expect(textoCara(8, 6)).toBe("6.");
		expect(textoCara(20, 9)).toBe("9.");
		expect(textoCara(20, 12)).toBe("12");
		expect(textoCara(10, 6, true)).toBe("60.");
		expect(textoCara(10, 9, true)).toBe("90.");
		expect(textoCara(10, 0, true)).toBe("00");
		expect(textoCara(10, 4, true)).toBe("40");
	});
});
