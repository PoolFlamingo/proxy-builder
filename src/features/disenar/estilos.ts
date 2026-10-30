import type { CardDesign } from "@/bindings/CardDesign";
import type { Collection } from "@/bindings/Collection";
import type { DesignStyle } from "@/bindings/DesignStyle";
import type { EstiloPropio } from "@/bindings/EstiloPropio";
import type { PackInfo } from "@/bindings/PackInfo";
import { ESTILOS } from "./textos";

/**
 * **Estilos de serie y estilos propios del pack de marcos** en un mismo
 * selector. Un estilo propio se guarda como `estiloPack` (su id en el pack)
 * con `style` = su estilo de serie (al que se cae si el pack no lo tiene).
 * En el selector, los propios van como `pack:<id>`.
 */
const PREFIJO = "pack:";

type T = (es: string) => string;

/** El pack de marcos con el que se pinta una carta. */
export function packEfectivo(d: CardDesign, colecciones: Collection[]): string {
	const col = colecciones.find((c) => c.id === d.collection);
	return d.frame?.pack ?? col?.frame.pack ?? "builtin";
}

/** Los estilos propios de un pack. */
export function propiosDe(packs: PackInfo[], pack: string): EstiloPropio[] {
	return packs.find((p) => p.id === pack)?.estilosPropios ?? [];
}

/**
 * El nombre de un estilo o variante de un pack en el idioma de la interfaz;
 * si el pack no lo trae, el inglés; si tampoco, su id.
 */
export function nombreEnIdioma(
	nombres: { [k in string]?: string },
	lang: string,
	id: string,
): string {
	return nombres[lang] ?? nombres.en ?? id;
}

export function opcionesEstilo(t: T, lang: string, propios: EstiloPropio[], actual?: string) {
	const opciones = [
		...ESTILOS.map((e) => ({ valor: e.valor as string, etiqueta: t(e.etiqueta) })),
		...propios.map((e) => ({
			valor: `${PREFIJO}${e.id}`,
			etiqueta: nombreEnIdioma(e.nombres, lang, e.id),
		})),
	];
	// Uno propio que este pack no tiene (se cambió de pack): sigue a la vista.
	if (actual && !opciones.some((o) => o.valor === actual)) {
		opciones.push({ valor: actual, etiqueta: actual.slice(PREFIJO.length) });
	}
	return opciones;
}

/**
 * El valor del selector. Sin estilo propio elegido, si el pack no trae el de
 * serie y Rust dice con cuál de los suyos lo pinta (`sustitutos`), ese: el
 * selector enseña el marco que se ve de verdad.
 */
export function valorEstilo(
	style: DesignStyle,
	estiloPack?: string | null,
	pack?: PackInfo,
): string {
	if (estiloPack) return `${PREFIJO}${estiloPack}`;
	const sustituto = pack?.sustitutos[style];
	return sustituto ? `${PREFIJO}${sustituto}` : style;
}

/** Lo elegido en el selector como `style` + `estiloPack`. */
export function elegirEstilo(
	valor: string,
	propios: EstiloPropio[],
	actual: DesignStyle,
): { style: DesignStyle; estiloPack: string | undefined } {
	if (!valor.startsWith(PREFIJO)) return { style: valor as DesignStyle, estiloPack: undefined };
	const id = valor.slice(PREFIJO.length);
	const p = propios.find((e) => e.id === id);
	return { style: p?.base ?? actual, estiloPack: id };
}
