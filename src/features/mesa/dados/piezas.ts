import type { CaraPlanar } from "@/bindings/CaraPlanar";
import type { PiezaTirada } from "./cola";
import { conResultado, geometria, type TipoDado } from "./geometria";

/**
 * Qué se pinta en cada cara: los números del dado ya cambiados para que la
 * cara que se lee lleve el resultado del motor. Lo comparten los tres
 * niveles (en CSS y 2D no hay física: la cara que se lee la eligen ellos).
 */

/** El dado planar es un d6: 1 = planeswalk, 6 = caos (opuestos), lo demás en blanco. */
export const VALOR_PLANAR: Record<CaraPlanar, number> = { planeswalk: 1, caos: 6, nada: 2 };

/** El modelo de una pieza (la moneda no tiene). */
export function tipoDe(p: PiezaTirada): TipoDado | null {
	if (p.forma === "dado") return p.tipo;
	if (p.forma === "planar") return 6;
	return null;
}

/** El valor que tiene que leerse en un dado (o en el planar). */
export function valorDe(p: PiezaTirada): number {
	if (p.forma === "dado") return p.valor;
	if (p.forma === "planar") return VALOR_PLANAR[p.cara];
	return 0;
}

/** Los números de las caras con `cara` (la que se lee) llevando el resultado. */
export function valoresPintados(p: PiezaTirada, cara: number): number[] {
	const t = tipoDe(p);
	if (t === null) return [];
	return conResultado(geometria(t), cara, valorDe(p));
}

/** La cara que lleva ya el resultado (para los niveles sin física). */
export function caraDelResultado(p: PiezaTirada): number {
	const t = tipoDe(p);
	if (t === null) return 0;
	return Math.max(0, geometria(t).valores.indexOf(valorDe(p)));
}
