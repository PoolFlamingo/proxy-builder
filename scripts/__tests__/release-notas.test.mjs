import { describe, expect, it } from "vitest";
import {
	componer,
	ErrorDeUso,
	leerArgs,
	limpiarNotas,
	plantilla,
	TEXTO_FIJO,
} from "../release-notas.mjs";

describe("leerArgs", () => {
	it("lee el tipo y las opciones sin valor", () => {
		expect(leerArgs(["patch", "--dry", "--publicar"])).toMatchObject({
			tipo: "patch",
			dry: true,
			publicarYa: true,
			sinNotas: false,
		});
	});

	it("el valor de --notas no se toma por el tipo", () => {
		const r = leerArgs(["--notas", "Arreglos", "minor"]);
		expect(r.tipo).toBe("minor");
		expect(r.notas).toBe("Arreglos");
		expect(leerArgs(["1.4.0", "--notas-fichero", "notas.md"]).notasFichero).toBe("notas.md");
	});

	it("falla con opciones que no conoce, sin valor o incompatibles", () => {
		expect(() => leerArgs(["patch", "--nota", "x"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "--notas"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "--notas", "--dry"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "--notas", "x", "--sin-notas"])).toThrow(ErrorDeUso);
		expect(() => leerArgs(["patch", "minor"])).toThrow(ErrorDeUso);
	});
});

describe("notas", () => {
	it("la plantilla entera es un comentario: limpia queda vacía", () => {
		const p = plantilla("2.3.0", ["abc123 Arreglo", "def456 Otra cosa"]);
		expect(p).toContain("abc123 Arreglo");
		expect(limpiarNotas(p)).toBe("");
		expect(limpiarNotas(`${p}## Novedades\n\n- OCR mejor\n`)).toBe("## Novedades\n\n- OCR mejor");
	});

	it("el texto fijo va debajo de las notas", () => {
		expect(componer("Hola")).toBe(`Hola\n\n${TEXTO_FIJO}\n`);
		expect(componer("<!-- nada -->")).toBe(`${TEXTO_FIJO}\n`);
	});
});
