import type { FiltrosCartas } from "@/bindings/FiltrosCartas";

export const FILTROS_DE_FABRICA: FiltrosCartas = {
	texto: "",
	exacto: false,
	colores: [],
	modoColores: "incluye",
	tipo: null,
	rarezas: [],
	formato: null,
	coste: null,
	set: null,
	idioma: null,
	orden: "nombre",
	descendente: false,
	unicas: "cartas",
};

/** ¿Se ha pedido algo? (Sin texto ni filtros no hay nada que buscar.) */
export function hayCriterio(f: FiltrosCartas): boolean {
	return f.texto.trim().length > 0 || cuantosFiltros(f) > 0;
}

/** Cuántos filtros hay puestos (el número del botón «Filtros»), sin contar el orden. */
export function cuantosFiltros(f: FiltrosCartas): number {
	return [
		f.colores.length > 0,
		f.tipo,
		f.rarezas.length > 0,
		f.formato,
		f.coste,
		f.set?.trim(),
		f.idioma,
	].filter(Boolean).length;
}

/** Los filtros sin el texto: cambiarlos busca solo; el texto, al pulsar «Buscar». */
export function claveDeFiltros({ texto: _, exacto: __, ...resto }: FiltrosCartas): string {
	return JSON.stringify(resto);
}

/** Un trozo de texto de reglas: letras o un símbolo (`{T}`, `{2/W}`…), sin las llaves. */
type Trozo = { simbolo: false; texto: string } | { simbolo: true; texto: string };

/** Parte un texto de Scryfall en letras y símbolos. */
export function partirSimbolos(texto: string): Trozo[] {
	const trozos: Trozo[] = [];
	let desde = 0;
	for (const m of texto.matchAll(/\{([^{}]+)\}/g)) {
		if (m.index > desde) trozos.push({ simbolo: false, texto: texto.slice(desde, m.index) });
		trozos.push({ simbolo: true, texto: m[1] });
		desde = m.index + m[0].length;
	}
	if (desde < texto.length) trozos.push({ simbolo: false, texto: texto.slice(desde) });
	return trozos;
}
