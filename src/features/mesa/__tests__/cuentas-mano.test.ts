import { describe, expect, it } from "vitest";
import {
	agruparMano,
	COSTE_MAXIMO,
	cartaEnFila,
	costeDeMano,
	tipoDeMano,
} from "../movil/cuentas-mano";

type Info = {
	tipo: string | null;
	cmcDecimas: number | null;
	esTierra: boolean;
	esCriatura: boolean;
};

function carta(id: number, i: Partial<Info>) {
	const info = { tipo: null, cmcDecimas: 0, esTierra: false, esCriatura: false, ...i };
	return { id, origen: { info } as never };
}

describe("la mano por tipo", () => {
	it("reparte por tipo, en el orden de siempre, y sin grupos vacíos", () => {
		const mano = [
			carta(1, { tipo: "Instant", cmcDecimas: 20 }),
			carta(2, { tipo: "Basic Land — Forest", esTierra: true }),
			carta(3, { tipo: "Creature — Elf", esCriatura: true, cmcDecimas: 30 }),
			carta(4, { tipo: "Sorcery", cmcDecimas: 40 }),
			carta(5, { tipo: "Basic Land — Island", esTierra: true }),
		];
		const g = agruparMano(mano, "tipo");
		expect(g.map((x) => [x.clave, x.ids])).toEqual([
			["tierra", [2, 5]],
			["criatura", [3]],
			["instantaneo", [1]],
			["conjuro", [4]],
		]);
	});

	it("una criatura artefacto es criatura; una de dos caras cuenta por la de delante", () => {
		expect(tipoDeMano(carta(1, { tipo: "Artifact Creature — Golem", esCriatura: true }))).toBe(
			"criatura",
		);
		expect(tipoDeMano(carta(2, { tipo: "Enchantment // Land", esTierra: false }))).toBe(
			"encantamiento",
		);
		expect(tipoDeMano(carta(3, { tipo: "Planeswalker — Jace" }))).toBe("planeswalker");
		expect(tipoDeMano(carta(4, { tipo: "Battle — Siege" }))).toBe("batalla");
		expect(tipoDeMano(carta(5, { tipo: null }))).toBe("otro");
		expect(tipoDeMano({ id: 6, origen: null })).toBe("otro");
	});
});

describe("la mano por coste", () => {
	it("las tierras primero y luego por coste, con tope", () => {
		const mano = [
			carta(1, { tipo: "Instant", cmcDecimas: 20 }),
			carta(2, { tipo: "Land", esTierra: true }),
			carta(3, { tipo: "Creature", esCriatura: true, cmcDecimas: 90 }),
			carta(4, { tipo: "Artifact", cmcDecimas: 0 }),
			carta(5, { tipo: "Sorcery", cmcDecimas: 20 }),
			carta(6, { tipo: "Creature", esCriatura: true, cmcDecimas: 70 }),
		];
		const g = agruparMano(mano, "coste");
		expect(g.map((x) => [x.clave, x.ids])).toEqual([
			["tierras", [2]],
			[0, [4]],
			[2, [1, 5]],
			[COSTE_MAXIMO, [3, 6]],
		]);
	});

	it("el coste redondea y las tierras no tienen", () => {
		expect(costeDeMano(carta(1, { cmcDecimas: 25 }))).toBe(3);
		expect(costeDeMano(carta(1, { esTierra: true }))).toBeNull();
		expect(costeDeMano(carta(1, { cmcDecimas: null }))).toBe(0);
	});

	it("una mano vacía no tiene grupos", () => {
		expect(agruparMano([], "tipo")).toEqual([]);
		expect(agruparMano([], "coste")).toEqual([]);
	});
});

describe("la mano inicial en una fila", () => {
	it("siete caben a lo ancho de un teléfono de lado", () => {
		const { w, h } = cartaEnFila(857, 250, 7);
		expect(7 * w + 6 * 6).toBeLessThanOrEqual(857);
		expect(h).toBeLessThanOrEqual(250);
	});
	it("si el alto es lo que falta, manda el alto", () => {
		const { w, h } = cartaEnFila(2000, 140, 7);
		expect(h).toBeLessThanOrEqual(140);
		expect(w).toBe(Math.floor((140 * 63) / 88));
	});
	it("nunca pasa del máximo ni baja de 24", () => {
		expect(cartaEnFila(5000, 5000, 1).w).toBe(172);
		expect(cartaEnFila(10, 10, 7).w).toBe(24);
	});
});
