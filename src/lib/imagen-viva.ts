import { useEffect, useRef, useState } from "react";

/**
 * **Vistas previas en tiempo real.** Lo que pinta Rust (una carta diseñada,
 * una carta retocada) se pide al ritmo al que cambia, no al soltar:
 *
 * - `CargaUltima` carga imágenes de una en una y, si mientras tanto se piden
 *   varias, solo la última: nunca se acumulan peticiones viejas.
 * - `useImagenViva` deja en pantalla la última imagen ya cargada hasta que
 *   llega la nueva, así no parpadea ni se queda en blanco al arrastrar.
 * - `useRafaga` dice si un valor está cambiando seguido (arrastrando), para
 *   pedir entretanto una imagen más pequeña y rápida.
 * - `useAlRitmo` deja pasar un valor como mucho cada `ms`, con el último
 *   siempre al final (no espera a que se deje de mover).
 */

export type Cargador = (url: string, listo: (ok: boolean) => void) => void;

/** Carga en el navegador: el `Image` deja la imagen en la caché para el `<img>`. */
export const cargarEnNavegador: Cargador = (url, listo) => {
	const img = new Image();
	img.onload = () => listo(true);
	img.onerror = () => listo(false);
	img.src = url;
};

export class CargaUltima {
	private enCurso: string | null = null;
	private siguiente: string | null = null;

	constructor(
		private readonly cargar: Cargador,
		/** Se llama con cada imagen que termina bien, en orden de petición. */
		private readonly alCargar: (url: string) => void,
		private readonly alCambiarOcupado: (ocupado: boolean) => void = () => {},
	) {}

	pedir(url: string) {
		if (this.enCurso !== null) {
			// La que está en marcha termina; después va solo la última pedida.
			this.siguiente = url === this.enCurso ? null : url;
			return;
		}
		this.alCambiarOcupado(true);
		this.empezar(url);
	}

	private empezar(url: string) {
		this.enCurso = url;
		this.cargar(url, (ok) => {
			if (ok) this.alCargar(url);
			this.enCurso = null;
			const s = this.siguiente;
			this.siguiente = null;
			if (s !== null) this.empezar(s);
			else this.alCambiarOcupado(false);
		});
	}

	get ocupado() {
		return this.enCurso !== null;
	}
}

/** La imagen que se ve: la última cargada de las pedidas (o ninguna). */
export function useImagenViva(
	url: string | null,
	cargar: Cargador = cargarEnNavegador,
): { mostrada: string | null; cargando: boolean } {
	const [mostrada, setMostrada] = useState<string | null>(null);
	const [cargando, setCargando] = useState(false);
	const vivo = useRef(true);
	const carga = useRef<CargaUltima | null>(null);
	if (carga.current === null) {
		carga.current = new CargaUltima(
			cargar,
			(u) => vivo.current && setMostrada(u),
			(o) => vivo.current && setCargando(o),
		);
	}
	useEffect(() => {
		vivo.current = true;
		return () => {
			vivo.current = false;
		};
	}, []);
	useEffect(() => {
		if (url === null) {
			setMostrada(null);
			return;
		}
		carga.current?.pedir(url);
	}, [url]);
	return { mostrada: url === null ? null : mostrada, cargando };
}

/** `true` mientras `valor` cambie más a menudo que cada `ms`; vuelve a `false` al parar. */
export function useRafaga(valor: unknown, ms = 250): boolean {
	const [rafaga, setRafaga] = useState(false);
	const ultimo = useRef(Number.NEGATIVE_INFINITY);
	const primero = useRef(true);
	useEffect(() => {
		if (primero.current) {
			primero.current = false;
			return;
		}
		const ahora = performance.now();
		if (ahora - ultimo.current < ms) setRafaga(true);
		ultimo.current = ahora;
		const id = setTimeout(() => setRafaga(false), ms);
		return () => clearTimeout(id);
	}, [valor, ms]);
	return rafaga;
}

/** `valor`, pero cambiando como mucho cada `ms` (el último llega siempre). */
export function useAlRitmo<T>(valor: T, ms = 60): T {
	const [salida, setSalida] = useState(valor);
	const ultimo = useRef(valor);
	const temporizador = useRef<ReturnType<typeof setTimeout> | null>(null);
	const cuando = useRef(Number.NEGATIVE_INFINITY);
	ultimo.current = valor;
	useEffect(() => {
		if (Object.is(valor, salida)) return;
		if (temporizador.current) return;
		const espera = Math.max(0, cuando.current + ms - performance.now());
		temporizador.current = setTimeout(() => {
			temporizador.current = null;
			cuando.current = performance.now();
			setSalida(ultimo.current);
		}, espera);
	}, [valor, salida, ms]);
	useEffect(
		() => () => {
			if (temporizador.current) clearTimeout(temporizador.current);
		},
		[],
	);
	return salida;
}
