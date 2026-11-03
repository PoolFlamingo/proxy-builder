import { describe, expect, it } from "vitest";
import { crearVigilaTeclado, UMBRAL_TECLADO } from "@/lib/teclado";

describe("crearVigilaTeclado", () => {
	const base = { ancho: 411, altoVentana: 900, altoVisual: 900, arriba: 0 };

	it("sin teclado, todo el alto de la ventana", () => {
		const v = crearVigilaTeclado();
		expect(v(base)).toEqual({ abierto: false, altoVisible: 900 });
	});

	it("el teclado encoge el visor y el alto visible es el del visor", () => {
		const v = crearVigilaTeclado();
		v(base);
		expect(v({ ...base, altoVisual: 560 })).toEqual({ abierto: true, altoVisible: 560 });
	});

	it("al cerrarse el teclado vuelve todo", () => {
		const v = crearVigilaTeclado();
		v(base);
		v({ ...base, altoVisual: 560 });
		expect(v(base)).toEqual({ abierto: false, altoVisible: 900 });
	});

	it("un encogimiento pequeño (la barra de direcciones) no es un teclado", () => {
		const v = crearVigilaTeclado();
		v(base);
		expect(v({ ...base, altoVisual: 900 - UMBRAL_TECLADO }).abierto).toBe(false);
	});

	it("el desplazamiento del visor cuenta como alto visible", () => {
		const v = crearVigilaTeclado();
		v(base);
		expect(v({ ...base, altoVisual: 500, arriba: 60 })).toEqual({
			abierto: true,
			altoVisible: 560,
		});
	});

	it("girar la pantalla reinicia lo recordado", () => {
		const v = crearVigilaTeclado();
		v(base);
		// Horizontal: otro ancho y menos alto; no es un teclado.
		expect(v({ ancho: 900, altoVentana: 411, altoVisual: 411, arriba: 0 })).toEqual({
			abierto: false,
			altoVisible: 411,
		});
	});

	it("si la ventana misma se encoge con el teclado, no hay nada que compensar", () => {
		const v = crearVigilaTeclado();
		v(base);
		const e = v({ ...base, altoVentana: 560, altoVisual: 560 });
		expect(e.abierto).toBe(true);
		expect(e.altoVisible).toBe(560);
	});
});
