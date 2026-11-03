import { describe, expect, it } from "vitest";
import { calidadVistaDeFabrica } from "../ui-store";

describe("calidadVistaDeFabrica", () => {
	it("en móvil y tableta, la miniatura pequeña; en escritorio, la de siempre", () => {
		expect(calidadVistaDeFabrica(true, false)).toBe(244);
		expect(calidadVistaDeFabrica(false, true)).toBe(488);
		// Un portátil con pantalla táctil sigue siendo escritorio.
		expect(calidadVistaDeFabrica(true, true)).toBe(488);
		// Una ventana de escritorio estrecha, con ratón, no baja la calidad.
		expect(calidadVistaDeFabrica(false, false)).toBe(488);
	});
});
