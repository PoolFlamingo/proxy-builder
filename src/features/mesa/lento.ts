import { useEffect, useRef, useState } from "react";
import { useMesa } from "@/stores/mesa-store";

type EstadoMesa = ReturnType<typeof useMesa.getState>;

/** Cada cuántos ms se mira la mesa para las miniaturas (10 por segundo). */
export const MS_MINIATURA = 100;

/**
 * Como `useMesa(selector)`, pero muestreado: mira la mesa como mucho cada
 * `ms` y solo repinta si lo elegido cambió de verdad según `igual`. Es lo que
 * usan las miniaturas, para que 9 tableros pequeños no frenen el grande.
 */
export function useMesaLenta<T>(
	selector: (m: EstadoMesa) => T,
	igual: (a: T, b: T) => boolean = Object.is,
	ms = MS_MINIATURA,
): T {
	const [valor, setValor] = useState(() => selector(useMesa.getState()));
	const sel = useRef(selector);
	const eq = useRef(igual);
	sel.current = selector;
	eq.current = igual;
	useEffect(() => {
		const mirar = () => {
			const nuevo = sel.current(useMesa.getState());
			setValor((previo) => (eq.current(previo, nuevo) ? previo : nuevo));
		};
		mirar();
		const i = window.setInterval(mirar, ms);
		return () => window.clearInterval(i);
	}, [ms]);
	return valor;
}

/** Dos listas iguales elemento a elemento (por identidad). */
export function igualesLista<T>(a: readonly T[], b: readonly T[]): boolean {
	return a === b || (a.length === b.length && a.every((x, i) => Object.is(x, b[i])));
}
