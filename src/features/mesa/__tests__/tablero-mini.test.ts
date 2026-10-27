import { describe, expect, it } from "vitest";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { VistaJugador } from "@/bindings/VistaJugador";
import { MESA_VACIA, type Mesa } from "@/stores/mesa-store";
import { igualesLista } from "../lento";
import { datosMini, igualMini } from "../tablero-mini";

const jugador = (id: number, parte: Partial<VistaJugador> = {}): VistaJugador =>
	({
		id,
		nombre: `J${id}`,
		color: "#fff",
		equipo: null,
		vida: 40,
		eliminado: null,
		...parte,
	}) as VistaJugador;
const carta = (id: number, parte: Partial<VistaCarta> = {}): VistaCarta =>
	({
		id,
		pos: { x: 100, y: 100 },
		girada: false,
		adjuntaA: null,
		agrupadaBajo: null,
		...parte,
	}) as VistaCarta;

function mesa(): Pick<Mesa, "jugadores" | "zonas" | "cartas" | "general"> {
	return {
		jugadores: { 0: jugador(0), 1: jugador(1, { equipo: 7 }) },
		cartas: { 10: carta(10), 11: carta(11, { girada: true }) },
		zonas: {
			"1:campo": { zona: { tipo: "de", jugador: 1, zona: "campo" }, cartas: [10, 11], cuantas: 2 },
			"1:mano": { zona: { tipo: "de", jugador: 1, zona: "mano" }, cartas: [], cuantas: 5 },
			"1:biblioteca": {
				zona: { tipo: "de", jugador: 1, zona: "biblioteca" },
				cartas: [],
				cuantas: 60,
			},
		},
		general: {
			...(MESA_VACIA.general ?? {}),
			turno: { activo: 1 },
			equipos: [{ id: 7, nombre: "A", color: "#00f", vidaCompartida: 30 }],
		},
	} as unknown as Pick<Mesa, "jugadores" | "zonas" | "cartas" | "general">;
}

describe("datosMini", () => {
	it("saca el campo, los números, el turno y el equipo del jugador", () => {
		const d = datosMini(mesa(), 1);
		expect(d.j?.nombre).toBe("J1");
		expect(d.campo.map((c) => c.id)).toEqual([10, 11]);
		expect(d.mano).toBe(5);
		expect(d.biblioteca).toBe(60);
		expect(d.activo).toBe(true);
		expect(d.equipo?.vidaCompartida).toBe(30);
	});

	it("un jugador sin nada tiene todo a cero", () => {
		const d = datosMini(mesa(), 0);
		expect(d.campo).toEqual([]);
		expect(d.mano).toBe(0);
		expect(d.activo).toBe(false);
		expect(d.equipo).toBeUndefined();
	});

	it("ignora las cartas del campo que no están en la mesa", () => {
		const m = mesa();
		delete m.cartas[11];
		expect(datosMini(m, 1).campo.map((c) => c.id)).toEqual([10]);
	});
});

describe("igualMini", () => {
	it("lo mismo leído dos veces no repinta", () => {
		const m = mesa();
		expect(igualMini(datosMini(m, 1), datosMini(m, 1))).toBe(true);
	});

	it("repinta si cambia una carta del campo, la vida o un contador de zona", () => {
		const m = mesa();
		const antes = datosMini(m, 1);
		const conCarta = { ...m, cartas: { ...m.cartas, 10: carta(10, { girada: true }) } };
		expect(igualMini(antes, datosMini(conCarta, 1))).toBe(false);
		const conVida = { ...m, jugadores: { ...m.jugadores, 1: jugador(1, { vida: 39, equipo: 7 }) } };
		expect(igualMini(antes, datosMini(conVida, 1))).toBe(false);
		const conMano = {
			...m,
			zonas: { ...m.zonas, "1:mano": { ...m.zonas["1:mano"], cuantas: 6 } },
		};
		expect(igualMini(antes, datosMini(conMano, 1))).toBe(false);
	});

	it("no repinta por lo que cambia en otra parte de la mesa", () => {
		const m = mesa();
		const antes = datosMini(m, 1);
		const otra = { ...m, cartas: { ...m.cartas, 99: carta(99) } };
		expect(igualMini(antes, datosMini(otra, 1))).toBe(true);
	});
});

describe("igualesLista", () => {
	it("compara por identidad elemento a elemento", () => {
		const a = {};
		expect(igualesLista([a], [a])).toBe(true);
		expect(igualesLista([a], [{}])).toBe(false);
		expect(igualesLista([a], [a, a])).toBe(false);
	});
});

describe("la mano en la miniatura", () => {
	it("cuenta las cartas que se ven (la mano de un compañero) aparte del total", () => {
		const m = mesa();
		m.zonas["1:mano"] = {
			zona: { tipo: "de", jugador: 1, zona: "mano" },
			cartas: [10],
			cuantas: 5,
		} as (typeof m.zonas)[string];
		const d = datosMini(m, 1);
		expect(d.mano).toBe(5);
		expect(d.manoVista.map((c) => c.id)).toEqual([10]);
	});

	it("de la mano de un rival solo sabe cuántas: todo reversos", () => {
		const d = datosMini(mesa(), 1);
		expect(d.mano).toBe(5);
		expect(d.manoVista).toEqual([]);
	});

	it("dos lecturas con otras cartas visibles no son iguales", () => {
		const a = datosMini(mesa(), 1);
		const m = mesa();
		m.zonas["1:mano"] = {
			zona: { tipo: "de", jugador: 1, zona: "mano" },
			cartas: [10],
			cuantas: 5,
		} as (typeof m.zonas)[string];
		expect(igualMini(a, datosMini(m, 1))).toBe(false);
	});
});
