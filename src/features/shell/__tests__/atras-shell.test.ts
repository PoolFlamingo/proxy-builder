import { describe, expect, it, vi } from "vitest";
import { crearSalidaConDobleToque, destinoDeAtras } from "../atras-shell";

describe("crearSalidaConDobleToque", () => {
	const montar = (ventana?: number) => {
		let reloj = 1000;
		const avisar = vi.fn();
		const salir = vi.fn();
		const atras = crearSalidaConDobleToque({ ahora: () => reloj, avisar, salir, ventana });
		return { atras, avisar, salir, pasar: (ms: number) => (reloj += ms) };
	};

	it("la primera pulsación avisa y no sale", () => {
		const { atras, avisar, salir } = montar();
		atras();
		expect(avisar).toHaveBeenCalledTimes(1);
		expect(salir).not.toHaveBeenCalled();
	});

	it("la segunda dentro de la ventana sale", () => {
		const { atras, avisar, salir, pasar } = montar();
		atras();
		pasar(1500);
		atras();
		expect(salir).toHaveBeenCalledTimes(1);
		expect(avisar).toHaveBeenCalledTimes(1);
	});

	it("pasada la ventana vuelve a avisar en vez de salir", () => {
		const { atras, avisar, salir, pasar } = montar(2000);
		atras();
		pasar(2500);
		atras();
		expect(salir).not.toHaveBeenCalled();
		expect(avisar).toHaveBeenCalledTimes(2);
	});

	it("tras salir, la cuenta empieza de cero", () => {
		const { atras, avisar, salir, pasar } = montar();
		atras();
		pasar(100);
		atras();
		pasar(100);
		atras();
		expect(salir).toHaveBeenCalledTimes(1);
		expect(avisar).toHaveBeenCalledTimes(2);
	});
});

describe("destinoDeAtras", () => {
	it("solo Diseñar vuelve a Deck", () => {
		expect(destinoDeAtras("disenar")).toBe("deck");
		expect(destinoDeAtras("deck")).toBeNull();
		expect(destinoDeAtras("biblioteca")).toBeNull();
		expect(destinoDeAtras("mesa")).toBeNull();
	});
});
