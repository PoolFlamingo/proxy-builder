import { type RefObject, useLayoutEffect } from "react";
import type { IdCarta } from "@/bindings/IdCarta";

/**
 * **Animar el cambio de sitio de una carta** (FLIP): se apunta dónde estaba
 * cada carta en pantalla la última vez que se pintó y, si aparece en otro
 * sitio (otra zona, otra posición), se anima desde allí. Vale también para
 * lo que mueve el motor (robar, moler, organizar). Con «Reducir
 * animaciones», nada.
 */
const ultimas = new Map<IdCarta, { x: number; y: number; t: number }>();
const DURACION = 180;
/**
 * Las cartas que se acaban de arrastrar (aquí o por otro): ya se vieron ir
 * a su sitio, así que al llegar el movimiento no se animan otra vez desde
 * donde estaban.
 */
const arrastradas = new Map<IdCarta, number>();
const GRACIA_ARRASTRE_MS = 1500;

/** Apunta que estas cartas se están arrastrando (o se acaban de soltar). */
export function marcarArrastradas(ids: IdCarta[]) {
	const t = performance.now();
	for (const id of ids) arrastradas.set(id, t);
}
/** Un sitio apuntado hace más de esto ya no cuenta (la carta se fue a una zona oculta). */
const CADUCA_MS = 5000;

export function useFlip(
	id: IdCarta,
	ref: RefObject<HTMLElement | null>,
	activo: boolean,
	clave: unknown,
	/** Otra duración (la repetición desliza más despacio que la partida). */
	duracion = DURACION,
) {
	useLayoutEffect(() => {
		const el = ref.current;
		if (!el) return;
		const r = el.getBoundingClientRect();
		const ahora = { x: r.left + r.width / 2, y: r.top + r.height / 2, t: performance.now() };
		const antes = ultimas.get(id);
		ultimas.set(id, ahora);
		const arrastrada = arrastradas.get(id);
		if (arrastrada !== undefined) {
			if (ahora.t - arrastrada <= GRACIA_ARRASTRE_MS) {
				arrastradas.delete(id);
				return;
			}
			arrastradas.delete(id);
		}
		if (!activo || !antes || ahora.t - antes.t > CADUCA_MS) return;
		const dx = antes.x - ahora.x;
		const dy = antes.y - ahora.y;
		if (Math.abs(dx) < 2 && Math.abs(dy) < 2) return;
		el.animate([{ translate: `${dx}px ${dy}px` }, { translate: "0 0" }], {
			duration: duracion,
			easing: duracion === DURACION ? "cubic-bezier(.2,.8,.2,1)" : "ease-in-out",
		});
	}, [id, activo, clave, ref, duracion]);
}

/** Apunta el sitio de una carta que se va a dejar de pintar (para la siguiente). */
export function apuntar(id: IdCarta, el: HTMLElement | null) {
	if (!el) return;
	const r = el.getBoundingClientRect();
	ultimas.set(id, { x: r.left + r.width / 2, y: r.top + r.height / 2, t: performance.now() });
}
