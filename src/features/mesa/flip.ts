import { type RefObject, useLayoutEffect } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import { useRepeticion } from "@/stores/repeticion-store";
import { useSinAnimaciones } from "./fps";

/**
 * **Animar el cambio de sitio de una carta** (FLIP): se apunta dónde estaba
 * cada carta en pantalla la última vez que se pintó y, si aparece en otro
 * sitio (otra zona, otra posición), se anima desde allí. Vale también para
 * lo que mueve el motor (robar, moler, organizar). Con «Reducir
 * animaciones», nada.
 *
 * Los sitios se apuntan por **ámbito**: el tablero grande y cada tipo de
 * miniatura tienen su escala, y un sitio de uno no sirve en el otro.
 */
const ultimas = new Map<string, { x: number; y: number; t: number }>();
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

const clave = (ambito: string, id: IdCarta) => `${ambito}:${id}`;

/**
 * Olvida dónde estaba cada carta: tras cambiar de tablero o saltar en una
 * repetición, lo de antes no es de donde vienen las cartas de ahora.
 */
export function olvidarSitios() {
	ultimas.clear();
}

/**
 * Cuánto dura (ms) deslizar una carta en una repetición: 0 si no hay
 * repetición, si el paso es un salto grande o si el sistema pide no animar.
 * Rust manda la partida de cada punto sin el movimiento de por medio; lo suave
 * lo pone la pantalla.
 */
export function useDeslizar(): number {
	const repitiendo = useRepeticion((s) => s.activa !== null);
	const ms = useRepeticion((s) => s.animarMs);
	const reducir = useSinAnimaciones();
	return repitiendo && !reducir ? ms : 0;
}

export type OpcionesFlip = {
	/** Otra duración (la repetición desliza más despacio que la partida). */
	duracion?: number;
	/** De qué escala es el sitio (`""`: el tablero grande). */
	ambito?: string;
	/**
	 * El sitio apuntado no caduca: en una repetición una carta puede llevar
	 * mucho quieta y su paso al cementerio también tiene que verse.
	 */
	sinCaducidad?: boolean;
};

export function useFlip(
	id: IdCarta,
	ref: RefObject<HTMLElement | null>,
	activo: boolean,
	clavePosicion: unknown,
	{ duracion = DURACION, ambito = "", sinCaducidad = false }: OpcionesFlip = {},
) {
	useLayoutEffect(() => {
		const el = ref.current;
		if (!el) return;
		const r = el.getBoundingClientRect();
		const ahora = { x: r.left + r.width / 2, y: r.top + r.height / 2, t: performance.now() };
		const k = clave(ambito, id);
		const antes = ultimas.get(k);
		ultimas.set(k, ahora);
		const arrastrada = arrastradas.get(id);
		if (arrastrada !== undefined) {
			if (ahora.t - arrastrada <= GRACIA_ARRASTRE_MS) {
				arrastradas.delete(id);
				return;
			}
			arrastradas.delete(id);
		}
		if (!activo || !antes || (!sinCaducidad && ahora.t - antes.t > CADUCA_MS)) return;
		const dx = antes.x - ahora.x;
		const dy = antes.y - ahora.y;
		if (Math.abs(dx) < 2 && Math.abs(dy) < 2) return;
		el.animate([{ translate: `${dx}px ${dy}px` }, { translate: "0 0" }], {
			duration: duracion,
			easing: duracion === DURACION ? "cubic-bezier(.2,.8,.2,1)" : "ease-in-out",
		});
	}, [id, activo, clavePosicion, ref, duracion, ambito, sinCaducidad]);
}

/** Apunta el sitio de una carta que se va a dejar de pintar (para la siguiente). */
export function apuntar(id: IdCarta, el: HTMLElement | null) {
	if (!el) return;
	const r = el.getBoundingClientRect();
	ultimas.set(clave("", id), {
		x: r.left + r.width / 2,
		y: r.top + r.height / 2,
		t: performance.now(),
	});
}
