import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PrintSettings } from "@/bindings/PrintSettings";

const convertFileSrc = vi.fn((_ruta: string, esquema: string) => `${esquema}://localhost`);
vi.mock("@tauri-apps/api/core", () => ({
	convertFileSrc: (ruta: string, esquema: string) => convertFileSrc(ruta, esquema),
}));

async function modulo() {
	vi.resetModules();
	return await import("@/lib/image-url");
}

const ajustes = {
	edgeDarkening: 3,
	bleedMode: "mirror",
	fillCorners: false,
	cardSize: { widthMm: 63, heightMm: 88 },
} as unknown as PrintSettings;

/** Deshace el base64url sin relleno de la URL. */
function decodificar(b64: string): unknown {
	const normal = b64.replace(/-/g, "+").replace(/_/g, "/");
	const relleno = normal + "=".repeat((4 - (normal.length % 4)) % 4);
	return JSON.parse(
		new TextDecoder().decode(Uint8Array.from(atob(relleno), (c) => c.charCodeAt(0))),
	);
}

describe("URLs del protocolo de imágenes", () => {
	beforeEach(() => {
		convertFileSrc.mockReset();
		convertFileSrc.mockImplementation((_r, e) => `${e}://localhost`);
	});

	it("añade la barra final a la base y la calcula una sola vez", async () => {
		const m = await modulo();
		expect(m.urlBusqueda("abc")).toBe("pbimg://localhost/t/abc");
		expect(m.urlDiseno("h1", 488)).toBe("pbimg://localhost/d/h1/488");
		expect(m.urlCapa("h1", 745)).toBe("pbimg://localhost/c/h1/745");
		expect(m.urlArte("b1", 2000)).toBe("pbimg://localhost/b/b1/2000");
		expect(convertFileSrc).toHaveBeenCalledTimes(1);
		expect(convertFileSrc).toHaveBeenCalledWith("", "pbimg");
	});

	it("respeta la base de Windows tal cual si ya acaba en barra", async () => {
		convertFileSrc.mockImplementation(() => "http://pbimg.localhost/");
		const m = await modulo();
		expect(m.urlBusqueda("x")).toBe("http://pbimg.localhost/t/x");
	});

	it("sin Tauri (convertFileSrc falla) usa pbimg://localhost/", async () => {
		convertFileSrc.mockImplementation(() => {
			throw new Error("sin ventana");
		});
		const m = await modulo();
		expect(m.urlDiseno("h", 146)).toBe("pbimg://localhost/d/h/146");
	});

	it("la miniatura lleva los parámetros en base64url sin relleno y se pueden leer", async () => {
		const m = await modulo();
		const p = m.parametros(ajustes, null, true, 3);
		const url = m.urlMiniatura("blob123", p, 745);
		const partes = url.replace("pbimg://localhost/", "").split("/");
		expect(partes[0]).toBe("r");
		expect(partes[1]).toBe("blob123");
		expect(partes[3]).toBe("745");
		expect(partes[2]).not.toMatch(/[+/=]/);
		expect(decodificar(partes[2])).toEqual(p);
	});

	it("los mismos parámetros dan la misma URL (la caché depende de ello)", async () => {
		const m = await modulo();
		const a = m.urlMiniatura("b", m.parametros(ajustes, null, false, 0), 488);
		const b = m.urlMiniatura("b", m.parametros(ajustes, null, false, 0), 488);
		expect(a).toBe(b);
		const c = m.urlMiniatura("b", m.parametros(ajustes, null, true, 0), 488);
		expect(c).not.toBe(a);
	});

	it("parametros: sin edición usa la neutra y copia los ajustes del deck", async () => {
		const m = await modulo();
		const p = m.parametros(ajustes, null, false, 1.5);
		expect(p).toEqual({
			edits: m.EDICION_NEUTRA,
			edge: 3,
			back: false,
			bleedMm: 1.5,
			bleedMode: "mirror",
			fillCorners: false,
			card: { widthMm: 63, heightMm: 88 },
		});
		const edicion = { ...m.EDICION_NEUTRA, rotation: "r90" } as typeof m.EDICION_NEUTRA;
		expect(m.parametros(ajustes, edicion, false, 0).edits).toBe(edicion);
	});

	it("la edición neutra no cambia nada y los anchos van de menor a mayor", async () => {
		const m = await modulo();
		expect(m.EDICION_NEUTRA.rotation).toBe("r0");
		expect(m.EDICION_NEUTRA.crop).toEqual({ zoom: 1, offsetX: 0, offsetY: 0 });
		expect(Object.values(m.EDICION_NEUTRA.adjust).every((v) => v === 0)).toBe(true);
		expect([...m.ANCHOS]).toEqual([...m.ANCHOS].sort((a, b) => a - b));
	});
});
