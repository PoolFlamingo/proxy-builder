import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { NOMBRES_SIN_TITULO, nombreVisible } from "../nombre-deck";

const IDIOMAS = [
	"es",
	"en",
	"fr",
	"de",
	"it",
	"pt",
	"ro",
	"pl",
	"nl",
	"ru",
	"uk",
	"ja",
	"ko",
	"zhs",
	"zht",
];

describe("nombreVisible", () => {
	it("tiene el «Deck sin título» de cada diccionario", () => {
		for (const l of IDIOMAS) {
			const dic = JSON.parse(
				readFileSync(new URL(`../../i18n/${l}/shell.json`, import.meta.url), "utf8"),
			) as Record<string, string>;
			expect(NOMBRES_SIN_TITULO.has(dic["Deck sin título"]), l).toBe(true);
		}
	});

	it("traduce el nombre de fábrica y deja los demás", () => {
		expect(nombreVisible("Untitled deck", "Deck sin título")).toBe("Deck sin título");
		expect(nombreVisible("無題のデッキ", "Untitled deck")).toBe("Untitled deck");
		expect(nombreVisible("Mi deck", "Deck sin título")).toBe("Mi deck");
		expect(nombreVisible("Untitled deck 2", "Deck sin título")).toBe("Untitled deck 2");
	});
});
