import { beforeEach, describe, expect, it } from "vitest";
import { MS_CARTA, MS_CURSOR, MS_SENAL, useVivo } from "../vivo-store";

const base = { tablero: 0, x: 10, y: 20 };

describe("vivo-store", () => {
	beforeEach(() => useVivo.getState().limpiar());

	it("cada jugador tiene un solo fantasma, el último", () => {
		const v = useVivo.getState();
		v.fantasma(1, { ...base, carta: 5 }, 100);
		v.fantasma(1, { ...base, carta: 5, x: 99 }, 120);
		expect(Object.keys(useVivo.getState().fantasmas)).toEqual(["1"]);
		expect(useVivo.getState().fantasmas[1]?.x).toBe(99);
	});

	it("una carta arrastrada caduca antes que un cursor quieto", () => {
		const v = useVivo.getState();
		v.fantasma(1, { ...base, carta: 5 }, 0);
		v.fantasma(2, { ...base, carta: null }, 0);
		v.purgar(MS_CARTA + 1);
		expect(Object.keys(useVivo.getState().fantasmas)).toEqual(["2"]);
		v.purgar(MS_CURSOR + 1);
		expect(useVivo.getState().fantasmas).toEqual({});
	});

	it("el aro de un aviso dura dos segundos", () => {
		const v = useVivo.getState();
		v.senal(1, 7, null, 1000);
		expect(useVivo.getState().senales).toHaveLength(1);
		v.purgar(1000 + MS_SENAL - 1);
		expect(useVivo.getState().senales).toHaveLength(1);
		v.purgar(1000 + MS_SENAL + 1);
		expect(useVivo.getState().senales).toHaveLength(0);
	});

	it("purgar sin nada que quitar no cambia el estado (no repinta)", () => {
		const v = useVivo.getState();
		v.fantasma(1, { ...base, carta: null }, 0);
		const antes = useVivo.getState().fantasmas;
		v.purgar(10);
		expect(useVivo.getState().fantasmas).toBe(antes);
	});
});
