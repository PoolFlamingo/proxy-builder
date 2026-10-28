import { useEffect } from "react";
import { useUi } from "@/stores/ui-store";
import { CapaAlta } from "../capa-alta";
import { ANCHO_PANEL, usePantalla } from "../pantalla";
import { CartaFantasma } from "./carta-fantasma";

/**
 * **La carta del fantasma en grande.** Las cartas del fantasma no están en la
 * mesa (no tienen `IdCarta`), así que no pasan por `VistaGrande`: se señalan
 * por su nombre (`usePantalla.fantasma`) desde el aviso de la reacción, el
 * registro y la lista del botón del fantasma, y esta vista las enseña al
 * tamaño de la vista grande de Ajustes, al mismo lado que aquella y por
 * encima del aviso.
 */
export function VistaFantasma() {
	const nombre = usePantalla((s) => s.fantasma);
	const panelLateral = usePantalla((s) => s.panel !== null);
	const ancho = useUi((s) => s.mesa.vistaGrande);
	if (nombre === null) return null;
	return (
		<CapaAlta>
			<div
				className="pointer-events-none absolute top-14 right-3 overflow-hidden rounded-[4.5%] shadow-2xl"
				style={{
					width: ancho,
					height: (ancho * 88) / 63,
					...(panelLateral ? { right: ANCHO_PANEL + 20 } : {}),
				}}
			>
				<CartaFantasma nombre={nombre} ancho={488} className="size-full" />
			</div>
		</CapaAlta>
	);
}

/**
 * Los manejadores para señalar la carta de un fantasma por su nombre. Si lo
 * señalado desaparece (la reacción se resuelve) sin soltar el puntero, la vista
 * se quita sola.
 */
export function useSenalarFantasma(nombre: string) {
	useEffect(
		() => () => {
			if (usePantalla.getState().fantasma === nombre) usePantalla.getState().setFantasma(null);
		},
		[nombre],
	);
	return {
		onPointerEnter: () => usePantalla.getState().setFantasma(nombre),
		onPointerLeave: () => {
			if (usePantalla.getState().fantasma === nombre) usePantalla.getState().setFantasma(null);
		},
	};
}
