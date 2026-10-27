import { describe, expect, it } from "vitest";
import type { FilaPartida } from "@/bindings/FilaPartida";
import type { JugadorHistorial } from "@/bindings/JugadorHistorial";
import {
	coincideTexto,
	FILTRO_VACIO,
	filtrarPartidas,
	hayFiltro,
	victoriasDe,
} from "../cuentas-historial";

const jugador = (
	i: number,
	deck: string,
	lista = "",
	puesto = 1,
	nombre = `J${i}`,
): JugadorHistorial => ({
	jugador: i,
	nombre,
	color: "#fff",
	deckId: deck,
	deck,
	listaId: lista ? `l-${lista}` : "",
	lista,
	cartasLista: lista ? 100 : 0,
	puesto,
});

const fila = (
	id: string,
	js: JugadorHistorial[],
	extra: Partial<FilaPartida> = {},
): FilaPartida => {
	const suyos = js.filter((j) => j.deckId).map((j) => j.puesto);
	const puesto = suyos.length === 0 ? 0 : Math.min(...suyos);
	return {
		id,
		terminada: 0,
		formato: "commander",
		modo: "local",
		jugadores: js,
		ganadores: [],
		como: "combate",
		nota: "",
		rondas: 5,
		duracionMs: 1000,
		puesto,
		gano: puesto === 1,
		repetible: false,
		...extra,
	};
};

describe("filtrarPartidas", () => {
	const a = fila("a", [jugador(0, "Elfos", "Cien", 1)]);
	const b = fila("b", [jugador(0, "Goblins", "", 3, "Ana")], {
		formato: "limited",
		nota: "Fue por un Combo",
	});
	const c = fila("c", [jugador(0, "", "", 1)], { puesto: 0, gano: false });
	const todas = [a, b, c];

	it("sin filtro deja todo", () => {
		expect(filtrarPartidas(todas, FILTRO_VACIO)).toEqual(todas);
		expect(hayFiltro(FILTRO_VACIO)).toBe(false);
	});

	it("busca en deck, lista, jugador y nota, sin tildes ni mayúsculas", () => {
		const q = (texto: string) =>
			filtrarPartidas(todas, { ...FILTRO_VACIO, texto }).map((x) => x.id);
		expect(q("elfos")).toEqual(["a"]);
		expect(q("CIEN")).toEqual(["a"]);
		expect(q("ana")).toEqual(["b"]);
		expect(q("combo")).toEqual(["b"]);
		expect(q("goblins ana")).toEqual(["b"]);
		expect(q("goblins elfos")).toEqual([]);
		expect(coincideTexto(a, "  ")).toBe(true);
		expect(hayFiltro({ ...FILTRO_VACIO, texto: "x" })).toBe(true);
	});

	it("las tildes no estorban", () => {
		const f = fila("t", [jugador(0, "Ént", "Árbol")]);
		expect(coincideTexto(f, "ent arbol")).toBe(true);
		expect(coincideTexto(f, "Ént Árbol")).toBe(true);
	});

	it("ganadas y perdidas; sin deck propio no es ni una ni otra", () => {
		const r = (resultado: "ganadas" | "perdidas") =>
			filtrarPartidas(todas, { ...FILTRO_VACIO, resultado }).map((x) => x.id);
		expect(r("ganadas")).toEqual(["a"]);
		expect(r("perdidas")).toEqual(["b"]);
	});

	it("por formato", () => {
		const r = filtrarPartidas(todas, { ...FILTRO_VACIO, formato: "limited" });
		expect(r.map((x) => x.id)).toEqual(["b"]);
	});

	it("cuenta victorias", () => {
		expect(victoriasDe(todas)).toBe(1);
	});
});
