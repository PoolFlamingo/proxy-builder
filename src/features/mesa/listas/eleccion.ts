import type { AvisoLista } from "@/bindings/AvisoLista";
import type { CartaDeLista } from "@/bindings/CartaDeLista";
import type { ListaDeJuego } from "@/bindings/ListaDeJuego";

/**
 * **Elegir las cartas de una lista para jugar**, sin reglas: lo que se pinta
 * y se pulsa en el editor. Las reglas (tamaño, copias, identidad) las
 * comprueba Rust (`mesa::lista::comprobar`); aquí solo se ayuda a no
 * equivocarse: en singleton, elegir una variante de diseño quita las otras
 * de la misma carta.
 */
export type Eleccion = Pick<ListaDeJuego, "comandantes" | "cartas">;

/** Lo que se elige al crear una lista: las copias sin marca (ni variante, ni opcional, ni banquillo). */
export function eleccionDeFabrica(todas: CartaDeLista[]): Eleccion {
	return { comandantes: [], cartas: todas.filter((c) => !c.marca).map((c) => c.id) };
}

/** Las copias que lleva la lista (comandantes incluidos). */
export function cuantas(e: Eleccion): number {
	return e.comandantes.length + e.cartas.length;
}

/** Elige o quita una copia. */
export function alternarCarta(
	e: Eleccion,
	c: CartaDeLista,
	todas: CartaDeLista[],
	singleton: boolean,
): Eleccion {
	if (e.cartas.includes(c.id)) return { ...e, cartas: e.cartas.filter((x) => x !== c.id) };
	// Una sola copia por carta: fuera las otras variantes (y el comandante, si era él).
	const hermanas =
		singleton && !c.sinLimite
			? new Set(todas.filter((x) => x.nombre === c.nombre && x.id !== c.id).map((x) => x.id))
			: new Set<string>();
	return {
		comandantes: e.comandantes.filter((x) => x !== c.id && !hermanas.has(x)),
		cartas: [...e.cartas.filter((x) => !hermanas.has(x)), c.id],
	};
}

/**
 * Elige o quita un comandante (hasta dos: compañeros o trasfondo). Al
 * elegirlo sale de las demás cartas, con sus otras variantes; con dos ya
 * elegidos, el nuevo reemplaza al segundo.
 */
export function alternarComandante(e: Eleccion, c: CartaDeLista, todas: CartaDeLista[]): Eleccion {
	if (e.comandantes.includes(c.id))
		return { ...e, comandantes: e.comandantes.filter((x) => x !== c.id) };
	const misma = new Set(todas.filter((x) => x.nombre === c.nombre).map((x) => x.id));
	const otros = e.comandantes.filter((x) => !misma.has(x)).slice(0, 1);
	return {
		comandantes: [...otros, c.id],
		cartas: e.cartas.filter((x) => !misma.has(x)),
	};
}

/** Deja `k` copias elegidas de una carta sin límite (las básicas): las primeras. */
export function conCopias(e: Eleccion, grupo: CartaDeLista[], k: number): Eleccion {
	const ids = new Set(grupo.map((c) => c.id));
	const n = Math.max(0, Math.min(k, grupo.length));
	return {
		comandantes: e.comandantes,
		cartas: [...e.cartas.filter((x) => !ids.has(x)), ...grupo.slice(0, n).map((c) => c.id)],
	};
}

/** Si algún aviso impide jugar (lo mismo que `AvisoLista::impide` en Rust). */
export function impide(a: AvisoLista): boolean {
	return (
		a.tipo === "tamano" ||
		a.tipo === "sinComandante" ||
		a.tipo === "demasiadosComandantes" ||
		a.tipo === "repetida"
	);
}
