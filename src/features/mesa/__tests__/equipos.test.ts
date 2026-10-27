import { describe, expect, it } from "vitest";
import type { Variantes } from "@/bindings/Variantes";
import { ataqueAUnCompanero, hayFuegoAmigo, sonCompaneros } from "../equipos";

const jugadores = { 0: { equipo: 0 }, 1: { equipo: 0 }, 2: { equipo: 1 }, 3: { equipo: null } };
const v = (equipos: Variantes["equipos"]): Variantes => ({
	equipos,
	archenemy: null,
	planechase: false,
});

describe("equipos en pantalla", () => {
	it("son compañeros con el mismo equipo y nunca sin equipo ni consigo mismo", () => {
		expect(sonCompaneros(jugadores, 0, 1)).toBe(true);
		expect(sonCompaneros(jugadores, 0, 2)).toBe(false);
		expect(sonCompaneros(jugadores, 3, 3)).toBe(false);
		expect(sonCompaneros(jugadores, 0, 0)).toBe(false);
		expect(sonCompaneros(jugadores, 0, 9)).toBe(false);
	});

	it("hay fuego amigo sin equipos y con la opción encendida", () => {
		expect(hayFuegoAmigo(null)).toBe(true);
		expect(hayFuegoAmigo(v({ tipo: "ninguno" }))).toBe(true);
		expect(hayFuegoAmigo(v({ tipo: "equipos", fuegoAmigo: true }))).toBe(true);
		expect(hayFuegoAmigo(v({ tipo: "equipos", fuegoAmigo: false }))).toBe(false);
		expect(hayFuegoAmigo(v({ tipo: "dosCabezas" }))).toBe(false);
	});

	it("avisa solo de un ataque a un compañero sin fuego amigo", () => {
		const sin = v({ tipo: "equipos", fuegoAmigo: false });
		expect(ataqueAUnCompanero(jugadores, sin, 0, 1)).toBe(true);
		expect(ataqueAUnCompanero(jugadores, sin, 0, 2)).toBe(false);
		expect(ataqueAUnCompanero(jugadores, v({ tipo: "equipos", fuegoAmigo: true }), 0, 1)).toBe(
			false,
		);
		expect(ataqueAUnCompanero(jugadores, v({ tipo: "dosCabezas" }), 0, 1)).toBe(true);
	});
});
