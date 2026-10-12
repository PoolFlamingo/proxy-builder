import { useEffect } from "react";

const MARCOS = new Set(["IFRAME", "EMBED", "OBJECT"]);

/**
 * **Que `Esc` no se pierda dentro de un visor** (Pablo, 02/10/2026).
 *
 * Un `<embed>` de PDF o un `<iframe>` es otro documento: cuando se hace clic
 * dentro se queda con el foco y SUS teclas ya no llegan a esta página, así que
 * el `Esc` del diálogo deja de funcionar «hasta que haces clic en el modal,
 * fuera del visualizador».
 *
 * No se puede oír una tecla que se pulsa dentro del visor, pero sí saber CUÁNDO
 * se lo ha llevado (la ventana pierde el foco y el elemento activo es el
 * marco) y devolverlo en cuanto el puntero sale de él. Mientras el puntero está
 * sobre el PDF el foco es suyo —las flechas, `Ctrl+F` y el cajón de búsqueda
 * del propio visor siguen funcionando—; en cuanto se mueve al título, al borde
 * o al pie del diálogo, `Esc` vuelve a cerrarlo, sin hacer clic.
 *
 * Dentro de un marco el puntero no genera eventos aquí: cualquier movimiento
 * que llega al diálogo ya es fuera de él.
 */
export function useFocoEnMarcos(el: HTMLElement | null) {
	useEffect(() => {
		if (!el) return;
		let marco: HTMLElement | null = null;

		const alPerderLaVentanaElFoco = () => {
			// El elemento activo se actualiza justo después del `blur`.
			window.setTimeout(() => {
				const a = document.activeElement;
				marco = a instanceof HTMLElement && MARCOS.has(a.tagName) && el.contains(a) ? a : null;
			}, 0);
		};
		const alVolverElPuntero = (ev: Event) => {
			if (!marco) return;
			if (ev.target === marco || marco.contains(ev.target as Node)) return;
			const m = marco;
			marco = null;
			m.blur();
			el.focus({ preventScroll: true });
		};

		window.addEventListener("blur", alPerderLaVentanaElFoco);
		el.addEventListener("pointermove", alVolverElPuntero);
		el.addEventListener("pointerdown", alVolverElPuntero);
		return () => {
			window.removeEventListener("blur", alPerderLaVentanaElFoco);
			el.removeEventListener("pointermove", alVolverElPuntero);
			el.removeEventListener("pointerdown", alVolverElPuntero);
		};
	}, [el]);
}
