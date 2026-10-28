import { describe, expect, it } from "vitest";
import { deckDelTablero, manoOcultaPorTurno, quienResponde } from "../privacidad";

const mesa = (activo: number, mulligan = false, asientos = [0, 1]) =>
	({ asientos, general: { turno: { activo }, mulligan: mulligan ? {} : null } }) as never;

describe("la mano en una partida local", () => {
	it("solo se ve la de quien tiene el turno", () => {
		expect(manoOcultaPorTurno(mesa(0), 0, false)).toBe(false);
		expect(manoOcultaPorTurno(mesa(0), 1, false)).toBe(true);
	});
	it("con el mulligan en marcha, todos ven la suya", () => {
		expect(manoOcultaPorTurno(mesa(0, true), 1, false)).toBe(false);
	});
	it("en solitario o en red no se oculta", () => {
		expect(manoOcultaPorTurno(mesa(0, false, [0]), 0, false)).toBe(false);
		expect(manoOcultaPorTurno(mesa(0), 1, true)).toBe(false);
	});
});

describe("el tapete de cada tablero", () => {
	it("en local, el del deck de su asiento", () => {
		expect(deckDelTablero(["a", "b"], [0, 1], 1, false, false)).toBe("b");
		expect(deckDelTablero(["a", "b"], [0, 1], 0, false, false)).toBe("a");
	});
	it("en red, solo el del anfitrión y mirando su tablero", () => {
		expect(deckDelTablero(["a", "b"], [0, 1], 0, true, true)).toBe("a");
		expect(deckDelTablero(["a", "b"], [0, 1], 1, true, true)).toBeNull();
		expect(deckDelTablero(["a", "b"], [0, 1], 0, true, false)).toBeNull();
	});
});

describe("responder fuera de tu turno", () => {
	it("la cortina destapa solo la mano de quien responde", () => {
		expect(manoOcultaPorTurno(mesa(0), 1, false, 1)).toBe(false);
		expect(manoOcultaPorTurno(mesa(0), 1, false, 0)).toBe(true);
		expect(manoOcultaPorTurno(mesa(0), 1, false, null)).toBe(true);
	});

	const con = (activo: number, eliminados: number[] = []) =>
		({
			asientos: [0, 1, 2, 3],
			general: { turno: { activo } },
			jugadores: Object.fromEntries(
				[0, 1, 2, 3].map((j) => [j, { eliminado: eliminados.includes(j) ? {} : null }]),
			),
		}) as never;

	it("responden los demás, en el orden de la mesa desde el del turno", () => {
		expect(quienResponde(con(0))).toEqual([1, 2, 3]);
		expect(quienResponde(con(2))).toEqual([3, 0, 1]);
	});
	it("los eliminados no responden", () => {
		expect(quienResponde(con(0, [1, 3]))).toEqual([2]);
	});
	it("sin turno no hay quien responda", () => {
		expect(quienResponde({ asientos: [0, 1], general: null, jugadores: {} } as never)).toEqual([]);
	});
});
