import { beforeEach, describe, expect, it } from "vitest";
import type { Evento } from "@/bindings/Evento";
import type { ParcheVista } from "@/bindings/ParcheVista";
import type { VistaMesa } from "@/bindings/VistaMesa";
import { conParche, desdeVista, useMesa } from "@/stores/mesa-store";
import datos from "./fixtures/mesa-parches.json";

// Una partida de ejemplo jugada en Rust (`crates/mesa/tests/fixtures.rs`):
// la primera vista, los parches de 60 acciones y la vista final.
const { inicial, parches, final } = datos as unknown as {
	inicial: VistaMesa;
	parches: ParcheVista[];
	final: VistaMesa;
};

const evento = (seq: number): Evento => ({
	seq,
	actor: { tipo: "mesa" },
	tMs: 0,
	cambio: { tipo: "accion", accion: { que: "barajar" } },
});

describe("mesa-store", () => {
	beforeEach(() => useMesa.getState().cerrar());

	it("los parches sobre la primera vista dan la vista final", () => {
		let m = desdeVista(inicial);
		for (const p of parches) {
			const siguiente = conParche(m, p);
			expect(siguiente).not.toBeNull();
			if (siguiente) m = siguiente;
		}
		expect(m).toEqual(desdeVista(final));
	});

	it("un salto de versión pide la vista entera", () => {
		const m = desdeVista(inicial);
		const p = parches[1];
		expect(conParche(m, p)).toBeNull();
		useMesa.getState().recibirVista(inicial);
		expect(useMesa.getState().recibirParche(p, evento(p.version))).toBe(false);
		expect(useMesa.getState().recibirParche(parches[0], evento(parches[0].version))).toBe(true);
		expect(useMesa.getState().registro).toHaveLength(1);
	});

	it("la vista con registro (deshacer, reiniciar) reemplaza el registro", () => {
		useMesa.getState().recibirVista(inicial);
		useMesa.getState().recibirParche(parches[0], evento(1));
		expect(useMesa.getState().registro).toHaveLength(1);
		// Sin registro (un salto de versión) se queda el que había.
		useMesa.getState().recibirVista(inicial);
		expect(useMesa.getState().registro).toHaveLength(1);
		useMesa.getState().recibirVista(inicial, []);
		expect(useMesa.getState().registro).toHaveLength(0);
	});

	it("una carta que no se tocó sigue siendo el mismo objeto", () => {
		const m = desdeVista(inicial);
		const p = parches[0];
		const siguiente = conParche(m, p);
		const tocadas = new Set([...p.cartas.map((c) => c.id), ...p.quitadas]);
		const intacta = Object.keys(m.cartas)
			.map(Number)
			.find((id) => !tocadas.has(id));
		if (intacta !== undefined && siguiente) {
			expect(siguiente.cartas[intacta]).toBe(m.cartas[intacta]);
		}
	});

	it("la selección pierde las cartas que desaparecen", () => {
		useMesa.getState().recibirVista(inicial);
		const p = parches.find((x) => x.quitadas.length > 0 && x.version === inicial.version + 1);
		if (!p) return;
		useMesa.getState().seleccionar([p.quitadas[0]]);
		useMesa.getState().recibirParche(p, evento(p.version));
		expect(useMesa.getState().seleccion).toEqual([]);
	});
});
