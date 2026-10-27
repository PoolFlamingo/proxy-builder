import { describe, expect, it } from "vitest";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { bonoDeContadores, ptDe } from "../pt";

function carta(fuerza: string | null, cambios: Partial<VistaCarta> = {}): VistaCarta {
	return {
		id: 1,
		zona: { tipo: "de", jugador: 0, zona: "campo" },
		dueno: 0,
		controlador: 0,
		origen: {
			cartaDeck: "",
			nombre: "Oso",
			caras: [],
			dorso: { tipo: "generica" },
			scryfallId: null,
			info: {
				tipo: "Creature",
				cmcDecimas: 20,
				coste: null,
				colores: [],
				esTierra: false,
				esCriatura: true,
				esLegendaria: false,
				fuerza,
				resistencia: fuerza,
				lealtad: null,
				palabrasClave: [],
				funciones: [],
				bonoFijo: null,
				esComandante: false,
			},
		},
		dorso: { tipo: "generica" },
		cara: 0,
		girada: false,
		bocaAbajo: false,
		faseFuera: false,
		noEndereza: false,
		contadores: {},
		palabrasClave: [],
		etiquetas: [],
		pt: { otros: [0, 0], hastaFinDeTurno: [0, 0], fijo: null },
		adjuntaA: null,
		pos: null,
		z: 0,
		resaltada: null,
		esFicha: false,
		copiaDe: null,
		cantidad: 1,
		agrupadaBajo: null,
		llegoEnRonda: 0,
		habilidad: null,
		mirada: false,
		revelada: false,
		...cambios,
	};
}

describe("contadores de fuerza y resistencia", () => {
	it("suma cualquier +X/+Y por su cantidad y se salta los demás", () => {
		expect(bonoDeContadores({ "+2/+2": 2, "-0/-1": 3, "+1/+0": 1, oil: 4 })).toEqual([5, 1]);
		expect(bonoDeContadores({ "-1/-1": 1 })).toEqual([-1, -1]);
		expect(bonoDeContadores({ "-0/-0": 2, stun: 1 })).toEqual([0, 0]);
	});
	it("ptDe los usa", () => {
		expect(ptDe(carta("1", { contadores: { "+2/+2": 1, "+0/+1": 2 } }))).toEqual({
			f: 3,
			r: 5,
			cambiada: true,
		});
	});
});

describe("fuerza y resistencia", () => {
	it("la impresa, sin cambios", () => {
		expect(ptDe(carta("2"))).toEqual({ f: 2, r: 2, cambiada: false });
		expect(ptDe(carta(null))).toBeNull();
	});
	it("contadores y filas suman; la fija tapa", () => {
		const c = carta("2", {
			contadores: { "+1/+1": 3, "-1/-1": 1 },
			pt: { otros: [1, 0], hastaFinDeTurno: [0, 2], fijo: null },
		});
		expect(ptDe(c)).toEqual({ f: 5, r: 6, cambiada: true });
		expect(ptDe({ ...c, pt: { ...c.pt, fijo: [0, 1] } })).toEqual({ f: 0, r: 1, cambiada: true });
	});
});
