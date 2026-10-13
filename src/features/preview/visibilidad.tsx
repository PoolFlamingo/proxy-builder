import { createContext, useContext, useEffect, useState } from "react";

/**
 * **Solo lo que se ve tiene imágenes** (petición de Pablo). Cada página mira
 * si toca el viewport del `ScrollArea` (más una pantalla por arriba y otra
 * por abajo); si no, se queda con las cajas y sin `<img>`, así WebKit suelta
 * los mapas de bits y Rust solo procesa lo visible.
 *
 * 🛑 El `root` es el VIEWPORT del `ScrollArea`, no el documento: el
 * documento no se desplaza nunca y todas las páginas «se verían».
 */
export const RaizScroll = createContext<HTMLElement | null>(null);

export function useVisible(el: HTMLElement | null): boolean {
	const raiz = useContext(RaizScroll);
	const [visible, setVisible] = useState(false);
	useEffect(() => {
		if (!el || !raiz) return;
		const obs = new IntersectionObserver(
			(entradas) => {
				for (const e of entradas) setVisible(e.isIntersecting);
			},
			{ root: raiz, rootMargin: "100% 0px" },
		);
		obs.observe(el);
		return () => obs.disconnect();
	}, [el, raiz]);
	return visible;
}
