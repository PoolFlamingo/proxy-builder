import { describe, expect, it } from "vitest";
import {
	claveCausa,
	DANO_MINIMO,
	debePreguntarDano,
	eliminadoPorDefecto,
	segundosAusente,
	siguienteVivo,
	vidaPerdida,
} from "../cuentas-muerte";

const base = {
	jugador: 0,
	activo: 1,
	ultimoDano: undefined,
	vivos: [0, 1, 2],
} as const;

describe("eliminadoPorDefecto", () => {
	it("por comandante, el dueño del comandante", () => {
		expect(eliminadoPorDefecto({ ...base, causa: { tipo: "comandante", de: 2, carta: 9 } })).toBe(
			2,
		);
	});
	it("antes que el del turno, a quien se dijo que hizo el daño", () => {
		expect(eliminadoPorDefecto({ ...base, causa: { tipo: "vida" }, ultimoDano: 2 })).toBe(2);
	});
	it("si no, el jugador del turno", () => {
		expect(eliminadoPorDefecto({ ...base, causa: { tipo: "veneno" } })).toBe(1);
	});
	it("en el propio turno o con el activo fuera, nadie en concreto", () => {
		expect(eliminadoPorDefecto({ ...base, activo: 0, causa: { tipo: "vida" } })).toBeNull();
		expect(eliminadoPorDefecto({ ...base, vivos: [0, 2], causa: { tipo: "vida" } })).toBeNull();
	});
	it("no se apunta a uno mismo aunque lo diga el comandante", () => {
		expect(eliminadoPorDefecto({ ...base, causa: { tipo: "comandante", de: 0, carta: 3 } })).toBe(
			1,
		);
	});
});

describe("debePreguntarDano", () => {
	const ok = {
		total: DANO_MINIMO,
		jugador: 0,
		activo: 1,
		vivos: [0, 1, 2],
		controlado: true,
		terminada: false,
	};
	it("pregunta con 5 o más en el turno de otro", () => {
		expect(debePreguntarDano(ok)).toBe(true);
		expect(debePreguntarDano({ ...ok, total: DANO_MINIMO - 1 })).toBe(false);
	});
	it("no en el turno propio, ni si el activo está fuera, ni terminada la partida", () => {
		expect(debePreguntarDano({ ...ok, activo: 0 })).toBe(false);
		expect(debePreguntarDano({ ...ok, vivos: [0, 2] })).toBe(false);
		expect(debePreguntarDano({ ...ok, terminada: true })).toBe(false);
	});
	it("solo a quien se puede contestar por él, y si sigue vivo", () => {
		expect(debePreguntarDano({ ...ok, controlado: false })).toBe(false);
		expect(debePreguntarDano({ ...ok, vivos: [1, 2] })).toBe(false);
	});
});

describe("cuentas pequeñas", () => {
	it("la vida perdida no es negativa", () => {
		expect(vidaPerdida(40, 33)).toBe(7);
		expect(vidaPerdida(33, 40)).toBe(0);
	});
	it("cambiar de comandante cambia la clave", () => {
		expect(claveCausa({ tipo: "comandante", de: 1, carta: 4 })).not.toBe(
			claveCausa({ tipo: "comandante", de: 1, carta: 5 }),
		);
		expect(claveCausa({ tipo: "vida" })).toBe("vida");
	});
});

describe("saltar el turno de un ausente", () => {
	it("el siguiente es el siguiente asiento que sigue en pie", () => {
		expect(siguienteVivo([0, 1, 2, 3], [0, 1, 2, 3], 1)).toBe(2);
		expect(siguienteVivo([0, 1, 2, 3], [0, 1, 3], 1)).toBe(3);
		expect(siguienteVivo([0, 1, 2, 3], [0, 1, 3], 3)).toBe(0);
	});
	it("solo, o sin nadie más, no hay siguiente", () => {
		expect(siguienteVivo([0, 1], [1], 1)).toBeNull();
		expect(siguienteVivo([0, 1], [0, 1], 7)).toBeNull();
	});
	it("los segundos cuentan lo que ya llevaba y lo que pasó desde que se supo", () => {
		expect(segundosAusente(12_400, 1_000, 6_000)).toBe(17);
		expect(segundosAusente(0, 5_000, 4_000)).toBe(0);
	});
});
