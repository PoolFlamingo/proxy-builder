import { describe, expect, it } from "vitest";
import type { Flecha } from "@/bindings/Flecha";
import { atacantesDe, duenoDeExtremo, entradaDesde, idDePunta } from "../cuentas-flechas";

const cartas = { 7: { controlador: 1 }, 8: { controlador: 2 } } as never;
const flecha = (a: Flecha["a"], tipo: Flecha["tipo"] = "ataque"): Flecha => ({
	de: { tipo: "carta", id: 3 },
	a,
	tipo,
	deJugador: 0,
});

describe("flechas con varios jugadores", () => {
	it("el extremo es del jugador al que se apunta o de quien controla la carta", () => {
		expect(duenoDeExtremo({ tipo: "jugador", id: 2 }, cartas)).toBe(2);
		expect(duenoDeExtremo({ tipo: "carta", id: 7 }, cartas)).toBe(1);
		expect(duenoDeExtremo({ tipo: "carta", id: 99 }, cartas)).toBeNull();
	});
	it("cuenta los ataques contra un jugador y sus cartas, no los objetivos", () => {
		const fs = [
			flecha({ tipo: "jugador", id: 1 }),
			flecha({ tipo: "carta", id: 7 }),
			flecha({ tipo: "carta", id: 7 }, "objetivo"),
			flecha({ tipo: "carta", id: 8 }),
		];
		expect(atacantesDe(fs, cartas, 1)).toBe(2);
		expect(atacantesDe(fs, cartas, 2)).toBe(1);
		expect(atacantesDe(fs, cartas, 0)).toBe(0);
	});
	it("cada color tiene un id de punta válido para SVG", () => {
		expect(idDePunta("#e11d48")).toBe("punta-e11d48");
		expect(idDePunta("rgb(1, 2, 3)")).toBe("punta-rgb123");
	});
});

describe("entradaDesde", () => {
	it("sin otra mesa conocida, arriba en el centro", () => {
		expect(entradaDesde(1000, 600, undefined)).toEqual({ x: 500, y: 30 });
	});

	it("entra por el borde que da a la otra mesa", () => {
		// La otra mesa, a la derecha: borde derecho, a media altura.
		expect(entradaDesde(1000, 600, { x: 1500, y: 300 })).toEqual({ x: 976, y: 300 });
		// Arriba a la izquierda, en diagonal: toca antes el borde de arriba.
		const p = entradaDesde(1000, 600, { x: 0, y: -300 });
		expect(p.y).toBeCloseTo(24);
		expect(p.x).toBeGreaterThan(0);
		expect(p.x).toBeLessThan(500);
	});
});
