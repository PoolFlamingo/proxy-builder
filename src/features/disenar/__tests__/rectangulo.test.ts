import { describe, expect, it } from "vitest";
import { agarreEn, arrastrar, esquina, type Rect } from "../rectangulo";

const marco = { w: 300, h: 420 };
const zona: Rect = { x: 0.2, y: 0.2, w: 0.5, h: 0.4 };

describe("agarreEn", () => {
	it("una esquina si el toque cae cerca (en px)", () => {
		// La esquina nw está en (60, 84) px.
		expect(agarreEn(zona, { x: 0.21, y: 0.21 }, marco, 28)).toBe("nw");
		// La se, en (210, 252).
		expect(agarreEn(zona, { x: 0.7, y: 0.6 }, marco, 28)).toBe("se");
	});

	it("dentro y lejos de las esquinas mueve; fuera marca otra", () => {
		expect(agarreEn(zona, { x: 0.45, y: 0.4 }, marco, 28)).toBe("mover");
		expect(agarreEn(zona, { x: 0.9, y: 0.9 }, marco, 28)).toBe("nuevo");
	});

	it("el radio manda: con el dedo agarra más ancho que con el ratón", () => {
		const p = { x: 0.2 + 20 / 300, y: 0.2 };
		expect(agarreEn(zona, p, marco, 28)).toBe("nw");
		expect(agarreEn(zona, p, marco, 10)).toBe("mover");
	});

	it("una zona sin tamaño siempre es nueva", () => {
		expect(agarreEn({ x: 0.5, y: 0.5, w: 0, h: 0 }, { x: 0.9, y: 0.9 }, marco, 4)).toBe("nuevo");
	});
});

describe("arrastrar", () => {
	it("nuevo: del punto de partida al de ahora, en cualquier sentido", () => {
		const r = arrastrar("nuevo", zona, { x: 0.6, y: 0.7 }, { x: 0.4, y: 0.5 });
		expect(r.x).toBeCloseTo(0.4);
		expect(r.y).toBeCloseTo(0.5);
		expect(r.w).toBeCloseTo(0.2);
		expect(r.h).toBeCloseTo(0.2);
	});

	it("mover: conserva el tamaño y no se sale de la carta", () => {
		const r = arrastrar("mover", zona, { x: 0.4, y: 0.4 }, { x: 0.9, y: 0.0 });
		expect(r.w).toBe(zona.w);
		expect(r.h).toBe(zona.h);
		expect(r.x).toBeCloseTo(0.5);
		expect(r.y).toBe(0);
	});

	it("esquina: la contraria se queda quieta", () => {
		// Arrastra la se 0.1 a la derecha y 0.1 abajo.
		const r = arrastrar("se", zona, esquina(zona, "se"), { x: 0.8, y: 0.7 });
		expect(r.x).toBeCloseTo(0.2);
		expect(r.y).toBeCloseTo(0.2);
		expect(r.w).toBeCloseTo(0.6);
		expect(r.h).toBeCloseTo(0.5);
	});

	it("esquina: cuenta lo que se mueve el dedo, no dónde cae (agarró con holgura)", () => {
		const agarro = { x: 0.23, y: 0.23 };
		const r = arrastrar("nw", zona, agarro, { x: 0.33, y: 0.33 });
		expect(r.x).toBeCloseTo(0.3);
		expect(r.w).toBeCloseTo(0.4);
	});

	it("esquina: se puede pasar al otro lado de la contraria", () => {
		const r = arrastrar("nw", zona, esquina(zona, "nw"), { x: 0.9, y: 0.9 });
		expect(r.x).toBeCloseTo(0.7);
		expect(r.w).toBeCloseTo(0.2);
		expect(r.y).toBeCloseTo(0.6);
	});

	it("esquina: no sale de la carta", () => {
		const r = arrastrar("se", zona, esquina(zona, "se"), { x: 2, y: 2 });
		expect(r.x + r.w).toBeLessThanOrEqual(1);
		expect(r.y + r.h).toBeLessThanOrEqual(1);
	});
});
