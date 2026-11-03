// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { IdCarta } from "@/bindings/IdCarta";

const hacer = vi.fn();
const mirarJugador = vi.fn();
vi.mock("../acciones", () => ({ hacer: (a: unknown) => hacer(a) }));
vi.mock("../operaciones", () => ({ adjuntar: vi.fn() }));
vi.mock("../mirar", () => ({ mirarJugador: (id: number) => mirarJugador(id) }));
vi.mock("../en-vivo/emitir", () => ({ senalar: vi.fn() }));
vi.mock("../originales", () => ({ alternarOriginales: vi.fn() }));
vi.mock("../selector-disposicion", () => ({
	useDisposicion: () => ({ config: { modo: "unico" } }),
}));
vi.mock("../conexion", () => ({ salir: vi.fn() }));
vi.mock("../red/conexion-red", () => ({ volverARed: vi.fn() }));
vi.mock("@/lib/ipc", () => ({ call: vi.fn() }));
vi.mock("sonner", () => ({ toast: { warning: vi.fn() } }));
vi.mock("@/components/icono", () => ({ Icono: () => null }));
vi.mock("@/components/confirmar", () => ({ Confirmar: () => null }));
vi.mock("@/components/contenido-contextual", () => ({ ContenidoContextual: () => null }));
vi.mock("@/components/ayuda-al-tocar", () => ({
	AyudaAlTocar: ({ children }: { children: React.ReactNode }) => children,
}));
vi.mock("@/components/ui/context-menu", () => ({
	ContextMenu: ({ children }: { children: React.ReactNode }) => children,
	ContextMenuTrigger: ({ children }: { children: React.ReactNode }) => children,
	ContextMenuItem: () => null,
}));
vi.mock("@/components/ui/with-tooltip", () => ({
	WithTooltip: ({ children }: { children: React.ReactNode }) => children,
}));
vi.mock("@/components/language-provider", () => ({
	useLang: () => ({
		t: (s: string, p?: Record<string, unknown>) =>
			Object.entries(p ?? {}).reduce((r, [k, v]) => r.replace(`{${k}}`, String(v)), s),
	}),
}));

const { useMesa } = await import("@/stores/mesa-store");
const { usePantalla } = await import("../pantalla");
const { SelectorTablero } = await import("../red/estado-red");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const jugador = (id: number, nombre: string) => ({
	id,
	nombre,
	color: "#fff",
	vida: 40,
	eliminado: null,
	equipo: null,
});

describe("la lista de jugadores de «Un solo tablero»", () => {
	let raiz: Root;
	let caja: HTMLDivElement;

	const pintar = () => act(() => raiz.render(<SelectorTablero />));
	const fila = (nombre: string) => {
		const b = [...caja.querySelectorAll("button")].find((x) => x.textContent?.includes(nombre));
		if (!b) throw new Error(`no está ${nombre}`);
		return b;
	};
	const pulsar = (b: Element) =>
		act(() => {
			b.dispatchEvent(new MouseEvent("click", { bubbles: true }));
		});

	beforeEach(() => {
		hacer.mockClear();
		mirarJugador.mockClear();
		usePantalla.getState().setModo(null);
		useMesa.setState({
			asientos: [1, 2],
			yo: 1,
			mirando: null,
			cartas: {},
			jugadores: { 1: jugador(1, "Ana"), 2: jugador(2, "Bea") },
			general: { flechas: [], equipos: [], turno: { activo: 1 } },
		} as never);
		caja = document.createElement("div");
		document.body.append(caja);
		raiz = createRoot(caja);
	});

	afterEach(() => {
		act(() => raiz.unmount());
		caja.remove();
	});

	it("sin flecha en marcha, pulsar a un jugador enseña su tablero", () => {
		pintar();
		pulsar(fila("Bea"));
		expect(mirarJugador).toHaveBeenCalledWith(2);
		expect(hacer).not.toHaveBeenCalled();
	});

	it("con una flecha en marcha, pulsar a un jugador lo marca como objetivo", () => {
		usePantalla.getState().setModo({ tipo: "flecha", de: 3 as IdCarta, flecha: "ataque" });
		pintar();
		pulsar(fila("Bea"));
		expect(hacer).toHaveBeenCalledWith({
			que: "flecha",
			de: { tipo: "carta", id: 3 },
			a: { tipo: "jugador", id: 2 },
			tipo: "ataque",
		});
		expect(mirarJugador).not.toHaveBeenCalled();
		expect(usePantalla.getState().modo).toBeNull();
	});

	it("el ojo enseña su tablero sin terminar la flecha", () => {
		usePantalla.getState().setModo({ tipo: "flecha", de: 3 as IdCarta, flecha: "ataque" });
		pintar();
		const ojo = caja.querySelector('[aria-label="Ver el tablero de Bea"]');
		if (!ojo) throw new Error("no está el ojo");
		pulsar(ojo);
		expect(mirarJugador).toHaveBeenCalledWith(2);
		expect(hacer).not.toHaveBeenCalled();
		expect(usePantalla.getState().modo).not.toBeNull();
		expect(caja.querySelector('[aria-label="Ver el tablero de Ana"]')).toBeNull();
	});

	it("adjuntar solo admite cartas: pulsar sigue cambiando de tablero", () => {
		usePantalla.getState().setModo({ tipo: "adjuntar", carta: 3 as IdCarta });
		pintar();
		pulsar(fila("Bea"));
		expect(mirarJugador).toHaveBeenCalledWith(2);
		expect(usePantalla.getState().modo).not.toBeNull();
	});

	it("enseña cuántos ataques tiene cada jugador", () => {
		useMesa.setState({
			general: {
				flechas: [
					{
						de: { tipo: "carta", id: 3 },
						a: { tipo: "jugador", id: 2 },
						tipo: "ataque",
						deJugador: 1,
					},
				],
				equipos: [],
				turno: { activo: 1 },
			},
		} as never);
		pintar();
		expect(caja.querySelector('[aria-label="Ataques contra Bea: 1"]')).not.toBeNull();
	});
});
