import { beforeEach, describe, expect, it, vi } from "vitest";
import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";

const hacer = vi.fn();
vi.mock("../acciones", () => ({ hacer: (a: unknown) => hacer(a) }));
vi.mock("../operaciones", () => ({ adjuntar: vi.fn() }));
vi.mock("sonner", () => ({ toast: { warning: vi.fn() } }));

const { elegirCarta, elegirJugador } = await import("../objetivo");
const { usePantalla } = await import("../pantalla");

const t = (s: string) => s;
const carta = (id: number) => ({ id: id as IdCarta, controlador: 1 }) as unknown as VistaCarta;

describe("elegir objetivo", () => {
	beforeEach(() => {
		hacer.mockClear();
		usePantalla.getState().setModo(null);
	});

	it("sin modo no hace nada y la carta se pulsa como siempre", () => {
		expect(elegirCarta(carta(5), t)).toBe(false);
		expect(elegirJugador(1, t)).toBe(false);
		expect(hacer).not.toHaveBeenCalled();
	});

	it("una flecha puede acabar en un jugador (pulsar su mesa) y termina el modo", () => {
		usePantalla.getState().setModo({ tipo: "flecha", de: 3 as IdCarta, flecha: "ataque" });
		expect(elegirJugador(1, t)).toBe(true);
		expect(hacer).toHaveBeenCalledWith({
			que: "flecha",
			de: { tipo: "carta", id: 3 },
			a: { tipo: "jugador", id: 1 },
			tipo: "ataque",
		});
		expect(usePantalla.getState().modo).toBeNull();
	});

	it("adjuntar solo admite cartas: pulsar un jugador no cambia nada", () => {
		usePantalla.getState().setModo({ tipo: "adjuntar", carta: 3 as IdCarta });
		expect(elegirJugador(1, t)).toBe(false);
		expect(usePantalla.getState().modo).not.toBeNull();
	});

	it("una flecha a una carta de otra mesa la apunta y termina el modo", () => {
		usePantalla.getState().setModo({ tipo: "flecha", de: 3 as IdCarta, flecha: "objetivo" });
		expect(elegirCarta(carta(9), t)).toBe(true);
		expect(hacer).toHaveBeenCalledWith({
			que: "flecha",
			de: { tipo: "carta", id: 3 },
			a: { tipo: "carta", id: 9 },
			tipo: "objetivo",
		});
	});
});
