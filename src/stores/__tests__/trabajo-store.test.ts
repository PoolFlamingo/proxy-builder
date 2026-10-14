import { beforeEach, describe, expect, it } from "vitest";
import { useTrabajo } from "@/stores/trabajo-store";

describe("store del trabajo en marcha", () => {
	beforeEach(() => useTrabajo.getState().terminar());

	it("empieza sin id y a cero", () => {
		useTrabajo.getState().empezar("resolver", "Buscando cartas…");
		expect(useTrabajo.getState().actual).toEqual({
			tipo: "resolver",
			jobId: null,
			fase: "Buscando cartas…",
			hechas: 0,
			total: 0,
		});
	});

	it("guarda el id y el avance sin perder lo demás", () => {
		const s = useTrabajo.getState;
		s().empezar("exportar", "Preparando");
		s().setId(42);
		s().avanzar("Pintando", 3, 10);
		expect(s().actual).toEqual({
			tipo: "exportar",
			jobId: 42,
			fase: "Pintando",
			hechas: 3,
			total: 10,
		});
		s().terminar();
		expect(s().actual).toBeNull();
	});

	it("sin trabajo, el id y el avance no crean uno", () => {
		useTrabajo.getState().setId(1);
		useTrabajo.getState().avanzar("x", 1, 2);
		expect(useTrabajo.getState().actual).toBeNull();
	});

	it("empezar otro trabajo sustituye al anterior", () => {
		const s = useTrabajo.getState;
		s().empezar("resolver", "a");
		s().setId(5);
		s().empezar("exportar", "b");
		expect(s().actual?.jobId).toBeNull();
		expect(s().actual?.tipo).toBe("exportar");
	});
});
