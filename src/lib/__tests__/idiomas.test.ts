import { describe, expect, it } from "vitest";
import { esLang, IDIOMAS, IDIOMAS_SCRYFALL, idioma, LANGS, localeDe } from "@/lib/idiomas";

describe("tabla de idiomas", () => {
	it("tiene los 15, sin repetir, en el orden de LANGS", () => {
		expect(IDIOMAS).toHaveLength(15);
		expect(new Set(IDIOMAS.map((i) => i.codigo)).size).toBe(15);
		expect(IDIOMAS.map((i) => i.codigo)).toEqual([...LANGS]);
	});

	it("cada locale es un BCP-47 canónico", () => {
		for (const i of IDIOMAS) expect(Intl.getCanonicalLocales(i.locale)).toEqual([i.locale]);
	});

	it("los de Scryfall son los 11 con cartas impresas", () => {
		expect(IDIOMAS_SCRYFALL.map((i) => i.codigo)).toEqual([
			"en",
			"es",
			"fr",
			"de",
			"it",
			"pt",
			"ja",
			"ko",
			"ru",
			"zhs",
			"zht",
		]);
		expect(IDIOMAS.filter((i) => i.impresas)).toHaveLength(11);
	});

	it("reconoce los códigos y cae al inglés con uno desconocido", () => {
		expect(esLang("zhs")).toBe(true);
		expect(esLang("ph")).toBe(false);
		expect(esLang(3)).toBe(false);
		expect(idioma("xx").codigo).toBe("en");
		expect(localeDe("pt")).toBe("pt-BR");
	});
});
