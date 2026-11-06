import { create } from "zustand";
import type { VistaMesa } from "@/bindings/VistaMesa";
import { useUi } from "@/stores/ui-store";
import { sonar } from "../sonidos";
import {
	anguloFrenando,
	type Frenado,
	frenado,
	partidaSinEmpezar,
	type Sector,
	VELOCIDAD,
} from "./cuentas";

/** Lo que se queda enseñando el resultado antes de abrir la mesa, en ms. */
const PAUSA_MS = 1100;

export type Parada = {
	/** Quién empieza (lo ha decidido Rust). */
	ganador: number;
	/** Si soy yo (en red; en una partida local se dice el nombre). */
	mio: boolean;
	/** Cuándo empezó a frenar (`performance.now()`). */
	empieza: number;
	frenado: Frenado;
};

export type Giro = {
	sectores: Sector[];
	/** Cuándo empezó a girar (`performance.now()`). */
	inicio: number;
	/** La persona pidió menos movimiento: la rueda no gira, solo enseña el resultado. */
	calma: boolean;
	parada: Parada | null;
	parado: boolean;
};

/**
 * **La ruleta de quién empieza.** Solo enseña un resultado que ya está decidido
 * (`mesa::preparar::iniciar`, `turno.primero`), como los dados: gira mientras
 * se prepara la mesa y, cuando llega la primera vista, frena en quien toca.
 * Los temporizadores viven aquí y no en el componente: aunque la pestaña «Jugar»
 * no esté a la vista, la mesa se abre igual.
 */
export const useRuleta = create<{ giro: Giro | null }>(() => ({ giro: null }));

let reloj: number | null = null;
let alTerminar: (() => void) | null = null;

const conMenosMovimiento = () =>
	typeof window.matchMedia === "function" &&
	window.matchMedia("(prefers-reduced-motion: reduce)").matches;

const nuevoGiro = (sectores: Sector[]): Giro => ({
	sectores,
	inicio: performance.now(),
	calma: conMenosMovimiento(),
	parada: null,
	parado: false,
});

/** El ángulo de la rueda en este momento. */
export function anguloDe(g: Giro, ahora: number): number {
	if (g.parada) return anguloFrenando(g.parada.frenado, ahora - g.parada.empieza);
	return g.calma ? 0 : (VELOCIDAD * (ahora - g.inicio)) / 1000;
}

const encendida = () => useUi.getState().mesa.ruletaEmpieza;

/** ¿Hay que enseñar la ruleta con esta vista? Apagada en Ajustes, nunca (el sorteo es el mismo). */
export function tocaRuleta(v: Pick<VistaMesa, "jugadores" | "general">): boolean {
	return encendida() && partidaSinEmpezar(v);
}

/**
 * Empieza a girar, mientras no se sepa quién empieza. Solo con jugadores de verdad:
 * el solitario tiene un asiento (los fantasmas no son jugadores) y no sortea nada.
 */
export function girar(sectores: Sector[]) {
	if (sectores.length < 2 || !encendida()) return;
	cerrar();
	useRuleta.setState({ giro: nuevoGiro(sectores) });
}

/**
 * Frena en `ganador` (si no giraba, empieza ahora). Se cumple cuando ya se ha
 * visto el resultado: quien llama abre la mesa y luego llama a `cerrar`.
 */
export function parar(ganador: number, sectores: Sector[], mio: boolean): Promise<void> {
	const antes = useRuleta.getState().giro;
	const giro = antes ? { ...antes, sectores } : nuevoGiro(sectores);
	const ahora = performance.now();
	const i = Math.max(
		0,
		sectores.findIndex((s) => s.id === ganador),
	);
	const f = frenado(anguloDe(giro, ahora), i, sectores.length, Math.random());
	const plan = giro.calma ? { ...f, desde: f.objetivo, ms: 0 } : f;
	useRuleta.setState({
		giro: { ...giro, parada: { ganador, mio, empieza: ahora, frenado: plan } },
	});
	return new Promise((resolver) => {
		alTerminar = resolver;
		reloj = window.setTimeout(() => {
			const g = useRuleta.getState().giro;
			if (g) useRuleta.setState({ giro: { ...g, parado: true } });
			sonar(mio ? "tuTurno" : "turno");
			reloj = window.setTimeout(() => {
				reloj = null;
				alTerminar = null;
				resolver();
			}, PAUSA_MS);
		}, plan.ms);
	});
}

/** Quita la ruleta (ya se abrió la mesa, o no se pudo empezar). */
export function cerrar() {
	if (reloj !== null) window.clearTimeout(reloj);
	reloj = null;
	const fin = alTerminar;
	alTerminar = null;
	fin?.();
	useRuleta.setState({ giro: null });
}
