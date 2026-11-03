import { afterEach, describe, expect, it } from "vitest";
import { useMesa } from "@/stores/mesa-store";
import {
	anchoDeLectura,
	hayEncima,
	lecturaMovil,
	manejadoresLeer,
	sobreTras,
} from "../leer-en-grande";

const evento = (pointerType: string) => ({ pointerType }) as never;

describe("leer una carta en grande", () => {
	afterEach(() => useMesa.setState({ sobre: null }));

	it("con el ratón y el lápiz hay «encima»; con el dedo no", () => {
		expect(hayEncima("mouse")).toBe(true);
		expect(hayEncima("pen")).toBe(true);
		expect(hayEncima("touch")).toBe(false);
	});

	it("con el ratón, entrar la pone y salir la quita (solo si sigue siendo ella)", () => {
		expect(sobreTras(null, 5, "entra", "mouse")).toBe(5);
		expect(sobreTras(5, 5, "sale", "mouse")).toBeNull();
		// Ya se pasó a otra: salir de la primera no la quita.
		expect(sobreTras(7, 5, "sale", "mouse")).toBeUndefined();
		// Un clic no cambia nada.
		expect(sobreTras(null, 5, "toque", "mouse")).toBeUndefined();
	});

	it("con el dedo, solo cuenta el toque; entrar y salir (bajar y subir el dedo) no", () => {
		expect(sobreTras(null, 5, "entra", "touch")).toBeUndefined();
		expect(sobreTras(5, 5, "sale", "touch")).toBeUndefined();
		expect(sobreTras(null, 5, "toque", "touch")).toBe(5);
		// Tocar otra la cambia; tocar la misma la deja.
		expect(sobreTras(5, 8, "toque", "touch")).toBe(8);
		expect(sobreTras(5, 5, "toque", "touch")).toBe(5);
	});

	it("los manejadores escriben en el store", () => {
		const m = manejadoresLeer(3);
		m.onPointerEnter(evento("mouse"));
		expect(useMesa.getState().sobre).toBe(3);
		m.onPointerLeave(evento("mouse"));
		expect(useMesa.getState().sobre).toBeNull();
		m.onPointerEnter(evento("touch"));
		m.onPointerUp(evento("touch"));
		expect(useMesa.getState().sobre).toBe(3);
		m.onPointerLeave(evento("touch"));
		expect(useMesa.getState().sobre).toBe(3);
	});

	it("sin toque propio, soltar el dedo no la lee (lo hace el gesto de la carta)", () => {
		const m = manejadoresLeer(3, false);
		m.onPointerUp(evento("touch"));
		expect(useMesa.getState().sobre).toBeNull();
		expect(m["data-lee"]).toBe("");
	});
});

describe("el ancho de la vista grande", () => {
	it("el de Ajustes si cabe de arriba abajo; si no, el que cabe", () => {
		expect(anchoDeLectura(320, 1080)).toBe(320);
		// Un móvil en horizontal: 411 de alto.
		expect(anchoDeLectura(320, 411)).toBe(248);
		// Con el panel de estado debajo, menos.
		expect(anchoDeLectura(320, 411, 120)).toBe(162);
		expect(anchoDeLectura(320, 100)).toBe(120);
	});
});

describe("la carta en grande en un móvil", () => {
	const dentro = (
		r: { ancho: number; alto: number },
		w: number,
		h: number,
		caras: number,
		horizontal: boolean,
	) => {
		expect(Math.abs(r.alto / r.ancho - 88 / 63)).toBeLessThan(0.02);
		if (horizontal) {
			expect(r.alto).toBeLessThanOrEqual(h - 16);
			expect(caras * r.ancho + (caras - 1) * 8 + 48 + 16).toBeLessThanOrEqual(w);
		} else {
			expect(caras * r.ancho + (caras - 1) * 8).toBeLessThanOrEqual(w - 16);
			expect(r.alto + 48 + 16).toBeLessThanOrEqual(h);
		}
	};

	it("de lado ocupa el alto y deja sitio a la X al lado", () => {
		const r = lecturaMovil(873, 392, true, 1, false);
		expect(r.alto).toBeGreaterThan(360);
		dentro(r, 873, 392, 1, true);
	});

	it("de lado, con dos caras y panel, se encoge para caber a lo ancho", () => {
		const r = lecturaMovil(640, 360, true, 2, true);
		dentro(r, 640 - 184, 360, 2, true);
	});

	it("de pie ocupa el ancho y deja la X debajo", () => {
		const r = lecturaMovil(392, 873, false, 1, true);
		expect(r.ancho).toBe(376);
		dentro(r, 392, 873, 1, false);
		dentro(lecturaMovil(392, 600, false, 2, false), 392, 600, 2, false);
		// Muy baja: manda el alto.
		const bajo = lecturaMovil(392, 420, false, 1, true);
		expect(bajo.alto).toBeLessThanOrEqual(420 - 16 - 48 - 120);
	});
});
