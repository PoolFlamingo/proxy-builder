import { type ComponentProps, useRef } from "react";
import { ContextMenuContent } from "@/components/ui/context-menu";

/** Lo que se ignoran los punteros tras abrir el menú, en ms. */
const GRACIA_MS = 250;

/**
 * `ContextMenuContent` que ignora el puntero justo después de abrirse.
 *
 * El menú sale bajo el cursor (y se desplaza para caber en la ventana), así que
 * al soltar el botón derecho que lo abrió la opción que quedó debajo recibe un
 * `pointerup` y Radix lo toma por un clic: ejecutaba una acción sin querer.
 * `components/ui/` es de solo lectura, por eso el envoltorio va aquí. El
 * teclado no se toca. El contenido se monta al abrir, así que la hora de
 * montaje es la de apertura.
 */
export function ContenidoContextual(props: ComponentProps<typeof ContextMenuContent>) {
	const abiertoEn = useRef(performance.now());
	const enGracia = () => performance.now() - abiertoEn.current < GRACIA_MS;
	const frenar = (e: { stopPropagation: () => void; preventDefault: () => void }) => {
		if (!enGracia()) return;
		e.stopPropagation();
		e.preventDefault();
	};
	return (
		<ContextMenuContent
			{...props}
			onPointerUpCapture={(e) => {
				frenar(e);
				props.onPointerUpCapture?.(e);
			}}
			onClickCapture={(e) => {
				frenar(e);
				props.onClickCapture?.(e);
			}}
		/>
	);
}
