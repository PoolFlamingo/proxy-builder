import { describe, expect, it } from "vitest";
import type { EstiloPropio } from "@/bindings/EstiloPropio";
import { elegirEstilo, opcionesEstilo, propiosDe, valorEstilo } from "../estilos";

const propios: EstiloPropio[] = [
	{ id: "tarot", nombres: { es: "Tarot", en: "Tarot card" }, base: "classic", variantes: [] },
	{ id: "planeta", nombres: { es: "Planeta", fr: "Planète" }, base: "borderless", variantes: [] },
];
const t = (s: string) => s;

describe("estilos propios del pack", () => {
	it("los de serie y los del pack en el mismo selector", () => {
		const o = opcionesEstilo(t, "en", propios);
		expect(o.map((x) => x.valor)).toEqual([
			"borderless",
			"extended",
			"classic",
			"fullArt",
			"pack:tarot",
			"pack:planeta",
		]);
		expect(o[4].etiqueta).toBe("Tarot card");
		// Cada idioma el suyo; sin él, el inglés; sin inglés, el id.
		expect(opcionesEstilo(t, "fr", propios)[5].etiqueta).toBe("Planète");
		expect(opcionesEstilo(t, "de", propios)[4].etiqueta).toBe("Tarot card");
		expect(opcionesEstilo(t, "de", propios)[5].etiqueta).toBe("planeta");
		// Uno que el pack ya no tiene sigue a la vista.
		expect(opcionesEstilo(t, "es", [], "pack:vale").at(-1)?.valor).toBe("pack:vale");
	});

	it("elegir uno propio guarda su base; uno de serie lo quita", () => {
		expect(elegirEstilo("pack:planeta", propios, "classic")).toEqual({
			style: "borderless",
			estiloPack: "planeta",
		});
		expect(elegirEstilo("extended", propios, "classic")).toEqual({
			style: "extended",
			estiloPack: undefined,
		});
		expect(valorEstilo("classic", "tarot")).toBe("pack:tarot");
		expect(valorEstilo("classic", null)).toBe("classic");
		expect(propiosDe([], "x")).toEqual([]);
	});
});
