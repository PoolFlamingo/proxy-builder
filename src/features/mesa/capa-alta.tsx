import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

/**
 * **Una capa por encima de los modales.** La mesa es un contexto de apilado
 * propio (`isolate`), así que nada de lo que lleva dentro, por alto que sea su
 * `z-index`, pasa por encima de un diálogo (que va por portal a `body`, `z-50`).
 * Las vistas grandes de las cartas tienen que poder leerse con un modal
 * abierto: se pintan en un portal, en una capa fija que cubre justo el hueco
 * de la mesa (se mide el elemento que las contiene), de modo que sus hijos
 * siguen colocándose con `absolute` como si estuvieran dentro de ella. La capa
 * no recibe el puntero.
 */
export function CapaAlta({ children }: { children: React.ReactNode }) {
	const marca = useRef<HTMLSpanElement>(null);
	const [caja, setCaja] = useState<DOMRect | null>(null);
	useLayoutEffect(() => {
		const contenedor = marca.current?.parentElement;
		if (!contenedor) return;
		const medir = () => setCaja(contenedor.getBoundingClientRect());
		medir();
		const observador = new ResizeObserver(medir);
		observador.observe(contenedor);
		window.addEventListener("resize", medir);
		return () => {
			observador.disconnect();
			window.removeEventListener("resize", medir);
		};
	}, []);
	return (
		<>
			<span ref={marca} hidden />
			{caja &&
				createPortal(
					<div
						className="pointer-events-none fixed z-60"
						style={{ left: caja.left, top: caja.top, width: caja.width, height: caja.height }}
					>
						{children}
					</div>,
					document.body,
				)}
		</>
	);
}
