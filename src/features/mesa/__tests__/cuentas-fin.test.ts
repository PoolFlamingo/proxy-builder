import { describe, expect, it } from "vitest";
import { type EstadoDeJugador, formatoDuracion, ganadoresEnPie } from "../cuentas-fin";

const estados =
	(m: Record<number, EstadoDeJugador>) =>
	(id: number): EstadoDeJugador | undefined =>
		m[id];
const vivo = (equipo: number | null = null): EstadoDeJugador => ({ eliminado: false, equipo });
const fuera = (equipo: number | null = null): EstadoDeJugador => ({ eliminado: true, equipo });

describe("ganadoresEnPie", () => {
	it("el único que sigue en pie", () => {
		const e = estados({ 0: fuera(), 1: vivo(), 2: fuera() });
		expect(ganadoresEnPie([0, 1, 2], e)).toEqual([1]);
	});
	it("con dos o más en pie, nadie", () => {
		const e = estados({ 0: vivo(), 1: vivo(), 2: fuera() });
		expect(ganadoresEnPie([0, 1, 2], e)).toEqual([]);
	});
	it("con equipos, todo el equipo que queda", () => {
		const e = estados({ 0: vivo(0), 1: vivo(0), 2: fuera(1), 3: fuera(1) });
		expect(ganadoresEnPie([0, 1, 2, 3], e)).toEqual([0, 1]);
	});
	it("con equipos distintos en pie, nadie", () => {
		const e = estados({ 0: vivo(0), 1: fuera(0), 2: vivo(1), 3: fuera(1) });
		expect(ganadoresEnPie([0, 1, 2, 3], e)).toEqual([]);
	});
	it("en solitario o sin nadie, nadie", () => {
		expect(ganadoresEnPie([0], estados({ 0: vivo() }))).toEqual([]);
		expect(ganadoresEnPie([0, 1], estados({ 0: fuera(), 1: fuera() }))).toEqual([]);
	});
});

describe("formatoDuracion", () => {
	it("minutos y segundos, y horas si hacen falta", () => {
		expect(formatoDuracion(0)).toBe("0:00");
		expect(formatoDuracion(65_000)).toBe("1:05");
		expect(formatoDuracion(3_723_000)).toBe("1:02:03");
		expect(formatoDuracion(-5)).toBe("0:00");
	});
});
