import { describe, expect, it } from "vitest";
import { distancia, puntoMedio, scrollTrasZoom, zoomDePellizco } from "../zoom-gestos";

describe("distancia y punto medio", () => {
	it("la distancia entre dos dedos", () => {
		expect(distancia({ x: 0, y: 0 }, { x: 3, y: 4 })).toBe(5);
	});

	it("el punto medio", () => {
		expect(puntoMedio({ x: 0, y: 10 }, { x: 10, y: 30 })).toEqual({ x: 5, y: 20 });
	});
});

describe("zoomDePellizco", () => {
	it("separar los dedos al doble duplica el zoom", () => {
		expect(zoomDePellizco(0.8, 100, 200)).toBe(1.6);
	});

	it("juntarlos lo reduce", () => {
		expect(zoomDePellizco(1, 200, 100)).toBe(0.5);
	});

	it("no pasa de los topes", () => {
		expect(zoomDePellizco(1, 100, 1000, 0.4, 2.5)).toBe(2.5);
		expect(zoomDePellizco(1, 100, 1, 0.4, 2.5)).toBe(0.4);
	});

	it("sin distancia de partida no hay cambio", () => {
		expect(zoomDePellizco(0.8, 0, 50)).toBe(0.8);
	});

	it("sale con dos decimales, como setZoom", () => {
		expect(zoomDePellizco(1, 300, 301)).toBe(1);
		expect(zoomDePellizco(1, 100, 123)).toBe(1.23);
	});
});

describe("scrollTrasZoom", () => {
	it("al doble de zoom, lo que estaba bajo los dedos sigue ahí", () => {
		// Un punto del contenido en y = 300 + 100 = 400; al duplicar está en 800.
		expect(scrollTrasZoom(300, 100, 1, 2)).toBe(700);
		expect(700 + 100).toBe(800);
	});

	it("sin cambio de zoom, el scroll no se mueve", () => {
		expect(scrollTrasZoom(123, 45, 0.8, 0.8)).toBe(123);
	});
});
