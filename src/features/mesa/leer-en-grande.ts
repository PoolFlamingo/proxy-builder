import { type PointerEvent, useEffect, useMemo } from "react";
import type { IdCarta } from "@/bindings/IdCarta";
import { useMesa } from "@/stores/mesa-store";

/**
 * **Leer una carta en grande** (la vista grande, `vista-grande.tsx`). Con el
 * ratón o el lápiz es entrar y salir de la carta; con el dedo no hay «encima»,
 * así que es un toque: la carta tocada se queda en grande hasta que se toca
 * otra, se toca fuera o se pulsa su botón de cerrar. Los 11 sitios de la
 * mesa que la enseñan pasan por aquí, así que tocar y señalar se comportan
 * igual en todos.
 */

export type EventoLeer = "entra" | "sale" | "toque";

/** Los punteros con los que se puede estar «encima» de una carta. */
export function hayEncima(tipoPuntero: string): boolean {
	return tipoPuntero === "mouse" || tipoPuntero === "pen";
}

/**
 * La carta en grande tras un evento sobre `id`: `undefined` si no cambia nada.
 * Salir solo la quita si es ella la que está (si ya se pasó a otra, no).
 */
export function sobreTras<T extends number | string>(
	actual: T | null,
	id: T,
	evento: EventoLeer,
	tipoPuntero: string,
): T | null | undefined {
	if (hayEncima(tipoPuntero)) {
		if (evento === "entra") return id;
		if (evento === "sale") return actual === id ? null : undefined;
		return undefined;
	}
	// Con el dedo, entrar y salir son bajar y subir el dedo: solo vale el toque.
	return evento === "toque" ? id : undefined;
}

function aplicar(id: IdCarta, evento: EventoLeer, tipo: string) {
	const m = useMesa.getState();
	const nuevo = sobreTras(m.sobre, id, evento, tipo);
	if (nuevo !== undefined && nuevo !== m.sobre) m.setSobre(nuevo);
}

export type ManejadoresLeer = {
	// táctil: un toque (`onPointerUp`); entrar y salir son del ratón.
	onPointerEnter: (e: PointerEvent) => void;
	onPointerLeave: (e: PointerEvent) => void;
	onPointerUp: (e: PointerEvent) => void;
	/** Marca para que tocar fuera de lo que se lee no la cierre al tocarlo a ella. */
	"data-lee": "";
};

const SIN_TOQUE = (): void => {};

/**
 * Los manejadores para leer `id` en grande. `conToque: false` para lo que ya
 * tiene su propio gesto de toque (las cartas del tablero: tocar, mantener o
 * arrastrar son tres cosas distintas, `tablero.tsx`).
 */
export function manejadoresLeer(id: IdCarta, conToque = true): ManejadoresLeer {
	return {
		// táctil: un toque (`onPointerUp` con el dedo); entrar y salir son del ratón.
		onPointerEnter: (e) => aplicar(id, "entra", e.pointerType),
		onPointerLeave: (e) => aplicar(id, "sale", e.pointerType),
		onPointerUp: conToque ? (e) => aplicar(id, "toque", e.pointerType) : SIN_TOQUE,
		"data-lee": "",
	};
}

/** `manejadoresLeer` para un componente con una carta fija (o ninguna: `null`). */
export function useLeerEnGrande(id: IdCarta | null, conToque = true): ManejadoresLeer | undefined {
	return useMemo(() => (id === null ? undefined : manejadoresLeer(id, conToque)), [id, conToque]);
}

/** Quita la carta que se lee en grande (el botón de cerrar y el toque fuera). */
export function cerrarLectura() {
	useMesa.getState().setSobre(null);
}

/** ¿Es lo tocado algo que ya sabe leerse en grande (y no debe cerrarse)? */
export const SELECTOR_LEIBLE = "[data-lee],[data-carta],[data-lee-cierra]";

/**
 * Con el dedo, tocar fuera de lo que se lee cierra la lectura. Los toques
 * sobre algo que se lee (`SELECTOR_LEIBLE`) no la cierran: la cambian ellos.
 */
export function useCerrarAlTocarFuera(abierto: boolean, cerrar: () => void) {
	useEffect(() => {
		if (!abierto) return;
		const alBajar = (e: globalThis.PointerEvent) => {
			if (hayEncima(e.pointerType)) return;
			if (e.target instanceof Element && e.target.closest(SELECTOR_LEIBLE)) return;
			cerrar();
		};
		window.addEventListener("pointerdown", alBajar, true);
		return () => window.removeEventListener("pointerdown", alBajar, true);
	}, [abierto, cerrar]);
}

/** Lo que se deja libre arriba (la barra de la mesa) y abajo en la pantalla, en px. */
const MARGEN_VERTICAL = 64;

/**
 * El ancho de la vista grande que cabe en una ventana de `altoVentana` px de
 * alto: el de Ajustes, salvo que la carta (63 × 88) no quepa de arriba abajo
 * (un móvil en horizontal mide ~411). `reserva`: lo que se necesita debajo
 * para el panel de estado. Nunca baja de 120 px.
 */
export function anchoDeLectura(ancho: number, altoVentana: number, reserva = 0): number {
	const cabe = ((altoVentana - MARGEN_VERTICAL - reserva) * 63) / 88;
	return Math.max(120, Math.min(ancho, Math.floor(cabe)));
}
