/**
 * El alto que de verdad se ve de una pantalla en el móvil. El teclado de
 * Android solo encoge el viewport visual (`window.visualViewport`): la
 * ventana sigue midiendo lo mismo y `dvh` no se entera, así que lo que había
 * abajo queda tapado. Esto da el alto de un contenedor que empieza en
 * `arriba` (px desde el borde de la ventana) recortado a lo que se ve.
 */
export function altoVisible(m: {
	/** El alto que tiene el contenedor sin teclado. */
	contenedor: number;
	/** Dónde empieza, respecto a la ventana. */
	arriba: number;
	/** El viewport visual: su alto y cuánto se ha desplazado. */
	visualAlto: number;
	visualArriba: number;
}): number {
	const hasta = m.visualArriba + m.visualAlto;
	return Math.max(0, Math.min(m.contenedor, Math.round(hasta - m.arriba)));
}

/** Lo que miden, con su margen, los controles que flotan a cada lado de la carta (px). */
export const CONTROLES_CARTA = 48;

/** Cuánto alto ocupa la carta en la pantalla de Diseñar del móvil: más
 *  pequeña mientras se escribe, para que quepa el campo. */
export function altoCarta(alto: number, escribiendo: boolean): number {
	return Math.round(alto * (escribiendo ? 0.22 : 0.44));
}

/** El teléfono de lado: el ancho de la columna de la carta, que va a toda la
 *  altura, con sitio a cada lado para sus controles (`CONTROLES_CARTA`). */
export function anchoColumnaCarta(alto: number): number {
	return Math.round((alto * 63) / 88) + 2 * CONTROLES_CARTA;
}

/** ¿Es este elemento un campo donde se escribe (y por tanto sale el teclado)? */
export function esCampoDeTexto(el: {
	tagName?: string;
	isContentEditable?: boolean;
	type?: string;
}) {
	const tag = el.tagName?.toLowerCase();
	if (el.isContentEditable) return true;
	if (tag === "textarea") return true;
	if (tag !== "input") return false;
	return !["checkbox", "radio", "range", "button", "submit", "color", "file"].includes(
		(el.type ?? "text").toLowerCase(),
	);
}
