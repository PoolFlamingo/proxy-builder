import { describe, expect, it } from "vitest";
import type { BeforeDesign } from "@/bindings/BeforeDesign";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";
import { huecos } from "@/lib/deck-ops";
import {
	agrupar,
	antesDe,
	aplicarAlDeck,
	conVariante,
	estadoDe,
	numerosUsados,
	quitarDiseno,
	scryfallDe,
	vecina,
} from "../grupos";

const img = (c: string): FaceImage => ({
	blob: c.repeat(64),
	width: 10,
	height: 14,
	origin: "upload",
});

function carta(id: string, scryfallId: string | null, extra: Partial<DeckCard> = {}): DeckCard {
	const [b] = huecos(1);
	return {
		...b,
		id,
		source: scryfallId ? { kind: "scryfall", scryfallId } : { kind: "custom" },
		front: img("a"),
		meta: { ...b.meta, name: `carta ${id}` },
		...extra,
	};
}

function diseno(
	numero: string,
	status: CardDesign["status"] = "draft",
	before?: BeforeDesign,
): CardDesign {
	const [b] = huecos(1);
	return {
		version: 1,
		status,
		style: "borderless",
		faces: [],
		before: before ?? {
			source: { kind: "scryfall", scryfallId: "x" },
			back: b.back,
			edits: b.edits,
		},
		footer: { number: numero, numberManual: false },
	} as unknown as CardDesign;
}

describe("scryfallDe", () => {
	it("da el id de Scryfall de cartas, tokens y propias vinculadas", () => {
		expect(scryfallDe(carta("a", "s1"))).toBe("s1");
		expect(scryfallDe(carta("t", null, { source: { kind: "token", scryfallId: "tk" } }))).toBe(
			"tk",
		);
		expect(scryfallDe(carta("p", null, { source: { kind: "custom", scryfallId: "s9" } }))).toBe(
			"s9",
		);
	});

	it("las propias sin vincular y los huecos no tienen", () => {
		expect(scryfallDe(carta("p", null))).toBeNull();
		expect(scryfallDe(huecos(1)[0])).toBeNull();
	});
});

describe("agrupar", () => {
	it("junta las copias de la misma carta y respeta el orden de aparición", () => {
		const gs = agrupar([carta("a", "s1"), carta("b", "s2"), carta("c", "s1")]);
		expect(gs.map((g) => g.clave)).toEqual(["s:s1", "s:s2"]);
		expect(gs[0].cartas.map((c) => c.id)).toEqual(["a", "c"]);
		expect(gs[0].scryfallId).toBe("s1");
	});

	it("una propia vinculada va con las copias de Scryfall de esa carta", () => {
		const gs = agrupar([
			carta("a", "s1"),
			carta("p", null, { source: { kind: "custom", scryfallId: "s1" } }),
		]);
		expect(gs).toHaveLength(1);
		expect(gs[0].cartas.map((c) => c.id)).toEqual(["a", "p"]);
	});

	it("las propias sin vincular van sueltas, una por carta", () => {
		const gs = agrupar([carta("p1", null), carta("p2", null)]);
		expect(gs.map((g) => [g.clave, g.scryfallId])).toEqual([
			["c:p1", null],
			["c:p2", null],
		]);
	});

	it("los huecos en blanco no salen; un deck vacío no da grupos", () => {
		expect(agrupar(huecos(3))).toEqual([]);
		expect(agrupar([])).toEqual([]);
	});

	it("un token y una carta con el mismo id de Scryfall van juntos", () => {
		const gs = agrupar([
			carta("a", "s1"),
			carta("t", null, { source: { kind: "token", scryfallId: "s1" } }),
		]);
		expect(gs).toHaveLength(1);
	});
});

describe("estadoDe", () => {
	it("sin vincular, sin diseño, borrador y listo", () => {
		const [suelta] = agrupar([carta("p", null)]);
		expect(estadoDe(suelta)).toBe("sinVincular");
		const [sin] = agrupar([carta("a", "s1")]);
		expect(estadoDe(sin)).toBe("sinDiseno");
		const [borr] = agrupar([carta("a", "s1", { design: diseno("0001", "draft") })]);
		expect(estadoDe(borr)).toBe("borrador");
		const [listo] = agrupar([carta("a", "s1", { design: diseno("0001", "ready") })]);
		expect(estadoDe(listo)).toBe("listo");
	});

	it("manda la primera copia del grupo", () => {
		const [g] = agrupar([carta("a", "s1"), carta("b", "s1", { design: diseno("0001", "ready") })]);
		expect(estadoDe(g)).toBe("sinDiseno");
	});

	it("un grupo sin cartas (no debería pasar) no revienta", () => {
		expect(estadoDe({ clave: "s:x", scryfallId: "x", cartas: [] })).toBe("sinDiseno");
	});
});

describe("antesDe", () => {
	it("una carta sin diseño se describe a sí misma", () => {
		const c = carta("a", "s1");
		expect(antesDe(c)).toEqual({ source: c.source, front: c.front, back: c.back, edits: c.edits });
	});

	it("sin imagen, `front` queda sin poner (no `null`)", () => {
		const c = carta("a", "s1", { front: null });
		expect(antesDe(c).front).toBeUndefined();
	});

	it("una carta ya diseñada guarda el «antes» original, no el diseño", () => {
		const original = antesDe(carta("a", "s1", { front: img("b") }));
		const c = carta("a", "s1", { front: img("d"), design: diseno("0001", "draft", original) });
		expect(antesDe(c)).toBe(original);
	});
});

describe("numerosUsados", () => {
	it("recoge los números de las cartas diseñadas, en orden y con repetidos", () => {
		const cs = [
			carta("a", "s1", { design: diseno("0002") }),
			carta("b", "s2"),
			carta("c", "s3", { design: diseno("T001") }),
			carta("d", "s1", { design: diseno("0002") }),
		];
		expect(numerosUsados(cs)).toEqual(["0002", "T001", "0002"]);
	});

	it("salta los números vacíos", () => {
		expect(numerosUsados([carta("a", "s1", { design: diseno("") })])).toEqual([]);
	});
});

describe("aplicarAlDeck", () => {
	const nuevaDe = (id: string) =>
		carta(id, "s1", {
			front: img("e"),
			meta: { ...carta("x", "s1").meta, name: "diseñada" },
			design: diseno("0007", "ready", antesDe(carta(id, "s1"))),
		});

	it("«todas las copias»: cambia las del grupo y deja las demás tal cual", () => {
		const a = carta("a", "s1");
		const b = carta("b", "s1", { locked: true, front: img("b") });
		const otra = carta("o", "s2");
		const cs = [a, otra, b];
		const [g] = agrupar(cs);
		const nueva = nuevaDe("a");
		const r = aplicarAlDeck(cs, g, "a", nueva, true);
		expect(r.map((c) => c.id)).toEqual(["a", "o", "b"]);
		expect(r[0]).toBe(nueva);
		expect(r[1]).toBe(otra);
		// La otra copia: el diseño nuevo con su id, su fijado y su propio «antes».
		expect(r[2].front).toEqual(nueva.front);
		expect(r[2].locked).toBe(true);
		expect(r[2].design?.footer.number).toBe("0007");
		expect(r[2].design?.before.front).toEqual(img("b"));
		expect(r[2].design?.before).not.toBe(nueva.design?.before);
	});

	it("«solo esta copia»: las demás del grupo no se tocan", () => {
		const a = carta("a", "s1");
		const b = carta("b", "s1");
		const [g] = agrupar([a, b]);
		const nueva = nuevaDe("b");
		const r = aplicarAlDeck([a, b], g, "b", nueva, false);
		expect(r[0]).toBe(a);
		expect(r[1]).toBe(nueva);
	});

	it("una copia ya diseñada conserva el «antes» de antes de su primer diseño", () => {
		const original = antesDe(carta("b", "s1", { front: img("c") }));
		const a = carta("a", "s1");
		const b = carta("b", "s1", { front: img("d"), design: diseno("0001", "draft", original) });
		const [g] = agrupar([a, b]);
		const r = aplicarAlDeck([a, b], g, "a", nuevaDe("a"), true);
		expect(r[1].design?.before).toBe(original);
	});

	it("si Rust devuelve la carta sin diseño, las copias quedan sin diseño", () => {
		const a = carta("a", "s1");
		const b = carta("b", "s1");
		const [g] = agrupar([a, b]);
		const { design: _fuera, ...sinDiseno } = nuevaDe("a");
		const r = aplicarAlDeck([a, b], g, "a", sinDiseno, true);
		expect(r[1].design).toBeUndefined();
		expect(r[1].id).toBe("b");
	});
});

describe("conVariante", () => {
	it("pone la copia justo detrás, con el mismo diseño y marcada como variante", () => {
		const d = diseno("0001", "ready");
		const cs = [
			carta("a", "s1"),
			carta("b", "s2", { design: d, marca: "opcional" }),
			carta("c", "s3"),
		];
		const out = conVariante(cs, "b", "nueva");
		expect(out.map((c) => c.id)).toEqual(["a", "b", "nueva", "c"]);
		expect(out[2].marca).toBe("variante");
		expect(out[2].design).toEqual(d);
		expect(out[2].source).toEqual(cs[1].source);
		expect(out[2].locked).toBe(false);
		// La original no cambia.
		expect(out[1]).toBe(cs[1]);
	});

	it("la variante es del mismo grupo (sigue siendo la misma carta de Scryfall)", () => {
		const out = conVariante([carta("a", "s1")], "a", "v");
		expect(agrupar(out).map((g) => g.cartas.length)).toEqual([2]);
	});

	it("con un id que no está, no cambia nada", () => {
		const cs = [carta("a", "s1")];
		expect(conVariante(cs, "no", "v")).toEqual(cs);
	});
});

describe("quitarDiseno", () => {
	it("devuelve las cartas indicadas a como estaban", () => {
		const antes = carta("a", "s1", { front: img("b") });
		const original = antesDe(antes);
		const disenada = {
			...antes,
			front: img("d"),
			source: { kind: "custom" as const, scryfallId: "s1" },
			design: diseno("0001", "ready", original),
		};
		const [r] = quitarDiseno([disenada], new Set(["a"]));
		expect(r.design).toBeUndefined();
		expect("design" in r).toBe(false);
		expect(r.source).toEqual(antes.source);
		expect(r.front).toEqual(img("b"));
		expect(r.back).toEqual(antes.back);
		expect(r.edits).toEqual(antes.edits);
		expect(r.id).toBe("a");
	});

	it("una carta que no tenía imagen vuelve a `front: null`", () => {
		const original = antesDe(carta("a", "s1", { front: null }));
		const c = carta("a", "s1", { design: diseno("0001", "ready", original) });
		expect(quitarDiseno([c], new Set(["a"]))[0].front).toBeNull();
	});

	it("no toca las que no están en la lista ni las que no tienen diseño", () => {
		const a = carta("a", "s1", { design: diseno("0001") });
		const b = carta("b", "s1");
		const r = quitarDiseno([a, b], new Set(["b"]));
		expect(r[0]).toBe(a);
		expect(r[1]).toBe(b);
	});
});

describe("vecina", () => {
	const gs = agrupar([carta("a1", "A"), carta("a2", "A"), carta("b1", "B"), carta("c1", "C")]);

	it("sin copias salta de carta en carta, desde cualquier copia, y da la vuelta", () => {
		expect(vecina(gs, "a1", 1, false)).toBe("b1");
		expect(vecina(gs, "a2", 1, false)).toBe("b1");
		expect(vecina(gs, "c1", 1, false)).toBe("a1");
		expect(vecina(gs, "a2", -1, false)).toBe("c1");
	});

	it("con copias pasa por cada copia en el orden del deck", () => {
		expect(vecina(gs, "a1", 1, true)).toBe("a2");
		expect(vecina(gs, "a2", 1, true)).toBe("b1");
		expect(vecina(gs, "b1", -1, true)).toBe("a2");
		expect(vecina(gs, "a1", -1, true)).toBe("c1");
	});

	it("sin carta de partida, la primera o la última; sin cartas, nada", () => {
		expect(vecina(gs, null, 1, true)).toBe("a1");
		expect(vecina(gs, null, -1, true)).toBe("c1");
		expect(vecina([], "a1", 1, true)).toBeNull();
	});
});
