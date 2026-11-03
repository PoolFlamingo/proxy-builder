import { describe, expect, it } from "vitest";
import { cambioDeGesto, crearGestos } from "../gestos";

describe("crearGestos", () => {
	it("con un dedo solo arrastra, sin pellizco", () => {
		const g = crearGestos();
		g.bajar(1, 100, 100);
		g.mover(1, 110, 95);
		expect(g.tomar()).toEqual({ dx: 10, dy: -5, escala: 1 });
		expect(g.tomar()).toEqual({ dx: 0, dy: 0, escala: 1 });
		expect(g.cantidad()).toBe(1);
	});

	it("varios movimientos entre dos tomas se juntan en uno", () => {
		const g = crearGestos();
		g.bajar(1, 0, 0);
		g.mover(1, 4, 0);
		g.mover(1, 10, 3);
		expect(g.tomar()).toEqual({ dx: 10, dy: 3, escala: 1 });
	});

	it("con dos dedos separándose, escala > 1 y el centro no se mueve", () => {
		const g = crearGestos();
		g.bajar(1, 90, 100);
		g.bajar(2, 110, 100);
		// Cada dedo se aleja 10 px: de 20 a 40 de distancia. Llegan como dos
		// movimientos, pero se cuentan juntos.
		g.mover(1, 80, 100);
		g.mover(2, 120, 100);
		const r = g.tomar();
		expect(r?.escala).toBeCloseTo(40 / 20);
		expect(r?.dx).toBeCloseTo(0);
		expect(r?.dy).toBeCloseTo(0);
	});

	it("juntar los dedos da una escala menor que 1", () => {
		const g = crearGestos();
		g.bajar(1, 50, 50);
		g.bajar(2, 150, 50);
		g.mover(2, 100, 50);
		expect(g.tomar()?.escala).toBeCloseTo(50 / 100);
	});

	it("al bajar el segundo dedo no hay salto: se mide de nuevo", () => {
		const g = crearGestos();
		g.bajar(1, 100, 100);
		g.mover(1, 120, 100);
		g.tomar();
		g.bajar(2, 220, 100);
		// Nada se ha movido desde que bajó el segundo.
		expect(g.tomar()).toEqual({ dx: 0, dy: 0, escala: 1 });
	});

	it("al levantar un dedo, el que queda sigue sin salto", () => {
		const g = crearGestos();
		g.bajar(1, 100, 100);
		g.bajar(2, 200, 100);
		g.subir(1);
		expect(g.cantidad()).toBe(1);
		// El centro pasó de 150 a 200, pero eso no cuenta como arrastre.
		g.mover(2, 205, 100);
		expect(g.tomar()).toEqual({ dx: 5, dy: 0, escala: 1 });
	});

	it("un dedo desconocido no hace nada", () => {
		const g = crearGestos();
		g.mover(7, 1, 1);
		expect(g.tomar()).toBeNull();
		g.bajar(1, 0, 0);
		g.subir(1);
		g.mover(1, 5, 5);
		expect(g.tomar()).toBeNull();
		expect(g.cantidad()).toBe(0);
	});

	it("un tercer dedo no mueve nada", () => {
		const g = crearGestos();
		g.bajar(1, 100, 100);
		g.bajar(2, 200, 100);
		g.bajar(3, 0, 0);
		g.mover(3, 50, 50);
		expect(g.tomar()).toEqual({ dx: 0, dy: 0, escala: 1 });
	});
});

describe("cambioDeGesto", () => {
	it("el arrastre va en fracciones de la carta", () => {
		const c = cambioDeGesto({ dx: 20, dy: -44, escala: 1 }, { w: 200, h: 440 }, 1.5);
		expect(c).toEqual({ dx: 0.1, dy: -0.1 });
	});

	it("el pellizco suma al zoom en proporción al de ahora", () => {
		const c = cambioDeGesto({ dx: 0, dy: 0, escala: 1.2 }, { w: 200, h: 300 }, 2);
		expect(c.zoom).toBeCloseTo(0.4);
		const d = cambioDeGesto({ dx: 0, dy: 0, escala: 0.5 }, { w: 200, h: 300 }, 2);
		expect(d.zoom).toBeCloseTo(-1);
	});

	it("un marco de tamaño cero no divide por cero", () => {
		const c = cambioDeGesto({ dx: 3, dy: 3, escala: 1 }, { w: 0, h: 0 }, 1);
		expect(Number.isFinite(c.dx)).toBe(true);
	});
});
