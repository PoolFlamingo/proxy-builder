import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";
import type { ResolveResult } from "@/bindings/ResolveResult";

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
vi.mock("@/lib/dialogos", () => ({
	abrirParaImportar: vi.fn(),
	recordar: vi.fn(),
	guardarParaExportar: vi.fn(),
}));

import { huecos } from "@/lib/deck-ops";
import { abrirParaImportar, recordar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";
import {
	anadirPropias,
	cancelarTrabajo,
	cargarLista,
	elegirImagenes,
	reaplicarIdioma,
	resolver,
	reversoGenerico,
	subirCartasPropias,
	subirReverso,
} from "../acciones";

const t = (es: string, p?: Record<string, string | number>) =>
	es.replace(/\{(\w+)\}/g, (_, k) => String(p?.[k]));

const img = (c: string): FaceImage => ({
	blob: c.repeat(64),
	width: 1,
	height: 1,
	origin: "upload",
});

function carta(id: string, nombre = id, extra: Partial<DeckCard> = {}): DeckCard {
	const [b] = huecos(1);
	return {
		...b,
		id,
		source: { kind: "scryfall", scryfallId: `s-${id}` },
		front: img("a"),
		meta: { ...b.meta, name: nombre },
		...extra,
	};
}

function cargar(cards: DeckCard[]) {
	const deck: Deck = {
		id: "d",
		name: "t",
		decklistText: "",
		cards,
		settings: {} as Deck["settings"],
		defaultBack: null,
		schemaVersion: 1,
	};
	useDeck.getState().cargar(deck);
	useUi.getState().seleccionar([], null);
	useUi.getState().anunciar(null);
}
const ids = () => useDeck.getState().deck?.cards.map((c) => c.id);
const ultima = () => useUi.getState().ultimaAccion;

type Args = Record<string, unknown>;
type Fin = { result: ResolveResult | null; error: unknown };
type Respuestas = { [cmd: string]: ((args: Args) => unknown) | Fin | undefined; fin?: Fin };

/**
 * Rust de mentira. `resolve_decklist` devuelve el id 7 y, en la siguiente
 * vuelta, manda las fases y el final con `fin`.
 */
function rust(r: Respuestas) {
	vi.mocked(call).mockImplementation((async (cmd: string, args: Args) => {
		if (cmd === "resolve_decklist" && r.fin) {
			const fin = r.fin;
			setTimeout(() => {
				for (const phase of ["lookup", "images"])
					bus.emitir("resolve://progress", { jobId: 7, progress: { phase, done: 1, total: 2 } });
				bus.emitir("resolve://done", { jobId: 7, ...fin });
			}, 0);
			return 7;
		}
		const f = r[cmd];
		if (typeof f !== "function") throw new Error(`comando inesperado: ${cmd}`);
		return f(args);
	}) as never);
}

const lista = (n: number) => ({ lines: [], cardCount: n, invalidCount: 0 });
const resultado = (cards: DeckCard[], noEncontradas: string[] = []): ResolveResult => ({
	cards,
	unresolved: noEncontradas.map((name, i) => ({ lineNo: i + 1, name, reason: "notFound" })),
});

beforeEach(() => {
	bus.oyentes.clear();
	useTrabajo.getState().terminar();
	useUi.setState({ lang: "en", imagePolicy: "withImage", addTokens: false });
	cargar([carta("x"), carta("y")]);
});

describe("resolver", () => {
	it("manda las preferencias del panel y devuelve el resultado", async () => {
		useUi.setState({ lang: "es", imagePolicy: "withImage", addTokens: true });
		const r = resultado([carta("a")]);
		rust({ fin: { result: r, error: null } });
		await expect(resolver(lista(1), t)).resolves.toEqual(r);
		const [cmd, args] = vi.mocked(call).mock.calls[0];
		expect(cmd).toBe("resolve_decklist");
		expect((args as Args).options).toEqual({
			lang: "es",
			imagePolicy: "withImage",
			addTokens: true,
		});
	});

	it("las opciones pueden quitar los tokens aunque el panel los pida", async () => {
		useUi.setState({ addTokens: true });
		rust({ fin: { result: resultado([]), error: null } });
		await resolver(lista(1), t, { addTokens: false });
		const args = vi.mocked(call).mock.calls[0][1] as { options: { addTokens: boolean } };
		expect(args.options.addTokens).toBe(false);
	});

	it("lleva el trabajo en marcha con sus fases y lo cierra al terminar", async () => {
		const fases: (string | undefined)[] = [];
		const dejar = useTrabajo.subscribe((s) => fases.push(s.actual?.fase));
		rust({ fin: { result: resultado([]), error: null } });
		const r = resolver(lista(1), t);
		expect(useTrabajo.getState().actual?.tipo).toBe("resolver");
		await r;
		dejar();
		expect(fases).toContain("Buscando cartas…");
		expect(fases).toContain("Descargando imágenes…");
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("con otro trabajo en marcha falla con Busy y no lo pisa", async () => {
		useTrabajo.getState().empezar("exportar", "x");
		await expect(resolver(lista(1), t)).rejects.toEqual({ code: "Busy", params: {} });
		expect(useTrabajo.getState().actual?.tipo).toBe("exportar");
		expect(call).not.toHaveBeenCalled();
	});

	it("cancelado devuelve null", async () => {
		rust({ fin: { result: null, error: { code: "Cancelled", params: {} } } });
		await expect(resolver(lista(1), t)).resolves.toBeNull();
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("otro error se lanza y el trabajo se cierra igual", async () => {
		rust({ fin: { result: null, error: { code: "Network", params: {} } } });
		await expect(resolver(lista(1), t)).rejects.toEqual({ code: "Network", params: {} });
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("si Rust no llega a lanzarlo, el trabajo se cierra", async () => {
		rust({});
		await expect(resolver(lista(1), t)).rejects.toThrow();
		expect(useTrabajo.getState().actual).toBeNull();
	});
});

describe("cargarLista", () => {
	it("una lista sin cartas lo dice y no resuelve nada", async () => {
		rust({ parse_decklist: () => lista(0) });
		await expect(cargarLista("// nada", "reemplazar", t)).resolves.toBeNull();
		expect(ultima()).toBe("La lista no tiene cartas");
		expect(vi.mocked(call).mock.calls.map((c) => c[0])).toEqual(["parse_decklist"]);
	});

	it("«Cargar lista» reemplaza las cartas y quita la selección", async () => {
		useUi.getState().seleccionar(["x"], "x");
		rust({
			parse_decklist: () => lista(2),
			fin: { result: resultado([carta("a"), carta("b")]), error: null },
		});
		await expect(cargarLista("2 Sol Ring", "reemplazar", t)).resolves.toBeNull();
		expect(ids()).toEqual(["a", "b"]);
		expect(useUi.getState().seleccion).toEqual([]);
		expect(ultima()).toBe("2 cartas cargadas");
		// Se deshace de una vez.
		useDeck.getState().deshacer();
		expect(ids()).toEqual(["x", "y"]);
	});

	it("«+ Añadir» las pone al final", async () => {
		rust({ parse_decklist: () => lista(1), fin: { result: resultado([carta("a")]), error: null } });
		await cargarLista("Sol Ring", "anadir", t);
		expect(ids()).toEqual(["x", "y", "a"]);
		expect(ultima()).toBe("1 cartas añadidas");
	});

	it("avisa de las no encontradas (cinco nombres y el resto contado)", async () => {
		const nombres = ["A", "B", "C", "D", "E", "F", "G"];
		rust({ parse_decklist: () => lista(8), fin: { result: resultado([], nombres), error: null } });
		await expect(cargarLista("…", "anadir", t)).resolves.toBe("Sin encontrar: A, B, C, D, E (+2)");
	});

	it("con cinco no encontradas exactas no pone el «+»", async () => {
		const nombres = ["A", "B", "C", "D", "E"];
		rust({ parse_decklist: () => lista(5), fin: { result: resultado([], nombres), error: null } });
		await expect(cargarLista("…", "anadir", t)).resolves.toBe("Sin encontrar: A, B, C, D, E");
	});

	it("cancelada no cambia nada", async () => {
		rust({
			parse_decklist: () => lista(1),
			fin: { result: null, error: { code: "Cancelled", params: {} } },
		});
		await expect(cargarLista("Sol Ring", "reemplazar", t)).resolves.toBeNull();
		expect(ids()).toEqual(["x", "y"]);
		expect(ultima()).toBe("Cancelado: no se ha cambiado nada");
		expect(useDeck.getState().pasado).toHaveLength(0);
	});
});

describe("reaplicarIdioma", () => {
	it("sin cartas de Scryfall sin fijar lo dice y no llama a Rust", async () => {
		cargar([carta("a", "A", { locked: true }), carta("p", "P", { source: { kind: "custom" } })]);
		await expect(reaplicarIdioma(t)).resolves.toBeNull();
		expect(ultima()).toBe("No hay cartas de Scryfall sin fijar");
		expect(call).not.toHaveBeenCalled();
	});

	it("vuelve a pedir las no fijadas, sin tokens, y conserva id y fijado", async () => {
		cargar([carta("a", "Uno"), carta("b", "Dos", { locked: true }), carta("c", "Tres")]);
		const uno = carta("n1", "Uno ES", { meta: { ...carta("z").meta, name: "Uno ES", lang: "es" } });
		const tres = carta("n3", "Tres ES");
		rust({ fin: { result: resultado([uno, tres]), error: null } });
		await reaplicarIdioma(t);
		const args = vi.mocked(call).mock.calls[0][1] as {
			list: { cardCount: number };
			options: { addTokens: boolean };
		};
		expect(args.options.addTokens).toBe(false);
		expect(args.list.cardCount).toBe(2);
		const cs = useDeck.getState().deck?.cards ?? [];
		expect(cs.map((c) => [c.id, c.meta.name])).toEqual([
			["a", "Uno ES"],
			["b", "Dos"],
			["c", "Tres ES"],
		]);
		expect(ultima()).toBe("Idioma reaplicado a 2 cartas");
	});

	it("cancelado no toca nada", async () => {
		rust({ fin: { result: null, error: { code: "Cancelled", params: {} } } });
		await expect(reaplicarIdioma(t)).resolves.toBeNull();
		expect(useDeck.getState().pasado).toHaveLength(0);
	});

	it("sin deck no hace nada", async () => {
		useDeck.setState({ deck: null });
		await expect(reaplicarIdioma(t)).resolves.toBeNull();
	});
});

describe("cancelarTrabajo", () => {
	beforeEach(() => vi.mocked(call).mockResolvedValue(true as never));

	it("sin trabajo, o sin id todavía, no hace nada", () => {
		cancelarTrabajo();
		useTrabajo.getState().empezar("resolver", "x");
		cancelarTrabajo();
		expect(call).not.toHaveBeenCalled();
	});

	it("cancela con el comando de su tipo", () => {
		useTrabajo.getState().empezar("resolver", "x");
		useTrabajo.getState().setId(3);
		cancelarTrabajo();
		expect(call).toHaveBeenLastCalledWith("job_cancel", { jobId: 3 });
		useTrabajo.getState().empezar("exportar", "x");
		useTrabajo.getState().setId(4);
		cancelarTrabajo();
		expect(call).toHaveBeenLastCalledWith("export_cancel", { jobId: 4 });
	});
});

describe("imágenes propias y reversos", () => {
	it("elegirImagenes: cancelar o no elegir nada da null", async () => {
		vi.mocked(abrirParaImportar).mockResolvedValueOnce(null);
		await expect(elegirImagenes(t)).resolves.toBeNull();
		vi.mocked(abrirParaImportar).mockResolvedValueOnce([]);
		await expect(elegirImagenes(t)).resolves.toBeNull();
		expect(call).not.toHaveBeenCalled();
	});

	it("elegirImagenes: pide imágenes (una o varias) y las mete en el almacén", async () => {
		vi.mocked(abrirParaImportar).mockResolvedValueOnce(["/a/b.png"]);
		rust({ blob_import_files: () => [img("b")] });
		const r = await elegirImagenes(t, false);
		expect(r).toEqual({ rutas: ["/a/b.png"], imagenes: [img("b")] });
		const opciones = vi.mocked(abrirParaImportar).mock.calls[0][0];
		expect(opciones.multiple).toBe(false);
		expect(opciones.filters?.[0].extensions).toEqual(["png", "jpg", "jpeg", "webp"]);
	});

	it("anadirPropias: recuerda la carpeta, nombra por el fichero e inserta donde se diga", async () => {
		rust({ blob_import_files: (a) => (a.paths as string[]).map((_, i) => img(String(i))) });
		await anadirPropias(["/fotos/Mi Carta.final.png", "C:\\x\\otra.jpg"], t, 1);
		expect(recordar).toHaveBeenCalledWith("importar", "/fotos/Mi Carta.final.png");
		const cs = useDeck.getState().deck?.cards ?? [];
		expect(cs.map((c) => c.meta.name)).toEqual(["x", "Mi Carta.final", "otra", "y"]);
		expect(cs[1].source).toEqual({ kind: "custom" });
		expect(cs[1].front).toEqual(img("0"));
		expect(ultima()).toBe("2 imágenes propias añadidas");
	});

	it("anadirPropias sin rutas no recuerda nada", async () => {
		rust({ blob_import_files: () => [] });
		await anadirPropias([], t);
		expect(recordar).not.toHaveBeenCalled();
		expect(ids()).toEqual(["x", "y"]);
	});

	it("subirCartasPropias las pone al final; cancelar no cambia nada", async () => {
		vi.mocked(abrirParaImportar).mockResolvedValueOnce(null);
		await subirCartasPropias(t);
		expect(useDeck.getState().pasado).toHaveLength(0);
		vi.mocked(abrirParaImportar).mockResolvedValueOnce(["/a/uno.webp"]);
		rust({ blob_import_files: () => [img("c")] });
		await subirCartasPropias(t);
		expect(useDeck.getState().deck?.cards.at(-1)?.meta.name).toBe("uno");
		expect(ultima()).toBe("1 imágenes propias añadidas");
	});

	it("subirReverso: la primera imagen pasa a ser el reverso del deck", async () => {
		vi.mocked(abrirParaImportar).mockResolvedValueOnce(["/a/dorso.png"]);
		rust({ blob_import_files: () => [img("d")] });
		await subirReverso(t);
		expect(useDeck.getState().deck?.defaultBack).toEqual(img("d"));
		expect(ultima()).toBe("Reverso del deck cambiado");
	});

	it("subirReverso: si el almacén no devuelve imagen, no cambia nada", async () => {
		vi.mocked(abrirParaImportar).mockResolvedValueOnce(["/a/roto.png"]);
		rust({ blob_import_files: () => [] });
		await subirReverso(t);
		expect(useDeck.getState().deck?.defaultBack).toBeNull();
		expect(useDeck.getState().pasado).toHaveLength(0);
	});

	it("reversoGenerico: pone el de la app y se puede deshacer", async () => {
		rust({ blob_generic_back: () => img("g") });
		await reversoGenerico(t);
		expect(useDeck.getState().deck?.defaultBack).toEqual(img("g"));
		expect(ultima()).toBe("Reverso genérico restablecido");
		useDeck.getState().deshacer();
		expect(useDeck.getState().deck?.defaultBack).toBeNull();
	});
});
