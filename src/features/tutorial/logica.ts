/**
 * La parte pura del tutorial: dónde va la tarjeta y qué versión de cada paso
 * se cuenta según cómo se toca la app. `tutorial.tsx` solo pinta.
 */

export type Rect = { x: number; y: number; w: number; h: number };

/** Ancho de la tarjeta y distancias a la zona y a los bordes (px). */
export const ANCHO = 320;
export const HUECO = 14;
export const MARGEN = 16;

/**
 * Dónde va la tarjeta (escritorio y tableta): a la derecha de la zona, a la
 * izquierda, debajo o encima (lo primero que quepa), y si no cabe en ningún
 * lado, en el centro.
 */
export function sitioTarjeta(
	r: Rect | null,
	alto: number,
	vw: number,
	vh: number,
): { left: number; top: number } {
	const centro = { left: (vw - ANCHO) / 2, top: Math.max(MARGEN, (vh - alto) / 2) };
	if (!r) return centro;
	const enY = (y: number) => Math.min(Math.max(MARGEN, y), vh - alto - MARGEN);
	const enX = (x: number) => Math.min(Math.max(MARGEN, x), vw - ANCHO - MARGEN);
	if (r.x + r.w + HUECO + ANCHO + MARGEN <= vw) return { left: r.x + r.w + HUECO, top: enY(r.y) };
	if (r.x - HUECO - ANCHO >= MARGEN) return { left: r.x - HUECO - ANCHO, top: enY(r.y) };
	if (r.y + r.h + HUECO + alto + MARGEN <= vh) return { left: enX(r.x), top: r.y + r.h + HUECO };
	if (r.y - HUECO - alto >= MARGEN) return { left: enX(r.x), top: r.y - HUECO - alto };
	return centro;
}

/**
 * En móvil la tarjeta ocupa todo el ancho y se pega arriba o abajo (sobre las
 * zonas seguras), del lado contrario a la zona resaltada. Sin zona, abajo.
 */
export function ladoMovil(r: Rect | null, vh: number): "arriba" | "abajo" {
	if (!r) return "abajo";
	return r.y + r.h / 2 > vh / 2 ? "arriba" : "abajo";
}

/**
 * Un paso del tutorial. `tactil` es lo que cambia cuando se toca con el dedo:
 * el texto (o el título) sin clic derecho, teclas ni ratón encima, y/o otra
 * zona (`ancla: ""` = ninguna, la tarjeta sale en el centro).
 */
export interface PasoBase {
	ancla?: string;
	titulo: string;
	texto: string;
	tactil?: { ancla?: string; titulo?: string; texto?: string };
}

export function aplicarFormato<T extends PasoBase>(
	pasos: T[],
	tactil: boolean,
): Omit<T, "tactil">[] {
	return pasos.map((p) => {
		const { tactil: alt, ...resto } = p;
		if (!tactil || !alt) return resto;
		return {
			...resto,
			ancla: alt.ancla === undefined ? p.ancla : alt.ancla || undefined,
			titulo: alt.titulo ?? p.titulo,
			texto: alt.texto ?? p.texto,
		};
	});
}
