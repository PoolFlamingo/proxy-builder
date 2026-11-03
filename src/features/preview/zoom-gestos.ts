import { useEffect } from "react";
import { useUi, ZOOM_MAX, ZOOM_MIN } from "@/stores/ui-store";

/**
 * **Zoom con dos dedos** en la vista previa (plan del móvil, M3.3). Las
 * cuentas son puras; el hook solo engancha los eventos táctiles del viewport.
 */

export interface Punto {
	x: number;
	y: number;
}

export function distancia(a: Punto, b: Punto): number {
	return Math.hypot(a.x - b.x, a.y - b.y);
}

export function puntoMedio(a: Punto, b: Punto): Punto {
	return { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
}

/**
 * El zoom tras un pellizco: el de partida por lo que han cambiado las
 * distancias entre los dedos, dentro de los topes y a dos decimales (como
 * `setZoom`).
 */
export function zoomDePellizco(
	zoomInicial: number,
	distanciaInicial: number,
	distanciaAhora: number,
	min = ZOOM_MIN,
	max = ZOOM_MAX,
): number {
	if (distanciaInicial <= 0) return zoomInicial;
	const z = zoomInicial * (distanciaAhora / distanciaInicial);
	return Math.round(Math.max(min, Math.min(max, z)) * 100) / 100;
}

/**
 * Dónde queda el scroll al cambiar el zoom de `antes` a `despues` para que lo
 * que hay bajo `punto` (relativo al viewport) siga bajo los dedos.
 */
export function scrollTrasZoom(scroll: number, punto: number, antes: number, despues: number) {
	return (scroll + punto) * (despues / antes) - punto;
}

interface Pellizco {
	distancia: number;
	zoom: number;
	/** El punto medio al empezar, relativo al viewport. */
	centro: Punto;
	top: number;
	left: number;
}

/**
 * Con dos dedos sobre `viewport`: acerca y aleja guardando el mismo `zoom` del
 * `ui-store` que la rueda y los botones. Con un dedo, el scroll de siempre
 * (`touch-action: pan-x pan-y` apaga el zoom de la página entera sin quitar el
 * desplazamiento).
 */
export function useZoomConDedos(viewport: HTMLElement | null, activo: boolean) {
	useEffect(() => {
		if (!viewport || !activo) return;
		let pellizco: Pellizco | null = null;
		let pendiente = 0;
		let ultimo = 1;
		const previo = viewport.style.touchAction;
		viewport.style.touchAction = "pan-x pan-y";

		const dedos = (e: TouchEvent): [Punto, Punto] | null => {
			const a = e.touches[0];
			const b = e.touches[1];
			if (!a || !b) return null;
			return [
				{ x: a.clientX, y: a.clientY },
				{ x: b.clientX, y: b.clientY },
			];
		};
		const alEmpezar = (e: TouchEvent) => {
			const d = dedos(e);
			if (!d) return;
			const r = viewport.getBoundingClientRect();
			const m = puntoMedio(d[0], d[1]);
			pellizco = {
				distancia: distancia(d[0], d[1]),
				zoom: useUi.getState().zoom,
				centro: { x: m.x - r.left, y: m.y - r.top },
				top: viewport.scrollTop,
				left: viewport.scrollLeft,
			};
			ultimo = pellizco.zoom;
		};
		const colocar = () => {
			pendiente = 0;
			if (!pellizco) return;
			// Dos cuadros: el primero deja que React pinte el zoom nuevo.
			const p = pellizco;
			const z = ultimo;
			requestAnimationFrame(() => {
				viewport.scrollTop = scrollTrasZoom(p.top, p.centro.y, p.zoom, z);
				viewport.scrollLeft = scrollTrasZoom(p.left, p.centro.x, p.zoom, z);
			});
		};
		const alMover = (e: TouchEvent) => {
			const d = dedos(e);
			if (!d || !pellizco) return;
			// Sin esto el navegador desplazaría la hoja con los dos dedos.
			if (e.cancelable) e.preventDefault();
			const z = zoomDePellizco(pellizco.zoom, pellizco.distancia, distancia(d[0], d[1]));
			if (z === ultimo) return;
			ultimo = z;
			useUi.getState().setZoom(z);
			if (pendiente === 0) pendiente = requestAnimationFrame(colocar);
		};
		const alTerminar = (e: TouchEvent) => {
			if (e.touches.length < 2) pellizco = null;
		};

		viewport.addEventListener("touchstart", alEmpezar, { passive: true });
		viewport.addEventListener("touchmove", alMover, { passive: false });
		viewport.addEventListener("touchend", alTerminar, { passive: true });
		viewport.addEventListener("touchcancel", alTerminar, { passive: true });
		return () => {
			viewport.style.touchAction = previo;
			viewport.removeEventListener("touchstart", alEmpezar);
			viewport.removeEventListener("touchmove", alMover);
			viewport.removeEventListener("touchend", alTerminar);
			viewport.removeEventListener("touchcancel", alTerminar);
			if (pendiente !== 0) cancelAnimationFrame(pendiente);
		};
	}, [viewport, activo]);
}
