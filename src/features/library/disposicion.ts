import type { Tamano } from "@/lib/formato";

/**
 * Cómo se reparte la biblioteca según la pantalla (plan del móvil, M3.2).
 * Todo puro: las cuentas de columnas se prueban sin pantalla.
 */

export type TamanoTarjeta = "s" | "m" | "l";

/** Las columnas de la cuadrícula de siempre (escritorio y tableta). */
const COLUMNAS: Record<TamanoTarjeta, string> = {
	s: "grid-cols-[repeat(auto-fill,minmax(140px,1fr))]",
	m: "grid-cols-[repeat(auto-fill,minmax(180px,1fr))]",
	l: "grid-cols-[repeat(auto-fill,minmax(240px,1fr))]",
};

/**
 * Las clases de la rejilla de tarjetas. En el móvil, dos columnas (una por
 * debajo de 360 px de ancho) sea cual sea el tamaño elegido: con el dedo no
 * se elige el tamaño; en el resto, lo de siempre.
 */
export function clasesCuadricula(tamano: Tamano, tarjeta: TamanoTarjeta): string {
	return tamano === "movil"
		? "grid-cols-1 gap-2.5 min-[360px]:grid-cols-2"
		: `${COLUMNAS[tarjeta]} gap-4`;
}

// ── La lista ─────────────────────────────────────────────────────────────

export interface ColumnaLista {
	id: string;
	/** Ancho en px (el nombre: el mínimo; se lleva lo que sobre). */
	px: number;
	/** Si no cabe todo, se quitan antes las de número más bajo. */
	quitar?: number;
	/** Las que se quedan en el móvil, si caben. */
	movil?: boolean;
}

export interface OpcionesLista {
	/** Hay una casilla para elegir. */
	casillas: boolean;
	movil: boolean;
	/** Con el dedo, el botón «⋯» ocupa más. */
	tactil: boolean;
}

const CASILLA = 28;
const PORTADA = 40;
const MENU = 32;
const MENU_TACTIL = 56;
const HUECO = 12;
const RELLENO = 16 + 2;
/** El nombre en el móvil se encoge hasta aquí antes de quitar columnas. */
const MIN_NOMBRE_MOVIL = 120;

/**
 * Las columnas que caben en `ancho`: quita las prescindibles (las de
 * `quitar` más bajo primero) hasta que entren. En el móvil, solo nombre y
 * las marcadas `movil`, y si ni así caben, solo el nombre.
 */
export function columnasQueCaben<C extends ColumnaLista>(
	todas: C[],
	ancho: number,
	o: OpcionesLista,
): C[] {
	const menu = o.tactil ? MENU_TACTIL : MENU;
	const mide = (c: C[]) =>
		PORTADA +
		menu +
		(o.casillas ? CASILLA : 0) +
		RELLENO +
		c.reduce((a, x) => a + (o.movil && x.id === "nombre" ? MIN_NOMBRE_MOVIL : x.px), 0) +
		HUECO * (c.length + 2);
	if (o.movil) {
		const pocas = todas.filter((c) => c.movil || c.id === "nombre");
		return mide(pocas) <= ancho ? pocas : todas.filter((c) => c.id === "nombre");
	}
	let cols = todas;
	const turno = todas.filter((c) => c.quitar).sort((a, b) => (a.quitar ?? 0) - (b.quitar ?? 0));
	for (const c of turno) {
		if (mide(cols) <= ancho) break;
		cols = cols.filter((x) => x !== c);
	}
	return cols;
}

/** El `grid-template-columns` de la cabecera y de cada fila. */
export function plantillaLista(cols: ColumnaLista[], o: OpcionesLista): string {
	const nombre = (c: ColumnaLista) => (o.movil ? "minmax(0,1fr)" : `minmax(${c.px}px,1fr)`);
	return `${o.casillas ? "28px " : ""}40px ${cols
		.map((c) => (c.id === "nombre" ? nombre(c) : `${c.px}px`))
		.join(" ")} ${o.tactil ? `${MENU_TACTIL}px` : `${MENU}px`}`;
}
