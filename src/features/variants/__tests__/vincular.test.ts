import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CardMeta } from "@/bindings/CardMeta";
import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";

vi.mock("@/lib/ipc", () => ({ call: vi.fn() }));

import { huecos } from "@/lib/deck-ops";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { desvincular, estaVinculada, propiasPorImagen, vincular } from "../vincular";

const img = (c: string): FaceImage => ({
	blob: c.repeat(64),
	width: 1,
	height: 1,
	origin: "upload",
});

/** Una propia con la imagen `c` (o sin imagen con `null`). */
function propia(id: string, c: string | null, extra: Partial<DeckCard> = {}): DeckCard {
	const [b] = huecos(1);
	return {
		...b,
		id,
		source: { kind: "custom" },
		front: c ? img(c) : null,
		meta: {
			...b.meta,
			name: `mía ${id}`,
			highres: true,
			imageStatus: "upload",
			langFallback: true,
		},
		...extra,
	};
}

function cargar(cards: DeckCard[]) {
	const deck: Deck = {
		id: "d",
		name: "t",
		decklistText: "",
		cards,
		settings: {} as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
	useDeck.getState().cargar(deck);
}
const cartas = () => useDeck.getState().deck?.cards ?? [];
const por = (id: string) => cartas().find((c) => c.id === id);

const META: CardMeta = {
	name: "Sol Ring",
	set: "cmm",
	collectorNumber: "1",
	lang: "en",
	langFallback: true,
	layout: "normal",
	isDfc: false,
	imageStatus: "highres_scan",
	highres: false,
	typeLine: "Artifact",
};

describe("vincular", () => {
	beforeEach(() => {
		vi.mocked(call).mockResolvedValue(META as never);
		cargar([
			propia("a", "a"),
			propia("b", "a"),
			propia("c", "c"),
			// Una de Scryfall con la misma imagen (no debería pasar, pero no se toca).
			{ ...propia("s", "a"), source: { kind: "scryfall", scryfallId: "otra" } },
		]);
	});

	it("vincula la carta y sus copias con la misma imagen, y devuelve el nombre", async () => {
		await expect(vincular("a", "sf-1")).resolves.toBe("Sol Ring");
		expect(call).toHaveBeenCalledWith("card_link", { scryfallId: "sf-1" });
		expect(por("a")?.source).toEqual({ kind: "custom", scryfallId: "sf-1" });
		expect(por("b")?.source).toEqual({ kind: "custom", scryfallId: "sf-1" });
		expect(por("c")?.source).toEqual({ kind: "custom" });
		expect(por("s")?.source).toEqual({ kind: "scryfall", scryfallId: "otra" });
	});

	it("toma la info de Scryfall pero no el estado de su imagen", async () => {
		await vincular("a", "sf-1");
		const m = por("a")?.meta;
		expect(m?.name).toBe("Sol Ring");
		expect(m?.typeLine).toBe("Artifact");
		expect(m?.imageStatus).toBeNull();
		expect(m?.highres).toBe(true);
		expect(m?.langFallback).toBe(false);
	});

	it("la imagen y los retoques de la carta no cambian", async () => {
		const antes = por("a");
		await vincular("a", "sf-1");
		expect(por("a")?.front).toBe(antes?.front);
		expect(por("a")?.edits).toBe(antes?.edits);
	});

	it("una propia sin imagen solo se vincula a sí misma", async () => {
		cargar([propia("x", null), propia("y", null)]);
		await vincular("x", "sf-2");
		expect(por("x")?.source).toEqual({ kind: "custom", scryfallId: "sf-2" });
		expect(por("y")?.source).toEqual({ kind: "custom" });
	});

	it("se deshace de una vez", async () => {
		await vincular("a", "sf-1");
		useDeck.getState().deshacer();
		expect(cartas().every((c) => c.source.kind !== "custom" || !c.source.scryfallId)).toBe(true);
	});

	it("si Rust falla, el deck no cambia", async () => {
		vi.mocked(call).mockRejectedValueOnce({ code: "Network", params: {} });
		await expect(vincular("a", "sf-1")).rejects.toEqual({ code: "Network", params: {} });
		expect(useDeck.getState().pasado).toHaveLength(0);
	});

	it("revincular cambia el vínculo anterior", async () => {
		await vincular("a", "sf-1");
		await vincular("b", "sf-9");
		expect(por("a")?.source).toEqual({ kind: "custom", scryfallId: "sf-9" });
	});
});

describe("desvincular", () => {
	it("quita el vínculo a la carta y a sus copias con la misma imagen", () => {
		cargar([
			propia("a", "a", { source: { kind: "custom", scryfallId: "s" } }),
			propia("b", "a", { source: { kind: "custom", scryfallId: "s" } }),
			propia("c", "c", { source: { kind: "custom", scryfallId: "s" } }),
		]);
		desvincular("b");
		expect(por("a")?.source).toEqual({ kind: "custom" });
		expect(por("b")?.source).toEqual({ kind: "custom" });
		expect(por("c")?.source).toEqual({ kind: "custom", scryfallId: "s" });
	});

	it("una carta que no existe no cambia nada", () => {
		cargar([propia("a", "a", { source: { kind: "custom", scryfallId: "s" } })]);
		desvincular("nada");
		expect(por("a")?.source).toEqual({ kind: "custom", scryfallId: "s" });
	});
});

describe("propiasPorImagen", () => {
	beforeEach(() =>
		cargar([
			propia("a", "a"),
			propia("b", "a"),
			propia("c", "c"),
			propia("sin", null),
			{ ...propia("s", "s"), source: { kind: "scryfall", scryfallId: "x" } },
		]),
	);

	it("una por imagen (la primera copia), sin las de Scryfall ni las que no tienen imagen", () => {
		expect(propiasPorImagen().map((c) => c.id)).toEqual(["a", "c"]);
	});

	it("con ids, solo de entre esas", () => {
		expect(propiasPorImagen(["b", "s", "sin"]).map((c) => c.id)).toEqual(["b"]);
		expect(propiasPorImagen([]).map((c) => c.id)).toEqual([]);
	});

	it("sin deck, ninguna", () => {
		useDeck.setState({ deck: null });
		expect(propiasPorImagen()).toEqual([]);
	});
});

describe("estaVinculada", () => {
	it("solo las propias con id de Scryfall", () => {
		expect(estaVinculada(propia("a", "a"))).toBe(false);
		expect(estaVinculada(propia("a", "a", { source: { kind: "custom", scryfallId: "s" } }))).toBe(
			true,
		);
		expect(estaVinculada(propia("a", "a", { source: { kind: "custom", scryfallId: "" } }))).toBe(
			false,
		);
		expect(estaVinculada(propia("a", "a", { source: { kind: "scryfall", scryfallId: "s" } }))).toBe(
			false,
		);
	});
});
