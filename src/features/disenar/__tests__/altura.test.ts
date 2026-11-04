import { describe, expect, it } from "vitest";
import { altoCarta, altoVisible, anchoColumnaCarta, esCampoDeTexto } from "../altura";

describe("altoVisible", () => {
	it("sin teclado el contenedor entero", () => {
		expect(altoVisible({ contenedor: 700, arriba: 50, visualAlto: 914, visualArriba: 0 })).toBe(
			700,
		);
	});

	it("con el teclado se recorta a lo que se ve", () => {
		// El teclado se come 330 px: el viewport visual mide 584.
		expect(altoVisible({ contenedor: 700, arriba: 50, visualAlto: 584, visualArriba: 0 })).toBe(
			534,
		);
	});

	it("si el viewport visual se ha desplazado, cuenta desde su arriba", () => {
		expect(altoVisible({ contenedor: 700, arriba: -20, visualAlto: 584, visualArriba: 40 })).toBe(
			644,
		);
	});

	it("nunca sale negativo", () => {
		expect(altoVisible({ contenedor: 700, arriba: 900, visualAlto: 300, visualArriba: 0 })).toBe(0);
	});
});

describe("altoCarta", () => {
	it("la carta encoge mientras se escribe", () => {
		expect(altoCarta(700, false)).toBe(308);
		expect(altoCarta(700, true)).toBe(154);
	});
});

describe("anchoColumnaCarta", () => {
	it("cabe la carta entera a todo el alto y sus controles a los lados", () => {
		expect(anchoColumnaCarta(352)).toBe(252 + 96);
	});
});

describe("esCampoDeTexto", () => {
	it("campos de texto sí; casillas y deslizadores no", () => {
		expect(esCampoDeTexto({ tagName: "INPUT", type: "text" })).toBe(true);
		expect(esCampoDeTexto({ tagName: "INPUT" })).toBe(true);
		expect(esCampoDeTexto({ tagName: "TEXTAREA" })).toBe(true);
		expect(esCampoDeTexto({ tagName: "DIV", isContentEditable: true })).toBe(true);
		expect(esCampoDeTexto({ tagName: "INPUT", type: "checkbox" })).toBe(false);
		expect(esCampoDeTexto({ tagName: "INPUT", type: "range" })).toBe(false);
		expect(esCampoDeTexto({ tagName: "BUTTON" })).toBe(false);
	});
});
