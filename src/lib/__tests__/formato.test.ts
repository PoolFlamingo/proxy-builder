import { describe, expect, it } from "vitest";
import { formatoDe, formatoForzado, tamanoPorAncho } from "@/lib/formato";

const base = {
	ancho: 1440,
	alto: 900,
	puntero_grueso: false,
	plataforma: "escritorio" as const,
	forzado: null,
};

describe("tamanoPorAncho", () => {
	it("corta en 640 y 1024", () => {
		expect(tamanoPorAncho(412)).toBe("movil");
		expect(tamanoPorAncho(639)).toBe("movil");
		expect(tamanoPorAncho(640)).toBe("tableta");
		expect(tamanoPorAncho(1023)).toBe("tableta");
		expect(tamanoPorAncho(1024)).toBe("escritorio");
		expect(tamanoPorAncho(1280)).toBe("escritorio");
	});
});

describe("formatoDe", () => {
	it("un ordenador con ratón no es táctil", () => {
		expect(formatoDe(base)).toEqual({ tamano: "escritorio", tactil: false, horizontal: true });
	});

	it("Android es táctil aunque el puntero no se declare grueso", () => {
		const f = formatoDe({ ...base, ancho: 412, alto: 915, plataforma: "android" });
		expect(f).toEqual({ tamano: "movil", tactil: true, horizontal: false });
	});

	it("una pantalla táctil de escritorio sí lo es", () => {
		expect(formatoDe({ ...base, puntero_grueso: true }).tactil).toBe(true);
	});

	it("girar la tableta cambia horizontal y no el tamaño si el ancho sigue siendo de tableta", () => {
		const h = formatoDe({ ...base, ancho: 1000, alto: 800, plataforma: "android" });
		const v = formatoDe({ ...base, ancho: 800, alto: 1000, plataforma: "android" });
		expect(h.horizontal).toBe(true);
		expect(v.horizontal).toBe(false);
		expect(h.tamano).toBe("tableta");
		expect(v.tamano).toBe("tableta");
	});

	it("la tableta de 1280 dp en horizontal sale como escritorio y en vertical como tableta", () => {
		expect(formatoDe({ ...base, ancho: 1280, alto: 800, plataforma: "android" }).tamano).toBe(
			"escritorio",
		);
		expect(formatoDe({ ...base, ancho: 800, alto: 1280, plataforma: "android" }).tamano).toBe(
			"tableta",
		);
	});

	it("un teléfono de lado (914 × 411 dp) es un móvil aunque mida más de 640 de ancho", () => {
		const f = formatoDe({ ...base, ancho: 914, alto: 411, plataforma: "android" });
		expect(f).toEqual({ tamano: "movil", tactil: true, horizontal: true });
		// Una tableta no: su lado corto pasa de 600.
		expect(formatoDe({ ...base, ancho: 960, alto: 600, plataforma: "android" }).tamano).toBe(
			"tableta",
		);
		// Y un ordenador con el ratón, por baja que sea la ventana, tampoco.
		expect(formatoDe({ ...base, ancho: 800, alto: 500 }).tamano).toBe("tableta");
	});

	it("forzar el formato manda sobre el ancho y activa lo táctil", () => {
		const f = formatoDe({ ...base, forzado: "movil" });
		expect(f.tamano).toBe("movil");
		expect(f.tactil).toBe(true);
	});
});

describe("formatoForzado", () => {
	it("solo acepta movil y tableta", () => {
		expect(formatoForzado("movil")).toBe("movil");
		expect(formatoForzado("tableta")).toBe("tableta");
		expect(formatoForzado("escritorio")).toBeNull();
		expect(formatoForzado(undefined)).toBeNull();
	});
});
