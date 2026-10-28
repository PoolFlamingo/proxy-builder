import { describe, expect, it } from "vitest";
import type { Flecha } from "@/bindings/Flecha";
import { atacantesDe, duenoDeExtremo, idDePunta } from "../cuentas-flechas";

const cartas = { 7: { controlador: 1 }, 8: { controlador: 2 } } as never;
const flecha = (a: Flecha["a"], tipo: Flecha["tipo"] = "ataque"): Flecha => ({
	de: { tipo: "carta", id: 3 },
	a,
	tipo,
	deJugador: 0,
});

describe("flechas con varios jugadores", () => {
	it("el extremo es del jugador al que se apunta o de quien controla la carta", () => {
		expect(duenoDeExtremo({ tipo: "jugador", id: 2 }, cartas)).toBe(2);
		expect(duenoDeExtremo({ tipo: "carta", id: 7 }, cartas)).toBe(1);
		expect(duenoDeExtremo({ tipo: "carta", id: 99 }, cartas)).toBeNull();
	});
	it("cuenta los ataques contra un jugador y sus cartas, no los objetivos", () => {
		const fs = [
			flecha({ tipo: "jugador", id: 1 }),
			flecha({ tipo: "carta", id: 7 }),
			flecha({ tipo: "carta", id: 7 }, "objetivo"),
			flecha({ tipo: "carta", id: 8 }),
		];
		expect(atacantesDe(fs, cartas, 1)).toBe(2);
		expect(atacantesDe(fs, cartas, 2)).toBe(1);
		expect(atacantesDe(fs, cartas, 0)).toBe(0);
	});
	it("cada color tiene un id de punta válido para SVG", () => {
		expect(idDePunta("#e11d48")).toBe("punta-e11d48");
		expect(idDePunta("rgb(1, 2, 3)")).toBe("punta-rgb123");
	});
});
