import { type PointerEvent, useEffect } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { useFormato } from "@/lib/formato";
import { useUi } from "@/stores/ui-store";
import { CapaAlta } from "../capa-alta";
import { useMesaMovil } from "../formato-mesa";
import { LecturaMovil } from "../lectura-movil";
import {
	anchoDeLectura,
	type EventoLeer,
	sobreTras,
	useCerrarAlTocarFuera,
} from "../leer-en-grande";
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
	const anchoAjustes = useUi((s) => s.mesa.vistaGrande);
	const { t } = useLang();
	const { tactil } = useFormato();
	const movil = useMesaMovil();
	const panelLateral = usePantalla((s) => s.panel !== null) && !movil;
	useCerrarAlTocarFuera(tactil && nombre !== null, cerrarFantasma);
	if (nombre === null) return null;
	if (movil) {
		return (
			<CapaAlta>
				<LecturaMovil
					caras={1}
					cara={() => <CartaFantasma nombre={nombre} ancho={488} className="size-full" contener />}
					aLaIzquierda={false}
					onCerrar={cerrarFantasma}
				/>
			</CapaAlta>
		);
	}
	const ancho = tactil ? anchoDeLectura(anchoAjustes, window.innerHeight) : anchoAjustes;
	return (
		<CapaAlta>
			<div
				className="pointer-events-none absolute top-14 right-3"
				style={{
					width: ancho,
					height: (ancho * 88) / 63,
					...(panelLateral ? { right: ANCHO_PANEL + 20 } : {}),
				}}
			>
				<div className="size-full overflow-hidden rounded-[4.5%] shadow-2xl">
					<CartaFantasma nombre={nombre} ancho={488} className="size-full" />
				</div>
				{tactil && (
					<Button
						size="icon-sm"
						variant="secondary"
						onPointerDown={(e) => e.stopPropagation()}
						onClick={cerrarFantasma}
						aria-label={t("Cerrar")}
						data-lee-cierra=""
						className="pointer-events-auto absolute -top-2 -left-2 rounded-full shadow-lg"
					>
						<Icono icon="lucide:x" className="size-4" />
					</Button>
				)}
			</div>
		</CapaAlta>
	);
}

function cerrarFantasma() {
	usePantalla.getState().setFantasma(null);
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
		// táctil: un toque (`onPointerUp` con el dedo); entrar y salir son del ratón.
		onPointerEnter: (e: PointerEvent) => poner(nombre, "entra", e.pointerType),
		onPointerLeave: (e: PointerEvent) => poner(nombre, "sale", e.pointerType),
		onPointerUp: (e: PointerEvent) => poner(nombre, "toque", e.pointerType),
		"data-lee": "" as const,
	};
}

function poner(nombre: string, evento: EventoLeer, tipo: string) {
	const pantalla = usePantalla.getState();
	const nuevo = sobreTras(pantalla.fantasma, nombre, evento, tipo);
	if (nuevo !== undefined && nuevo !== pantalla.fantasma) pantalla.setFantasma(nuevo);
}
