import { describe, expect, it } from "vitest";
import { esArchienemigo, planeswalkPendiente } from "../planar";

const dado = (cara: "planeswalk" | "caos" | "nada") =>
	({ cambio: { tipo: "dadoPlanar", cara, coste: 0, semillaAnimacion: 1 } }) as never;
const walk = { cambio: { tipo: "accion", accion: { que: "planeswalk" } } } as never;
const otra = { cambio: { tipo: "accion", accion: { que: "pausar", si: true } } } as never;

describe("planeswalkPendiente", () => {
	it("sin tiradas, no", () => {
		expect(planeswalkPendiente([])).toBe(false);
		expect(planeswalkPendiente([otra])).toBe(false);
	});
	it("tras una tirada con Planeswalk, sí (aunque pasen otras cosas)", () => {
		expect(planeswalkPendiente([dado("planeswalk"), otra])).toBe(true);
	});
	it("tras caos o nada, no", () => {
		expect(planeswalkPendiente([dado("caos")])).toBe(false);
		expect(planeswalkPendiente([dado("nada")])).toBe(false);
	});
	it("ya hecho el Planeswalk, no", () => {
		expect(planeswalkPendiente([dado("planeswalk"), walk])).toBe(false);
	});
	it("manda la última tirada", () => {
		expect(planeswalkPendiente([dado("planeswalk"), walk, dado("caos")])).toBe(false);
		expect(planeswalkPendiente([dado("caos"), dado("planeswalk")])).toBe(true);
	});
});

describe("esArchienemigo", () => {
	it("solo el elegido", () => {
		expect(esArchienemigo(1, 1)).toBe(true);
		expect(esArchienemigo(1, 0)).toBe(false);
		expect(esArchienemigo(null, 0)).toBe(false);
		expect(esArchienemigo(0, null)).toBe(false);
	});
});
