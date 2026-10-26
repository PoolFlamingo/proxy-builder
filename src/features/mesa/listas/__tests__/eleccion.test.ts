import { describe, expect, it } from "vitest";
import type { CartaDeLista } from "@/bindings/CartaDeLista";
import {
	alternarCarta,
	alternarComandante,
	conCopias,
	cuantas,
	eleccionDeFabrica,
	impide,
} from "../eleccion";

const carta = (id: string, nombre: string, cambios: Partial<CartaDeLista> = {}): CartaDeLista => ({
	id,
	nombre,
	tipo: null,
	identidad: [],
	puedeSerComandante: false,
	conPareja: false,
	sinLimite: false,
	imagen: null,
	marca: null,
	...cambios,
});

const todas = [
	carta("k1", "Kruphix", { puedeSerComandante: true }),
	carta("k2", "Kruphix", { puedeSerComandante: true, marca: "variante" }),
	carta("o1", "Opt"),
	carta("o2", "Opt", { marca: "variante" }),
	carta("f1", "Forest", { sinLimite: true }),
	carta("f2", "Forest", { sinLimite: true }),
	carta("x", "Brainstorm", { marca: "opcional" }),
];

describe("elegir una lista para jugar", () => {
	it("de fábrica, las copias sin marca", () => {
		expect(eleccionDeFabrica(todas)).toEqual({ comandantes: [], cartas: ["k1", "o1", "f1", "f2"] });
	});

	it("en singleton, una variante quita la otra; las básicas no", () => {
		let e = eleccionDeFabrica(todas);
		e = alternarCarta(e, todas[3], todas, true);
		expect(e.cartas).toEqual(["k1", "f1", "f2", "o2"]);
		e = alternarCarta(e, todas[3], todas, true);
		expect(e.cartas).not.toContain("o2");
		const sin = alternarCarta({ comandantes: [], cartas: ["o1"] }, todas[3], todas, false);
		expect(sin.cartas).toEqual(["o1", "o2"]);
		expect(alternarCarta(e, todas[5], todas, true).cartas).toContain("f1");
	});

	it("el comandante es una variante concreta y sale de las demás cartas", () => {
		let e = eleccionDeFabrica(todas);
		e = alternarComandante(e, todas[1], todas);
		expect(e.comandantes).toEqual(["k2"]);
		expect(e.cartas).not.toContain("k1");
		expect(cuantas(e)).toBe(4);
		// Otra variante del mismo comandante lo reemplaza.
		e = alternarComandante(e, todas[0], todas);
		expect(e.comandantes).toEqual(["k1"]);
		// Elegirlo como carta lo quita de comandante.
		expect(alternarCarta(e, todas[1], todas, true).comandantes).toEqual([]);
	});

	it("las básicas se eligen por número", () => {
		const bosques = todas.filter((c) => c.nombre === "Forest");
		const e = conCopias({ comandantes: [], cartas: ["o1", "f2"] }, bosques, 1);
		expect(e.cartas).toEqual(["o1", "f1"]);
		expect(conCopias(e, bosques, 9).cartas).toEqual(["o1", "f1", "f2"]);
		expect(conCopias(e, bosques, -1).cartas).toEqual(["o1"]);
	});

	it("qué avisos impiden jugar", () => {
		expect(impide({ tipo: "sinComandante" })).toBe(true);
		expect(impide({ tipo: "fueraDeIdentidad", nombre: "Shock" })).toBe(false);
	});
});
