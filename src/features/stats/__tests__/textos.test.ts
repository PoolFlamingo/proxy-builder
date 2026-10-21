import { describe, expect, it } from "vitest";
import type { DeckKind } from "@/bindings/DeckKind";
import {
	BRACKETS,
	CLASES_MAZO,
	COLORES,
	FUNCIONES,
	formatoMoneda,
	RAREZAS,
	SUPERTIPOS,
	TIPOS,
} from "../textos";

/** Con espacios normales (Intl pone espacios duros). */
const plano = (s: string) => s.replace(/\s/g, " ");

describe("formatoMoneda", () => {
	it("en español: coma decimal, punto de miles y el símbolo detrás", () => {
		const eur = formatoMoneda("es", "eur");
		expect(plano(eur.format(12345.5))).toBe("12.345,50 €");
		expect(plano(eur.format(0))).toBe("0,00 €");
		expect(plano(formatoMoneda("es", "usd").format(3))).toBe("3,00 US$");
	});

	it("en inglés: el símbolo delante", () => {
		expect(formatoMoneda("en", "eur").format(1234.5)).toBe("€1,234.50");
		expect(formatoMoneda("en", "usd").format(1234.5)).toBe("US$1,234.50");
	});

	it("cada idioma con su formato; uno desconocido, el inglés", () => {
		expect(plano(formatoMoneda("fr", "eur").format(1.5))).toBe("1,50 €");
		expect(plano(formatoMoneda("ru", "eur").format(1234.5))).toBe("1 234,50 €");
		expect(formatoMoneda("ja", "usd").format(3)).toBe("$3.00");
		expect(formatoMoneda("xx", "eur").format(1234.5)).toBe("€1,234.50");
	});

	it("redondea a dos decimales", () => {
		expect(formatoMoneda("en", "eur").format(0.005)).toBe("€0.01");
		expect(formatoMoneda("en", "eur").format(2.344)).toBe("€2.34");
	});
});

/**
 * Las claves las pone Rust (`proxy-core::stats`); si una no está aquí, la
 * pantalla enseña la clave en inglés. Estas listas copian las de Rust.
 */
describe("las tablas cubren lo que manda Rust", () => {
	it("funciones (stats/funciones.rs)", () => {
		expect(Object.keys(FUNCIONES).sort()).toEqual(
			[
				"ramp",
				"draw",
				"removal",
				"wipe",
				"counter",
				"tutor",
				"protection",
				"recursion",
				"tokens",
				"anthem",
				"lifegain",
				"mill",
				"extraTurn",
				"landDenial",
			].sort(),
		);
		for (const [k, f] of Object.entries(FUNCIONES)) {
			expect(f.nombre, k).toBeTruthy();
			expect(f.ayuda, k).toBeTruthy();
		}
	});

	it("tipos (los de la curva, las tierras y «Other»)", () => {
		for (const k of [
			"Creature",
			"Planeswalker",
			"Battle",
			"Instant",
			"Sorcery",
			"Artifact",
			"Enchantment",
			"Land",
			"Other",
		])
			expect(TIPOS[k]?.nombre, k).toBeTruthy();
		// Cada tipo con su color propio en la curva.
		const clases = Object.values(TIPOS).map((x) => x.clase);
		expect(new Set(clases).size).toBe(clases.length);
	});

	it("supertipos", () => {
		expect(Object.keys(SUPERTIPOS).sort()).toEqual(
			["Legendary", "Basic", "Snow", "World", "Ongoing"].sort(),
		);
	});

	it("colores de maná (WUBRG e incoloro)", () => {
		expect(Object.keys(COLORES)).toEqual(["W", "U", "B", "R", "G", "C"]);
		for (const [k, c] of Object.entries(COLORES)) {
			expect(c.nombre, k).toBeTruthy();
			expect(c.chip, k).toBeTruthy();
			expect(c.barra, k).toBeTruthy();
		}
	});

	it("rarezas de Scryfall", () => {
		for (const r of ["common", "uncommon", "rare", "mythic", "special", "bonus"])
			expect(RAREZAS[r], r).toBeTruthy();
	});

	it("clases de mazo: todas las de DeckKind", () => {
		const todas: DeckKind[] = ["commander", "constructed", "limited"];
		expect(Object.keys(CLASES_MAZO).sort()).toEqual([...todas].sort());
	});

	it("brackets: los que puede deducir Rust (2, 3 y 4)", () => {
		expect(Object.keys(BRACKETS).map(Number)).toEqual([2, 3, 4]);
		for (const b of Object.values(BRACKETS)) {
			expect(b.nombre).toBeTruthy();
			expect(b.ayuda).toBeTruthy();
		}
	});
});
