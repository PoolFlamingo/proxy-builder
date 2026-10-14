import { beforeEach, describe, expect, it, vi } from "vitest";

const llamadas: { cmd: string; args: Record<string, unknown> }[] = [];
let carpeta: string | null = "/inicio";
let fallaInicial = false;
let fallaRecordar = false;
vi.mock("@/lib/ipc", () => ({
	call: vi.fn(async (cmd: string, args: Record<string, unknown>) => {
		llamadas.push({ cmd, args });
		if (cmd === "carpeta_inicial") {
			if (fallaInicial) throw { code: "Db", params: {} };
			return carpeta;
		}
		if (cmd === "carpeta_recordar" && fallaRecordar) throw { code: "Db", params: {} };
		return null;
	}),
}));

let respuestaOpen: string | string[] | null = null;
let respuestaSave: string | null = null;
const open = vi.fn(async (_o: unknown) => respuestaOpen);
const save = vi.fn(async (_o: unknown) => respuestaSave);
vi.mock("@tauri-apps/plugin-dialog", () => ({
	open: (o: unknown) => open(o),
	save: (o: unknown) => save(o),
}));

import {
	abrirParaImportar,
	elegirCarpetaParaExportar,
	guardarParaExportar,
	recordar,
} from "@/lib/dialogos";

const recordadas = () => llamadas.filter((l) => l.cmd === "carpeta_recordar").map((l) => l.args);

describe("diálogos de ficheros", () => {
	beforeEach(() => {
		llamadas.length = 0;
		carpeta = "/inicio";
		fallaInicial = false;
		fallaRecordar = false;
		respuestaOpen = null;
		respuestaSave = null;
		open.mockClear();
		save.mockClear();
	});

	it("importar empieza en la carpeta de importar y recuerda la primera ruta", async () => {
		respuestaOpen = ["/a/uno.png", "/a/dos.png"];
		const r = await abrirParaImportar({ multiple: true });
		expect(r).toEqual(["/a/uno.png", "/a/dos.png"]);
		expect(llamadas[0]).toEqual({
			cmd: "carpeta_inicial",
			args: { tipo: "importar", nombre: null },
		});
		expect(open).toHaveBeenCalledWith({ multiple: true, defaultPath: "/inicio" });
		expect(recordadas()).toEqual([{ tipo: "importar", ruta: "/a/uno.png" }]);
	});

	it("importar con una sola ruta la devuelve en una lista", async () => {
		respuestaOpen = "/a/lista.txt";
		expect(await abrirParaImportar({})).toEqual(["/a/lista.txt"]);
	});

	it("si se cancela no devuelve nada ni recuerda", async () => {
		respuestaOpen = null;
		expect(await abrirParaImportar({})).toBeNull();
		respuestaSave = null;
		expect(await guardarParaExportar({}, "deck.pdf")).toBeNull();
		respuestaOpen = null;
		expect(await elegirCarpetaParaExportar()).toBeNull();
		expect(recordadas()).toEqual([]);
	});

	it("una lista vacía no recuerda nada", async () => {
		respuestaOpen = [];
		expect(await abrirParaImportar({})).toEqual([]);
		expect(recordadas()).toEqual([]);
	});

	it("sin carpeta guardada el diálogo empieza donde quiera el sistema", async () => {
		carpeta = null;
		respuestaOpen = "/x.png";
		await abrirParaImportar({});
		expect(open).toHaveBeenCalledWith({ defaultPath: undefined });
	});

	it("exportar propone el nombre en la carpeta de exportar y la recuerda", async () => {
		carpeta = "/salida/deck.pdf";
		respuestaSave = "/otra/deck.pdf";
		const r = await guardarParaExportar({ filters: [] }, "deck.pdf");
		expect(r).toBe("/otra/deck.pdf");
		expect(llamadas[0].args).toEqual({ tipo: "exportar", nombre: "deck.pdf" });
		expect(save).toHaveBeenCalledWith({ filters: [], defaultPath: "/salida/deck.pdf" });
		expect(recordadas()).toEqual([{ tipo: "exportar", ruta: "/otra/deck.pdf" }]);
	});

	it("si Rust no responde, exportar propone al menos el nombre", async () => {
		fallaInicial = true;
		respuestaSave = "/x.pdf";
		await guardarParaExportar({}, "mi deck.pdf");
		expect(save).toHaveBeenCalledWith({ defaultPath: "mi deck.pdf" });
	});

	it("elegir carpeta pide un directorio y recuerda la elegida", async () => {
		respuestaOpen = ["/png"];
		expect(await elegirCarpetaParaExportar()).toBe("/png");
		expect(open).toHaveBeenCalledWith({
			directory: true,
			multiple: false,
			defaultPath: "/inicio",
		});
		expect(recordadas()).toEqual([{ tipo: "exportar", ruta: "/png" }]);
	});

	it("recordar no rompe si Rust falla", async () => {
		fallaRecordar = true;
		expect(() => recordar("importar", "/a")).not.toThrow();
		await Promise.resolve();
		expect(recordadas()).toEqual([{ tipo: "importar", ruta: "/a" }]);
	});
});
