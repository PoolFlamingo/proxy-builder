import { afterEach, describe, expect, it, vi } from "vitest";

const CLAVE = "proxybuilder.preferencias";

/** Un localStorage de mentira (en node no hay). */
function almacen(inicial: Record<string, string> = {}, rompe = false) {
	const datos = new Map(Object.entries(inicial));
	return {
		datos,
		getItem: (k: string) => {
			if (rompe) throw new Error("bloqueado");
			return datos.get(k) ?? null;
		},
		setItem: (k: string, v: string) => {
			if (rompe) throw new Error("bloqueado");
			datos.set(k, v);
		},
	};
}

async function store(ls?: ReturnType<typeof almacen>) {
	vi.resetModules();
	if (ls) vi.stubGlobal("localStorage", ls);
	return await import("@/stores/ui-store");
}

describe("store de pantalla", () => {
	afterEach(() => vi.unstubAllGlobals());

	it("sin almacenamiento arranca con las preferencias de fábrica", async () => {
		const { useUi } = await store();
		const s = useUi.getState();
		expect(s.lang).toBe("en");
		expect(s.zoom).toBe(0.8);
		expect(s.calidadVista).toBe(488);
		expect(s.pedirConfirmacionAlCargar).toBe(true);
		expect(s.vista).toBe("deck");
		expect(s.seleccion).toEqual([]);
	});

	it("lee lo guardado y completa lo que falte con lo de fábrica", async () => {
		const ls = almacen({ [CLAVE]: JSON.stringify({ lang: "es", zoom: 1.2 }) });
		const { useUi } = await store(ls);
		expect(useUi.getState().lang).toBe("es");
		expect(useUi.getState().zoom).toBe(1.2);
		expect(useUi.getState().imagePolicy).toBe("withImage");
	});

	it("un JSON roto no impide arrancar", async () => {
		const { useUi } = await store(almacen({ [CLAVE]: "{roto" }));
		expect(useUi.getState().lang).toBe("en");
	});

	it("un almacenamiento bloqueado no rompe ni leer ni guardar", async () => {
		const { useUi } = await store(almacen({}, true));
		expect(() => useUi.getState().setZoom(1)).not.toThrow();
		expect(useUi.getState().zoom).toBe(1);
	});

	it("el zoom se limita y se redondea a centésimas, y se recuerda", async () => {
		const ls = almacen();
		const { useUi, ZOOM_MAX, ZOOM_MIN } = await store(ls);
		const s = useUi.getState;
		s().setZoom(10);
		expect(s().zoom).toBe(ZOOM_MAX);
		s().setZoom(0.01);
		expect(s().zoom).toBe(ZOOM_MIN);
		s().setZoom(1.23456);
		expect(s().zoom).toBe(1.23);
		expect(JSON.parse(ls.datos.get(CLAVE) ?? "{}").zoom).toBe(1.23);
	});

	it("solo se guardan las preferencias, no la selección ni los diálogos", async () => {
		const ls = almacen();
		const { useUi } = await store(ls);
		useUi.getState().seleccionar(["a"]);
		useUi.getState().abrir({ tipo: "sets" });
		useUi.getState().setPreferencia("addTokens", true);
		const guardado = JSON.parse(ls.datos.get(CLAVE) ?? "{}");
		expect(guardado.addTokens).toBe(true);
		expect(Object.keys(guardado).sort()).toEqual(
			[
				"actualizarSola",
				"addTokens",
				"calidadVista",
				"disenarCopias",
				"imagePolicy",
				"infoCarta",
				"lang",
				"mesa",
				"pedirConfirmacionAlCargar",
				"tutorialesVistos",
				"zoom",
			].sort(),
		);
	});

	it("abrir un tutorial lo apunta como visto (una vez) y se recuerda; el abierto no", async () => {
		const ls = almacen();
		const { useUi } = await store(ls);
		const s = useUi.getState;
		expect(s().tutorialesVistos).toEqual([]);
		s().abrirTutorial("disenar");
		expect(s().tutorial).toBe("disenar");
		s().abrirTutorial(null);
		s().abrirTutorial("disenar");
		s().abrirTutorial(null);
		expect(s().tutorial).toBeNull();
		const guardado = JSON.parse(ls.datos.get(CLAVE) ?? "{}");
		expect(guardado.tutorialesVistos).toEqual(["disenar"]);
		expect(guardado.tutorial).toBeUndefined();
	});

	it("los ajustes de la mesa se recuerdan y se completan con los de fábrica", async () => {
		const ls = almacen({ [CLAVE]: JSON.stringify({ mesa: { tamanoCartas: 150 } }) });
		const { useUi, MESA_DE_FABRICA } = await store(ls);
		expect(useUi.getState().mesa).toEqual({ ...MESA_DE_FABRICA, tamanoCartas: 150 });
		useUi.getState().setMesa({ sombras: false });
		const guardado = JSON.parse(ls.datos.get(CLAVE) ?? "{}");
		expect(guardado.mesa.sombras).toBe(false);
		expect(guardado.mesa.tamanoCartas).toBe(150);
	});

	it("la info de carta se recuerda", async () => {
		const ls = almacen();
		const { useUi } = await store(ls);
		useUi.getState().setInfoCarta(true);
		expect(JSON.parse(ls.datos.get(CLAVE) ?? "{}").infoCarta).toBe(true);
	});

	it("seleccionar sin ancla conserva la anterior; con null la quita", async () => {
		const { useUi } = await store();
		const s = useUi.getState;
		s().seleccionar(["a"], "a");
		s().seleccionar(["a", "b"]);
		expect(s().ancla).toBe("a");
		expect(s().seleccion).toEqual(["a", "b"]);
		s().seleccionar([], null);
		expect(s().ancla).toBeNull();
	});

	it("diseñar una carta cambia a la vista «Diseñar»; soltarla no cambia la vista", async () => {
		const { useUi } = await store();
		const s = useUi.getState;
		s().disenar("c1");
		expect(s().vista).toBe("disenar");
		expect(s().disenando).toBe("c1");
		s().setVista("biblioteca");
		s().disenar(null);
		expect(s().vista).toBe("biblioteca");
		expect(s().disenando).toBeNull();
	});

	it("abrir y anunciar guardan el diálogo y la última acción", async () => {
		const { useUi } = await store();
		useUi.getState().abrir({ tipo: "editar", id: "x" });
		expect(useUi.getState().dialogo).toEqual({ tipo: "editar", id: "x" });
		useUi.getState().abrir(null);
		expect(useUi.getState().dialogo).toBeNull();
		useUi.getState().anunciar("PDF guardado");
		expect(useUi.getState().ultimaAccion).toBe("PDF guardado");
	});
});
