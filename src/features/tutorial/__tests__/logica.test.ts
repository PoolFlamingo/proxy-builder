import { describe, expect, it } from "vitest";
import { aplicarFormato, ladoMovil, MARGEN, type PasoBase, sitioTarjeta } from "../logica";

describe("sitioTarjeta", () => {
	it("sin zona, en el centro", () => {
		expect(sitioTarjeta(null, 200, 1000, 800)).toEqual({ left: 340, top: 300 });
	});
	it("a la derecha de la zona si cabe", () => {
		const s = sitioTarjeta({ x: 100, y: 100, w: 200, h: 100 }, 200, 1200, 800);
		expect(s.left).toBe(314);
		expect(s.top).toBe(100);
	});
	it("debajo cuando a los lados no cabe", () => {
		const s = sitioTarjeta({ x: 20, y: 20, w: 360, h: 40 }, 150, 400, 800);
		expect(s.top).toBe(74);
	});
	it("no sale por abajo", () => {
		const s = sitioTarjeta({ x: 800, y: 780, w: 100, h: 10 }, 200, 1200, 800);
		expect(s.top).toBeLessThanOrEqual(800 - 200 - MARGEN);
	});
});

describe("ladoMovil", () => {
	it("la tarjeta va al lado contrario de la zona", () => {
		expect(ladoMovil({ x: 0, y: 700, w: 100, h: 50 }, 900)).toBe("arriba");
		expect(ladoMovil({ x: 0, y: 20, w: 100, h: 50 }, 900)).toBe("abajo");
	});
	it("sin zona, abajo", () => {
		expect(ladoMovil(null, 900)).toBe("abajo");
	});
});

describe("aplicarFormato", () => {
	const pasos: PasoBase[] = [
		{ ancla: "a", titulo: "T", texto: "con clic derecho" },
		{
			ancla: "b",
			titulo: "U",
			texto: "con teclas",
			tactil: { titulo: "U2", texto: "con una pulsación larga", ancla: "c" },
		},
		{ ancla: "d", titulo: "V", texto: "x", tactil: { ancla: "" } },
	];
	it("con ratón deja los pasos como están (sin la clave tactil)", () => {
		const r = aplicarFormato(pasos, false);
		expect(r[1]).toEqual({ ancla: "b", titulo: "U", texto: "con teclas" });
		expect("tactil" in (r[0] ?? {})).toBe(false);
	});
	it("con el dedo cambia texto, título y ancla; sin cambio el paso sigue igual", () => {
		const r = aplicarFormato(pasos, true);
		expect(r[0]).toEqual({ ancla: "a", titulo: "T", texto: "con clic derecho" });
		expect(r[1]).toEqual({ ancla: "c", titulo: "U2", texto: "con una pulsación larga" });
	});
	it("ancla vacía = sin zona", () => {
		expect(aplicarFormato(pasos, true)[2]?.ancla).toBeUndefined();
	});
});
