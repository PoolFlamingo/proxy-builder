import { describe, expect, it } from "vitest";
import { sinMarca } from "../sin-hover-solo.mjs";

describe("sinMarca", () => {
	it("pilla un onPointerEnter sin comentario", () => {
		expect(sinMarca("<div\n  onPointerEnter={f}\n/>")).toEqual([2]);
	});
	it("lo deja pasar con el comentario en las líneas de antes", () => {
		expect(
			sinMarca("<div\n  // táctil: un toque\n  onPointerEnter={f}\n  onPointerLeave={g}\n/>"),
		).toEqual([]);
	});
	it("pilla un addEventListener de contextmenu", () => {
		expect(sinMarca('window.addEventListener("contextmenu", f);')).toEqual([1]);
		expect(
			sinMarca('// táctil: no hace falta\nwindow.addEventListener("contextmenu", f);'),
		).toEqual([]);
	});
	it("no mira comentarios ni onPointerDown", () => {
		expect(sinMarca("// onPointerEnter = cosa\n<div onPointerDown={f} />")).toEqual([]);
	});
});
