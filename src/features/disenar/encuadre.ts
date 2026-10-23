import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";

/** Un paso de arrastre (en fracciones del ancho y del alto de la carta) o de rueda. */
export type CambioEncuadre = { dx?: number; dy?: number; zoom?: number };

/** Medidas de la carta (mm) con las que se cuentan los arrastres. */
export const CARTA_MM = { w: 63, h: 88 };

/** El zoom más pequeño (`ZOOM_MIN` en Rust): por debajo de 1, la imagen
 *  cabe entera en su hueco. */
export const ZOOM_MIN = 0.1;
export const ZOOM_MAX = 3;

/** El hueco de la ilustración (mm) y la proporción de la imagen. */
export type HuecoArte = { w: number; h: number; iw: number; ih: number };

/**
 * Cómo cubre la imagen el hueco (la cuenta de `pintar_arte` en Rust): su
 * tamaño sin girar (`rw`, `rh`) y cuánto sobra a cada lado de su caja girada
 * (`hx`, `hy`), que es lo que recorre el desplazamiento de -1 a 1. Con zoom
 * por debajo de 1 es negativo: lo que falta, y el desplazamiento la mueve
 * dentro del hueco.
 */
function cubrir(f: ArtFraming, { w, h, iw, ih }: HuecoArte) {
	const a = (f.rotation * Math.PI) / 180;
	const c = Math.abs(Math.cos(a));
	const s = Math.abs(Math.sin(a));
	const zoom = Math.max(ZOOM_MIN, f.zoom);
	const cubre = Math.max((w * c + h * s) / iw, (w * s + h * c) / ih) * zoom;
	const rw = iw * cubre;
	const rh = ih * cubre;
	return {
		rw,
		rh,
		hx: (rw * c + rh * s - w) / 2,
		hy: (rw * s + rh * c - h) / 2,
	};
}

const limitar = (v: number) => Math.min(1, Math.max(-1, v));

/**
 * El encuadre tras arrastrar `dx`/`dy` (fracciones del ancho y del alto de la
 * carta) o girar la rueda. La ilustración sigue al ratón 1:1, tenga el zoom
 * o el giro que tenga, hasta que llega a un borde.
 */
export function moverEncuadre(f: ArtFraming, c: CambioEncuadre, hueco: HuecoArte): ArtFraming {
	const zoom = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, f.zoom + (c.zoom ?? 0)));
	const { hx, hy } = cubrir({ ...f, zoom }, hueco);
	// Lo que se mueve el ratón, en mm; el desplazamiento recorre la holgura.
	const mx = (c.dx ?? 0) * CARTA_MM.w;
	const my = (c.dy ?? 0) * CARTA_MM.h;
	return {
		...f,
		zoom,
		offsetX: Math.abs(hx) > 1e-6 ? limitar(f.offsetX - mx / hx) : f.offsetX,
		offsetY: Math.abs(hy) > 1e-6 ? limitar(f.offsetY - my / hy) : f.offsetY,
	};
}

/** ¿El hueco va girado de lado (la ilustración se ve tumbada)? */
export const deLado = (giro: number) => Math.abs(Math.abs(giro) - 90) < 1;

/**
 * El hueco tal como lo ve la ilustración (mm), sin girar: en la mitad de abajo
 * de una Secuela o en una partida, la pieza va girada y el hueco tumbado.
 * `arte` es el hueco en fracciones de la carta, como lo da `design_overlay`.
 */
export function huecoSinGirar(arte: readonly number[], giro: number): { w: number; h: number } {
	const w = (arte[2] ?? 0) * CARTA_MM.w;
	const h = (arte[3] ?? 0) * CARTA_MM.h;
	return deLado(giro) ? { w: h, h: w } : { w, h };
}

/**
 * Un arrastre en la pantalla (fracciones de la carta) visto desde la pieza
 * girada `giro` grados (positivo, a la derecha): así la ilustración sigue al
 * ratón aunque esté tumbada.
 */
export function arrastreEnLaPieza(c: CambioEncuadre, giro: number): CambioEncuadre {
	if (!giro || (c.dx === undefined && c.dy === undefined)) return c;
	const a = (giro * Math.PI) / 180;
	const [cos, sin] = [Math.round(Math.cos(a)), Math.round(Math.sin(a))];
	const sx = (c.dx ?? 0) * CARTA_MM.w;
	const sy = (c.dy ?? 0) * CARTA_MM.h;
	const px = cos * sx + sin * sy;
	const py = -sin * sx + cos * sy;
	return { ...c, dx: px / CARTA_MM.w, dy: py / CARTA_MM.h };
}

/**
 * Lo que identifica la capa de un diseño (`design_overlay`): todo menos el
 * encuadre de la ilustración que se mueve (la de `cara`). Mientras solo cambia
 * ese encuadre, la capa sirve. El de las demás sí cuenta: en una compuesta la
 * capa de una parte lleva pintada la ilustración de la otra con su encuadre.
 */
export function claveCapa(d: CardDesign, cara: number, lang: DesignLang): string {
	const sin = {
		...d,
		faces: d.faces.map((f, i) =>
			i === cara && f.art ? { ...f, art: { ...f.art, framing: null } } : f,
		),
	};
	return `${cara}|${lang}|${JSON.stringify(sin)}`;
}

/** Por debajo de 1 la imagen no cubre el hueco: lleva el fondo desenfocado. */
export const cubreElHueco = (f: ArtFraming) => f.zoom >= 1;

/** Dónde va la ilustración dentro de su hueco, para CSS. */
export type GeometriaArte = {
	/** Ancho y alto de la imagen, en % del hueco. */
	ancho: number;
	alto: number;
	/** Desplazamiento desde la esquina del hueco, en % del tamaño de la imagen
	 *  (lo que pide `translate`). */
	tx: number;
	ty: number;
	/** Giro en grados alrededor de su centro. */
	giro: number;
};

/**
 * La misma cuenta que `pintar_arte` en Rust (`proxy-core::diseno::render`):
 * la imagen cubre el hueco (girada, su caja), con el zoom, y el
 * desplazamiento va de un borde al otro de lo que sobra. `iw`/`ih` solo
 * cuentan por su proporción; `w`/`h` son el hueco en cualquier unidad.
 */
export function geometriaArte(
	f: ArtFraming,
	iw: number,
	ih: number,
	w: number,
	h: number,
): GeometriaArte {
	const { rw, rh, hx, hy } = cubrir(f, { w, h, iw, ih });
	const dx = limitar(f.offsetX) * hx;
	const dy = limitar(f.offsetY) * hy;
	// Centro de la imagen y, de ahí, su esquina (sin girar).
	const izq = w / 2 - dx - rw / 2;
	const arriba = h / 2 - dy - rh / 2;
	return {
		ancho: (rw / w) * 100,
		alto: (rh / h) * 100,
		tx: (izq / rw) * 100,
		ty: (arriba / rh) * 100,
		giro: f.rotation,
	};
}
