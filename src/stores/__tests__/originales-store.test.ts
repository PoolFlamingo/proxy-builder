import { describe, expect, it } from "vitest";
import type { RefImagen } from "@/bindings/RefImagen";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { conOriginales } from "../originales-store";

const scry = (id: string, cara = 0): RefImagen => ({ tipo: "scryfall", id, cara, url: `u/${id}` });

function carta(dueno: number, scryfallId: string | null, caras: RefImagen[]): VistaCarta {
	return {
		id: 1,
		dueno,
		origen: { scryfallId, caras, nombre: "X" },
	} as unknown as VistaCarta;
}

describe("conOriginales", () => {
	const caras = { "id-en": [scry("id-es")] };

	it("cambia las caras si el dueño está en el modo y la impresión ya está resuelta", () => {
		const c = carta(1, "id-en", [scry("id-en")]);
		const r = conOriginales(c, { modos: { 1: true }, caras });
		expect(r.origen?.caras).toEqual([scry("id-es")]);
		// No toca la original.
		expect(c.origen?.caras).toEqual([scry("id-en")]);
	});

	it("deja la misma carta si el dueño no está en el modo, no hay vínculo o falta la resolución", () => {
		const c = carta(1, "id-en", [scry("id-en")]);
		expect(conOriginales(c, { modos: { 2: true }, caras })).toBe(c);
		const sin = carta(1, null, [scry("x")]);
		expect(conOriginales(sin, { modos: { 1: true }, caras })).toBe(sin);
		expect(conOriginales(c, { modos: { 1: true }, caras: {} })).toBe(c);
		// «Vacío»: no hay otra impresión.
		expect(conOriginales(c, { modos: { 1: true }, caras: { "id-en": [] } })).toBe(c);
	});

	it("de una carta de dos caras con solo una resuelta, la otra se queda", () => {
		const c = carta(1, "id-en", [scry("id-en", 0), scry("id-en", 1)]);
		const r = conOriginales(c, { modos: { 1: true }, caras });
		expect(r.origen?.caras).toEqual([scry("id-es"), scry("id-en", 1)]);
	});
});
