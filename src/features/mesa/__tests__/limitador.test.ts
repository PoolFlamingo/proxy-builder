import { describe, expect, it } from "vitest";
import { crearLimitador } from "../en-vivo/limitador";

describe("limitador de lo que se manda en vivo", () => {
	it("deja pasar 20 por segundo como mucho", () => {
		let t = 0;
		const pasa = crearLimitador(50, () => t);
		let n = 0;
		// Un puntero que se mueve cada milisegundo durante un segundo.
		for (t = 0; t < 1000; t++) if (pasa()) n++;
		expect(n).toBe(20);
	});

	it("el primero siempre pasa y el forzado también", () => {
		let t = 100;
		const pasa = crearLimitador(100, () => t);
		expect(pasa()).toBe(true);
		t = 110;
		expect(pasa()).toBe(false);
		expect(pasa(true)).toBe(true);
		// Y cuenta como el último.
		t = 150;
		expect(pasa()).toBe(false);
		t = 215;
		expect(pasa()).toBe(true);
	});
});
