import { describe, expect, it } from "vitest";
import type { Mesa } from "@/stores/mesa-store";
import { quienJuega } from "../turno";

const mesa = (
	cambios: {
		numero?: number;
		activo?: number;
		primero?: number;
		mulligan?: boolean;
		fin?: boolean;
		yo?: number | null;
		n?: number;
	} = {},
) => {
	const { numero = 3, activo = 1, primero = 0, mulligan = false, fin = false, yo = 0 } = cambios;
	const n = cambios.n ?? 2;
	return {
		asientos: Array.from({ length: n }, (_, i) => i),
		jugadores: { 0: { nombre: "Ana", color: "#f00" }, 1: { nombre: "Bea", color: "#00f" } },
		yo,
		general: {
			turno: { numero, activo, primero },
			mulligan: mulligan ? { decididos: [] } : null,
			fin: fin ? {} : null,
			equipos: [],
		},
	} as unknown as Pick<Mesa, "general" | "jugadores" | "asientos" | "yo">;
};

describe("quién juega", () => {
	it("en una partida empezada, quien tiene el turno", () => {
		expect(quienJuega(mesa(), null)).toEqual({
			id: 1,
			nombre: "Bea",
			color: "#00f",
			empezando: false,
			mio: false,
		});
	});

	it("durante el mulligan, quien va a empezar", () => {
		const q = quienJuega(mesa({ numero: 0, mulligan: true, activo: 1, primero: 0 }), null);
		expect(q?.id).toBe(0);
		expect(q?.empezando).toBe(true);
	});

	it("«mío» solo en red: en local el «yo» es a quien se mira", () => {
		const m = mesa({ activo: 0, yo: 0 });
		expect(quienJuega(m, null)?.mio).toBe(false);
		expect(quienJuega(m, "anfitrion")?.mio).toBe(true);
		expect(quienJuega(m, "jugador")?.mio).toBe(true);
		expect(quienJuega(m, "espectador")?.mio).toBe(false);
	});

	it("no señala nada en solitario ni con la partida terminada", () => {
		expect(quienJuega(mesa({ n: 1 }), null)).toBeNull();
		expect(quienJuega(mesa({ fin: true }), null)).toBeNull();
	});

	it("con equipos, el color del equipo", () => {
		const m = mesa();
		(m.jugadores[1] as { equipo?: number }).equipo = 7;
		(m.general as unknown as { equipos: unknown[] }).equipos = [{ id: 7, color: "#0f0" }];
		expect(quienJuega(m, null)?.color).toBe("#0f0");
	});
});
