import { describe, expect, it } from "vitest";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { filtrar } from "../dialogos/buscar";

function carta(id: number, nombre: string, tipo: string, colores: string[]): VistaCarta {
	return {
		id,
		origen: {
			cartaDeck: "",
			nombre,
			caras: [],
			dorso: { tipo: "generica" },
			scryfallId: null,
			info: {
				tipo,
				cmcDecimas: 0,
				coste: null,
				colores,
				esTierra: tipo.includes("Land"),
				esCriatura: tipo.includes("Creature"),
				esLegendaria: false,
				fuerza: null,
				resistencia: null,
				lealtad: null,
				palabrasClave: [],
				funciones: [],
				bonoFijo: null,
				esComandante: false,
			},
		},
	} as unknown as VistaCarta;
}

const cartas = [
	carta(1, "Forest", "Basic Land — Forest", []),
	carta(2, "Llanowar Elves", "Creature — Elf Druid", ["G"]),
	carta(3, "Lightning Bolt", "Instant", ["R"]),
	carta(4, "Sol Ring", "Artifact", []),
];

describe("buscador de la biblioteca", () => {
	it("por nombre o tipo, sin mayúsculas", () => {
		expect(filtrar(cartas, { texto: "bolt", tipo: "todo", color: "" }).map((c) => c.id)).toEqual([
			3,
		]);
		expect(filtrar(cartas, { texto: "elf", tipo: "todo", color: "" }).map((c) => c.id)).toEqual([
			2,
		]);
	});
	it("por tipo y color", () => {
		expect(filtrar(cartas, { texto: "", tipo: "tierra", color: "" }).map((c) => c.id)).toEqual([1]);
		expect(filtrar(cartas, { texto: "", tipo: "noTierra", color: "" }).map((c) => c.id)).toEqual([
			2, 3, 4,
		]);
		expect(filtrar(cartas, { texto: "", tipo: "todo", color: "G" }).map((c) => c.id)).toEqual([2]);
		expect(filtrar(cartas, { texto: "", tipo: "todo", color: "C" }).map((c) => c.id)).toEqual([
			1, 4,
		]);
	});
	it("las boca abajo que no se conocen no salen", () => {
		const oculta = { ...cartas[0], origen: null } as unknown as VistaCarta;
		expect(filtrar([oculta], { texto: "", tipo: "todo", color: "" })).toEqual([]);
	});
});
