import { describe, expect, it } from "vitest";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CardDesign } from "@/bindings/CardDesign";
import { claveCapa, geometriaArte, moverEncuadre } from "../encuadre";

const neutro: ArtFraming = { zoom: 1, offsetX: 0, offsetY: 0, rotation: 0 };

describe("moverEncuadre", () => {
	// Un hueco de 57 × 42 mm y una imagen de 4:3.
	const hueco = { w: 57, h: 42, iw: 1200, ih: 900 };
	/** Dónde queda el centro de la imagen en el hueco (mm). */
	const centro = (f: ArtFraming) => {
		const g = geometriaArte(f, hueco.iw, hueco.ih, hueco.w, hueco.h);
		const rw = (g.ancho / 100) * hueco.w;
		const rh = (g.alto / 100) * hueco.h;
		return { x: (g.tx / 100) * rw + rw / 2, y: (g.ty / 100) * rh + rh / 2 };
	};

	it("la imagen sigue al ratón 1:1, con cualquier zoom o giro", () => {
		for (const f of [
			{ ...neutro, zoom: 1.2 },
			{ ...neutro, zoom: 2.5, offsetX: 0.2, offsetY: -0.3 },
			{ ...neutro, zoom: 1.6, rotation: 20 },
		]) {
			// 2 mm a la derecha y 1 mm abajo.
			const g = moverEncuadre(f, { dx: 2 / 63, dy: 1 / 88 }, hueco);
			const antes = centro(f);
			const despues = centro(g);
			expect(despues.x - antes.x).toBeCloseTo(2, 6);
			expect(despues.y - antes.y).toBeCloseTo(1, 6);
		}
	});

	it("sin holgura no se mueve, y no se sale de [-1, 1]", () => {
		// Zoom 1 con una imagen 4:3 en un hueco más ancho: a lo ancho no sobra nada.
		expect(moverEncuadre(neutro, { dx: 0.1 }, hueco).offsetX).toBe(0);
		expect(moverEncuadre(neutro, { dy: -5 }, hueco).offsetY).toBe(1);
		expect(moverEncuadre(neutro, { dy: 5 }, hueco).offsetY).toBe(-1);
	});

	it("el zoom va de 1 a 3", () => {
		expect(moverEncuadre(neutro, { zoom: -0.5 }, hueco).zoom).toBe(1);
		expect(moverEncuadre({ ...neutro, zoom: 2.99 }, { zoom: 0.04 }, hueco).zoom).toBe(3);
	});
});

describe("claveCapa", () => {
	const d = {
		faces: [
			{
				art: { image: { blob: "b" }, source: { type: "upload" }, framing: neutro },
				texts: { es: { name: "Uno", flavor: "" }, en: { name: "One", flavor: "" } },
			},
		],
	} as unknown as CardDesign;

	it("no cambia con el encuadre y sí con lo demás", () => {
		const movida = {
			...d,
			faces: d.faces.map((f) => ({
				...f,
				art: f.art ? { ...f.art, framing: { ...neutro, offsetX: 0.4, zoom: 2 } } : f.art,
			})),
		};
		expect(claveCapa(movida, 0, "es")).toBe(claveCapa(d, 0, "es"));
		const otroNombre = {
			...d,
			faces: d.faces.map((f) => ({ ...f, texts: { ...f.texts, es: { name: "Dos", flavor: "" } } })),
		};
		expect(claveCapa(otroNombre, 0, "es")).not.toBe(claveCapa(d, 0, "es"));
		expect(claveCapa(d, 0, "en")).not.toBe(claveCapa(d, 0, "es"));
		expect(claveCapa(d, 1, "es")).not.toBe(claveCapa(d, 0, "es"));
	});
});

describe("geometriaArte", () => {
	// Un hueco de 2×1 y una imagen de 4×1: cubre a lo alto y sobra a lo ancho.
	it("sin giro: centrada, y el desplazamiento llega justo a cada borde", () => {
		const g = geometriaArte(neutro, 4, 1, 2, 1);
		expect(g).toMatchObject({ ancho: 200, alto: 100, tx: -25, ty: 0, giro: 0 });
		// -1 enseña el borde izquierdo; 1, el derecho.
		expect(geometriaArte({ ...neutro, offsetX: -1 }, 4, 1, 2, 1).tx).toBe(0);
		expect(geometriaArte({ ...neutro, offsetX: 1 }, 4, 1, 2, 1).tx).toBe(-50);
		// A lo alto no sobra nada: no se mueve.
		expect(geometriaArte({ ...neutro, offsetY: 1 }, 4, 1, 2, 1).ty).toBe(0);
	});

	it("la proporción de la imagen es lo que cuenta, no su tamaño", () => {
		const a = geometriaArte({ ...neutro, zoom: 1.7, offsetX: 0.3 }, 4, 1, 2, 1);
		const b = geometriaArte({ ...neutro, zoom: 1.7, offsetX: 0.3 }, 4000, 1000, 2, 1);
		expect(b.ancho).toBeCloseTo(a.ancho);
		expect(b.tx).toBeCloseTo(a.tx);
	});

	it("girada 90°, su caja cubre el hueco y queda centrada", () => {
		const g = geometriaArte({ ...neutro, rotation: 90 }, 2, 1, 2, 1);
		// Girada, la imagen (4×2) ocupa 2×4: cubre el hueco de 2×1.
		expect(g.ancho).toBeCloseTo(200);
		expect(g.alto).toBeCloseTo(200);
		expect(g.tx).toBeCloseTo(-25);
		expect(g.ty).toBeCloseTo(-25);
		expect(g.giro).toBe(90);
	});
});
