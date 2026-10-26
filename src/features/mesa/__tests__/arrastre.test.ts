import { describe, expect, it } from "vitest";
import { destinoDeSoltar } from "../arrastre";
import { abanico, disponer, POS_MAX, posAPx, pxAPos } from "../disposicion";

describe("geometría de la mesa", () => {
	it("de píxeles a posición del motor y vuelta, en tableros de distinto tamaño", () => {
		for (const [w, h] of [
			[1280, 720],
			[1920, 1080],
			[800, 600],
		]) {
			const d = disponer(w, h);
			for (const p of [
				{ x: 0, y: 0 },
				{ x: 5000, y: 2500 },
				{ x: POS_MAX, y: POS_MAX },
			]) {
				const px = posAPx(d.campo, p);
				const vuelta = pxAPos(d.campo, px.x, px.y);
				expect(Math.abs(vuelta.x - p.x)).toBeLessThanOrEqual(1);
				expect(Math.abs(vuelta.y - p.y)).toBeLessThanOrEqual(1);
			}
			// Fuera del campo, se recorta.
			expect(pxAPos(d.campo, -50, -50)).toEqual({ x: 0, y: 0 });
		}
	});

	it("las zonas no se pisan y caben en el tablero", () => {
		const d = disponer(1280, 720, 100, true);
		const rs = Object.values(d.zonas);
		for (const r of rs) {
			expect(r.x + r.w).toBeLessThanOrEqual(1280);
			expect(r.y + r.h).toBeLessThanOrEqual(720);
			expect(r.x).toBeGreaterThanOrEqual(d.mano.x + d.mano.w);
		}
		for (let i = 1; i < rs.length; i++) expect(rs[i].x).toBeGreaterThan(rs[i - 1].x + rs[i - 1].w);
	});

	it("el destino depende de dónde se suelta", () => {
		const d = disponer(1280, 720);
		const lib = d.zonas.biblioteca;
		expect(destinoDeSoltar(d, lib.x + 5, lib.y + 5)?.sitio).toEqual({ tipo: "arriba" });
		expect(destinoDeSoltar(d, lib.x + 5, lib.y + lib.h - 5)?.sitio).toEqual({ tipo: "abajo" });
		const cem = d.zonas.cementerio;
		expect(destinoDeSoltar(d, cem.x + 2, cem.y + 2)?.zona).toEqual({
			tipo: "de",
			zona: "cementerio",
		});
		expect(destinoDeSoltar(d, d.mano.x + 10, d.mano.y + 10)?.zona).toEqual({
			tipo: "de",
			zona: "mano",
		});
		const enCampo = destinoDeSoltar(d, 640, 200);
		expect(enCampo?.zona).toEqual({ tipo: "de", zona: "campo" });
		expect(enCampo?.sitio.tipo).toBe("enCampo");
		expect(destinoDeSoltar(d, 5000, 5000)).toBeNull();
	});

	it("el abanico de la mano cabe y no se sale", () => {
		const d = disponer(1280, 720);
		for (const n of [1, 7, 15, 30]) {
			const a = abanico(d.mano, n, d.cartaPequena);
			expect(a).toHaveLength(n);
			for (const c of a) {
				expect(c.x - d.cartaPequena.w / 2).toBeGreaterThanOrEqual(d.mano.x - 1);
				expect(c.x + d.cartaPequena.w / 2).toBeLessThanOrEqual(d.mano.x + d.mano.w + 1);
			}
		}
	});
});
