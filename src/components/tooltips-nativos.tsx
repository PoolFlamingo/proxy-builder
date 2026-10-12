/**
 * **Ningún `title` nativo llega a pintarse.** Red de seguridad para toda la
 * ventana: cualquier elemento que aparezca con atributo `title` —escrito a
 * mano, puesto por una librería, o que venga dentro del HTML de la
 * documentación— lo pierde en el acto y su texto se enseña con el `Tooltip`
 * de la casa, el mismo componente y el mismo retardo que el resto.
 *
 * 🛑 Por qué hace falta, además de no escribirlos: el tooltip nativo en la
 * ventana de Linux (WebKitGTK) lo pinta GTK con los colores del tema del
 * sistema —en el Mint de Pablo, un rectángulo morado sin bordes redondos—, así
 * que se lee como si fuera de otra aplicación. Se barrieron todos a mano el
 * 25/09/2026 y al día siguiente ya habían vuelto cuatro (26/09/2026, Pablo:
 * «haz algo para que de forma programática todos se conviertan
 * automáticamente nada más aparezcan»). Esta pieza es la mitad de ese
 * «algo»; la otra es `scripts/sin-title-nativo.mjs`, que no deja escribirlos
 * (va en `npm run check` y en el pre-commit).
 *
 * Cómo funciona:
 *
 * - Un `MutationObserver` sobre el `body` mira atributos `title` y nodos
 *   nuevos. Al ver uno, lo mueve a `data-titulo-nativo` —el navegador ya no
 *   tiene nada que enseñar— y, si el elemento se quedaba sin nombre accesible
 *   (un botón con solo un icono), lo pasa también a `aria-label`.
 * - React no se entera ni le importa: compara contra sus props, no contra el
 *   DOM. Si el valor cambia, lo vuelve a escribir y el observador lo vuelve a
 *   mover.
 * - Un solo `Tooltip` para toda la app, abierto a mano: su disparador es un
 *   `span` invisible con posición fija encima del elemento que tiene el ratón.
 *   Así el globo es EXACTAMENTE el de `components/ui/tooltip.tsx`, con su
 *   flecha, sus colores y su animación.
 *
 * Coste: el observador corre en microtareas agrupadas y solo busca `[title]`
 * dentro de lo que acaba de entrar. Al hacer scroll en una tabla virtualizada
 * eso son unas pocas filas, sin tocar el resto del documento (ver
 * `judlex-scroll-webkitgtk` en la memoria del proyecto).
 *
 * Excepciones: `iframe`/`frame`, donde `title` es el nombre accesible del
 * marco y no un tooltip (`visor-entregable.tsx`).
 */

import { useEffect, useRef, useState } from "react";

import { RETARDO_TOOLTIPS, Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

const ATRIBUTO = "data-titulo-nativo";
const EXCEPCIONES = new Set(["IFRAME", "FRAME"]);

/** Quita el `title` de UN elemento y lo guarda donde el navegador no lo ve. */
function desactivar(el: Element) {
	if (EXCEPCIONES.has(el.tagName)) return;
	const texto = el.getAttribute("title");
	if (texto === null) return;
	el.removeAttribute("title");
	if (!texto.trim()) return;
	el.setAttribute(ATRIBUTO, texto);
	if (
		!el.hasAttribute("aria-label") &&
		!el.hasAttribute("aria-labelledby") &&
		!el.textContent?.trim()
	) {
		el.setAttribute("aria-label", texto);
	}
	if (import.meta.env.DEV) {
		// Para que se note programando: cada uno de estos es un `title` que
		// alguien escribió y que debería ser un `<Tooltip>` de verdad.
		console.warn("[tooltips-nativos] `title` convertido en Tooltip:", texto, el);
	}
}

/** Él mismo y todo lo que lleva dentro. */
function desactivarEnArbol(raiz: Element) {
	desactivar(raiz);
	for (const el of raiz.querySelectorAll("[title]")) desactivar(el);
}

interface Activo {
	el: Element;
	texto: string;
	caja: DOMRect;
}

export function TooltipsNativos() {
	const [activo, setActivo] = useState<Activo | null>(null);
	const pendiente = useRef<number | null>(null);
	const actual = useRef<Element | null>(null);

	// 1) Que no quede ni uno en el DOM.
	useEffect(() => {
		desactivarEnArbol(document.body);
		const observador = new MutationObserver((cambios) => {
			for (const c of cambios) {
				if (c.type === "attributes") {
					desactivar(c.target as Element);
					continue;
				}
				for (const nodo of c.addedNodes) {
					if (nodo.nodeType === Node.ELEMENT_NODE) desactivarEnArbol(nodo as Element);
				}
			}
		});
		observador.observe(document.body, {
			subtree: true,
			childList: true,
			attributes: true,
			attributeFilter: ["title"],
		});
		return () => observador.disconnect();
	}, []);

	// 2) Enseñar su texto con el Tooltip de la casa.
	useEffect(() => {
		const cancelarPendiente = () => {
			if (pendiente.current !== null) {
				window.clearTimeout(pendiente.current);
				pendiente.current = null;
			}
		};
		const cerrar = () => {
			cancelarPendiente();
			actual.current = null;
			setActivo(null);
		};
		const alEntrar = (e: PointerEvent) => {
			const destino = e.target instanceof Element ? e.target : null;
			const el = destino?.closest(`[${ATRIBUTO}]`) ?? null;
			if (el === actual.current) return;
			cerrar();
			if (!el) return;
			actual.current = el;
			pendiente.current = window.setTimeout(() => {
				pendiente.current = null;
				const texto = el.getAttribute(ATRIBUTO);
				if (!texto || !el.isConnected || actual.current !== el) return;
				setActivo({ el, texto, caja: el.getBoundingClientRect() });
			}, RETARDO_TOOLTIPS);
		};
		// Salir de la ventana: el `pointerover` del siguiente elemento no llega.
		const alSalir = (e: PointerEvent) => {
			if (e.relatedTarget === null) cerrar();
		};
		document.addEventListener("pointerover", alEntrar, { passive: true });
		document.addEventListener("pointerout", alSalir, { passive: true });
		// Como el Tooltip de Radix: se va al pulsar, al hacer scroll o al perder
		// la ventana el foco. Con la caja fija, si no se fuera se quedaría
		// flotando donde ya no está su elemento.
		document.addEventListener("pointerdown", cerrar, { passive: true });
		window.addEventListener("scroll", cerrar, { capture: true, passive: true });
		window.addEventListener("blur", cerrar);
		return () => {
			cancelarPendiente();
			document.removeEventListener("pointerover", alEntrar);
			document.removeEventListener("pointerout", alSalir);
			document.removeEventListener("pointerdown", cerrar);
			window.removeEventListener("scroll", cerrar, { capture: true });
			window.removeEventListener("blur", cerrar);
		};
	}, []);

	const caja = activo?.caja;
	return (
		<Tooltip
			open={activo !== null}
			onOpenChange={(abierto) => {
				if (!abierto) {
					actual.current = null;
					setActivo(null);
				}
			}}
		>
			<TooltipTrigger asChild>
				<span
					aria-hidden
					style={{
						position: "fixed",
						left: caja?.left ?? 0,
						top: caja?.top ?? 0,
						width: caja?.width ?? 0,
						height: caja?.height ?? 0,
						pointerEvents: "none",
					}}
				/>
			</TooltipTrigger>
			<TooltipContent className="max-w-96">{activo?.texto}</TooltipContent>
		</Tooltip>
	);
}
