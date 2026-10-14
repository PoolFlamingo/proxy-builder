import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Deck } from "@/bindings/Deck";
import type { ExportResult } from "@/bindings/ExportResult";

/** Los eventos de Rust, de mentira: `on` apunta y `emitir` reparte. */
const bus = vi.hoisted(() => {
	const oyentes = new Map<string, Set<(p: unknown) => void>>();
	return {
		oyentes,
		emitir(ev: string, p: unknown) {
			for (const f of oyentes.get(ev) ?? []) f(p);
		},
	};
});

vi.mock("@/lib/ipc", () => ({
	call: vi.fn(),
	on: vi.fn(async (ev: string, f: (p: unknown) => void) => {
		const s = bus.oyentes.get(ev) ?? new Set();
		s.add(f);
		bus.oyentes.set(ev, s);
		return () => s.delete(f);
	}),
}));
vi.mock("@/lib/dialogos", () => ({ guardarParaExportar: vi.fn() }));
vi.mock("sonner", () => ({ toast: { error: vi.fn(), success: vi.fn() } }));

import { toast } from "sonner";
import { huecos } from "@/lib/deck-ops";
import { guardarParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";
import {
	AVISO_BYTES,
	exportarA,
	exportarCalibracion,
	megas,
	nombreSeguro,
	pedirDestino,
} from "../acciones";

const t = (es: string, p?: Record<string, string | number>) =>
	es.replace(/\{(\w+)\}/g, (_, k) => String(p?.[k]));

function cargar(nombre = "Mi deck") {
	const deck: Deck = {
		id: "d",
		name: nombre,
		decklistText: "",
		cards: huecos(3),
		settings: { designLang: "es" } as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
	useDeck.getState().cargar(deck);
	useUi.getState().anunciar(null);
}

type Args = Record<string, unknown>;

/** Rust de mentira: `export_pdf` devuelve el id 9 y luego manda fases y final. */
function rust(fin: { result: ExportResult | null; error: unknown } | null, otros: Args = {}) {
	vi.mocked(call).mockImplementation((async (cmd: string) => {
		if (cmd === "export_pdf") {
			if (!fin) throw { code: "Io", params: {} };
			setTimeout(() => {
				for (const phase of ["prepare", "compose", "write"])
					bus.emitir("export://progress", { jobId: 9, phase, done: 1, total: 3 });
				bus.emitir("export://done", { jobId: 9, ...fin });
			}, 0);
			return 9;
		}
		return otros[cmd] ?? null;
	}) as never);
}

const hecho = (paths: string[], pages = 6): ExportResult => ({
	paths,
	pages,
	sheets: 3,
	bytes: 12 * 1024 * 1024,
});

/** Las opciones del último `toast.success`. */
function ultimoToast() {
	const c = vi.mocked(toast.success).mock.calls.at(-1);
	return { texto: c?.[0], opciones: c?.[1] as Record<string, unknown> | undefined };
}

beforeEach(() => {
	bus.oyentes.clear();
	useTrabajo.getState().terminar();
	cargar();
});

describe("megas y nombreSeguro", () => {
	it("megas redondea a MB enteros", () => {
		expect(megas(0)).toBe("0 MB");
		expect(megas(1.4 * 1024 * 1024)).toBe("1 MB");
		expect(megas(1.5 * 1024 * 1024)).toBe("2 MB");
		expect(megas(AVISO_BYTES)).toBe("500 MB");
	});

	it("nombreSeguro quita lo que los sistemas no admiten", () => {
		expect(nombreSeguro("Mi deck", "pdf")).toBe("Mi deck.pdf");
		expect(nombreSeguro('a/b\\c:d*e?f"g<h>i|j', "pdf")).toBe("a b c d e f g h i j.pdf");
		expect(nombreSeguro("a//b", "pdf")).toBe("a b.pdf");
		expect(nombreSeguro("  Atraxa  ", "png")).toBe("Atraxa.png");
	});

	it("nombreSeguro con un nombre vacío o solo de símbolos usa «deck»", () => {
		expect(nombreSeguro("", "pdf")).toBe("deck.pdf");
		expect(nombreSeguro("   ", "pdf")).toBe("deck.pdf");
		expect(nombreSeguro("///", "pdf")).toBe("deck.pdf");
	});

	it("nombreSeguro respeta los acentos y la ñ", () => {
		expect(nombreSeguro("Niño Ñandú", "pdf")).toBe("Niño Ñandú.pdf");
	});
});

describe("pedirDestino", () => {
	it("propone el nombre del deck limpio y devuelve la ruta elegida", async () => {
		cargar("Elfos: v2");
		vi.mocked(guardarParaExportar).mockResolvedValueOnce("/tmp/x.pdf");
		await expect(pedirDestino(t)).resolves.toBe("/tmp/x.pdf");
		const [opciones, nombre] = vi.mocked(guardarParaExportar).mock.calls[0];
		expect(nombre).toBe("Elfos  v2.pdf");
		expect(opciones.filters).toEqual([{ name: "PDF", extensions: ["pdf"] }]);
	});

	it("sin deck no abre el diálogo", async () => {
		useDeck.setState({ deck: null });
		await expect(pedirDestino(t)).resolves.toBeNull();
		expect(guardarParaExportar).not.toHaveBeenCalled();
	});
});

describe("exportarA", () => {
	it("manda la instantánea del store, no lo guardado", async () => {
		rust({ result: hecho(["/tmp/a.pdf"]), error: null });
		const deck = useDeck.getState().deck;
		await exportarA("/tmp/a.pdf", t);
		expect(call).toHaveBeenCalledWith("export_pdf", { deck, path: "/tmp/a.pdf" });
	});

	it("al terminar lo anuncia y ofrece abrirlo o verlo en la carpeta", async () => {
		rust({ result: hecho(["/tmp/a.pdf"]), error: null });
		const r = await exportarA("/tmp/a.pdf", t);
		expect(r?.pages).toBe(6);
		expect(useUi.getState().ultimaAccion).toBe("PDF guardado (6 páginas)");
		const { texto, opciones } = ultimoToast();
		expect(texto).toBe("PDF guardado (6 páginas)");
		expect(opciones?.description).toBe("12 MB · /tmp/a.pdf");
		(opciones?.action as { onClick: () => void }).onClick();
		expect(call).toHaveBeenLastCalledWith("export_open", { path: "/tmp/a.pdf" });
		(opciones?.cancel as { onClick: () => void }).onClick();
		expect(call).toHaveBeenLastCalledWith("export_reveal", { path: "/tmp/a.pdf" });
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("partido en varios ficheros lo dice y abre el primero", async () => {
		rust({ result: hecho(["/tmp/a-1.pdf", "/tmp/a-2.pdf"], 40), error: null });
		await exportarA("/tmp/a.pdf", t);
		expect(useUi.getState().ultimaAccion).toBe("PDF guardado en 2 ficheros (40 páginas)");
		expect(ultimoToast().opciones?.description).toBe("12 MB · /tmp/a-1.pdf");
	});

	it("pasa por las tres fases", async () => {
		const fases: (string | undefined)[] = [];
		const dejar = useTrabajo.subscribe((s) => fases.push(s.actual?.fase));
		rust({ result: hecho(["/tmp/a.pdf"]), error: null });
		await exportarA("/tmp/a.pdf", t);
		dejar();
		expect(fases).toEqual(
			expect.arrayContaining([
				"Preparando imágenes…",
				"Componiendo páginas…",
				"Escribiendo el PDF…",
			]),
		);
	});

	it("cancelada: lo anuncia, devuelve null y no saca el aviso de hecho", async () => {
		rust({ result: null, error: { code: "Cancelled", params: {} } });
		await expect(exportarA("/tmp/a.pdf", t)).resolves.toBeNull();
		expect(useUi.getState().ultimaAccion).toBe("Exportación cancelada");
		expect(toast.success).not.toHaveBeenCalled();
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("un error de Rust se lanza y el trabajo se cierra", async () => {
		rust({ result: null, error: { code: "Io", params: { ruta: "/x" } } });
		await expect(exportarA("/tmp/a.pdf", t)).rejects.toEqual({
			code: "Io",
			params: { ruta: "/x" },
		});
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("si no llega a lanzarse, también se cierra", async () => {
		rust(null);
		await expect(exportarA("/tmp/a.pdf", t)).rejects.toEqual({ code: "Io", params: {} });
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("con otro trabajo en marcha falla con Busy", async () => {
		useTrabajo.getState().empezar("resolver", "x");
		await expect(exportarA("/tmp/a.pdf", t)).rejects.toEqual({ code: "Busy", params: {} });
		expect(useTrabajo.getState().actual?.tipo).toBe("resolver");
		expect(call).not.toHaveBeenCalled();
	});

	it("sin deck devuelve null sin empezar nada", async () => {
		useDeck.setState({ deck: null });
		await expect(exportarA("/tmp/a.pdf", t)).resolves.toBeNull();
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("sin resultado ni error (no debería pasar) devuelve null sin avisar", async () => {
		rust({ result: null, error: null });
		await expect(exportarA("/tmp/a.pdf", t)).resolves.toBeNull();
		expect(toast.success).not.toHaveBeenCalled();
	});
});

describe("exportarCalibracion", () => {
	it("si se cancela el diálogo no llama a Rust", async () => {
		vi.mocked(guardarParaExportar).mockResolvedValueOnce(null);
		await exportarCalibracion(t);
		expect(call).not.toHaveBeenCalled();
	});

	it("manda los ajustes del deck y los textos ya traducidos", async () => {
		vi.mocked(guardarParaExportar).mockResolvedValueOnce("/tmp/cal.pdf");
		rust(null, { export_calibration: "/tmp/cal.pdf" });
		await exportarCalibracion(t);
		expect(vi.mocked(guardarParaExportar).mock.calls[0][1]).toBe("calibracion-duplex.pdf");
		const [cmd, args] = vi.mocked(call).mock.calls[0] as unknown as [string, Args];
		expect(cmd).toBe("export_calibration");
		expect(args.path).toBe("/tmp/cal.pdf");
		expect(args.settings).toBe(useDeck.getState().deck?.settings);
		const texts = args.texts as { title: string; instructions: string[] };
		expect(texts.title).toBe("Hoja de calibración dúplex");
		expect(texts.instructions).toHaveLength(4);
		expect(useUi.getState().ultimaAccion).toBe("Hoja de calibración guardada");
		const { opciones } = ultimoToast();
		(opciones?.action as { onClick: () => void }).onClick();
		expect(call).toHaveBeenLastCalledWith("export_open", { path: "/tmp/cal.pdf" });
	});

	it("sin deck no hace nada", async () => {
		useDeck.setState({ deck: null });
		await exportarCalibracion(t);
		expect(guardarParaExportar).not.toHaveBeenCalled();
	});
});
