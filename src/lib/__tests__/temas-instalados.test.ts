import { beforeEach, describe, expect, it, vi } from "vitest";
import type { TemaInstalado } from "@/bindings/TemaInstalado";

let instalados: TemaInstalado[] = [];
vi.mock("@/lib/ipc", () => ({
	call: vi.fn(async (cmd: string, args: { id?: string }) => {
		if (cmd === "themes_installed") return instalados;
		if (cmd === "theme_install") return tema(args.id ?? "");
		return null;
	}),
}));
vi.mock("@tauri-apps/api/core", () => ({
	convertFileSrc: (_r: string, esquema: string) => `${esquema}://localhost/`,
}));

function tema(id: string): TemaInstalado {
	return {
		id: `tw-${id}`,
		nombre: id.toUpperCase(),
		css: `[data-theme="tw-${id}"] {\n  --primary: red;\n}\n@font-face { src: url({{PBIMG}}f/${"a".repeat(64)}.woff2) format('woff2'); }\n`,
		muestra: "red",
	};
}

// Lo justo del navegador: un `<head>` con `<style>` y el almacenamiento.
const estilos = new Map<string, { id: string; textContent: string }>();
const guardado = new Map<string, string>();
vi.stubGlobal("document", {
	getElementById: (id: string) => estilos.get(id) ?? null,
	createElement: () => ({ id: "", textContent: "" }),
	head: { appendChild: (e: { id: string; textContent: string }) => estilos.set(e.id, e) },
});
vi.stubGlobal("localStorage", {
	getItem: (k: string) => guardado.get(k) ?? null,
	setItem: (k: string, v: string) => guardado.set(k, v),
	removeItem: (k: string) => guardado.delete(k),
});

async function modulos() {
	vi.resetModules();
	const temas = await import("@/lib/temas-instalados");
	const { THEMES } = await import("@/lib/themes");
	return { ...temas, THEMES };
}

const css = () => estilos.get("temas-tweakcn")?.textContent ?? "";

describe("temas de tweakcn instalados", () => {
	beforeEach(() => {
		instalados = [];
		estilos.clear();
		guardado.clear();
	});

	it("entran en la lista de temas y su CSS lleva la raíz del protocolo", async () => {
		instalados = [tema("uno")];
		const { useTemasInstalados, THEMES } = await modulos();
		const deSerie = THEMES.length;
		await useTemasInstalados.getState().cargar();
		expect(THEMES.length).toBe(deSerie + 1);
		expect(THEMES.at(-1)).toEqual({ name: "tw-uno", label: "UNO", swatch: "red" });
		expect(css()).toContain(`url(pbimg://localhost/f/${"a".repeat(64)}.woff2)`);
		expect(css()).not.toContain("{{PBIMG}}");
	});

	it("instalar y quitar no duplican ni dejan restos", async () => {
		const { useTemasInstalados, THEMES } = await modulos();
		const deSerie = THEMES.length;
		const s = useTemasInstalados.getState();
		await s.cargar();
		await s.instalar("dos");
		await s.instalar("dos");
		expect(THEMES.filter((t) => t.name === "tw-dos")).toHaveLength(1);
		await useTemasInstalados.getState().quitar("tw-dos");
		expect(THEMES.length).toBe(deSerie);
		expect(css()).toBe("");
		expect(guardado.size).toBe(0);
	});

	it("se recuerdan para ponerlos antes de montar la pantalla", async () => {
		instalados = [tema("tres")];
		const primero = await modulos();
		await primero.useTemasInstalados.getState().cargar();
		estilos.clear();

		const despues = await modulos();
		despues.prepararTemasInstalados();
		expect(despues.THEMES.some((t) => t.name === "tw-tres")).toBe(true);
		expect(css()).toContain('[data-theme="tw-tres"]');
	});

	it("un recuerdo roto no rompe el arranque", async () => {
		guardado.set("proxybuilder.temasInstalados", "{no es json");
		const { prepararTemasInstalados, THEMES } = await modulos();
		const antes = THEMES.length;
		expect(() => prepararTemasInstalados()).not.toThrow();
		expect(THEMES.length).toBe(antes);
	});
});
