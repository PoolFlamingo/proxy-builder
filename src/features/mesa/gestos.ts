import type { Disposicion, Rect } from "./disposicion";

/**
 * **Los gestos de la mesa con el dedo**, sin React (el hook y los componentes
 * son finos y viven en `tablero.tsx`). Lo que se mide aquí es lo mismo con el
 * ratón y con el dedo, por los `pointer events`; solo cambian los umbrales.
 */

/** Cuánto se mueve el dedo antes de que sea un arrastre, en px (plan del móvil, B6.0). */
export const UMBRAL_ARRASTRE_DEDO = 8;
/** Lo mismo con el ratón o el lápiz de una tableta con puntero fino. */
export const UMBRAL_ARRASTRE_RATON = 5;

/** El umbral de arrastre según con qué se toca (`PointerEvent.pointerType`). */
export function umbralDe(tipo: string): number {
	return tipo === "touch" ? UMBRAL_ARRASTRE_DEDO : UMBRAL_ARRASTRE_RATON;
}

/** ¿Se ha movido lo bastante para que sea un arrastre y no un toque? */
export function esArrastre(tipo: string, dx: number, dy: number): boolean {
	return Math.hypot(dx, dy) >= umbralDe(tipo);
}

// ── Doble toque ──────────────────────────────────────────────────────────

/** Lo que pueden separarse dos toques en el tiempo para ser un doble toque, en ms. */
export const MS_DOBLE_TOQUE = 300;
/** Y en el espacio, en px. */
export const PX_DOBLE_TOQUE = 30;

export interface DetectorDobleToque {
	/**
	 * Apunta un toque sobre `id` y dice si es el segundo de un doble toque.
	 * Tras un doble toque, el siguiente empieza de cero (tres no son dos dobles).
	 */
	toque(id: number, tMs: number, x: number, y: number): boolean;
	olvidar(): void;
}

export function crearDobleToque(ms = MS_DOBLE_TOQUE, px = PX_DOBLE_TOQUE): DetectorDobleToque {
	let ultimo: { id: number; t: number; x: number; y: number } | null = null;
	return {
		toque(id, t, x, y) {
			const u = ultimo;
			if (u && u.id === id && t - u.t <= ms && Math.hypot(x - u.x, y - u.y) <= px) {
				ultimo = null;
				return true;
			}
			ultimo = { id, t, x, y };
			return false;
		},
		olvidar() {
			ultimo = null;
		},
	};
}

// ── Mover y acercar el tablero con dos dedos ─────────────────────────────

export type Punto = { x: number; y: number };

/**
 * Cómo se ve el campo: el zoom `s` (1 = entero) y cuánto se ha corrido
 * (`tx`, `ty`, en px de pantalla, siempre ≤ 0: el campo ampliado nunca deja
 * un hueco al borde).
 */
export type VistaCampo = { s: number; tx: number; ty: number };

export const VISTA_ENTERA: VistaCampo = { s: 1, tx: 0, ty: 0 };
export const ZOOM_MIN = 1;
export const ZOOM_MAX = 3;

export function esVistaEntera(v: VistaCampo): boolean {
	return v.s === 1 && v.tx === 0 && v.ty === 0;
}

/** Recorta la vista: zoom entre `ZOOM_MIN` y `ZOOM_MAX` y el campo siempre cubriendo `w × h`. */
export function limitarVista(v: VistaCampo, w: number, h: number): VistaCampo {
	const s = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, v.s));
	const tx = Math.max(w - w * s, Math.min(0, v.tx));
	const ty = Math.max(h - h * s, Math.min(0, v.ty));
	return { s, tx, ty };
}

const medio = (a: Punto, b: Punto): Punto => ({ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 });
const distancia = (a: Punto, b: Punto): number => Math.hypot(b.x - a.x, b.y - a.y);

/**
 * La vista tras mover dos dedos: el punto del campo que estaba bajo el medio
 * de los dedos al empezar sigue bajo el medio de los dedos ahora (se acerca o
 * aleja con lo que se separan y se corre con lo que se mueven). Los puntos
 * van relativos a la esquina del campo.
 */
export function vistaDeDosDedos(
	inicio: { vista: VistaCampo; a: Punto; b: Punto },
	ahora: { a: Punto; b: Punto },
	w: number,
	h: number,
): VistaCampo {
	const d0 = distancia(inicio.a, inicio.b);
	const k = d0 < 1 ? 1 : distancia(ahora.a, ahora.b) / d0;
	const s = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, inicio.vista.s * k));
	const kReal = s / inicio.vista.s;
	const m0 = medio(inicio.a, inicio.b);
	const m1 = medio(ahora.a, ahora.b);
	return limitarVista(
		{
			s,
			tx: m1.x - kReal * (m0.x - inicio.vista.tx),
			ty: m1.y - kReal * (m0.y - inicio.vista.ty),
		},
		w,
		h,
	);
}

/**
 * El `transform` de CSS (origen arriba a la izquierda) que lleva lo pintado
 * con la vista `de` a como se vería con la vista `a`: se usa mientras los
 * dedos se mueven, para no repintar todas las cartas en cada fotograma.
 */
export function transformEntre(de: VistaCampo, a: VistaCampo): string {
	const k = a.s / de.s;
	return `translate(${a.tx - k * de.tx}px, ${a.ty - k * de.ty}px) scale(${k})`;
}

/**
 * La disposición del tablero vista con `v`: `campo` pasa a ser el campo
 * ampliado y corrido (todo lo que convierte posiciones del motor a píxeles
 * y al revés sigue valiendo sin saber del zoom), `carta` crece con él y
 * `visible` guarda el hueco que se ve de verdad (donde se suelta algo en el
 * campo y lo que se recorta).
 */
export function conVista(d: Disposicion, v: VistaCampo): Disposicion {
	if (esVistaEntera(v)) return d;
	const campo: Rect = {
		x: d.campo.x + v.tx,
		y: d.campo.y + v.ty,
		w: d.campo.w * v.s,
		h: d.campo.h * v.s,
	};
	return {
		...d,
		visible: d.campo,
		campo,
		carta: { w: d.carta.w * v.s, h: d.carta.h * v.s },
	};
}

// ── El clic que llega al soltar tras abrir un menú ───────────────────────

export interface FiltroClicTrasMenu {
	/** Se abrió un menú contextual (clic derecho o pulsación larga). */
	menu(): void;
	/** Empieza otro toque o pulsación: el siguiente clic ya es suyo. */
	bajar(): void;
	/** Llega un clic: `true` si hay que descartarlo (es el que cierra la pulsación larga). */
	clic(): boolean;
}

/**
 * Mantener el dedo sobre algo que también hace algo al tocarlo (la biblioteca
 * roba, una miniatura cambia de tablero, un `−` resta) abre el menú pero, al
 * soltar, llega además el clic. Ese clic no se quiere: se descarta el primero
 * que llega tras abrir un menú sin otro toque de por medio.
 */
export function crearFiltroClicTrasMenu(): FiltroClicTrasMenu {
	let pendiente = false;
	return {
		menu() {
			pendiente = true;
		},
		bajar() {
			pendiente = false;
		},
		clic() {
			const era = pendiente;
			pendiente = false;
			return era;
		},
	};
}
