import { describe, expect, it } from "vitest";
import type { Accion } from "@/bindings/Accion";
import type { Cambio } from "@/bindings/Cambio";
import type { Evento } from "@/bindings/Evento";
import type { VistaCarta } from "@/bindings/VistaCarta";
import {
	CATEGORIAS_DE_ACCION,
	CATEGORIAS_DE_CAMBIO,
	categoriaDeEvento,
} from "../categorias-eventos";
import { cartaParaApilar, duracionDeTurno } from "../cuentas-registro";
import { FRASES, piezasDeTexto, textoDeEvento } from "../textos-eventos";

const t = (es: string, vars?: Record<string, string | number>) =>
	es.replace(/\{(\w+)\}/g, (_, k: string) => String(vars?.[k] ?? `{${k}}`));

const ev = (seq: number, tMs: number, cambio: Cambio): Evento => ({
	seq,
	tMs,
	actor: { tipo: "jugador", id: 0 },
	cambio,
});
const accion = (seq: number, tMs: number, a: Accion) => ev(seq, tMs, { tipo: "accion", accion: a });

describe("categorías y frases", () => {
	it("cada acción tiene frase y categoría (y al revés)", () => {
		expect(Object.keys(CATEGORIAS_DE_ACCION).sort()).toEqual(Object.keys(FRASES).sort());
	});
	it("cada tipo de cambio tiene categoría o la da la acción", () => {
		expect(Object.keys(CATEGORIAS_DE_CAMBIO).sort()).toEqual(
			["accion", "dados", "dadoPlanar", "monedas", "restaurar", "sparring"].sort(),
		);
	});
	it("otra partida no es «deshacer»", () => {
		expect(categoriaDeEvento(ev(1, 0, { tipo: "restaurar", deshecho: null }))).toBe("fin");
		expect(categoriaDeEvento(ev(1, 0, { tipo: "restaurar", deshecho: 3 }))).toBe("deshacer");
		expect(categoriaDeEvento(accion(1, 0, { que: "robar", n: 1 }))).toBe("biblioteca");
	});
});

describe("duracionDeTurno", () => {
	const reg = [
		accion(0, 1_000, { que: "mulligan" }),
		accion(1, 11_000, { que: "pasarTurno" }),
		accion(2, 20_000, { que: "robar", n: 1 }),
		accion(3, 62_000, { que: "pasarTurno" }),
	];
	it("cuenta desde el turno anterior o desde el principio", () => {
		expect(duracionDeTurno(reg, 1)).toBe(10_000);
		expect(duracionDeTurno(reg, 3)).toBe(51_000);
		expect(duracionDeTurno(reg, 2)).toBeNull();
	});
	it("sin el principio en la lista no se sabe", () => {
		expect(duracionDeTurno(reg.slice(2, 4), 1)).toBeNull();
		expect(duracionDeTurno(reg.slice(2, 4), 0)).toBeNull();
	});
	it("otra partida reinicia la cuenta", () => {
		const r = [
			accion(5, 100, { que: "pasarTurno" }),
			ev(6, 5_000, { tipo: "restaurar", deshecho: null }),
			accion(7, 9_000, { que: "pasarTurno" }),
		];
		expect(duracionDeTurno(r, 2)).toBe(4_000);
	});
});

describe("cartaParaApilar", () => {
	const mover = (id: number, zona: "campo" | "mano"): Accion => ({
		que: "mover",
		cartas: [id],
		a: { zona: { tipo: "de", zona }, jugador: null, sitio: "arriba" } as never,
		como: null,
	});
	const enCampo = { zona: { tipo: "de", jugador: 0, zona: "campo" } } as VistaCarta;
	it("solo una carta que se puso en el campo y sigue allí", () => {
		const reg = [accion(1, 0, mover(7, "campo")), accion(2, 0, mover(8, "mano"))];
		expect(cartaParaApilar(reg, 0, { 7: enCampo, 8: enCampo })).toBe(7);
		expect(cartaParaApilar(reg, 1, { 7: enCampo, 8: enCampo })).toBeNull();
		expect(cartaParaApilar(reg, 0, {})).toBeNull();
	});
	it("solo en las últimas líneas", () => {
		const reg = Array.from({ length: 20 }, (_, i) => accion(i, 0, mover(7, "campo")));
		expect(cartaParaApilar(reg, 0, { 7: enCampo })).toBeNull();
		expect(cartaParaApilar(reg, 19, { 7: enCampo })).toBe(7);
	});
});

describe("textos en piezas", () => {
	const cartas = { 7: { origen: { nombre: "Sol Ring" } } } as unknown as Record<number, VistaCarta>;
	const jugadores = { 0: { nombre: "Ana" } } as never;
	it("el nombre de la carta sale como pieza aparte", () => {
		const e = accion(1, 0, { que: "adjuntar", carta: 7, a: 3 } as never);
		const texto = textoDeEvento(e, { cartas, jugadores }, t, true);
		expect(piezasDeTexto(texto)).toEqual(["Ana adjunta ", { carta: 7, nombre: "Sol Ring" }]);
		expect(textoDeEvento(e, { cartas, jugadores }, t)).toBe("Ana adjunta Sol Ring");
	});
	it("lo oculto sigue siendo «una carta»", () => {
		const e = accion(1, 0, { que: "adjuntar", carta: 9, a: 3 } as never);
		expect(textoDeEvento(e, { cartas, jugadores }, t, true)).toBe("Ana adjunta una carta");
	});
	it("pasar el turno dice cuánto tardó", () => {
		const e = accion(1, 0, { que: "pasarTurno" });
		expect(textoDeEvento(e, { cartas, jugadores }, t, false, "00:51")).toBe(
			"Ana pasa el turno tras 00:51",
		);
	});
});
