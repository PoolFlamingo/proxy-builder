import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";

/** Un paso de arrastre (en fracciones del ancho y del alto de la carta) o de rueda. */
export type CambioEncuadre = { dx?: number; dy?: number; zoom?: number };

/** Medidas de la carta (mm) con las que se cuentan los arrastres. */
export const CARTA_MM = { w: 63, h: 88 };

/** El hueco de la ilustración (mm) y la proporción de la imagen. */
export type HuecoArte = { w: number; h: number; iw: number; ih: number };

/**
 * Cómo cubre la imagen el hueco (la cuenta de `pintar_arte` en Rust): su
 * tamaño sin girar (`rw`, `rh`) y cuánto sobra a cada lado de su caja girada
 * (`hx`, `hy`), que es lo que recorre el desplazamiento de -1 a 1.
 */
function cubrir(f: ArtFraming, { w, h, iw, ih }: HuecoArte) {
	const a = (f.rotation * Math.PI) / 180;
	const c = Math.abs(Math.cos(a));
	const s = Math.abs(Math.sin(a));
	const zoom = Math.max(1, f.zoom);
	const cubre = Math.max((w * c + h * s) / iw, (w * s + h * c) / ih) * zoom;
	const rw = iw * cubre;
	const rh = ih * cubre;
	return {
		rw,
		rh,
		hx: Math.max(0, (rw * c + rh * s - w) / 2),
		hy: Math.max(0, (rw * s + rh * c - h) / 2),
	};
}

const limitar = (v: number) => Math.min(1, Math.max(-1, v));

/**
 * El encuadre tras arrastrar `dx`/`dy` (fracciones del ancho y del alto de la
 * carta) o girar la rueda. La ilustración sigue al ratón 1:1, tenga el zoom
 * o el giro que tenga, hasta que llega a un borde.
 */
export function moverEncuadre(f: ArtFraming, c: CambioEncuadre, hueco: HuecoArte): ArtFraming {
	const zoom = Math.min(3, Math.max(1, f.zoom + (c.zoom ?? 0)));
	const { hx, hy } = cubrir({ ...f, zoom }, hueco);
	// Lo que se mueve el ratón, en mm; el desplazamiento recorre la holgura.
	const mx = (c.dx ?? 0) * CARTA_MM.w;
	const my = (c.dy ?? 0) * CARTA_MM.h;
	return {
		...f,
		zoom,
		offsetX: hx > 1e-6 ? limitar(f.offsetX - mx / hx) : f.offsetX,
		offsetY: hy > 1e-6 ? limitar(f.offsetY - my / hy) : f.offsetY,
	};
}

/**
 * Lo que identifica la capa de un diseño (`design_overlay`): todo menos el
 * encuadre de las ilustraciones. Mientras solo cambia el encuadre, la capa
 * sirve.
 */
export function claveCapa(d: CardDesign, cara: number, lang: DesignLang): string {
	const sin = {
		...d,
		faces: d.faces.map((f) => (f.art ? { ...f, art: { ...f.art, framing: null } } : f)),
	};
	return `${cara}|${lang}|${JSON.stringify(sin)}`;
}

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
