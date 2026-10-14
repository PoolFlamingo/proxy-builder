import { describe, expect, it } from "vitest";
import type { DeckCard } from "@/bindings/DeckCard";
import { aplicarReaplicado, insignias, listaParaReaplicar, nombreDeFichero } from "@/lib/cartas";
import { huecos } from "@/lib/deck-ops";

function carta(id: string, nombre: string, extra: Partial<DeckCard> = {}): DeckCard {
	const [b] = huecos(1);
	return {
		...b,
		id,
		source: { kind: "scryfall", scryfallId: `sf-${id}` },
		meta: { ...b.meta, name: nombre, set: "cmm", collectorNumber: "1" },
		...extra,
	};
}

describe("reaplicar idioma", () => {
	it("pide solo las cartas de Scryfall no fijadas, una línea por copia", () => {
		const cs = [
			carta("a", "Sol Ring"),
			carta("b", "Forest", { locked: true }),
			{ ...carta("c", "Mía"), source: { kind: "custom" } as const },
			carta("d", "Sol Ring"),
		];
		const { lista, ids } = listaParaReaplicar(cs);
		expect(ids).toEqual(["a", "d"]);
		expect(lista.lines.map((l) => [l.lineNo, l.name, l.qty, l.set])).toEqual([
			[1, "Sol Ring", 1, "cmm"],
			[2, "Sol Ring", 1, "cmm"],
		]);
	});

	it("conserva id, retoques y fijado, y salta las no resueltas", () => {
		const a = carta("a", "Uno");
		a.edits = { ...a.edits, rotation: "r90" };
		const b = carta("b", "Dos");
		const c = carta("c", "Tres");
		const nuevaA = carta("x", "Uno ES", { meta: { ...a.meta, lang: "es" } });
		const nuevaC = carta("y", "Tres ES", { meta: { ...c.meta, name: "Tres ES", lang: "es" } });
		// La línea 2 (b) no se resolvió: llegan solo dos cartas, en orden.
		const r = aplicarReaplicado([a, b, c], ["a", "b", "c"], [nuevaA, nuevaC], [2]);
		expect(r.map((x) => x.id)).toEqual(["a", "b", "c"]);
		expect(r[0].meta.lang).toBe("es");
		expect(r[0].edits.rotation).toBe("r90");
		expect(r[1]).toBe(b);
		expect(r[2].meta.name).toBe("Tres ES");
	});

	it("un reverso elegido a mano no se pierde", () => {
		const img = { blob: "f".repeat(64), width: 1, height: 1, origin: "upload" as const };
		const a = carta("a", "Uno", { back: { kind: "custom", image: img } });
		const r = aplicarReaplicado([a], ["a"], [carta("x", "Uno")], []);
		expect(r[0].back).toEqual({ kind: "custom", image: img });
	});
});

describe("marcas y nombres", () => {
	it("insignias según el origen y el estado de la imagen", () => {
		const c = carta("a", "X", {
			meta: { ...carta("a", "X").meta, langFallback: true, imageStatus: "lowres" },
		});
		expect(insignias(c).map((i) => i.texto)).toEqual(["↩ EN", "LOWRES"]);
		expect(insignias({ ...c, source: { kind: "custom" } }).map((i) => i.texto)[0]).toBe("CUSTOM");
	});

	it("una carta diseñada solo lleva la marca de su diseño", () => {
		const c = carta("a", "X", {
			meta: { ...carta("a", "X").meta, langFallback: true, imageStatus: "lowres" },
		});
		const conDiseno = (status: "draft" | "ready") =>
			({ ...c, design: { status } }) as unknown as DeckCard;
		expect(insignias(conDiseno("draft")).map((i) => i.texto)).toEqual(["BORRADOR"]);
		expect(insignias(conDiseno("ready")).map((i) => i.texto)).toEqual(["PROXY"]);
	});

	it("el nombre de un fichero sin carpeta ni extensión", () => {
		expect(nombreDeFichero("/home/p/Mi carta.final.png")).toBe("Mi carta.final");
		expect(nombreDeFichero("C:\\x\\dorso.jpg")).toBe("dorso");
	});
});
