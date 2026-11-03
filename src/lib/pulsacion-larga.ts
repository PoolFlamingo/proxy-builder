import { type PointerEvent, useEffect, useMemo, useRef } from "react";

/** Lo que tarda en abrirse lo que haría el clic derecho, en ms. */
export const MS_PULSACION = 500;
/** Cuánto puede moverse el dedo sin que deje de ser «quieto», en px. */
export const TOLERANCIA_PULSACION = 8;

export interface DetectorPulsacion {
	empezar(x: number, y: number): void;
	mover(x: number, y: number): void;
	cancelar(): void;
	/** ¿Ha saltado la pulsación larga y aún no se ha consumido? */
	consumir(): boolean;
}

/**
 * La pulsación larga, sin React: un dedo (o lápiz) quieto `ms` milisegundos
 * dispara `alDisparar`; moverse más de `tolerancia` px, soltar o cancelar la
 * deshace. El `ContextMenu` de Radix ya lo hace a 700 ms y sin tolerancia;
 * esto es para todo lo demás, con los mismos 500 ms en toda la app.
 */
export function crearDetector(
	alDisparar: (x: number, y: number) => void,
	ms = MS_PULSACION,
	tolerancia = TOLERANCIA_PULSACION,
): DetectorPulsacion {
	let temporizador: ReturnType<typeof setTimeout> | null = null;
	let x0 = 0;
	let y0 = 0;
	let disparada = false;
	const parar = () => {
		if (temporizador !== null) clearTimeout(temporizador);
		temporizador = null;
	};
	return {
		empezar(x, y) {
			parar();
			x0 = x;
			y0 = y;
			disparada = false;
			temporizador = setTimeout(() => {
				temporizador = null;
				disparada = true;
				alDisparar(x0, y0);
			}, ms);
		},
		mover(x, y) {
			if (temporizador !== null && Math.hypot(x - x0, y - y0) > tolerancia) parar();
		},
		cancelar: parar,
		consumir() {
			const era = disparada;
			disparada = false;
			return era;
		},
	};
}

/**
 * Los manejadores para ponerle la pulsación larga a un elemento. Solo con
 * dedo o lápiz: con el ratón sigue valiendo el clic derecho. Tras saltar, el
 * clic que llega al soltar se descarta (no debe hacer además lo de un toque).
 */
export function usePulsacionLarga(
	alPulsar: (x: number, y: number) => void,
	opciones?: { ms?: number; tolerancia?: number },
) {
	const ultimo = useRef(alPulsar);
	ultimo.current = alPulsar;
	const detector = useMemo(
		() =>
			crearDetector(
				(x, y) => ultimo.current(x, y),
				opciones?.ms ?? MS_PULSACION,
				opciones?.tolerancia ?? TOLERANCIA_PULSACION,
			),
		[opciones?.ms, opciones?.tolerancia],
	);
	useEffect(() => () => detector.cancelar(), [detector]);
	return {
		onPointerDown: (e: PointerEvent) => {
			if (e.pointerType === "mouse") return;
			detector.empezar(e.clientX, e.clientY);
		},
		onPointerMove: (e: PointerEvent) => detector.mover(e.clientX, e.clientY),
		onPointerUp: () => detector.cancelar(),
		onPointerCancel: () => detector.cancelar(),
		onClickCapture: (e: { preventDefault(): void; stopPropagation(): void }) => {
			if (detector.consumir()) {
				e.preventDefault();
				e.stopPropagation();
			}
		},
	};
}
