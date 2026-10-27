import { describe, expect, it } from "vitest";
import {
	AJUSTES_LOCALES,
	type AjustesLocal,
	type AsientoLocal,
	ajustesTrasOrden,
	ajustesTrasQuitar,
	aplicarOrden,
	construirConfig,
	ordenAlAzar,
	ordenTrasMover,
	problemaDeEquipos,
} from "../config-local";

const asiento = (i: number, equipo: number | null = null): AsientoLocal => ({
	nombre: ` J${i} `,
	deckId: `d${i}`,
	equipo,
});
const nombre = (n: number) => `Equipo ${n}`;

describe("construirConfig", () => {
	it("sin equipos no manda ninguno", () => {
		const c = construirConfig([asiento(1), asiento(2, 0)], AJUSTES_LOCALES, nombre);
		expect(c.variantes.equipos).toEqual({ tipo: "ninguno" });
		expect(c.equipos).toEqual([]);
		// Aunque el asiento tuviera un equipo apuntado, sin el modo no cuenta.
		expect(c.jugadores.every((j) => j.equipo === undefined)).toBe(true);
		expect(c.jugadores[0].nombre).toBe("J1");
		expect(c.opciones?.vidaInicial).toBe(40);
	});

	it("por equipos: crea solo los que tienen gente y los reparte", () => {
		const asientos = [asiento(1, 0), asiento(2, 0), asiento(3, 2), asiento(4, 2), asiento(5)];
		const c = construirConfig(
			asientos,
			{ ...AJUSTES_LOCALES, modo: "equipos", fuegoAmigo: true },
			nombre,
		);
		expect(c.variantes.equipos).toEqual({ tipo: "equipos", fuegoAmigo: true });
		expect(c.equipos.map((e) => [e.id, e.nombre])).toEqual([
			[0, "Equipo 1"],
			[2, "Equipo 3"],
		]);
		expect(c.equipos.every((e) => e.vidaCompartida === null)).toBe(true);
		expect(c.jugadores.map((j) => j.equipo)).toEqual([0, 0, 2, 2, undefined]);
	});

	it("con vida compartida, cada equipo lleva la vida inicial", () => {
		const c = construirConfig(
			[asiento(1, 0), asiento(2, 0), asiento(3, 1), asiento(4, 1)],
			{ ...AJUSTES_LOCALES, formato: "libre", modo: "equipos", vidaCompartida: true },
			nombre,
		);
		expect(c.opciones?.vidaInicial).toBe(20);
		expect(c.equipos.map((e) => e.vidaCompartida)).toEqual([20, 20]);
	});

	it("Dos Cabezas: 30 de vida y los equipos los monta el motor", () => {
		const c = construirConfig(
			[asiento(1), asiento(2), asiento(3), asiento(4)],
			{ ...AJUSTES_LOCALES, modo: "dosCabezas" },
			nombre,
		);
		expect(c.variantes.equipos).toEqual({ tipo: "dosCabezas" });
		expect(c.opciones?.vidaInicial).toBe(30);
		expect(c.equipos).toEqual([]);
	});

	it("cada jugador recibe su color por orden", () => {
		const c = construirConfig([asiento(1), asiento(2), asiento(3)], AJUSTES_LOCALES, nombre);
		expect(new Set(c.jugadores.map((j) => j.color)).size).toBe(3);
	});
});

describe("problemaDeEquipos", () => {
	it("sin equipos no hay problema", () => {
		expect(problemaDeEquipos([asiento(1), asiento(2)], "ninguno")).toBeNull();
	});
	it("por equipos hace falta uno con dos o más", () => {
		expect(problemaDeEquipos([asiento(1, 0), asiento(2, 1)], "equipos")).toBe("sinCompaneros");
		expect(problemaDeEquipos([asiento(1), asiento(2)], "equipos")).toBe("sinCompaneros");
		expect(problemaDeEquipos([asiento(1, 0), asiento(2, 0), asiento(3)], "equipos")).toBeNull();
	});
	it("Dos Cabezas pide un número par desde cuatro", () => {
		const n = (k: number) => Array.from({ length: k }, (_, i) => asiento(i));
		expect(problemaDeEquipos(n(2), "dosCabezas")).toBe("dosCabezasPares");
		expect(problemaDeEquipos(n(5), "dosCabezas")).toBe("dosCabezasPares");
		expect(problemaDeEquipos(n(4), "dosCabezas")).toBeNull();
		expect(problemaDeEquipos(n(10), "dosCabezas")).toBeNull();
	});
});

describe("Archenemy y Planechase", () => {
	const cuatro = [asiento(1), asiento(2), asiento(3), asiento(4)];

	it("un archienemigo contra todos: los demás son un equipo sin fuego amigo", () => {
		const c = construirConfig(
			cuatro,
			{ ...AJUSTES_LOCALES, modo: "archienemigo", archienemigo: 2, fuegoAmigo: true },
			nombre,
			"Héroes",
		);
		expect(c.variantes.archenemy).toBe(2);
		// El fuego amigo del modo «por equipos» no cuenta aquí.
		expect(c.variantes.equipos).toEqual({ tipo: "equipos", fuegoAmigo: false });
		expect(c.equipos).toHaveLength(1);
		expect(c.equipos[0].nombre).toBe("Héroes");
		expect(c.jugadores.map((j) => j.equipo)).toEqual([0, 0, undefined, 0]);
	});

	it("el asiento del archienemigo se queda dentro de los que hay", () => {
		const c = construirConfig(
			[asiento(1), asiento(2)],
			{ ...AJUSTES_LOCALES, modo: "archienemigo", archienemigo: 7 },
			nombre,
		);
		expect(c.variantes.archenemy).toBe(1);
	});

	it("fuera del modo, no hay archienemigo", () => {
		const c = construirConfig(cuatro, { ...AJUSTES_LOCALES, archienemigo: 3 }, nombre);
		expect(c.variantes.archenemy).toBeNull();
	});

	it("Planechase se enciende solo y vale con cualquier modo", () => {
		expect(construirConfig(cuatro, AJUSTES_LOCALES, nombre).variantes.planechase).toBe(false);
		for (const modo of ["ninguno", "equipos", "dosCabezas", "archienemigo"] as const) {
			const c = construirConfig(cuatro, { ...AJUSTES_LOCALES, modo, planechase: true }, nombre);
			expect(c.variantes.planechase).toBe(true);
		}
	});

	it("Dos Cabezas empieza con 30 vidas", () => {
		const c = construirConfig(cuatro, { ...AJUSTES_LOCALES, modo: "dosCabezas" }, nombre);
		expect(c.opciones?.vidaInicial).toBe(30);
	});
});

describe("orden de los asientos", () => {
	it("mover un asiento lo lleva a su sitio y deja los demás en orden", () => {
		expect(ordenTrasMover(4, 0, 2)).toEqual([1, 2, 0, 3]);
		expect(ordenTrasMover(4, 3, 0)).toEqual([3, 0, 1, 2]);
		expect(ordenTrasMover(4, 1, 1)).toEqual([0, 1, 2, 3]);
	});

	it("mover con índices fuera de rango no rompe nada", () => {
		expect(ordenTrasMover(3, 7, 0)).toEqual([0, 1, 2]);
		expect(ordenTrasMover(3, 0, 99)).toEqual([1, 2, 0]);
		expect(ordenTrasMover(3, 2, -4)).toEqual([2, 0, 1]);
	});

	it("al azar es una permutación y depende del azar que se le da", () => {
		for (const n of [1, 2, 5, 10]) {
			const o = ordenAlAzar(n);
			expect([...o].sort((a, b) => a - b)).toEqual(Array.from({ length: n }, (_, i) => i));
		}
		expect(ordenAlAzar(4, () => 0)).toEqual([1, 2, 3, 0]);
		expect(ordenAlAzar(4, () => 0.999)).toEqual([0, 1, 2, 3]);
	});

	it("aplica el orden a una lista", () => {
		expect(aplicarOrden(["a", "b", "c"], [2, 0, 1])).toEqual(["c", "a", "b"]);
	});

	const base: AjustesLocal = { ...AJUSTES_LOCALES, primero: 0, archienemigo: 2 };

	it("el primero y el archienemigo siguen siendo los mismos jugadores", () => {
		// El jugador de la posición 0 pasa a la 2, y los otros suben una.
		const a = ajustesTrasOrden(base, ordenTrasMover(4, 0, 2));
		expect(a.primero).toBe(2);
		expect(a.archienemigo).toBe(1);
	});

	it("sin primero elegido sigue sorteándose", () => {
		expect(ajustesTrasOrden({ ...base, primero: null }, [1, 0, 2, 3]).primero).toBeNull();
	});

	it("quitar un asiento recoloca o borra lo que apuntaba a él", () => {
		expect(ajustesTrasQuitar(base, 1)).toMatchObject({ primero: 0, archienemigo: 1 });
		expect(ajustesTrasQuitar(base, 0)).toMatchObject({ primero: null, archienemigo: 1 });
		expect(ajustesTrasQuitar(base, 2)).toMatchObject({ primero: 0, archienemigo: 0 });
	});

	it("el primero va en la configuración, y solo si existe", () => {
		const dos = [asiento(1), asiento(2)];
		expect(construirConfig(dos, { ...AJUSTES_LOCALES, primero: 1 }, nombre).primero).toBe(1);
		expect(
			construirConfig(dos, { ...AJUSTES_LOCALES, primero: null }, nombre).primero,
		).toBeUndefined();
		expect(
			construirConfig(dos, { ...AJUSTES_LOCALES, primero: 5 }, nombre).primero,
		).toBeUndefined();
	});
});
