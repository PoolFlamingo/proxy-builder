import { beforeEach, describe, expect, it } from "vitest";
import {
	cartasDeFantasma,
	enVuelo,
	MS_CARTA,
	MS_CURSOR,
	MS_SENAL,
	MS_SOLTADA,
	type Origen,
	origenDe,
	soltadaSigue,
	useVivo,
} from "../vivo-store";

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

	describe("lo que está en vuelo", () => {
		const sitio = (x: number): Origen => ({ zona: "campo", x, y: 0 });

		it("una carta que otro arrastra (la primera o las de más) está en vuelo", () => {
			useVivo.getState().fantasma(1, { ...base, carta: 5, mas: [6, 7] }, 0);
			const s = useVivo.getState();
			expect(cartasDeFantasma(s.fantasmas[1])).toEqual([5, 6, 7]);
			for (const id of [5, 6, 7]) expect(enVuelo(s, id, () => sitio(0))).toBe(true);
			expect(enVuelo(s, 8, () => sitio(0))).toBe(false);
		});

		it("al soltar se sigue ocultando en su sitio de antes, y deja de estarlo al moverse", () => {
			const v = useVivo.getState();
			v.fantasma(1, { ...base, carta: 5, mas: [6] }, 0, { 5: sitio(1), 6: sitio(2) });
			// Suelta: solo queda el cursor.
			v.fantasma(1, { ...base, carta: null }, 50);
			const s = useVivo.getState();
			expect(enVuelo(s, 5, () => sitio(1))).toBe(true);
			expect(enVuelo(s, 6, () => sitio(2))).toBe(true);
			// Llegó el movimiento: ya está en otro sitio.
			expect(enVuelo(s, 5, () => sitio(900))).toBe(false);
		});

		it("lo soltado caduca si el movimiento no llega", () => {
			const v = useVivo.getState();
			v.fantasma(1, { ...base, carta: 5 }, 0, { 5: sitio(1) });
			v.fantasma(1, { ...base, carta: null }, 10);
			v.purgar(10 + MS_SOLTADA - 1);
			expect(enVuelo(useVivo.getState(), 5, () => sitio(1))).toBe(true);
			v.purgar(10 + MS_SOLTADA + 1);
			expect(enVuelo(useVivo.getState(), 5, () => sitio(1))).toBe(false);
		});

		it("mis cartas recién soltadas también se ocultan hasta que se muevan", () => {
			const v = useVivo.getState();
			v.soltar({ 3: sitio(4) }, 0);
			expect(enVuelo(useVivo.getState(), 3, () => sitio(4))).toBe(true);
			expect(enVuelo(useVivo.getState(), 3, () => sitio(5))).toBe(false);
		});

		it("el sitio de antes es el de la primera vez que se la ve arrastrar", () => {
			const v = useVivo.getState();
			v.fantasma(1, { ...base, carta: 5 }, 0, { 5: sitio(1) });
			v.fantasma(1, { ...base, carta: 5, x: 50 }, 20, { 5: sitio(50) });
			expect(useVivo.getState().fantasmas[1].origenes[5]).toEqual(sitio(1));
		});

		it("lo que suelta otro recuerda dónde lo soltó para pintarlo ahí hasta que llegue", () => {
			const v = useVivo.getState();
			const c = { zona: "campo", pos: { x: 1, y: 0 } };
			v.fantasma(1, { tablero: 2, carta: 5, x: 700, y: 300 }, 0, { 5: origenDe(c) });
			v.fantasma(1, { tablero: 2, carta: null, x: 710, y: 310 }, 40);
			const soltada = useVivo.getState().soltadas[5];
			expect(soltada.donde).toEqual({ tablero: 2, x: 700, y: 300 });
			expect(soltadaSigue(soltada, c)).toBe(true);
			expect(soltadaSigue(soltada, { zona: "campo", pos: { x: 9, y: 9 } })).toBe(false);
		});
	});
});
