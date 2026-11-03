import { describe, expect, it } from "vitest";
import { destinoDeSoltar, pasaUmbral } from "../arrastre";
import { disponer, POS_MAX } from "../disposicion";
import {
	conVista,
	crearDobleToque,
	crearFiltroClicTrasMenu,
	esArrastre,
	esVistaEntera,
	limitarVista,
	transformEntre,
	umbralDe,
	VISTA_ENTERA,
	vistaDeDosDedos,
	ZOOM_MAX,
} from "../gestos";

describe("arrastrar con el dedo", () => {
	it("con el dedo hace falta moverse más de 8 px; con el ratón, 5", () => {
		expect(umbralDe("touch")).toBe(8);
		expect(umbralDe("mouse")).toBe(5);
		expect(esArrastre("touch", 7, 0)).toBe(false);
		expect(esArrastre("touch", 6, 6)).toBe(true);
		expect(esArrastre("mouse", 5, 0)).toBe(true);
		expect(pasaUmbral(6, 0)).toBe(true);
		expect(pasaUmbral(6, 0, "touch")).toBe(false);
	});
});

describe("doble toque", () => {
	it("dos toques seguidos en la misma carta son un doble toque, y el tercero empieza de cero", () => {
		const d = crearDobleToque(300, 30);
		expect(d.toque(1, 1000, 10, 10)).toBe(false);
		expect(d.toque(1, 1200, 14, 12)).toBe(true);
		expect(d.toque(1, 1300, 14, 12)).toBe(false);
	});
	it("otra carta, demasiado tarde o demasiado lejos no cuentan", () => {
		const d = crearDobleToque(300, 30);
		d.toque(1, 0, 0, 0);
		expect(d.toque(2, 100, 0, 0)).toBe(false);
		expect(d.toque(2, 600, 0, 0)).toBe(false);
		expect(d.toque(2, 700, 100, 0)).toBe(false);
		d.olvidar();
		expect(d.toque(2, 710, 100, 0)).toBe(false);
	});
});

describe("mover y acercar el tablero con dos dedos", () => {
	const W = 800;
	const H = 400;
	const a = { x: 300, y: 200 };
	const b = { x: 500, y: 200 };

	it("sin mover los dedos, la vista no cambia", () => {
		const v = vistaDeDosDedos({ vista: VISTA_ENTERA, a, b }, { a, b }, W, H);
		expect(esVistaEntera(v)).toBe(true);
	});

	it("separar los dedos acerca, alrededor del medio de los dedos", () => {
		const v = vistaDeDosDedos(
			{ vista: VISTA_ENTERA, a, b },
			{ a: { x: 200, y: 200 }, b: { x: 600, y: 200 } },
			W,
			H,
		);
		expect(v.s).toBe(2);
		// El punto (400, 200) sigue bajo el medio de los dedos (400, 200).
		expect(v.tx + 400 * v.s).toBeCloseTo(400);
		expect(v.ty + 200 * v.s).toBeCloseTo(200);
	});

	it("no se acerca más del máximo ni se aleja de 1", () => {
		const cerca = vistaDeDosDedos(
			{ vista: VISTA_ENTERA, a, b },
			{ a: { x: 0, y: 200 }, b: { x: 800, y: 200 } },
			W,
			H,
		);
		expect(cerca.s).toBe(ZOOM_MAX);
		const lejos = vistaDeDosDedos(
			{ vista: { s: 2, tx: -100, ty: -50 }, a, b },
			{ a: { x: 395, y: 200 }, b: { x: 405, y: 200 } },
			W,
			H,
		);
		expect(lejos.s).toBe(1);
		expect(esVistaEntera(lejos)).toBe(true);
	});

	it("mover los dos dedos corre el campo, sin dejar hueco en los bordes", () => {
		const base = { s: 2, tx: -400, ty: -200 };
		const v = vistaDeDosDedos(
			{ vista: base, a, b },
			{ a: { x: 350, y: 220 }, b: { x: 550, y: 220 } },
			W,
			H,
		);
		expect(v.s).toBe(2);
		expect(v.tx).toBe(-350);
		expect(v.ty).toBe(-180);
		const fuera = vistaDeDosDedos(
			{ vista: base, a, b },
			{ a: { x: 2000, y: 2000 }, b: { x: 2200, y: 2000 } },
			W,
			H,
		);
		expect(fuera.tx).toBe(0);
		expect(fuera.ty).toBe(0);
	});

	it("limitarVista deja el campo cubriendo la pantalla", () => {
		const v = limitarVista({ s: 2, tx: -5000, ty: 100 }, W, H);
		expect(v).toEqual({ s: 2, tx: -W, ty: 0 });
		expect(limitarVista({ s: 9, tx: 0, ty: 0 }, W, H).s).toBe(ZOOM_MAX);
	});

	it("el transform de CSS lleva lo pintado con una vista a como se vería con otra", () => {
		expect(transformEntre(VISTA_ENTERA, { s: 2, tx: -100, ty: -40 })).toBe(
			"translate(-100px, -40px) scale(2)",
		);
		// Del doble al doble corrido: sin escala, solo el corrimiento.
		expect(transformEntre({ s: 2, tx: -100, ty: -40 }, { s: 2, tx: -150, ty: -40 })).toBe(
			"translate(-50px, 0px) scale(1)",
		);
	});
});

describe("la disposición con zoom", () => {
	it("sin zoom es la misma; con zoom, el campo ampliado y las cartas más grandes", () => {
		const d = disponer(1000, 500);
		expect(conVista(d, VISTA_ENTERA)).toBe(d);
		const z = conVista(d, { s: 2, tx: -200, ty: -50 });
		expect(z.campo).toEqual({ x: -200, y: -50, w: d.campo.w * 2, h: d.campo.h * 2 });
		expect(z.visible).toEqual(d.campo);
		expect(z.carta.w).toBe(d.carta.w * 2);
		// La mano y las zonas no se tocan.
		expect(z.mano).toEqual(d.mano);
		expect(z.zonas).toEqual(d.zonas);
	});

	it("soltar en el campo ampliado da la posición del motor correcta, y la mano sigue siendo la mano", () => {
		const d = disponer(1000, 500);
		const z = conVista(d, { s: 2, tx: -300, ty: 0 });
		// El punto (300, 100) de pantalla es el (600, 100) del campo ampliado: el (300, 50) del entero.
		const dest = destinoDeSoltar(z, 300, 100);
		expect(dest?.zona).toEqual({ tipo: "de", zona: "campo" });
		if (dest?.sitio.tipo !== "enCampo") throw new Error("no es del campo");
		expect(dest.sitio.valor.x).toBe(Math.round((600 / (d.campo.w * 2)) * POS_MAX));
		// Un punto de la mano no cae en el campo aunque el campo ampliado lo cubra.
		const enMano = destinoDeSoltar(z, d.mano.x + 20, d.mano.y + 20);
		expect(enMano?.zona).toMatchObject({ zona: "mano" });
	});
});

describe("el clic que llega al soltar tras abrir un menú", () => {
	it("se descarta una vez y los clics normales pasan", () => {
		const f = crearFiltroClicTrasMenu();
		expect(f.clic()).toBe(false);
		// Mantener pulsado: baja el dedo, sale el menú, se suelta y llega el clic.
		f.bajar();
		f.menu();
		expect(f.clic()).toBe(true);
		expect(f.clic()).toBe(false);
	});
	it("otro toque de por medio (elegir una opción del menú) deja pasar su clic", () => {
		const f = crearFiltroClicTrasMenu();
		f.menu();
		f.bajar();
		expect(f.clic()).toBe(false);
	});
});
