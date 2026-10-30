import { describe, expect, it } from "vitest";
import {
	deckDelTablero,
	manoOcultaPorTurno,
	quienResponde,
	siguienteAlResponder,
} from "../privacidad";

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
		expect(deckDelTablero(["a", "b"], [0, 1], 1)).toBe("b");
		expect(deckDelTablero(["a", "b"], [0, 1], 0)).toBe("a");
	});
	it("en red, solo el propio (el de los demás llega vacío), seas anfitrión o invitado", () => {
		// El anfitrión (asiento 0).
		expect(deckDelTablero(["a", ""], [0, 1], 0)).toBe("a");
		expect(deckDelTablero(["a", ""], [0, 1], 1)).toBeNull();
		// Un invitado (asiento 1).
		expect(deckDelTablero(["", "b"], [0, 1], 1)).toBe("b");
		expect(deckDelTablero(["", "b"], [0, 1], 0)).toBeNull();
		expect(deckDelTablero(["", "b"], [0, 1], 5)).toBeNull();
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

describe("el atajo de responder con varios jugadores", () => {
	const mesa4 = (yo: number, activo = 0) =>
		({
			yo,
			asientos: [0, 1, 2, 3],
			general: { turno: { activo } },
			jugadores: Object.fromEntries([0, 1, 2, 3].map((j) => [j, { eliminado: null }])),
		}) as never;

	it("desde el del turno va al primero que puede responder", () => {
		expect(siguienteAlResponder(mesa4(0))).toBe(1);
	});
	it("de uno que responde pasa al siguiente y, tras el último, vuelve al turno", () => {
		expect(siguienteAlResponder(mesa4(1))).toBe(2);
		expect(siguienteAlResponder(mesa4(3))).toBe(0);
		expect(siguienteAlResponder(mesa4(0, 2))).toBe(1);
		expect(siguienteAlResponder(mesa4(1, 2))).toBe(2);
	});
});
