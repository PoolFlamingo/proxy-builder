import { useEffect, useRef, useState } from "react";
import { altoVisible, esCampoDeTexto } from "./altura";

/**
 * Para la pantalla de Diseñar del móvil: el alto que se ve del contenedor
 * (`ref`, que ocupa el sitio y tiene su alto) y si se está escribiendo en un
 * campo. Con el teclado abierto el alto se recorta al viewport visual y el
 * campo enfocado se trae a la vista. Fuera del móvil no se usa (`activo`
 * falso) y no se escucha nada.
 */
export function useAltoVisible(activo: boolean) {
	const ref = useRef<HTMLDivElement>(null);
	const [alto, setAlto] = useState<number | null>(null);
	const [escribiendo, setEscribiendo] = useState(false);

	useEffect(() => {
		const el = ref.current;
		if (!activo || !el) return;
		const vv = window.visualViewport;
		const medir = () => {
			setAlto(
				altoVisible({
					contenedor: el.clientHeight,
					arriba: el.getBoundingClientRect().top,
					visualAlto: vv?.height ?? window.innerHeight,
					visualArriba: vv?.offsetTop ?? 0,
				}),
			);
		};
		// El campo enfocado se trae a la vista cuando el teclado acaba de salir.
		const traerCampo = () => {
			const a = document.activeElement;
			if (a instanceof HTMLElement && el.contains(a) && esCampoDeTexto(a)) {
				a.scrollIntoView({ block: "center", behavior: "smooth" });
			}
		};
		// Pasar de un campo a otro no debe encoger y estirar la carta.
		let fin: number | undefined;
		const alEnfocar = (e: FocusEvent) => {
			window.clearTimeout(fin);
			if (e.target instanceof HTMLElement && esCampoDeTexto(e.target)) setEscribiendo(true);
		};
		const alDesenfocar = () => {
			window.clearTimeout(fin);
			fin = window.setTimeout(() => setEscribiendo(false), 150);
		};
		medir();
		const ro = new ResizeObserver(medir);
		ro.observe(el);
		const alCambiarVisual = () => {
			medir();
			traerCampo();
		};
		vv?.addEventListener("resize", alCambiarVisual);
		vv?.addEventListener("scroll", medir);
		window.addEventListener("resize", medir);
		el.addEventListener("focusin", alEnfocar);
		el.addEventListener("focusout", alDesenfocar);
		return () => {
			window.clearTimeout(fin);
			ro.disconnect();
			vv?.removeEventListener("resize", alCambiarVisual);
			vv?.removeEventListener("scroll", medir);
			window.removeEventListener("resize", medir);
			el.removeEventListener("focusin", alEnfocar);
			el.removeEventListener("focusout", alDesenfocar);
		};
	}, [activo]);

	return { ref, alto, escribiendo };
}
