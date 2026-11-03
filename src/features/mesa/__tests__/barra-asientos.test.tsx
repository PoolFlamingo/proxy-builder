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
vi.mock("sonner", () => ({ toast: { warning: vi.fn() } }));
vi.mock("@/components/icono", () => ({ Icono: () => null }));
vi.mock("@/components/ui/scroll-area", () => ({
	ScrollArea: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));
vi.mock("@/components/language-provider", () => ({
	useLang: () => ({
		t: (s: string, p?: Record<string, unknown>) =>
			Object.entries(p ?? {}).reduce((r, [k, v]) => r.replace(`{${k}}`, String(v)), s),
	}),
}));

const { useMesa } = await import("@/stores/mesa-store");
const { usePantalla } = await import("../pantalla");
const { BarraAsientos } = await import("../movil/barra-asientos");

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const jugador = (id: number, nombre: string) => ({
	id,
	nombre,
	color: "#fff",
	vida: 40,
	eliminado: null,
	equipo: null,
});

describe("la barra de asientos del móvil", () => {
	let raiz: Root;
	let caja: HTMLDivElement;

	const pintar = () => act(() => raiz.render(<BarraAsientos />));
	const botones = () => [...caja.querySelectorAll("button")];
	const fila = (nombre: string) => {
		const b = botones().find((x) => x.textContent?.includes(nombre));
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
			general: { flechas: [], turno: { activo: 1 } },
		} as never);
		caja = document.createElement("div");
		document.body.append(caja);
		raiz = createRoot(caja);
	});

	afterEach(() => {
		act(() => raiz.unmount());
		caja.remove();
	});

	it("sin flecha en marcha, tocar a un jugador enseña su tablero", () => {
		pintar();
		pulsar(fila("Bea"));
		expect(mirarJugador).toHaveBeenCalledWith(2);
		expect(hacer).not.toHaveBeenCalled();
	});

	it("con una flecha en marcha, tocar a un jugador lo marca como objetivo", () => {
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

	it("el ojo enseña su tablero sin terminar la flecha, para apuntar a una de sus cartas", () => {
		usePantalla.getState().setModo({ tipo: "flecha", de: 3 as IdCarta, flecha: "ataque" });
		pintar();
		const ojo = caja.querySelector('[aria-label="Ver el tablero de Bea"]');
		if (!ojo) throw new Error("no está el ojo");
		pulsar(ojo);
		expect(mirarJugador).toHaveBeenCalledWith(2);
		expect(hacer).not.toHaveBeenCalled();
		expect(usePantalla.getState().modo).not.toBeNull();
		// El tablero que ya se ve no lo necesita.
		expect(caja.querySelector('[aria-label="Ver el tablero de Ana"]')).toBeNull();
	});

	it("adjuntar solo admite cartas: tocar sigue cambiando de tablero", () => {
		usePantalla.getState().setModo({ tipo: "adjuntar", carta: 3 as IdCarta });
		pintar();
		pulsar(fila("Bea"));
		expect(mirarJugador).toHaveBeenCalledWith(2);
		expect(usePantalla.getState().modo).not.toBeNull();
	});

	it("plegada, se abre sola al empezar una flecha", () => {
		pintar();
		const plegar = caja.querySelector('[aria-label="Plegar la barra de jugadores"]');
		if (!plegar) throw new Error("no está el botón de plegar");
		pulsar(plegar);
		expect(botones().some((b) => b.textContent?.includes("Bea"))).toBe(false);
		act(() =>
			usePantalla.getState().setModo({ tipo: "flecha", de: 3 as IdCarta, flecha: "ataque" }),
		);
		expect(botones().some((b) => b.textContent?.includes("Bea"))).toBe(true);
		// Al terminar vuelve a como estaba.
		act(() => usePantalla.getState().setModo(null));
		expect(botones().some((b) => b.textContent?.includes("Bea"))).toBe(false);
		pulsar(caja.querySelector('[aria-label="Enseñar los jugadores"]') as Element);
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
				turno: { activo: 1 },
			},
		} as never);
		pintar();
		expect(caja.querySelector('[aria-label="Ataques contra Bea: 1"]')).not.toBeNull();
		expect(caja.querySelector('[aria-label^="Ataques contra Ana"]')).toBeNull();
	});
});
