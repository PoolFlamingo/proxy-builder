import { describe, expect, it } from "vitest";
import type { CardDesign } from "@/bindings/CardDesign";
import { cambiarEnCara, conAjustesPropios, proyectar, tieneAjustes } from "../por-cara";

const base = {
	style: "borderless",
	status: "draft",
	collection: "c1",
	emblem: { kind: "inherit" },
	options: { originalName: "auto", showFlavor: true },
	faces: [{ texts: { es: {}, en: {}, otros: {} } }, { texts: { es: {}, en: {}, otros: {} } }],
} as unknown as CardDesign;

/** Aplica `f` como lo haría el store: sobre el diseño actual. */
const aplicar = (d: CardDesign, cara: number, f: (d: CardDesign) => CardDesign) => {
	let r = d;
	cambiarEnCara((g) => {
		r = g(d);
	}, cara)(f);
	return r;
};

describe("estilo y ajustes por cara", () => {
	it("una cara sin ajustes propios se edita como la carta", () => {
		const r = aplicar(base, 1, (d) => ({ ...d, style: "classic" }));
		expect(r.style).toBe("classic");
		expect(tieneAjustes(r, 1)).toBe(false);
	});

	it("dar ajustes propios parte de los de la carta y quitarlos los borra", () => {
		const d = conAjustesPropios({ ...base, style: "extended" }, 1, true);
		expect(d.faces[1]?.ajustes?.style).toBe("extended");
		expect(d.faces[0]?.ajustes).toBeUndefined();
		expect(tieneAjustes(conAjustesPropios(d, 1, false), 1)).toBe(false);
	});

	it("lo que cambia la cara con ajustes propios no toca el estilo de la carta", () => {
		const d = conAjustesPropios(base, 1, true);
		const r = aplicar(d, 1, (x) => ({
			...x,
			style: "classic",
			variantePack: "v2",
			options: { ...x.options, showFlavor: false },
		}));
		expect(r.style).toBe("borderless");
		expect(r.options.showFlavor).toBe(true);
		expect(r.faces[1]?.ajustes?.style).toBe("classic");
		expect(r.faces[1]?.ajustes?.variantePack).toBe("v2");
		expect(r.faces[1]?.ajustes?.options.showFlavor).toBe(false);
		expect(proyectar(r, 1).style).toBe("classic");
		expect(proyectar(r, 0).style).toBe("borderless");
	});

	it("lo que no es de la cara (la colección, el estado) va a la carta", () => {
		const d = conAjustesPropios(base, 1, true);
		const r = aplicar(d, 1, (x) => ({ ...x, collection: "c2", status: "ready" }));
		expect(r.collection).toBe("c2");
		expect(r.status).toBe("ready");
		expect(r.faces[1]?.ajustes?.style).toBe("borderless");
	});
});
