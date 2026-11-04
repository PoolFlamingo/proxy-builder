import { describe, expect, it } from "vitest";
import type { DeckRevisado } from "@/bindings/DeckRevisado";
import type { EstadoReceptor } from "@/bindings/EstadoReceptor";
import {
	aDecisiones,
	cuentaAtras,
	eleccionesIniciales,
	emisorEnCurso,
	faltaResolver,
	formatoCodigo6,
	opcionesDe,
	pareceCodigoDeDeck,
	pesar,
	pesoDe,
	porcentaje,
	puedeResponder,
	quedaAlgo,
	receptorEnCurso,
	reemplazados,
	textoDeError,
} from "../cuentas";

function deck(nombre: string, proxies: boolean, conflicto = false): DeckRevisado {
	return {
		deck: {
			nombre,
			cartas: 60,
			proxies,
			tamano: 1000,
			tamanoOriginales: 100,
			linaje: "l",
			actualizado: 5,
			portada: null,
			optimizado: false,
		},
		conflicto: conflicto ? { deckId: "local", nombre, actualizado: 3, masReciente: "llega" } : null,
	};
}

describe("elegir qué recibir", () => {
	const oferta = [deck("A", true), deck("B", true, true), deck("C", false)];

	it("empieza con todo aceptado y los que ya existen sin resolver", () => {
		const e = eleccionesIniciales(oferta);
		expect(e.every((x) => x.eleccion === "conProxies" && x.resolucion === null)).toBe(true);
		expect(faltaResolver(oferta, e)).toEqual([1]);
		expect(puedeResponder(oferta, e)).toBe(false);
	});

	it("se puede seguir al resolver o al decir que no a ese deck", () => {
		const e = eleccionesIniciales(oferta);
		e[1] = { eleccion: "conProxies", resolucion: "reemplazar" };
		expect(puedeResponder(oferta, e)).toBe(true);
		const f = eleccionesIniciales(oferta);
		f[1] = { eleccion: "no", resolucion: null };
		expect(puedeResponder(oferta, f)).toBe(true);
	});

	it("manda la resolución solo de los que existen y se aceptan", () => {
		const e = eleccionesIniciales(oferta);
		e[0] = { eleccion: "soloOriginales", resolucion: "guardarLosDos" };
		e[1] = { eleccion: "conProxies", resolucion: "guardarLosDos" };
		e[2] = { eleccion: "no", resolucion: "reemplazar" };
		expect(aDecisiones(oferta, e)).toEqual([
			{ indice: 0, eleccion: "soloOriginales", resolucion: null },
			{ indice: 1, eleccion: "conProxies", resolucion: "guardarLosDos" },
			{ indice: 2, eleccion: "no", resolucion: null },
		]);
		expect(quedaAlgo(e)).toBe(true);
		expect(quedaAlgo([{ eleccion: "no", resolucion: null }])).toBe(false);
	});

	it("dice qué decks de aquí se reemplazan", () => {
		const e = eleccionesIniciales(oferta);
		expect(reemplazados(oferta, e)).toEqual([]);
		e[1] = { eleccion: "conProxies", resolucion: "reemplazar" };
		e[0] = { eleccion: "conProxies", resolucion: "reemplazar" };
		expect(reemplazados(oferta, e)).toEqual(["local"]);
		e[1] = { eleccion: "no", resolucion: "reemplazar" };
		expect(reemplazados(oferta, e)).toEqual([]);
	});

	it("«solo originales» solo se ofrece si el deck trae proxies", () => {
		expect(opcionesDe(oferta[0])).toEqual(["conProxies", "soloOriginales", "no"]);
		expect(opcionesDe(oferta[2])).toEqual(["conProxies", "no"]);
	});
});

describe("textos y cuentas", () => {
	it("pesa en la unidad que toca", () => {
		expect(pesar(500)).toBe("500 B");
		expect(pesar(340 * 1024)).toBe("340 KB");
		expect(pesar(1.5 * 1024 * 1024, "en")).toBe("1.5 MB");
		expect(pesar(120 * 1024 * 1024)).toBe("120 MB");
		expect(pesar(3 * 1024 * 1024 * 1024, "en")).toBe("3 GB");
	});

	it("agrupa el código de 6 cifras y la cuenta atrás no baja de cero", () => {
		expect(formatoCodigo6("012345")).toBe("012 345");
		expect(formatoCodigo6("12")).toBe("12");
		expect(cuentaAtras(300_000, 0)).toBe("5:00");
		expect(cuentaAtras(300_000, 268_000)).toBe("0:32");
		expect(cuentaAtras(1000, 5000)).toBe("0:00");
	});

	it("el porcentaje suma los decks hechos y lo que va del actual", () => {
		expect(porcentaje(null)).toBe(0);
		const p = (hechos: number, total: number, dh: number, dt: number) => ({
			indice: 0,
			nombre: "x",
			hechos,
			total,
			decksHechos: dh,
			decksTotal: dt,
		});
		expect(porcentaje(p(50, 100, 1, 2))).toBe(75);
		expect(porcentaje(p(0, 0, 0, 0))).toBe(0);
		expect(porcentaje(p(100, 100, 2, 2))).toBe(100);
	});

	it("sabe cuándo cerrar la pantalla cancelaría algo", () => {
		const r = (fase: EstadoReceptor["fase"]) => ({ fase }) as EstadoReceptor;
		expect(receptorEnCurso(r("esperando"))).toBe(false);
		expect(receptorEnCurso(r("oferta"))).toBe(true);
		expect(receptorEnCurso(r("terminado"))).toBe(false);
		expect(receptorEnCurso(null)).toBe(false);
		expect(emisorEnCurso(null)).toBe(false);
	});

	it("reconoce un código de deck pegado con espacios y saltos", () => {
		expect(pareceCodigoDeDeck("  PBD1-abc\n def")).toBe(true);
		expect(pareceCodigoDeDeck("PB1-abc")).toBe(false);
	});

	it("un error con su motivo sale con su texto y, si no, con el general", () => {
		const t = (es: string) => es;
		const general = () => "GENERAL";
		expect(
			textoDeError({ code: "InvalidInput", params: { motivo: "codigo_de_partida" } }, t, general),
		).toContain("PBD1-");
		expect(textoDeError({ code: "Network", params: { motivo: "raro" } }, t, general)).toBe(
			"GENERAL",
		);
		expect(textoDeError(new Error("x"), t, general)).toBe("GENERAL");
	});
});

describe("pesoDe", () => {
	const info = { proxies: true, tamano: 500, tamanoOptimizado: 120, tamanoOriginales: 30 };
	it("da lo que pesa en cada modo", () => {
		expect(pesoDe(info, "original")).toBe(500);
		expect(pesoDe(info, "optimizado")).toBe(120);
		expect(pesoDe(info, "sinProxies")).toBe(30);
		expect(pesoDe(undefined, "original")).toBe(0);
	});
});
