import { useEffect } from "react";
import { MS_PULSACION, TOLERANCIA_PULSACION } from "@/lib/pulsacion-larga";

/**
 * En táctil, mantener pulsado un botón que solo lleva un icono enseña su
 * nombre (su `aria-label`) en una etiqueta pequeña encima, como hace Android:
 * el `Tooltip` de Radix no sale con un toque y es la única forma de saber qué
 * hace. Se monta una vez en el shell. `tooltips-nativos.tsx` ya pasa los
 * `title` de esos botones a `aria-label`.
 */
export function EtiquetasTactiles() {
	useEffect(() => {
		let temporizador: ReturnType<typeof setTimeout> | null = null;
		let quitar: ReturnType<typeof setTimeout> | null = null;
		let etiqueta: HTMLDivElement | null = null;
		let x0 = 0;
		let y0 = 0;
		let comerClic = false;

		const borrar = () => {
			if (quitar) clearTimeout(quitar);
			quitar = null;
			etiqueta?.remove();
			etiqueta = null;
		};
		const parar = () => {
			if (temporizador) clearTimeout(temporizador);
			temporizador = null;
		};
		const alBajar = (e: PointerEvent) => {
			parar();
			// Un clic que no llegó (el dedo se movió y fue scroll) no se come el siguiente toque.
			comerClic = false;
			if (e.pointerType === "mouse" || !(e.target instanceof Element)) return;
			// Donde mantener pulsado ya hace algo (un menú, elegir), no se mezcla otro gesto.
			if (e.target.closest('[data-pulsacion-larga], [data-slot="context-menu-trigger"]')) return;
			const boton = e.target.closest<HTMLElement>("button[aria-label], [role=button][aria-label]");
			const nombre = boton?.getAttribute("aria-label");
			if (!boton || !nombre || boton.textContent?.trim()) return;
			x0 = e.clientX;
			y0 = e.clientY;
			temporizador = setTimeout(() => {
				temporizador = null;
				borrar();
				const r = boton.getBoundingClientRect();
				const div = document.createElement("div");
				div.textContent = nombre;
				div.setAttribute("role", "tooltip");
				div.className =
					"pointer-events-none fixed z-[100] max-w-[80vw] -translate-x-1/2 -translate-y-full rounded-md bg-foreground px-2 py-1 text-xs text-background shadow-md";
				div.style.left = `${Math.min(Math.max(r.left + r.width / 2, 48), window.innerWidth - 48)}px`;
				div.style.top = `${Math.max(r.top - 6, 28)}px`;
				document.body.append(div);
				etiqueta = div;
				comerClic = true;
				quitar = setTimeout(borrar, 1800);
			}, MS_PULSACION);
		};
		const alMover = (e: PointerEvent) => {
			if (temporizador && Math.hypot(e.clientX - x0, e.clientY - y0) > TOLERANCIA_PULSACION) {
				parar();
			}
		};
		const alCancelar = () => {
			parar();
			comerClic = false;
		};
		const alClic = (e: MouseEvent) => {
			if (!comerClic) return;
			comerClic = false;
			e.preventDefault();
			e.stopPropagation();
		};
		document.addEventListener("pointerdown", alBajar, { passive: true });
		document.addEventListener("pointermove", alMover, { passive: true });
		document.addEventListener("pointerup", parar, { passive: true });
		document.addEventListener("pointercancel", alCancelar, { passive: true });
		document.addEventListener("click", alClic, true);
		return () => {
			parar();
			borrar();
			document.removeEventListener("pointerdown", alBajar);
			document.removeEventListener("pointermove", alMover);
			document.removeEventListener("pointerup", parar);
			document.removeEventListener("pointercancel", alCancelar);
			document.removeEventListener("click", alClic, true);
		};
	}, []);
	return null;
}
