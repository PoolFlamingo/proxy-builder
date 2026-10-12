import { describe, expect, it } from "vitest";
import type { DeckCard } from "@/bindings/DeckCard";
import { duplicar, huecos, insertar, mover, moverGrupo, quitar } from "@/lib/deck-ops";

let n = 0;
const id = () => `n${n++}`;
const ids = (c: DeckCard[]) => c.map((x) => x.id);
const base = () =>
	huecos(
		5,
		(() => {
			let i = 0;
			return () => `c${i++}`;
		})(),
	);

describe("operaciones sobre las cartas", () => {
	it("mover lleva la carta a su sitio", () => {
		expect(ids(mover(base(), 0, 3))).toEqual(["c1", "c2", "c3", "c0", "c4"]);
		expect(ids(mover(base(), 4, 0))).toEqual(["c4", "c0", "c1", "c2", "c3"]);
		expect(ids(mover(base(), 1, 99))).toEqual(["c0", "c2", "c3", "c4", "c1"]);
	});

	it("mover un grupo conserva su orden", () => {
		expect(ids(moverGrupo(base(), ["c3", "c1"], 0))).toEqual(["c1", "c3", "c0", "c2", "c4"]);
	});

	it("duplicar pone la copia detrás y desfijada", () => {
		const c = base();
		c[1] = { ...c[1], locked: true };
		const r = duplicar(c, ["c1"], id);
		expect(r).toHaveLength(6);
		expect(r[2].id).not.toBe("c1");
		expect(r[2].locked).toBe(false);
	});

	it("quitar e insertar", () => {
		expect(ids(quitar(base(), ["c0", "c4"]))).toEqual(["c1", "c2", "c3"]);
		const nuevas = huecos(2, id);
		expect(ids(insertar(base(), nuevas, 1)).slice(0, 3)).toEqual([
			"c0",
			nuevas[0].id,
			nuevas[1].id,
		]);
	});
});
