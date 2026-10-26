import { describe, expect, it } from "vitest";
import { generador, girar } from "../azar";
import { ANCHO, FONDO, HZ, type Pieza, simular } from "../fisica";
import { caraLeida, conResultado, geometria, type TipoDado, type V3 } from "../geometria";

const TIPOS: TipoDado[] = [4, 6, 8, 10, 12, 20];

/** La cara que se lee al final de la grabación, mirando el último cuadro. */
function ultimoGiro(rot: Float32Array): number[] {
	const n = rot.length / 4 - 1;
	return [rot[n * 4], rot[n * 4 + 1], rot[n * 4 + 2], rot[n * 4 + 3]];
}

describe("física de los dados (Rapier determinista)", () => {
	it("la misma semilla da la misma animación", async () => {
		const piezas: Pieza[] = [
			{ forma: "dado", tipo: 20 },
			{ forma: "dado", tipo: 6 },
			{ forma: "moneda" },
		];
		const a = await simular(piezas, 1234);
		const b = await simular(piezas, 1234);
		expect(a.cuadros).toBe(b.cuadros);
		expect(Array.from(a.piezas[0].pos)).toEqual(Array.from(b.piezas[0].pos));
		expect(Array.from(a.piezas[2].rot)).toEqual(Array.from(b.piezas[2].rot));
		expect(a.finales).toEqual(b.finales);
	});

	it("300 tiradas: se paran a tiempo, no salen de la mesa y la cara es la del motor", async () => {
		const r = generador(7);
		for (let i = 0; i < 300; i++) {
			const cuantos = 1 + Math.floor(r() * 4);
			const tipos = Array.from({ length: cuantos }, () => TIPOS[Math.floor(r() * TIPOS.length)]);
			const g = await simular(
				tipos.map((tipo) => ({ forma: "dado", tipo })),
				Math.floor(r() * 2 ** 32),
			);
			expect(g.cuadros).toBeLessThanOrEqual(4 * HZ);
			tipos.forEach((tipo, k) => {
				const p = g.piezas[k].pos;
				const ult = p.length / 3 - 1;
				expect(Math.abs(p[ult * 3])).toBeLessThanOrEqual(ANCHO / 2);
				expect(Math.abs(p[ult * 3 + 2])).toBeLessThanOrEqual(FONDO / 2);
				const geo = geometria(tipo);
				const q = ultimoGiro(g.piezas[k].rot);
				const { cara } = caraLeida(geo, (v: V3) => girar(q, v));
				expect(cara).toBe(g.finales[k].cara);
				// El valor que pide el motor, en la cara que quedó para leer.
				const valor = geo.valores[Math.floor(r() * geo.valores.length)];
				expect(conResultado(geo, cara, valor)[cara]).toBe(valor);
			});
		}
	}, 120_000);
});
