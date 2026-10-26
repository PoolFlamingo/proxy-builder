/**
 * **Geometría de la mesa en pantalla.** Dado el tamaño del tablero, dónde
 * va cada zona (campo, mano, biblioteca, cementerio…). Es solo de pantalla:
 * dónde está cada carta en el campo lo guarda el motor (`Pos`, 0–10.000) y
 * aquí solo se convierte a píxeles.
 *
 * La Fase G le añade los modos de varios tableros; hoy hay uno.
 */

export type Rect = { x: number; y: number; w: number; h: number };

/** Proporción de una carta (63 × 88 mm). */
export const PROPORCION = 63 / 88;
/** Lo más lejos que llega una posición del motor en cada eje. */
export const POS_MAX = 10_000;

export type ZonaPantalla = "biblioteca" | "cementerio" | "exilio" | "mando" | "banquillo";

export type Disposicion = {
	tablero: Rect;
	campo: Rect;
	mano: Rect;
	zonas: Record<ZonaPantalla, Rect>;
	/** Medida de una carta del campo (sin girar). */
	carta: { w: number; h: number };
	/** Medida de una carta de la mano y de las zonas. */
	cartaPequena: { w: number; h: number };
};

/**
 * Reparte el tablero: el campo arriba; abajo, la mano a la izquierda y las
 * zonas a la derecha. `tamano` es el de Ajustes > Mesa (25–200 %).
 */
export function disponer(w: number, h: number, tamano = 100, conBanquillo = false): Disposicion {
	const escala = Math.max(0.25, Math.min(2, tamano / 100));
	// La franja de abajo: un 22 % del alto, entre 90 y 220 px.
	const franja = Math.round(Math.max(90, Math.min(220, h * 0.22)));
	const ch = franja - 26;
	const cw = Math.round(ch * PROPORCION);
	const campo: Rect = { x: 0, y: 0, w, h: Math.max(0, h - franja) };
	const nZonas = conBanquillo ? 5 : 4;
	const hueco = 8;
	const anchoZonas = nZonas * (cw + hueco) + hueco;
	const yZonas = h - franja + 8;
	const nombres: ZonaPantalla[] = conBanquillo
		? ["mando", "banquillo", "biblioteca", "cementerio", "exilio"]
		: ["mando", "biblioteca", "cementerio", "exilio"];
	const zonas = {} as Record<ZonaPantalla, Rect>;
	nombres.forEach((n, i) => {
		zonas[n] = { x: w - anchoZonas + hueco + i * (cw + hueco), y: yZonas, w: cw, h: ch };
	});
	if (!conBanquillo) zonas.banquillo = { x: -1000, y: -1000, w: 0, h: 0 };
	const mano: Rect = { x: 8, y: h - franja, w: Math.max(0, w - anchoZonas - 16), h: franja };
	// En el campo, la carta se ve algo más grande que en la franja.
	const fh = Math.round(Math.min(campo.h * 0.24, 240) * escala);
	return {
		tablero: { x: 0, y: 0, w, h },
		campo,
		mano,
		zonas,
		carta: { w: Math.round(fh * PROPORCION), h: fh },
		cartaPequena: { w: cw, h: ch },
	};
}

/** El centro en píxeles de una posición del motor dentro del campo. */
export function posAPx(campo: Rect, p: { x: number; y: number }): { x: number; y: number } {
	return {
		x: campo.x + (p.x / POS_MAX) * campo.w,
		y: campo.y + (p.y / POS_MAX) * campo.h,
	};
}

/** La posición del motor de un punto del campo (recortada al campo). */
export function pxAPos(campo: Rect, x: number, y: number): { x: number; y: number } {
	const rx = campo.w > 0 ? (x - campo.x) / campo.w : 0;
	const ry = campo.h > 0 ? (y - campo.y) / campo.h : 0;
	const recorta = (v: number) => Math.round(Math.max(0, Math.min(1, v)) * POS_MAX);
	return { x: recorta(rx), y: recorta(ry) };
}

/** ¿Está el punto dentro del rectángulo? */
export function dentro(r: Rect, x: number, y: number): boolean {
	return x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
}

/**
 * Las posiciones de las cartas de la mano en abanico: el centro de cada una
 * y su giro. Si no caben, se solapan.
 */
export function abanico(
	mano: Rect,
	n: number,
	carta: { w: number; h: number },
): { x: number; y: number; giro: number }[] {
	if (n === 0) return [];
	const paso = n > 1 ? Math.min(carta.w * 0.92, (mano.w - carta.w) / (n - 1)) : 0;
	const total = paso * (n - 1) + carta.w;
	const x0 = mano.x + (mano.w - total) / 2 + carta.w / 2;
	const mitad = (n - 1) / 2;
	return Array.from({ length: n }, (_, i) => {
		const d = n > 1 ? (i - mitad) / Math.max(mitad, 1) : 0;
		return {
			x: x0 + i * paso,
			y: mano.y + mano.h / 2 + Math.abs(d) * 6,
			giro: d * Math.min(8, n),
		};
	});
}
