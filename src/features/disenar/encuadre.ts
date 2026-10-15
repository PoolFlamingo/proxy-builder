import type { ArtFraming } from "@/bindings/ArtFraming";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DesignLang } from "@/bindings/DesignLang";

/** Un paso de arrastre (en fracciones de la carta) o de rueda. */
export type CambioEncuadre = { dx?: number; dy?: number; zoom?: number };

/**
 * El encuadre tras arrastrar `dx`/`dy` (fracciones del ancho y del alto de la
 * carta) o girar la rueda. Arrastrar a la derecha enseña lo que hay a la
 * izquierda, y con más zoom se mueve más despacio.
 */
export function moverEncuadre(f: ArtFraming, c: CambioEncuadre): ArtFraming {
	const k = 2 / Math.max(0.15, 1 - 1 / f.zoom + 0.15);
	return {
		...f,
		zoom: Math.min(3, Math.max(1, f.zoom + (c.zoom ?? 0))),
		offsetX: Math.min(1, Math.max(-1, f.offsetX - (c.dx ?? 0) * k)),
		offsetY: Math.min(1, Math.max(-1, f.offsetY - (c.dy ?? 0) * k)),
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
	const a = (f.rotation * Math.PI) / 180;
	const c = Math.abs(Math.cos(a));
	const s = Math.abs(Math.sin(a));
	const zoom = Math.max(1, f.zoom);
	const cubre = Math.max((w * c + h * s) / iw, (w * s + h * c) / ih) * zoom;
	const rw = iw * cubre;
	const rh = ih * cubre;
	const bw = rw * c + rh * s;
	const bh = rw * s + rh * c;
	const lim = (v: number) => Math.min(1, Math.max(-1, v));
	const dx = lim(f.offsetX) * Math.max(0, (bw - w) / 2);
	const dy = lim(f.offsetY) * Math.max(0, (bh - h) / 2);
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
