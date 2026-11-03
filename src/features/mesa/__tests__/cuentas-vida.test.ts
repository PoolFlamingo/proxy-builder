import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
	accionDe,
	alternarObjetivo,
	claveDe,
	crearAcumulador,
	crearMantener,
	type Objetivo,
	objetivosQueCuentan,
	vidaDe,
} from "../movil/cuentas-vida";

const J = (id: number): Objetivo => ({ tipo: "jugador", id });
const F: Objetivo = { tipo: "fantasma" };

describe("la acción de cada cuenta", () => {
	it("es la misma que mandan hoy los marcadores del tablero", () => {
		expect(accionDe(J(2), { que: "vida" }, -3)).toEqual({ que: "vida", jugador: 2, delta: -3 });
		expect(accionDe(F, { que: "vida" }, 5)).toEqual({ que: "vidaFantasma", delta: 5 });
		expect(accionDe(J(1), { que: "veneno" }, 1)).toEqual({ que: "veneno", jugador: 1, delta: 1 });
		expect(accionDe(F, { que: "veneno" }, -1)).toEqual({ que: "venenoFantasma", delta: -1 });
		expect(accionDe(J(0), { que: "comandante", comandante: 7 }, 2, 4)).toEqual({
			que: "danoComandante",
			jugador: 0,
			comandante: 7,
			delta: 2,
		});
		expect(accionDe(F, { que: "comandante", comandante: 7 }, 2)).toEqual({
			que: "danoComandanteFantasma",
			comandante: 7,
			delta: 2,
		});
	});

	it("el daño de comandante nunca baja de 0 y sin cambio no manda nada", () => {
		const c = { que: "comandante", comandante: 7 } as const;
		expect(accionDe(J(0), c, -10, 3)).toEqual({
			que: "danoComandante",
			jugador: 0,
			comandante: 7,
			delta: -3,
		});
		expect(accionDe(J(0), c, -1, 0)).toBeNull();
		expect(accionDe(J(0), { que: "vida" }, 0)).toBeNull();
	});

	it("la clave distingue objetivo y cuenta", () => {
		const claves = [
			claveDe(J(1), { que: "vida" }),
			claveDe(J(2), { que: "vida" }),
			claveDe(J(1), { que: "veneno" }),
			claveDe(F, { que: "vida" }),
			claveDe(J(1), { que: "comandante", comandante: 4 }),
			claveDe(J(1), { que: "comandante", comandante: 5 }),
		];
		expect(new Set(claves).size).toBe(claves.length);
	});
});

describe("varios objetivos a la vez", () => {
	it("tocar uno lo suma o lo quita, y siempre queda al menos uno", () => {
		let e: Objetivo[] = [J(0)];
		e = alternarObjetivo(e, J(2));
		expect(e).toEqual([J(0), J(2)]);
		e = alternarObjetivo(e, F);
		expect(e).toHaveLength(3);
		e = alternarObjetivo(e, J(0));
		expect(e).toEqual([J(2), F]);
		e = alternarObjetivo(alternarObjetivo(e, J(2)), F);
		expect(e).toEqual([F]);
	});
});

describe("mantener pulsado ±", () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	it("un toque corto es un toque y no suma de 10 en 10", () => {
		const largo = vi.fn();
		const m = crearMantener(largo, 500, 350);
		m.bajar();
		vi.advanceTimersByTime(200);
		expect(m.subir()).toBe(true);
		vi.advanceTimersByTime(2000);
		expect(largo).not.toHaveBeenCalled();
	});

	it("mantenido suma a los 500 ms y repite cada 350 mientras siga el dedo", () => {
		const largo = vi.fn();
		const m = crearMantener(largo, 500, 350);
		m.bajar();
		vi.advanceTimersByTime(499);
		expect(largo).not.toHaveBeenCalled();
		vi.advanceTimersByTime(1);
		expect(largo).toHaveBeenCalledTimes(1);
		vi.advanceTimersByTime(700);
		expect(largo).toHaveBeenCalledTimes(3);
		// Al soltar, no era un toque (no hay que sumar 1 además) y se para.
		expect(m.subir()).toBe(false);
		vi.advanceTimersByTime(2000);
		expect(largo).toHaveBeenCalledTimes(3);
	});

	it("cancelar lo deja todo parado", () => {
		const largo = vi.fn();
		const m = crearMantener(largo, 500, 350);
		m.bajar();
		m.cancelar();
		vi.advanceTimersByTime(2000);
		expect(largo).not.toHaveBeenCalled();
	});
});

describe("juntar los toques", () => {
	beforeEach(() => vi.useFakeTimers());
	afterEach(() => vi.useRealTimers());

	it("manda la suma de cada clave cuando dejan de llegar", () => {
		const enviar = vi.fn();
		const avisar = vi.fn();
		const a = crearAcumulador(enviar, avisar, 700);
		a.sumar("j0:vida", -1);
		a.sumar("j0:vida", -1);
		a.sumar("j1:vida", 10);
		expect(a.pendiente("j0:vida")).toBe(-2);
		vi.advanceTimersByTime(500);
		a.sumar("j0:vida", -1);
		vi.advanceTimersByTime(500);
		// j1 ya pasó su espera; j0 se renovó con el último toque.
		expect(enviar).toHaveBeenCalledTimes(1);
		expect(enviar).toHaveBeenCalledWith("j1:vida", 10);
		vi.advanceTimersByTime(300);
		expect(enviar).toHaveBeenCalledWith("j0:vida", -3);
		expect(a.pendiente("j0:vida")).toBe(0);
	});

	it("lo que se suma y se deshace no manda nada, y vaciar manda ya", () => {
		const enviar = vi.fn();
		const a = crearAcumulador(enviar, () => {}, 700);
		a.sumar("f:vida", 1);
		a.sumar("f:vida", -1);
		vi.advanceTimersByTime(800);
		expect(enviar).not.toHaveBeenCalled();
		a.sumar("j2:veneno", 2);
		a.vaciar();
		expect(enviar).toHaveBeenCalledWith("j2:veneno", 2);
		vi.advanceTimersByTime(2000);
		expect(enviar).toHaveBeenCalledTimes(1);
	});
});

describe("la vida compartida (2HG)", () => {
	const jugadores = {
		0: { id: 0, vida: 20, equipo: 1 },
		1: { id: 1, vida: 20, equipo: 1 },
		2: { id: 2, vida: 20, equipo: 2 },
		3: { id: 3, vida: 20, equipo: 2 },
	};
	const compartida = [
		{ id: 1, vidaCompartida: 30 },
		{ id: 2, vidaCompartida: 25 },
	];
	const aparte = [
		{ id: 1, vidaCompartida: null },
		{ id: 2, vidaCompartida: null },
	];

	it("se enseña la del equipo", () => {
		expect(vidaDe(jugadores[1], compartida)).toBe(30);
		expect(vidaDe(jugadores[3], compartida)).toBe(25);
		expect(vidaDe(jugadores[1], aparte)).toBe(20);
		expect(vidaDe({ id: 9, vida: 7, equipo: null }, compartida)).toBe(7);
	});

	it("los dos compañeros elegidos son un solo objetivo de vida", () => {
		const vida = { que: "vida" } as const;
		expect(objetivosQueCuentan([J(1), J(0), J(3)], vida, jugadores, compartida)).toEqual([
			J(0),
			J(2),
		]);
		expect(objetivosQueCuentan([J(1), J(0)], vida, jugadores, aparte)).toEqual([J(1), J(0)]);
		// El veneno es de cada uno aunque compartan la vida.
		expect(objetivosQueCuentan([J(1), J(0)], { que: "veneno" }, jugadores, compartida)).toEqual([
			J(1),
			J(0),
		]);
		expect(objetivosQueCuentan([F, J(1)], vida, jugadores, compartida)).toEqual([F, J(0)]);
	});
});
