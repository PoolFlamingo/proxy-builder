import { describe, expect, it } from "vitest";
import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CardDesign } from "@/bindings/CardDesign";
import {
	arrastreEnLaPieza,
	claveCapa,
	geometriaArte,
	huecoSinGirar,
	moverEncuadre,
} from "../encuadre";

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

	it("el zoom va de 0,1 a 3", () => {
		expect(moverEncuadre(neutro, { zoom: -0.5 }, hueco).zoom).toBeCloseTo(0.5);
		expect(moverEncuadre({ ...neutro, zoom: 0.12 }, { zoom: -0.04 }, hueco).zoom).toBe(0.1);
		expect(moverEncuadre({ ...neutro, zoom: 2.99 }, { zoom: 0.04 }, hueco).zoom).toBe(3);
	});

	it("con zoom por debajo de 1 cabe entera y también sigue al ratón 1:1", () => {
		const f = { ...neutro, zoom: 0.6, offsetX: 0.3 };
		const g = geometriaArte(f, hueco.iw, hueco.ih, hueco.w, hueco.h);
		// Más pequeña que el hueco en los dos lados.
		expect(g.ancho).toBeLessThan(100);
		expect(g.alto).toBeLessThan(100);
		const m = moverEncuadre(f, { dx: 2 / 63, dy: 1 / 88 }, hueco);
		expect(centro(m).x - centro(f).x).toBeCloseTo(2, 6);
		expect(centro(m).y - centro(f).y).toBeCloseTo(1, 6);
		// De -1 a 1 va de un borde del hueco al otro (-1, pegada a la izquierda).
		const izq = geometriaArte({ ...f, offsetX: -1 }, hueco.iw, hueco.ih, hueco.w, hueco.h);
		const der = geometriaArte({ ...f, offsetX: 1 }, hueco.iw, hueco.ih, hueco.w, hueco.h);
		expect(izq.tx).toBeCloseTo(0, 6);
		expect(der.tx).toBeCloseTo(100 * (100 / g.ancho - 1), 6);
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

	it("el encuadre de la otra parte sí cuenta (va pintada en la capa)", () => {
		const dos = { ...d, faces: [d.faces[0], d.faces[0]] } as CardDesign;
		const otraMovida = {
			...dos,
			faces: dos.faces.map((f, i) =>
				i === 0 && f.art ? { ...f, art: { ...f.art, framing: { ...neutro, offsetY: -0.9 } } } : f,
			),
		} as CardDesign;
		// La capa de la parte 2 lleva la ilustración de la 1: cambia.
		expect(claveCapa(otraMovida, 1, "es")).not.toBe(claveCapa(dos, 1, "es"));
		// La de la parte 1, no: esa ilustración es la que se mueve.
		expect(claveCapa(otraMovida, 0, "es")).toBe(claveCapa(dos, 0, "es"));
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

describe("piezas giradas", () => {
	it("el hueco tumbado se ve sin girar con los lados cambiados", () => {
		// Un hueco de 18,9 × 25,2 mm en la carta (0,3 × 0,2864…).
		const arte = [0.1, 0.5, 0.3, 25.2 / 88];
		const recto = huecoSinGirar(arte, 0);
		expect(recto.w).toBeCloseTo(18.9, 6);
		expect(recto.h).toBeCloseTo(25.2, 6);
		const tumbado = huecoSinGirar(arte, 90);
		expect(tumbado.w).toBeCloseTo(25.2, 6);
		expect(tumbado.h).toBeCloseTo(18.9, 6);
		expect(huecoSinGirar(arte, -90).w).toBeCloseTo(25.2, 6);
	});

	it("girada a la derecha, bajar el ratón la lleva hacia el principio de la pieza", () => {
		// 3 mm hacia abajo en la pantalla.
		const c = arrastreEnLaPieza({ dy: 3 / 88 }, 90);
		// En la pieza (lo de arriba queda a la derecha): 3 mm hacia su derecha.
		expect((c.dx ?? 0) * 63).toBeCloseTo(3, 6);
		expect((c.dy ?? 0) * 88).toBeCloseTo(0, 6);
		// 2 mm a la derecha en la pantalla: 2 mm hacia arriba en la pieza.
		const d = arrastreEnLaPieza({ dx: 2 / 63 }, 90);
		expect((d.dy ?? 0) * 88).toBeCloseTo(-2, 6);
	});

	it("girada a la izquierda, al revés; sin giro, igual; la rueda no se toca", () => {
		const c = arrastreEnLaPieza({ dx: 2 / 63, dy: 3 / 88 }, -90);
		expect((c.dx ?? 0) * 63).toBeCloseTo(-3, 6);
		expect((c.dy ?? 0) * 88).toBeCloseTo(2, 6);
		expect(arrastreEnLaPieza({ dx: 0.1 }, 0)).toEqual({ dx: 0.1 });
		expect(arrastreEnLaPieza({ zoom: 0.04 }, 90)).toEqual({ zoom: 0.04 });
	});
});
