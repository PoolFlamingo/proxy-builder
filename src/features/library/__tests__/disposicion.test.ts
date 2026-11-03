import { describe, expect, it } from "vitest";
import {
	type ColumnaLista,
	clasesCuadricula,
	columnasQueCaben,
	plantillaLista,
} from "../disposicion";

const COLS: ColumnaLista[] = [
	{ id: "nombre", px: 240 },
	{ id: "colores", px: 96 },
	{ id: "clase", px: 104, quitar: 5 },
	{ id: "cartas", px: 64, movil: true },
	{ id: "coste", px: 96, quitar: 4 },
	{ id: "precio", px: 80, quitar: 6 },
	{ id: "idioma", px: 120, quitar: 2 },
	{ id: "propias", px: 64, quitar: 1 },
	{ id: "modificado", px: 112 },
	{ id: "abierto", px: 112, quitar: 3 },
];

const escritorio = { casillas: true, movil: false, tactil: false };
const ids = (c: ColumnaLista[]) => c.map((x) => x.id);

describe("clasesCuadricula", () => {
	it("el móvil lleva dos columnas, y una por debajo de 360 px, sea cual sea el tamaño", () => {
		for (const t of ["s", "m", "l"] as const) {
			const c = clasesCuadricula("movil", t);
			expect(c).toContain("grid-cols-1");
			expect(c).toContain("min-[360px]:grid-cols-2");
		}
	});

	it("escritorio y tableta conservan las columnas de siempre", () => {
		expect(clasesCuadricula("escritorio", "m")).toContain("minmax(180px,1fr)");
		expect(clasesCuadricula("tableta", "l")).toContain("minmax(240px,1fr)");
		expect(clasesCuadricula("escritorio", "s")).toContain("gap-4");
	});
});

describe("columnasQueCaben", () => {
	it("con sitio de sobra caben todas", () => {
		expect(ids(columnasQueCaben(COLS, 2000, escritorio))).toEqual(ids(COLS));
	});

	it("quita primero las de número más bajo", () => {
		const cols = ids(columnasQueCaben(COLS, 1300, escritorio));
		expect(cols).not.toContain("propias");
		expect(cols).toContain("precio");
	});

	it("en estrecho se quedan nombre, colores, cartas y modificado", () => {
		expect(ids(columnasQueCaben(COLS, 200, escritorio))).toEqual([
			"nombre",
			"colores",
			"cartas",
			"modificado",
		]);
	});

	it("el móvil se queda con nombre y cartas", () => {
		const movil = { casillas: true, movil: true, tactil: true };
		expect(ids(columnasQueCaben(COLS, 391, movil))).toEqual(["nombre", "cartas"]);
	});

	it("en un móvil muy estrecho, solo el nombre", () => {
		const movil = { casillas: true, movil: true, tactil: true };
		expect(ids(columnasQueCaben(COLS, 250, movil))).toEqual(["nombre"]);
	});
});

describe("plantillaLista", () => {
	it("escritorio: casilla, portada, columnas y menú", () => {
		const cols = COLS.slice(0, 2);
		expect(plantillaLista(cols, escritorio)).toBe("28px 40px minmax(240px,1fr) 96px 32px");
	});

	it("sin casillas no hay columna de casilla", () => {
		expect(plantillaLista(COLS.slice(0, 1), { ...escritorio, casillas: false })).toBe(
			"40px minmax(240px,1fr) 32px",
		);
	});

	it("en el móvil el nombre se encoge del todo y el menú crece para el dedo", () => {
		const movil = { casillas: true, movil: true, tactil: true };
		expect(plantillaLista([COLS[0], COLS[3]], movil)).toBe("28px 40px minmax(0,1fr) 64px 56px");
	});
});
