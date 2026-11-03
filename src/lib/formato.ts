import { useSyncExternalStore } from "react";
import type { Plataforma } from "@/bindings/Plataforma";
import { call } from "@/lib/ipc";

/**
 * En qué tipo de pantalla está la app y cómo se toca. Una sola interfaz que
 * cambia de disposición según esto (nada de pantallas aparte para el móvil).
 *
 * El ancho es el de la ventana en px de CSS (en Android, dp): menos de 640 es
 * móvil y menos de 1024, tableta. Una ventana de escritorio estrecha puede
 * salir como «tableta» a propósito: es la misma disposición.
 */
export type Tamano = "escritorio" | "tableta" | "movil";

export interface Formato {
	tamano: Tamano;
	/** Se toca con el dedo (no hay ratón encima ni clic derecho). */
	tactil: boolean;
	horizontal: boolean;
}

export const CORTE_MOVIL = 640;
export const CORTE_TABLETA = 1024;

export function tamanoPorAncho(ancho: number): Tamano {
	if (ancho < CORTE_MOVIL) return "movil";
	if (ancho < CORTE_TABLETA) return "tableta";
	return "escritorio";
}

/** Solo en desarrollo: fuerza el formato (`VITE_PB_FORMATO=movil|tableta`). */
export function formatoForzado(valor: string | undefined): Tamano | null {
	return valor === "movil" || valor === "tableta" ? valor : null;
}

export interface Medidas {
	ancho: number;
	alto: number;
	/** `(pointer: coarse)`: el puntero principal es un dedo. */
	puntero_grueso: boolean;
	plataforma: Plataforma | null;
	forzado: Tamano | null;
}

/**
 * Con el dedo, un lado corto de menos de 600 dp es un teléfono aunque esté de
 * lado (914 × 411 dp mide más de 640 de ancho): es la regla `sw600dp` de
 * Android. Sin ella, un móvil girado saldría como «tableta» y la mesa, que se
 * juega en horizontal, nunca tendría su formato de móvil.
 */
export const LADO_CORTO_MOVIL = 600;

export function formatoDe(m: Medidas): Formato {
	const movilDeVerdad = m.plataforma === "android" || m.plataforma === "ios";
	const tactil = m.puntero_grueso || movilDeVerdad || m.forzado !== null;
	const deLado = tactil && Math.min(m.ancho, m.alto) < LADO_CORTO_MOVIL;
	const tamano = m.forzado ?? (deLado ? "movil" : tamanoPorAncho(m.ancho));
	return {
		tamano,
		tactil,
		horizontal: m.ancho >= m.alto,
	};
}

// ── El hook ──────────────────────────────────────────────────────────────

let plataforma: Plataforma | null = null;
let cargando = false;
let actual: Formato | null = null;
const oyentes = new Set<() => void>();
let enlazado = false;

function medir(): Formato {
	const w = typeof window === "undefined" ? undefined : window;
	return formatoDe({
		ancho: w?.innerWidth ?? CORTE_TABLETA,
		alto: w?.innerHeight ?? 0,
		puntero_grueso: w?.matchMedia?.("(pointer: coarse)").matches ?? false,
		plataforma,
		forzado: import.meta.env.DEV ? formatoForzado(import.meta.env.VITE_PB_FORMATO) : null,
	});
}

function refrescar() {
	const n = medir();
	if (
		actual &&
		n.tamano === actual.tamano &&
		n.tactil === actual.tactil &&
		n.horizontal === actual.horizontal
	) {
		return;
	}
	actual = n;
	for (const f of oyentes) f();
}

function marcarHtml(f: Formato) {
	if (typeof document === "undefined") return;
	const raiz = document.documentElement;
	raiz.dataset.formato = f.tamano;
	if (f.tactil) raiz.dataset.tactil = "";
	else delete raiz.dataset.tactil;
}

function suscribirse(avisar: () => void) {
	oyentes.add(avisar);
	if (!enlazado && typeof window !== "undefined") {
		enlazado = true;
		window.addEventListener("resize", refrescar);
		window.addEventListener("orientationchange", refrescar);
		window.matchMedia?.("(pointer: coarse)").addEventListener?.("change", refrescar);
		if (!cargando) {
			cargando = true;
			call("app_plataforma")
				.then((p) => {
					plataforma = p;
					refrescar();
				})
				.catch(() => {});
		}
	}
	return () => {
		oyentes.delete(avisar);
	};
}

export function formatoActual(): Formato {
	if (!actual) {
		actual = medir();
		marcarHtml(actual);
	}
	return actual;
}

/** La suscripción de `useFormato`, estable (una nueva en cada render haría
 *  que React se suscribiera y se desuscribiera cada vez). */
function suscribirHook(avisar: () => void) {
	return suscribirse(() => {
		marcarHtml(formatoActual());
		avisar();
	});
}

/** El formato de la pantalla, al día al girarla o cambiar el tamaño. */
export function useFormato(): Formato {
	return useSyncExternalStore(suscribirHook, formatoActual, formatoActual);
}
