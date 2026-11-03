import { describe, expect, it, vi } from "vitest";
import { volverConEscape } from "../seccion-movil";

describe("volverConEscape", () => {
	it("con una sección abierta, vuelve a la lista y frena el cierre del diálogo", () => {
		const volver = vi.fn();
		const preventDefault = vi.fn();
		volverConEscape(true, volver)({ preventDefault });
		expect(volver).toHaveBeenCalledTimes(1);
		expect(preventDefault).toHaveBeenCalledTimes(1);
	});

	it("sin sección abierta, deja que el diálogo se cierre", () => {
		const volver = vi.fn();
		const preventDefault = vi.fn();
		volverConEscape(false, volver)({ preventDefault });
		expect(volver).not.toHaveBeenCalled();
		expect(preventDefault).not.toHaveBeenCalled();
	});
});
