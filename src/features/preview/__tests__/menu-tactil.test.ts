import { describe, expect, it } from "vitest";
import { tocarAbreElMenu } from "../menu-tactil";

const base = {
	tactil: true,
	estabaSeleccionada: true,
	hueco: false,
	modificador: false,
	toques: 1,
};

describe("tocarAbreElMenu", () => {
	it("un toque en la casilla seleccionada abre el menú", () => {
		expect(tocarAbreElMenu(base)).toBe(true);
	});

	it("el primer toque solo selecciona", () => {
		expect(tocarAbreElMenu({ ...base, estabaSeleccionada: false })).toBe(false);
	});

	it("con ratón no cambia nada", () => {
		expect(tocarAbreElMenu({ ...base, tactil: false })).toBe(false);
	});

	it("un hueco abre «Añadir carta», no el menú", () => {
		expect(tocarAbreElMenu({ ...base, hueco: true })).toBe(false);
	});

	it("el segundo toque de un doble toque abre el editor, no el menú", () => {
		expect(tocarAbreElMenu({ ...base, toques: 2 })).toBe(false);
	});

	it("con Ctrl o Mayús es una selección múltiple", () => {
		expect(tocarAbreElMenu({ ...base, modificador: true })).toBe(false);
	});
});
