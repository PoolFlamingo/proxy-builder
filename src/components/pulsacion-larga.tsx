import type { ComponentProps } from "react";
import { usePulsacionLarga } from "@/lib/pulsacion-larga";

/**
 * Un contenedor que, con el dedo quieto 500 ms, hace lo que haría el clic
 * derecho (`alPulsar` recibe dónde). Donde se usa un `ContextMenu` de Radix
 * basta con él (ya lo hace, a 700 ms); esto es para todo lo demás.
 */
export function PulsacionLarga({
	alPulsar,
	ms,
	tolerancia,
	children,
	...resto
}: Omit<ComponentProps<"div">, "onPointerDown" | "onPointerMove" | "onPointerUp"> & {
	alPulsar: (x: number, y: number) => void;
	ms?: number;
	tolerancia?: number;
}) {
	const manejadores = usePulsacionLarga(alPulsar, { ms, tolerancia });
	return (
		<div {...resto} {...manejadores}>
			{children}
		</div>
	);
}
