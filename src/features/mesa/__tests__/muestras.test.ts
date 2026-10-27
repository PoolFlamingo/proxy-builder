import { describe, expect, it } from "vitest";
import { bytesDe, existeMuestra, MUESTRAS } from "../muestras";

describe("muestras de sonido", () => {
	it("cada muestra que se pide tiene su fichero", () => {
		for (const [grupo, nombres] of Object.entries(MUESTRAS)) {
			for (const n of nombres) expect(existeMuestra(n), `${grupo}: ${n}`).toBe(true);
		}
	});
	it("decodifica un data URL en base64", () => {
		const bytes = new Uint8Array(bytesDe("data:audio/wav;base64,UklGRg=="));
		expect([...bytes]).toEqual([0x52, 0x49, 0x46, 0x46]);
	});
});
