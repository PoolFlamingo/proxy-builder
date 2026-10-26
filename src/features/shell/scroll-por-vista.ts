import { type RefObject, useEffect, useRef } from "react";
import type { Vista } from "@/stores/ui-store";

/** Lo que se espera a que el contenido de la vista sea lo bastante alto, en ms. */
const ESPERA_MS = 2000;

const VIEWPORT = '[data-slot="scroll-area-viewport"]';

function viewports(raiz: HTMLElement) {
	return Array.from(raiz.querySelectorAll<HTMLElement>(VIEWPORT));
}

/**
 * Recuerda el scroll de cada vista (Deck, Diseñar, Biblioteca) y lo devuelve al
 * volver a ella.
 *
 * Cambiar de vista desmonta la anterior, así que su scroll se perdía. No se
 * mantienen montadas y ocultas: `display: none` también reinicia el scroll, y
 * la vista previa y el diseñador seguirían trabajando por debajo. En su lugar
 * se apunta cada `ScrollArea` de la vista (por su orden dentro de ella) al
 * moverse, y al volver se repone en cuanto el contenido da de sí (las listas y
 * la vista previa llegan después del primer pintado). Si el usuario toca la
 * rueda, el teclado o el ratón antes, se deja de reponer.
 */
export function useScrollPorVista(vista: Vista, raiz: RefObject<HTMLElement | null>) {
	const guardado = useRef(new Map<Vista, Map<number, number>>());

	useEffect(() => {
		const main = raiz.current;
		if (!main) return;
		const alMover = (e: Event) => {
			const el = e.target;
			if (!(el instanceof HTMLElement) || !el.matches(VIEWPORT)) return;
			const i = viewports(main).indexOf(el);
			if (i < 0) return;
			let de = guardado.current.get(vista);
			if (!de) {
				de = new Map();
				guardado.current.set(vista, de);
			}
			de.set(i, el.scrollTop);
		};
		// `scroll` no sube por el DOM: se escucha en la fase de captura.
		main.addEventListener("scroll", alMover, true);

		const pendientes = new Map(guardado.current.get(vista));
		let parado = false;
		let cuadro = 0;
		const parar = () => {
			parado = true;
		};
		const eventos = ["wheel", "pointerdown", "keydown"] as const;
		for (const nombre of eventos) main.addEventListener(nombre, parar, { passive: true });
		const inicio = performance.now();
		const reponer = () => {
			if (parado || pendientes.size === 0) return;
			const todos = viewports(main);
			for (const [i, top] of pendientes) {
				const v = todos[i];
				if (v && v.scrollHeight - v.clientHeight >= top - 1) {
					v.scrollTop = top;
					pendientes.delete(i);
				}
			}
			if (pendientes.size > 0 && performance.now() - inicio < ESPERA_MS) {
				cuadro = requestAnimationFrame(reponer);
			}
		};
		if (pendientes.size > 0) cuadro = requestAnimationFrame(reponer);

		return () => {
			cancelAnimationFrame(cuadro);
			main.removeEventListener("scroll", alMover, true);
			for (const nombre of eventos) main.removeEventListener(nombre, parar);
		};
	}, [vista, raiz]);
}
