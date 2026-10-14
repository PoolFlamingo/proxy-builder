import { describe, expect, it } from "vitest";
import type { CardEdits } from "@/bindings/CardEdits";
import type { DeckCard } from "@/bindings/DeckCard";
import { aplicarEdicion, copiasDe } from "@/features/card-editor/propagar";
import { huecos } from "@/lib/deck-ops";
import { EDICION_NEUTRA } from "@/lib/image-url";

const carta = (id: string, blob: string | null): DeckCard => ({
	...huecos(1)[0],
	id,
	front: blob ? { blob, width: 10, height: 14, origin: "upload" } : null,
});

const edicion: CardEdits = {
	rotation: "r90",
	crop: { zoom: 1.5, offsetX: 0.1, offsetY: -0.2 },
	adjust: { ...EDICION_NEUTRA.adjust, brightness: 20, contrast: -10 },
};

describe("propagar la edición de una carta", () => {
	const deck = [
		carta("a1", "bolt"),
		carta("b", "elfo"),
		carta("a2", "bolt"),
		carta("hueco", null),
		carta("a3", "bolt"),
	];
	const a1 = deck[0];

	it("las copias son las otras con la misma imagen", () => {
		expect(copiasDe(deck, a1).map((c) => c.id)).toEqual(["a2", "a3"]);
		expect(copiasDe(deck, deck[1])).toEqual([]);
	});

	it("una carta sin imagen no tiene copias (aunque haya otros huecos)", () => {
		const conHuecos = [...deck, carta("hueco2", null)];
		expect(copiasDe(conHuecos, deck[3])).toEqual([]);
	});

	it("solo esta carta: el resto queda intacto (mismo objeto)", () => {
		const r = aplicarEdicion(deck, a1, edicion, "carta");
		expect(r[0].edits).toBe(edicion);
		for (const i of [1, 2, 3, 4]) expect(r[i]).toBe(deck[i]);
	});

	it("a sus copias: giro, encuadre y retoques, y nada a las demás", () => {
		const r = aplicarEdicion(deck, a1, edicion, "copias");
		expect(r[0].edits).toEqual(edicion);
		expect(r[2].edits).toEqual(edicion);
		expect(r[4].edits).toEqual(edicion);
		expect(r[1]).toBe(deck[1]);
		expect(r[3]).toBe(deck[3]);
	});

	it("a todas: la carta entera y en las demás solo el color", () => {
		const otra = { ...deck[1], edits: { ...EDICION_NEUTRA, rotation: "r180" as const } };
		const d = [a1, otra, deck[2], deck[3]];
		const r = aplicarEdicion(d, a1, edicion, "todas");
		expect(r[0].edits).toEqual(edicion);
		expect(r[1].edits.rotation).toBe("r180");
		expect(r[1].edits.crop).toEqual(EDICION_NEUTRA.crop);
		expect(r[1].edits.adjust).toEqual(edicion.adjust);
		// Las copias también reciben solo el color con «todas».
		expect(r[2].edits.rotation).toBe("r0");
		expect(r[2].edits.adjust).toEqual(edicion.adjust);
		// Sin imagen no se toca.
		expect(r[3]).toBe(deck[3]);
	});

	it("no cambia el orden ni el número de cartas", () => {
		for (const alcance of ["carta", "copias", "todas"] as const) {
			const r = aplicarEdicion(deck, a1, edicion, alcance);
			expect(r.map((c) => c.id)).toEqual(deck.map((c) => c.id));
		}
	});
});
