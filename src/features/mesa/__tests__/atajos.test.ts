import { describe, expect, it } from "vitest";
import { ATAJOS, atajoDe, teclaDe } from "../atajos";

describe("atajos de la mesa", () => {
	it("ninguna tecla se repite en el mismo contexto", () => {
		const vistas = new Set<string>();
		for (const a of ATAJOS) {
			const k = `${a.contexto}|${a.tecla}`;
			expect(vistas.has(k), k).toBe(false);
			vistas.add(k);
		}
	});

	it("la tecla de un evento se escribe como en la tabla", () => {
		expect(teclaDe({ key: "T", ctrlKey: false, metaKey: false, shiftKey: false })).toBe("t");
		expect(teclaDe({ key: "E", ctrlKey: false, metaKey: false, shiftKey: true })).toBe("Mayús+E");
		expect(teclaDe({ key: "z", ctrlKey: true, metaKey: false, shiftKey: false })).toBe("Ctrl+z");
		expect(teclaDe({ key: "Escape", ctrlKey: false, metaKey: false, shiftKey: false })).toBe(
			"Escape",
		);
	});

	it("la carta gana a la biblioteca y la biblioteca a la mesa", () => {
		expect(atajoDe("t", 3, false)).not.toBeNull();
		expect(atajoDe("d", null, true)).not.toBeNull();
		expect(atajoDe("u", null, false)).not.toBeNull();
		expect(atajoDe("q", null, false)).toBeNull();
		expect(atajoDe("5", null, true)).not.toBeNull();
		expect(atajoDe("5", null, false)).toBeNull();
	});
});
