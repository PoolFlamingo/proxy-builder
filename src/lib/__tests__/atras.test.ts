import { describe, expect, it, vi } from "vitest";
import { crearAtras } from "@/lib/atras";

function montar(capa = false) {
	const e = { capaAbierta: vi.fn(() => capa), cerrarCapa: vi.fn() };
	return { e, a: crearAtras(e) };
}

describe("crearAtras", () => {
	it("una capa abierta se cierra antes que nada", () => {
		const { e, a } = montar(true);
		const m = vi.fn();
		a.registrar(m);
		expect(a.atras()).toBe("capa");
		expect(e.cerrarCapa).toHaveBeenCalled();
		expect(m).not.toHaveBeenCalled();
	});

	it("el último registrado va primero y quita lo suyo al soltarse", () => {
		const { a } = montar();
		const orden: string[] = [];
		a.registrar(() => {
			orden.push("a");
			return true;
		});
		const quitar = a.registrar(() => {
			orden.push("b");
			return true;
		});
		a.atras();
		quitar();
		a.atras();
		expect(orden).toEqual(["b", "a"]);
	});

	it("un manejador que no puede pasa al siguiente y luego al de por defecto", () => {
		const { a } = montar();
		const defecto = vi.fn();
		a.fijarPorDefecto(defecto);
		a.registrar(() => false);
		expect(a.atras()).toBe("defecto");
		expect(defecto).toHaveBeenCalled();
	});

	it("sin nada, nadie", () => {
		expect(montar().a.atras()).toBe("nadie");
	});
});
