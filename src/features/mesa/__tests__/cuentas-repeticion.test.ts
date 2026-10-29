import { describe, expect, it } from "vitest";
import type { Hito } from "@/bindings/Hito";
import {
	duracionMovimiento,
	ESPERA_MAXIMA_MS,
	ESPERA_MINIMA_MS,
	eventosNuevos,
	hitoAnterior,
	hitoSiguiente,
	pausaEntre,
} from "../cuentas-repeticion";

const hito = (pos: number): Hito => ({ pos, turno: pos, ronda: 1, activo: 0, tMs: 0 });
const hitos = [hito(4), hito(10), hito(17)];

describe("hitoAnterior y hitoSiguiente", () => {
	it("el anterior es el principio del turno de antes, o el inicio", () => {
		expect(hitoAnterior(hitos, 12)).toBe(10);
		expect(hitoAnterior(hitos, 10)).toBe(4);
		expect(hitoAnterior(hitos, 4)).toBe(0);
		expect(hitoAnterior([], 7)).toBe(0);
	});
	it("el siguiente es el del turno que viene, o el final", () => {
		expect(hitoSiguiente(hitos, 0, 30)).toBe(4);
		expect(hitoSiguiente(hitos, 10, 30)).toBe(17);
		expect(hitoSiguiente(hitos, 17, 30)).toBe(30);
	});
});

describe("pausaEntre", () => {
	it("acota el hueco real y lo divide por la velocidad", () => {
		expect(pausaEntre(1_000, 1_020, 1)).toBe(ESPERA_MINIMA_MS);
		expect(pausaEntre(1_000, 61_000, 1)).toBe(ESPERA_MAXIMA_MS);
		expect(pausaEntre(1_000, 61_000, 4)).toBe(ESPERA_MAXIMA_MS / 4);
		expect(pausaEntre(0, 600, 2)).toBe(300);
	});
	it("sin horas (el primer evento) espera lo mínimo", () => {
		expect(pausaEntre(undefined, 500, 1)).toBe(ESPERA_MINIMA_MS);
	});
});

describe("duracionMovimiento", () => {
	it("anima los pasos cortos y acaba antes de la espera mínima", () => {
		for (const v of [0.5, 1, 2, 4, 8]) {
			const d = duracionMovimiento(5, 6, v);
			expect(d).toBeGreaterThan(0);
			expect(d).toBeLessThanOrEqual(ESPERA_MINIMA_MS / v);
		}
		expect(duracionMovimiento(6, 5, 1)).toBeGreaterThan(0);
	});
	it("no anima un salto grande ni quedarse donde se está", () => {
		expect(duracionMovimiento(0, 40, 1)).toBe(0);
		expect(duracionMovimiento(7, 7, 1)).toBe(0);
	});
});

describe("eventosNuevos", () => {
	const ev = (...seqs: number[]) => seqs.map((seq) => ({ seq }));
	it("un paso hacia delante trae el evento nuevo", () => {
		expect(eventosNuevos(ev(1, 2, 3), ev(1, 2, 3, 4))).toEqual(ev(4));
		expect(eventosNuevos(ev(1, 2, 3), ev(1, 2, 3, 4, 5))).toEqual(ev(4, 5));
	});
	it("con la ventana corrida sigue funcionando", () => {
		expect(eventosNuevos(ev(2, 3, 4), ev(3, 4, 5))).toEqual(ev(5));
	});
	it("desde el principio, todos los de un paso corto", () => {
		expect(eventosNuevos(ev(), ev(7))).toEqual(ev(7));
		expect(eventosNuevos(ev(), ev(1, 2, 3, 4, 5))).toEqual([]);
	});
	it("atrás, quieto, en un salto o sin relación: ninguno", () => {
		expect(eventosNuevos(ev(1, 2, 3), ev(1, 2))).toEqual([]);
		expect(eventosNuevos(ev(1, 2, 3), ev(1, 2, 3))).toEqual([]);
		expect(eventosNuevos(ev(1, 2, 3), ev(1, 2, 3, 4, 5, 6, 7))).toEqual([]);
		expect(eventosNuevos(ev(1, 2, 3), ev(10, 11))).toEqual([]);
	});
});
