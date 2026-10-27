import { describe, expect, it } from "vitest";
import {
	type ConfigDisposicion,
	DISPOSICION_DE_FABRICA,
	disposicionDe,
	disposicionPorDefecto,
	HUECO,
	limitarProporcion,
	modosPara,
	ordenTableros,
	proporcionPorPuntero,
	type Rect,
	repartir,
	VACIO,
} from "../disposicion";

const PANTALLA: Rect = { x: 10, y: 20, w: 1280, h: 720 };

const config = (parte: Partial<ConfigDisposicion>): ConfigDisposicion => ({
	...DISPOSICION_DE_FABRICA,
	...parte,
});

function solapan(a: Rect, b: Rect): boolean {
	return a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
}

function dentro(a: Rect, r: Rect): boolean {
	return a.x >= r.x && a.y >= r.y && a.x + a.w <= r.x + r.w && a.y + a.h <= r.y + r.h;
}

/** Los rectángulos que se ven (los de «único» tienen los demás vacíos). */
const visibles = (rs: Rect[]) => rs.filter((r) => r.w > 0 && r.h > 0);

describe("repartir", () => {
	it("no devuelve nada sin tableros", () => {
		expect(repartir(0, PANTALLA, config({ modo: "rejilla" }))).toEqual([]);
	});

	it("con un tablero ocupa todo", () => {
		expect(repartir(1, PANTALLA, config({ modo: "rejilla" }))).toEqual([PANTALLA]);
	});

	it("«único» deja el propio entero y los demás sin área", () => {
		const rs = repartir(4, PANTALLA, config({ modo: "unico" }));
		expect(rs).toHaveLength(4);
		expect(rs[0]).toEqual(PANTALLA);
		expect(rs.slice(1)).toEqual([VACIO, VACIO, VACIO]);
	});

	for (const modo of ["dividido", "foco", "rejilla", "equipos"] as const) {
		for (let n = 2; n <= 10; n++) {
			it(`${modo} con ${n} tableros: dentro de la pantalla y sin solaparse`, () => {
				const equipos = Array.from({ length: n }, (_, i) => (i % 3 === 0 ? null : i % 2));
				for (const tira of ["arriba", "derecha", "abajo"] as const) {
					for (const vertical of [true, false]) {
						const rs = repartir(n, PANTALLA, config({ modo, tira, vertical }), n - 1, equipos);
						expect(rs).toHaveLength(n);
						const vis = visibles(rs);
						// «dividido» con más de dos cae a rejilla: todos se ven.
						expect(vis).toHaveLength(n);
						for (const r of vis) {
							expect(Number.isInteger(r.x) || Number.isFinite(r.x)).toBe(true);
							expect(dentro(r, PANTALLA)).toBe(true);
						}
						for (let i = 0; i < vis.length; i++) {
							for (let j = i + 1; j < vis.length; j++) {
								expect(solapan(vis[i], vis[j])).toBe(false);
							}
						}
					}
				}
			});
		}
	}

	it("«dividido» reparte según la proporción, con el hueco entre los dos", () => {
		const [a, b] = repartir(
			2,
			PANTALLA,
			config({ modo: "dividido", proporcion: 0.75, vertical: true }),
		);
		expect(a.h).toBe(PANTALLA.h);
		expect(b.x).toBe(a.x + a.w + HUECO);
		expect(a.w + HUECO + b.w).toBe(PANTALLA.w);
		expect(a.w).toBeGreaterThan(b.w);
	});

	it("«dividido» horizontal pone uno encima del otro", () => {
		const [a, b] = repartir(
			2,
			PANTALLA,
			config({ modo: "dividido", proporcion: 0.5, vertical: false }),
		);
		expect(a.w).toBe(PANTALLA.w);
		expect(b.y).toBe(a.y + a.h + HUECO);
		expect(a.h + HUECO + b.h).toBe(PANTALLA.h);
	});

	it("«dividido» limita la proporción para que ninguno desaparezca", () => {
		expect(limitarProporcion(0)).toBe(0.2);
		expect(limitarProporcion(1)).toBe(0.8);
		const [a, b] = repartir(
			2,
			PANTALLA,
			config({ modo: "dividido", proporcion: 0, vertical: true }),
		);
		expect(a.w).toBeGreaterThan(100);
		expect(b.w).toBeGreaterThan(a.w);
	});

	it("«foco» da el tablero grande al elegido y una tira al resto", () => {
		const rs = repartir(4, PANTALLA, config({ modo: "foco", tira: "abajo" }), 2);
		const grande = rs[2];
		for (const [i, r] of rs.entries()) {
			if (i === 2) continue;
			expect(grande.w * grande.h).toBeGreaterThan(r.w * r.h);
			expect(r.y).toBeGreaterThan(grande.y + grande.h - 1);
		}
	});

	it("«foco» con la tira a la derecha deja el grande a la izquierda", () => {
		const rs = repartir(3, PANTALLA, config({ modo: "foco", tira: "derecha" }), 0);
		expect(rs[0].x).toBe(PANTALLA.x);
		expect(rs[1].x).toBeGreaterThan(rs[0].x + rs[0].w - 1);
		expect(rs[1].x).toBe(rs[2].x);
	});

	it("«foco» con un grande fuera de rango usa el último", () => {
		const rs = repartir(3, PANTALLA, config({ modo: "foco" }), 99);
		expect(rs[2].w).toBeGreaterThan(rs[0].w);
	});

	it("«rejilla» de 4 hace 2×2 en una pantalla apaisada", () => {
		const rs = repartir(4, PANTALLA, config({ modo: "rejilla" }));
		expect(new Set(rs.map((r) => r.x)).size).toBe(2);
		expect(new Set(rs.map((r) => r.y)).size).toBe(2);
	});

	it("«rejilla» centra la última fila si no está llena", () => {
		const rs = repartir(3, PANTALLA, config({ modo: "rejilla" }));
		// Con 3 hay una fila de 2 y otra de 1, o tres en fila: en cualquier caso, simétrico.
		const fila = rs.filter((r) => r.y === rs[2].y);
		if (fila.length === 1) {
			const izq = rs[2].x - PANTALLA.x;
			const der = PANTALLA.x + PANTALLA.w - (rs[2].x + rs[2].w);
			expect(Math.abs(izq - der)).toBeLessThanOrEqual(1);
		}
	});

	it("«equipos» pone juntos a los compañeros del propio", () => {
		// Tableros 0 y 3 son del equipo 0; el 1 y el 2, del equipo 1.
		const rs = repartir(4, PANTALLA, config({ modo: "equipos" }), 0, [0, 1, 1, 0]);
		const celda = (r: Rect) => `${r.x},${r.y}`;
		const orden = [0, 3, 1, 2].map((i) => celda(rs[i]));
		// Las celdas siguen el orden de lectura de la rejilla.
		const lectura = [...rs].sort((a, b) => a.y - b.y || a.x - b.x).map(celda);
		expect(lectura).toEqual(orden);
	});

	it("«equipos» deja a los que no tienen equipo al final", () => {
		const rs = repartir(3, PANTALLA, config({ modo: "equipos" }), 0, [null, null, 0]);
		const lectura = [...rs.keys()].sort((a, b) => rs[a].y - rs[b].y || rs[a].x - rs[b].x);
		expect(lectura[0]).toBe(0);
	});

	it("aguanta una pantalla diminuta sin tamaños negativos", () => {
		for (const modo of ["dividido", "foco", "rejilla", "equipos"] as const) {
			const rs = repartir(10, { x: 0, y: 0, w: 30, h: 20 }, config({ modo }));
			for (const r of rs) {
				expect(r.w).toBeGreaterThanOrEqual(0);
				expect(r.h).toBeGreaterThanOrEqual(0);
			}
		}
	});
});

describe("modosPara", () => {
	it("con uno solo no hay nada que elegir", () => {
		expect(modosPara(1)).toEqual(["unico"]);
	});
	it("«dividido» solo con dos", () => {
		expect(modosPara(2)).toContain("dividido");
		expect(modosPara(3)).not.toContain("dividido");
	});
	it("«equipos» desde tres y solo si hay equipos", () => {
		expect(modosPara(2, true)).not.toContain("equipos");
		expect(modosPara(4)).not.toContain("equipos");
		expect(modosPara(4, true)).toContain("equipos");
	});
});

describe("requisitos de tamaño", () => {
	const hd: Rect = { x: 0, y: 0, w: 1280, h: 720 };
	for (const modo of ["foco", "rejilla", "equipos"] as const) {
		for (let n = 2; n <= 10; n++) {
			it(`${modo} con ${n}: todos miden al menos 40 × 28`, () => {
				for (const r of repartir(n, hd, config({ modo }))) {
					expect(r.w).toBeGreaterThanOrEqual(40);
					expect(r.h).toBeGreaterThanOrEqual(28);
				}
			});
		}
	}
	for (let n = 2; n <= 10; n++) {
		it(`foco con ${n}: el grande es el mayor`, () => {
			const rs = repartir(n, hd, config({ modo: "foco" }), 0);
			for (const r of rs.slice(1)) expect(rs[0].w * rs[0].h).toBeGreaterThan(r.w * r.h);
		});
	}
});

describe("disposicionDe", () => {
	it("por defecto: dividido con 2 y foco con más", () => {
		expect(disposicionPorDefecto(1).modo).toBe("unico");
		expect(disposicionPorDefecto(2).modo).toBe("dividido");
		expect(disposicionPorDefecto(6).modo).toBe("foco");
	});
	it("usa la guardada y recorta su proporción", () => {
		const g = { 2: config({ modo: "foco", proporcion: 5 }) };
		const d = disposicionDe(g, 2);
		expect(d.modo).toBe("foco");
		expect(d.proporcion).toBe(0.8);
	});
	it("descarta una guardada que no sirve para ese número", () => {
		expect(disposicionDe({ 3: config({ modo: "dividido" }) }, 3).modo).toBe("foco");
		expect(disposicionDe(undefined, 4).modo).toBe("foco");
	});
	it("«equipos» guardado sin equipos vuelve a la de fábrica", () => {
		const g = { 4: config({ modo: "equipos" }) };
		expect(disposicionDe(g, 4, false).modo).toBe("foco");
		expect(disposicionDe(g, 4, true).modo).toBe("equipos");
	});
});

describe("ordenTableros", () => {
	it("en local deja los asientos tal cual", () => {
		expect(ordenTableros([0, 1, 2, 3], 2, true)).toEqual([0, 1, 2, 3]);
	});
	it("en red empieza por el propio y sigue por asiento", () => {
		expect(ordenTableros([0, 1, 2, 3], 2, false)).toEqual([2, 3, 0, 1]);
	});
	it("quien mira (sin asiento) ve el orden de los asientos", () => {
		expect(ordenTableros([0, 1, 2], null, false)).toEqual([0, 1, 2]);
		expect(ordenTableros([0, 1, 2], 9, false)).toEqual([0, 1, 2]);
	});
});

describe("proporcionPorPuntero", () => {
	const r: Rect = { x: 100, y: 0, w: 1004, h: 600 };
	it("el puntero en la mitad da la mitad", () => {
		expect(proporcionPorPuntero(r, 100 + 502, 0, true)).toBeCloseTo(0.5, 2);
	});
	it("coincide con lo que reparte repartir()", () => {
		const c = config({ modo: "dividido", vertical: true, proporcion: 0.3 });
		const [a] = repartir(2, r, c);
		const p = proporcionPorPuntero(r, a.x + a.w + HUECO / 2, 0, true);
		expect(p).toBeCloseTo(0.3, 2);
	});
	it("no pasa de los límites", () => {
		expect(proporcionPorPuntero(r, -500, 0, true)).toBe(0.2);
		expect(proporcionPorPuntero(r, 9999, 0, true)).toBe(0.8);
		expect(proporcionPorPuntero(r, 0, 9999, false)).toBe(0.8);
	});
});
