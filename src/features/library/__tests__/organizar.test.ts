import { describe, expect, it } from "vitest";
import type { DeckDetails } from "@/bindings/DeckDetails";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Tag } from "@/bindings/Tag";
import {
	agrupar,
	contar,
	type Detalles,
	FILTROS_VACIOS,
	filtrar,
	hayFiltros,
	idiomasDe,
	normalizar,
	ordenar,
	seleccionar,
} from "../organizar";

function deck(id: string, c: Partial<DeckSummary> = {}): DeckSummary {
	return {
		id,
		name: id,
		cardCount: 100,
		createdAt: 1,
		updatedAt: 1,
		openedAt: null,
		thumbnail: null,
		coverName: null,
		coverCardId: null,
		pinned: false,
		notes: "",
		langOverride: null,
		tags: [],
		...c,
	};
}

function det(id: string, c: Partial<DeckDetails> = {}): DeckDetails {
	return {
		id,
		kind: "commander",
		colorIdentity: [],
		deckSize: 100,
		lands: 36,
		tokens: 0,
		unknown: 0,
		averageCmc: 3,
		priceEur: null,
		priceUsd: null,
		designed: 0,
		designedReady: 0,
		custom: 0,
		customLinked: 0,
		dfc: 0,
		langs: ["en"],
		legalFormats: [],
		bracket: null,
		cardNames: [],
		...c,
	};
}

const tags: Tag[] = [
	{ id: "es", name: "Español", color: "red", icon: null },
	{ id: "tor", name: "Torneo", color: "blue", icon: null },
];

const decks = [
	deck("milim", {
		name: "ES Milim Defensores",
		tags: ["es", "tor"],
		pinned: true,
		updatedAt: 5,
		notes: "Para el viernes",
	}),
	deck("rimuru", { name: "EN Rimuru", tags: ["tor"], updatedAt: 9 }),
	deck("shion", {
		name: "ES Shion",
		tags: ["es"],
		updatedAt: 3,
		coverName: "Shion, la Tirana Oni",
	}),
	deck("coliseo", { name: "Coliseo", updatedAt: 7 }),
];
const d: Detalles = {
	milim: det("milim", {
		colorIdentity: ["W", "U"],
		priceEur: 120,
		cardNames: ["Sol Ring", "Wall of Omens"],
		langs: ["es"],
		legalFormats: ["commander"],
	}),
	rimuru: det("rimuru", { colorIdentity: ["U", "R"], priceEur: 80, designed: 3 }),
	shion: det("shion", { colorIdentity: ["R"], kind: "limited", langs: ["es"], custom: 2 }),
	// «coliseo» aún sin detalles.
};

describe("filtrar", () => {
	const ids = (f: Partial<typeof FILTROS_VACIOS>) =>
		filtrar(decks, d, tags, { ...FILTROS_VACIOS, ...f }).map((x) => x.id);

	it("sin filtros, todos", () => {
		expect(ids({})).toEqual(["milim", "rimuru", "shion", "coliseo"]);
		expect(hayFiltros(FILTROS_VACIOS)).toBe(false);
	});

	it("el texto busca en nombre, portada, notas, etiquetas y cartas, sin tildes", () => {
		expect(ids({ texto: "milim" })).toEqual(["milim"]);
		expect(ids({ texto: "tirana" })).toEqual(["shion"]);
		expect(ids({ texto: "viernes" })).toEqual(["milim"]);
		expect(ids({ texto: "espanol" })).toEqual(["milim", "shion"]);
		expect(ids({ texto: "sol ring" })).toEqual(["milim"]);
		expect(ids({ texto: "es torneo" })).toEqual(["milim"]);
	});

	it("etiquetas: alguna o todas, sin etiqueta y favoritos", () => {
		expect(ids({ etiquetas: ["es", "tor"] })).toEqual(["milim", "rimuru", "shion"]);
		expect(ids({ etiquetas: ["es", "tor"], todasLasEtiquetas: true })).toEqual(["milim"]);
		expect(ids({ sinEtiqueta: true })).toEqual(["coliseo"]);
		expect(ids({ favoritos: true })).toEqual(["milim"]);
	});

	it("colores: los incluye, exactos o dentro de ellos; sin detalles no pasa", () => {
		expect(ids({ colores: ["U"] })).toEqual(["milim", "rimuru"]);
		expect(ids({ colores: ["U", "R"], modoColor: "exacto" })).toEqual(["rimuru"]);
		expect(ids({ colores: ["U", "R"], modoColor: "dentro" })).toEqual(["rimuru", "shion"]);
		expect(ids({ colores: ["C"] })).toEqual([]);
	});

	it("clase, formato, idioma, diseños y propias", () => {
		expect(ids({ clases: ["limited"] })).toEqual(["shion"]);
		expect(ids({ formato: "commander" })).toEqual(["milim"]);
		expect(ids({ idioma: "es" })).toEqual(["milim", "shion"]);
		expect(ids({ conDisenos: true })).toEqual(["rimuru"]);
		expect(ids({ conPropias: true })).toEqual(["shion"]);
	});

	it("el idioma elegido a mano manda sobre el calculado", () => {
		const con = decks.map((x) => (x.id === "rimuru" ? { ...x, langOverride: "es" } : x));
		const f = { ...FILTROS_VACIOS, idioma: "es" };
		expect(filtrar(con, d, tags, f).map((x) => x.id)).toEqual(["milim", "rimuru", "shion"]);
		const en = { ...FILTROS_VACIOS, idioma: "en" };
		expect(filtrar(con, d, tags, en).map((x) => x.id)).toEqual([]);
		expect(idiomasDe(con[1], d.rimuru)).toEqual(["es"]);
		expect(idiomasDe(decks[1], d.rimuru)).toEqual(["en"]);
		expect(idiomasDe(decks[3], undefined)).toEqual([]);
	});
});

describe("ordenar", () => {
	const ids = (
		...a: Parameters<typeof ordenar> extends [unknown, unknown, ...infer R] ? R : never
	) => ordenar(decks, d, ...a).map((x) => x.id);

	it("por fecha, nombre y precio, y lo que no tiene dato al final", () => {
		expect(ids("modificado", false)).toEqual(["rimuru", "coliseo", "milim", "shion"]);
		expect(ids("nombre", true)).toEqual(["coliseo", "rimuru", "milim", "shion"]);
		expect(ids("precio", false)).toEqual(["milim", "rimuru", "coliseo", "shion"]);
		expect(ids("precio", true)).toEqual(["rimuru", "milim", "coliseo", "shion"]);
	});

	it("con favoritos arriba", () => {
		expect(ids("modificado", false, true)[0]).toBe("milim");
	});

	it("por colores: primero los de menos", () => {
		expect(ids("colores", true)).toEqual(["shion", "milim", "rimuru", "coliseo"]);
	});
});

describe("agrupar", () => {
	it("por etiqueta, un deck en cada una de las suyas y luego los sin etiqueta", () => {
		const g = agrupar(decks, d, tags, "etiqueta");
		expect(g.map((x) => [x.id, x.decks.map((y) => y.id)])).toEqual([
			["t:es", ["milim", "shion"]],
			["t:tor", ["milim", "rimuru"]],
			["sin-etiqueta", ["coliseo"]],
		]);
	});

	it("por color, de menos a más colores, y los que no tienen datos al final", () => {
		const g = agrupar(decks, d, tags, "color");
		expect(g.map((x) => x.id)).toEqual(["c:13", "c:201", "c:213", "sin-datos"]);
	});

	it("por idioma: el elegido a mano, aunque falten los detalles", () => {
		const con = decks.map((x) => (x.id === "coliseo" ? { ...x, langOverride: "es" } : x));
		const g = agrupar(con, d, tags, "idioma");
		expect(g.map((x) => [x.id, x.decks.map((y) => y.id)])).toEqual([
			["l:en", ["rimuru"]],
			["l:es", ["milim", "shion", "coliseo"]],
		]);
	});

	it("por clase", () => {
		const g = agrupar(decks, d, tags, "clase");
		expect(g.map((x) => x.id)).toEqual(["k:commander", "k:limited", "sin-datos"]);
	});
});

describe("ayudas", () => {
	it("contar", () => {
		const c = contar(decks);
		expect(c.porEtiqueta.get("es")).toBe(2);
		expect(c.sinEtiqueta).toBe(1);
		expect(c.favoritos).toBe(1);
	});

	it("normalizar", () => {
		expect(normalizar("Señora ÁNGEL")).toBe("senora angel");
	});

	it("seleccionar con Ctrl y con Mayús", () => {
		const v = ["a", "b", "c", "d"];
		expect(seleccionar([], v, "b", null, "alternar")).toEqual(["b"]);
		expect(seleccionar(["b"], v, "b", "b", "alternar")).toEqual([]);
		expect(seleccionar(["b"], v, "d", "b", "tramo")).toEqual(["b", "c", "d"]);
		expect(seleccionar([], v, "a", "c", "tramo")).toEqual(["a", "b", "c"]);
	});
});
