import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/** El webview de Tauri, de mentira: guarda el manejador del arrastre. */
const tauri = vi.hoisted(() => ({
	manejador: null as ((ev: unknown) => void) | null,
	dejar: vi.fn(),
	/** Lo que tarda en cumplirse `onDragDropEvent` (se resuelve a mano). */
	soltarPromesa: null as (() => void) | null,
	fallar: false,
	esperar: false,
}));

vi.mock("@tauri-apps/api/webview", () => ({
	getCurrentWebview: () => ({
		onDragDropEvent: (f: (ev: unknown) => void) => {
			if (tauri.fallar) return Promise.reject(new Error("fuera de Tauri"));
			tauri.manejador = f;
			if (!tauri.esperar) return Promise.resolve(tauri.dejar);
			return new Promise((r) => {
				tauri.soltarPromesa = () => r(tauri.dejar);
			});
		},
	}),
}));

/** `useEffect` de mentira: lo ejecuta ya y guarda su limpieza. */
const efecto = vi.hoisted(() => ({ limpiar: null as (() => void) | null }));
vi.mock("react", async (original) => ({
	...(await original<typeof import("react")>()),
	useEffect: (f: () => (() => void) | undefined) => {
		efecto.limpiar = f() ?? null;
	},
}));

vi.mock("@/components/language-provider", () => ({
	paraTraducir: (s: string) => s,
	useLang: () => ({
		t: (s: string) => s,
		mensaje: (e: unknown) => `error: ${(e as { code: string }).code}`,
	}),
}));
vi.mock("sonner", () => ({ toast: { error: vi.fn() } }));
vi.mock("../acciones-carta", () => ({ reemplazarConFichero: vi.fn(async () => {}) }));
vi.mock("@/features/decklist/acciones", async (original) => ({
	...(await original<typeof import("@/features/decklist/acciones")>()),
	anadirPropias: vi.fn(async () => {}),
}));

import { toast } from "sonner";
import { anadirPropias } from "@/features/decklist/acciones";
import { reemplazarConFichero } from "../acciones-carta";
import { soloImagenes, soltarFicheros, useArrastrarFicheros } from "../arrastrar-ficheros";

const t = (s: string) => s;
const nada = () => undefined;
const esperarVuelta = () => new Promise((r) => setTimeout(r, 0));

describe("soloImagenes", () => {
	it("se queda con png, jpg, jpeg y webp, sin mirar mayúsculas", () => {
		expect(
			soloImagenes([
				"/a/uno.png",
				"/a/dos.JPG",
				"/a/tres.Jpeg",
				"/a/cuatro.webp",
				"/a/lista.txt",
				"/a/deck.pbdeck",
				"/a/anim.gif",
			]),
		).toEqual(["/a/uno.png", "/a/dos.JPG", "/a/tres.Jpeg", "/a/cuatro.webp"]);
	});

	it("mira solo la última extensión", () => {
		expect(soloImagenes(["/a/carta.png.txt", "/a/carta.txt.png"])).toEqual(["/a/carta.txt.png"]);
	});

	it("un fichero sin extensión no cuenta, aunque su carpeta lleve punto", () => {
		expect(soloImagenes(["/a/fotos.png/carta", "/a/png", "C:\\x.jpg\\carta"])).toEqual([]);
	});

	it("rutas de Windows", () => {
		expect(soloImagenes(["C:\\Users\\yo\\Carta.PNG"])).toEqual(["C:\\Users\\yo\\Carta.PNG"]);
	});
});

describe("soltarFicheros", () => {
	it("sin imágenes no hace nada (ni mira dónde se soltó)", () => {
		const cartaEn = vi.fn(nada);
		expect(soltarFicheros(["/a/x.txt"], { x: 1, y: 1 }, 1, cartaEn, t)).toBeNull();
		expect(soltarFicheros([], { x: 1, y: 1 }, 1, cartaEn, t)).toBeNull();
		expect(cartaEn).not.toHaveBeenCalled();
		expect(anadirPropias).not.toHaveBeenCalled();
	});

	it("sobre una carta la reemplaza con la primera imagen", async () => {
		await soltarFicheros(["/a/l.txt", "/a/1.png", "/a/2.png"], { x: 1, y: 1 }, 1, () => "c7", t);
		expect(reemplazarConFichero).toHaveBeenCalledWith("c7", "/a/1.png");
		expect(anadirPropias).not.toHaveBeenCalled();
	});

	it("fuera de las cartas añade todas las imágenes como propias", async () => {
		await soltarFicheros(["/a/1.png", "/a/l.txt", "/a/2.webp"], { x: 1, y: 1 }, 1, nada, t);
		expect(anadirPropias).toHaveBeenCalledWith(["/a/1.png", "/a/2.webp"], t);
		expect(reemplazarConFichero).not.toHaveBeenCalled();
	});

	it("pasa los píxeles físicos de Tauri a píxeles CSS", () => {
		const cartaEn = vi.fn(nada);
		soltarFicheros(["/a/1.png"], { x: 300, y: 150 }, 1.5, cartaEn, t);
		expect(cartaEn).toHaveBeenCalledWith(200, 100);
	});

	it("devuelve la promesa para poder avisar del error", async () => {
		vi.mocked(anadirPropias).mockRejectedValueOnce({ code: "Io" });
		await expect(soltarFicheros(["/a/1.png"], { x: 0, y: 0 }, 1, nada, t)).rejects.toEqual({
			code: "Io",
		});
	});
});

describe("useArrastrarFicheros", () => {
	beforeEach(() => {
		tauri.manejador = null;
		tauri.soltarPromesa = null;
		tauri.fallar = false;
		tauri.esperar = false;
		efecto.limpiar = null;
		vi.stubGlobal("window", { devicePixelRatio: 2 });
		vi.stubGlobal("document", {
			elementFromPoint: (x: number, y: number) =>
				x === 50 && y === 25
					? { closest: () => ({ dataset: { carta: "c1" } }) }
					: { closest: () => null },
		});
	});
	afterEach(() => {
		vi.unstubAllGlobals();
	});

	const soltar = (paths: string[], x: number, y: number, type = "drop") =>
		tauri.manejador?.({ payload: { type, paths, position: { x, y } } });

	it("soltar sobre una carta (con la escala de la pantalla) la reemplaza", async () => {
		useArrastrarFicheros();
		await esperarVuelta();
		soltar(["/a/1.png"], 100, 50);
		expect(reemplazarConFichero).toHaveBeenCalledWith("c1", "/a/1.png");
	});

	it("soltar fuera de las cartas las añade", async () => {
		useArrastrarFicheros();
		await esperarVuelta();
		soltar(["/a/1.png"], 10, 10);
		expect(anadirPropias).toHaveBeenCalledWith(["/a/1.png"], expect.any(Function));
	});

	it("con la escala sin definir usa 1", async () => {
		vi.stubGlobal("window", { devicePixelRatio: 0 });
		useArrastrarFicheros();
		await esperarVuelta();
		soltar(["/a/1.png"], 50, 25);
		expect(reemplazarConFichero).toHaveBeenCalledWith("c1", "/a/1.png");
	});

	it("solo actúa al soltar (no al pasar por encima)", async () => {
		useArrastrarFicheros();
		await esperarVuelta();
		soltar(["/a/1.png"], 10, 10, "over");
		soltar(["/a/1.png"], 10, 10, "enter");
		expect(anadirPropias).not.toHaveBeenCalled();
	});

	it("un fallo al importar sale en un aviso", async () => {
		vi.mocked(anadirPropias).mockRejectedValueOnce({ code: "Io" });
		useArrastrarFicheros();
		await esperarVuelta();
		soltar(["/a/1.png"], 10, 10);
		await esperarVuelta();
		expect(toast.error).toHaveBeenCalledWith("error: Io");
	});

	it("al desmontar deja de escuchar", async () => {
		useArrastrarFicheros();
		await esperarVuelta();
		efecto.limpiar?.();
		expect(tauri.dejar).toHaveBeenCalledTimes(1);
	});

	it("si se desmonta antes de que Tauri conteste, deja de escuchar al contestar", async () => {
		tauri.esperar = true;
		useArrastrarFicheros();
		efecto.limpiar?.();
		expect(tauri.dejar).not.toHaveBeenCalled();
		tauri.soltarPromesa?.();
		await esperarVuelta();
		expect(tauri.dejar).toHaveBeenCalledTimes(1);
	});

	it("fuera de Tauri no revienta", async () => {
		tauri.fallar = true;
		useArrastrarFicheros();
		await esperarVuelta();
		expect(() => efecto.limpiar?.()).not.toThrow();
	});
});
