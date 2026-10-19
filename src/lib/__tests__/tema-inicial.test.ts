import { beforeEach, describe, expect, it, vi } from "vitest";

// Lo justo del navegador: `<html>` con atributos y clases, y el almacenamiento.
const atributos = new Map<string, string>();
const clases = new Set<string>();
const guardado = new Map<string, string>();
let sistemaOscuro = false;
vi.stubGlobal("document", {
	documentElement: {
		setAttribute: (k: string, v: string) => atributos.set(k, v),
		classList: { toggle: (c: string, si: boolean) => (si ? clases.add(c) : clases.delete(c)) },
	},
});
vi.stubGlobal("localStorage", { getItem: (k: string) => guardado.get(k) ?? null });
vi.stubGlobal("window", { matchMedia: () => ({ matches: sistemaOscuro }) });

const { ponerTemaAntesDeMontar } = await import("@/lib/tema-inicial");

describe("el tema antes de montar", () => {
	beforeEach(() => {
		atributos.clear();
		clases.clear();
		guardado.clear();
		sistemaOscuro = false;
	});

	it("sin nada guardado, el de fábrica en oscuro y en mate", () => {
		ponerTemaAntesDeMontar();
		expect(atributos.get("data-theme")).toBe("amber-minimal");
		expect(clases.has("dark")).toBe(true);
		expect(atributos.get("data-material")).toBe("matte");
	});

	it("el guardado, en claro y con su superficie", () => {
		guardado.set("proxybuilder.theme", "claude");
		guardado.set("proxybuilder.mode", "light");
		guardado.set("proxybuilder.skin", "hera");
		guardado.set("proxybuilder.material", "solid");
		ponerTemaAntesDeMontar();
		expect(atributos.get("data-theme")).toBe("claude");
		expect(clases.has("dark")).toBe(false);
		expect(atributos.get("data-skin")).toBe("hera");
		expect(atributos.has("data-material")).toBe(false);
	});

	it("en modo sistema, el del sistema", () => {
		guardado.set("proxybuilder.mode", "system");
		sistemaOscuro = true;
		ponerTemaAntesDeMontar();
		expect(clases.has("dark")).toBe(true);
	});

	it("un tema que ya no existe cae al de fábrica; el base va limpio", () => {
		guardado.set("proxybuilder.theme", "no-existe");
		ponerTemaAntesDeMontar();
		expect(atributos.get("data-theme")).toBe("amber-minimal");
		atributos.clear();
		guardado.set("proxybuilder.theme", "base");
		guardado.set("proxybuilder.skin", "zeus");
		ponerTemaAntesDeMontar();
		expect(atributos.get("data-theme")).toBe("base");
		expect(atributos.has("data-skin")).toBe(false);
		expect(atributos.has("data-material")).toBe(false);
	});
});
