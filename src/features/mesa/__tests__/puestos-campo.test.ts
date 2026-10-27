import { describe, expect, it } from "vitest";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { Disposicion } from "../disposicion";
import { ASOMA, acompanantes, puestosDelCampo } from "../puestos-campo";

const disp = {
	campo: { x: 0, y: 0, w: 1000, h: 500 },
	carta: { w: 100, h: 140 },
} as unknown as Pick<Disposicion, "campo" | "carta">;

function carta(id: number, extra: Partial<VistaCarta> = {}): VistaCarta {
	return {
		id,
		pos: { x: 5000, y: 5000 },
		z: 5,
		adjuntaA: null,
		agrupadaBajo: null,
		...extra,
	} as unknown as VistaCarta;
}

describe("puestos del campo", () => {
	it("la adjunta asoma por arriba a la izquierda de su carta", () => {
		const cartas = { 1: carta(1), 2: carta(2, { pos: { x: 0, y: 0 }, adjuntaA: 1 }) };
		const { puestos, hijas } = puestosDelCampo([1, 2], cartas, disp);
		expect(puestos.get(1)).toEqual({ x: 500, y: 250 });
		expect(puestos.get(2)).toMatchObject({ x: 500 - ASOMA * 100, y: 250 - ASOMA * 140, z: 4 });
		expect(hijas.get(1)).toEqual([2]);
	});
	it("la amontonada se separa solo 4 px", () => {
		const cartas = { 1: carta(1), 2: carta(2, { agrupadaBajo: 1 }) };
		const { puestos } = puestosDelCampo([1, 2], cartas, disp);
		expect(puestos.get(2)).toMatchObject({ x: 496, y: 246 });
	});
	it("al llevar la carta, la acompañan sus adjuntas, y no las que ya van", () => {
		const cartas = {
			1: carta(1),
			2: carta(2, { adjuntaA: 1 }),
			3: carta(3, { adjuntaA: 1 }),
			4: carta(4, { pos: { x: 100, y: 100 } }),
		};
		const { hijas } = puestosDelCampo([1, 2, 3, 4], cartas, disp);
		expect(acompanantes([1], hijas)).toEqual([2, 3]);
		expect(acompanantes([1, 3], hijas)).toEqual([2]);
		expect(acompanantes([4], hijas)).toEqual([]);
	});
});
