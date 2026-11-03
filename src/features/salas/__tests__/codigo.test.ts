import { describe, expect, it } from "vitest";
import {
	codigoBonito,
	codigoDeSala,
	codigoDeSalaCompleto,
	limpiarCodigoDirecto,
	textoParaCompartir,
} from "../codigo";

describe("el código de una sala pegado", () => {
	it("un enlace entero deja solo su código", () => {
		expect(codigoDeSala("proxybuilder://unirse/ABCDE12345")).toBe("ABCDE12345");
		expect(codigoDeSala("  PROXYBUILDER://unirse/ABCDE12345?x=1#y  ")).toBe("ABCDE12345");
		expect(codigoDeSala("proxybuilder://unirse/ABCDE-12345")).toBe("ABCDE-12345");
	});
	it("un código suelto se queda como está", () => {
		expect(codigoDeSala("  ABCDE-12345 ")).toBe("ABCDE-12345");
		expect(codigoDeSala("https://otra.cosa/ABCDE12345")).toBe("https://otra.cosa/ABCDE12345");
	});
	it("se sabe si está completo con o sin guion, suelto o en enlace", () => {
		expect(codigoDeSalaCompleto("ABCDE-12345")).toBe(true);
		expect(codigoDeSalaCompleto("ABCDE 12345")).toBe(true);
		expect(codigoDeSalaCompleto("proxybuilder://unirse/ABCDE12345")).toBe(true);
		expect(codigoDeSalaCompleto("ABCDE-1234")).toBe(false);
		expect(codigoDeSalaCompleto("proxybuilder://unirse/ABC")).toBe(false);
	});
	it("se enseña como XXXXX-XXXXX", () => {
		expect(codigoBonito("ABCDE12345")).toBe("ABCDE-12345");
		expect(codigoBonito("ABCDE-12345")).toBe("ABCDE-12345");
		expect(codigoBonito("corto")).toBe("corto");
	});
});

describe("el código de partida directa pegado", () => {
	it("quita el principio del enlace y los espacios, y pone el prefijo en mayúsculas", () => {
		expect(limpiarCodigoDirecto(" proxybuilder://unirse/pb1-AbC dEf\n")).toBe("PB1-AbCdEf");
		expect(limpiarCodigoDirecto("pb1-xYz")).toBe("PB1-xYz");
		expect(limpiarCodigoDirecto("PB1-xYz")).toBe("PB1-xYz");
	});
});

describe("lo que se comparte", () => {
	it("el enlace y, debajo, el código; sin enlace, solo el código", () => {
		expect(textoParaCompartir("proxybuilder://unirse/ABCDE12345", "ABCDE-12345")).toBe(
			"proxybuilder://unirse/ABCDE12345\nABCDE-12345",
		);
		expect(textoParaCompartir(null, "ABCDE-12345")).toBe("ABCDE-12345");
	});
});
