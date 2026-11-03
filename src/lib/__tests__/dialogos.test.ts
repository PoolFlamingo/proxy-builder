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

let movil = false;
vi.mock("@/lib/plataforma", () => ({ esMovil: async () => movil }));

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
	elegirDestinoDeImagenes,
	filtrosParaAbrirEnMovil,
	filtrosParaGuardarEnMovil,
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
		movil = false;
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

	describe("en Android", () => {
		beforeEach(() => {
			movil = true;
		});

		it("importar no pregunta ni recuerda carpetas y devuelve los content://", async () => {
			respuestaOpen = ["content://docs/1", "content://docs/2"];
			const r = await abrirParaImportar({
				multiple: true,
				filters: [{ name: "Imágenes", extensions: ["png", "jpg"] }],
			});
			expect(r).toEqual(["content://docs/1", "content://docs/2"]);
			expect(llamadas).toEqual([]);
			expect(open).toHaveBeenCalledWith({
				multiple: true,
				filters: [{ name: "Imágenes", extensions: ["image/png", "image/jpeg"] }],
			});
		});

		it("exportar propone solo el nombre, con el tipo del formato, y no recuerda nada", async () => {
			respuestaSave = "content://docs/9";
			const r = await guardarParaExportar(
				{ title: "x", filters: [{ name: "PDF", extensions: ["pdf"] }] },
				"deck.pdf",
			);
			expect(r).toBe("content://docs/9");
			expect(llamadas).toEqual([]);
			expect(save).toHaveBeenCalledWith({
				title: "x",
				filters: [{ name: "PDF", extensions: ["application/pdf"] }],
				defaultPath: "deck.pdf",
			});
		});

		it("elegir carpeta no existe", async () => {
			await expect(elegirCarpetaParaExportar()).rejects.toMatchObject({ code: "Unsupported" });
			expect(open).not.toHaveBeenCalled();
		});

		it("las imágenes con carpetas por idioma salen en un ZIP", async () => {
			respuestaSave = "content://docs/zip";
			const r = await elegirDestinoDeImagenes({ titulo: "PNG", nombre: "deck.zip" });
			expect(r).toBe("content://docs/zip");
			expect(save).toHaveBeenCalledWith({
				title: "PNG",
				filters: [{ name: "ZIP", extensions: ["application/zip"] }],
				defaultPath: "deck.zip",
			});
		});
	});

	it("en escritorio las imágenes salen en una carpeta", async () => {
		respuestaOpen = ["/png"];
		expect(await elegirDestinoDeImagenes({ titulo: "PNG", nombre: "deck.zip" })).toBe("/png");
		expect(save).not.toHaveBeenCalled();
	});
});

describe("filtros de Android", () => {
	it("para abrir: con un formato sin tipo conocido no se filtra nada", () => {
		expect(filtrosParaAbrirEnMovil([{ name: "Proxy Builder", extensions: ["pbdeck"] }])).toEqual(
			[],
		);
		expect(filtrosParaAbrirEnMovil([{ name: "Letras", extensions: ["ttf", "otf"] }])).toEqual([]);
		expect(filtrosParaAbrirEnMovil(undefined)).toEqual([]);
	});

	it("para abrir: los conocidos pasan a tipo, sin repetir, y un zip admite sus dos tipos", () => {
		expect(
			filtrosParaAbrirEnMovil([{ name: "Imágenes", extensions: ["jpg", "JPEG", "png", "webp"] }]),
		).toEqual([{ name: "Imágenes", extensions: ["image/jpeg", "image/png", "image/webp"] }]);
		expect(filtrosParaAbrirEnMovil([{ name: "Pack", extensions: ["zip"] }])).toEqual([
			{ name: "Pack", extensions: ["application/zip", "application/x-zip-compressed"] },
		]);
	});

	it("para guardar: un solo tipo y, si no se conoce, el binario genérico", () => {
		expect(filtrosParaGuardarEnMovil([{ name: "ZIP", extensions: ["zip"] }])).toEqual([
			{ name: "ZIP", extensions: ["application/zip"] },
		]);
		expect(filtrosParaGuardarEnMovil([{ name: "Proxy Builder", extensions: ["pbdeck"] }])).toEqual([
			{ name: "Proxy Builder", extensions: ["application/octet-stream"] },
		]);
		expect(filtrosParaGuardarEnMovil(undefined)).toEqual([
			{ name: "", extensions: ["application/octet-stream"] },
		]);
	});
});
