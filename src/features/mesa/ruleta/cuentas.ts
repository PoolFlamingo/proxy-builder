import type { VistaMesa } from "@/bindings/VistaMesa";

/** Un jugador en la ruleta. */
export type Sector = { id: number; nombre: string; color: string };

/** Cuánto gira mientras se prepara la mesa, en grados por segundo. */
export const VELOCIDAD = 480;
/** La forma del frenado: cuanto más alta, más tarda en pararse del todo. */
const CURVA = 2.4;
/** Hasta dónde del centro del sector puede quedarse la aguja (no siempre en el centro). */
const HOLGURA = 0.35;

/** Cómo frena: de `desde` a `objetivo` en `ms`. */
export type Frenado = { desde: number; objetivo: number; ms: number };

const modulo = (x: number, m: number) => ((x % m) + m) % m;

/** El centro de un sector, en grados desde arriba y en el sentido de las agujas. */
export function centroSector(i: number, n: number): number {
	return ((i + 0.5) * 360) / n;
}

/**
 * El frenado que deja el sector `ganador` bajo la aguja (arriba). Da entre una y
 * dos vueltas más, y la duración sale de la velocidad a la que venía: así la
 * rueda no pega un tirón al pasar de girar a frenar. `azar` (0 a 1) solo mueve
 * la aguja dentro del sector: quién gana lo ha decidido ya Rust.
 */
export function frenado(angulo: number, ganador: number, n: number, azar: number): Frenado {
	const ancho = 360 / n;
	const destino = modulo(-centroSector(ganador, n) + (azar * 2 - 1) * HOLGURA * ancho, 360);
	const recorrido = modulo(destino - angulo, 360) + 360;
	return {
		desde: angulo,
		objetivo: angulo + recorrido,
		ms: ((recorrido * CURVA) / VELOCIDAD) * 1000,
	};
}

/** El ángulo a los `ms` de empezar a frenar. */
export function anguloFrenando(f: Frenado, ms: number): number {
	if (ms >= f.ms) return f.objetivo;
	const falta = 1 - Math.max(0, ms) / f.ms;
	return f.desde + (f.objetivo - f.desde) * (1 - falta ** CURVA);
}

/** Qué sector queda bajo la aguja con la rueda girada `angulo` grados. */
export function sectorBajoLaAguja(angulo: number, n: number): number {
	return Math.floor(modulo(-angulo, 360) / (360 / n));
}

/** El trazado SVG de un sector, con centro en (0, 0). */
export function trazoSector(i: number, n: number, r: number): string {
	const punto = (grados: number) => {
		const a = (grados * Math.PI) / 180;
		return `${(r * Math.sin(a)).toFixed(3)} ${(-r * Math.cos(a)).toFixed(3)}`;
	};
	const desde = (i * 360) / n;
	const hasta = ((i + 1) * 360) / n;
	return `M0 0 L${punto(desde)} A${r} ${r} 0 0 1 ${punto(hasta)} Z`;
}

/**
 * El nombre, cortado para que quepa en `maxEm` (el ancho de un radio de la
 * ruleta). Una letra latina o cirílica ocupa cerca de media `em`; una de japonés,
 * coreano o chino, una entera. Se cuenta por caracteres, no por unidades de
 * UTF-16, para no partir un emoji.
 */
export function recortarNombre(nombre: string, maxEm: number): string {
	const letras = Array.from(nombre);
	let ocupado = 0;
	for (const [i, c] of letras.entries()) {
		ocupado += (c.codePointAt(0) ?? 0) >= 0x2e80 ? 1 : 0.56;
		if (ocupado > maxEm) return `${letras.slice(0, Math.max(1, i - 1)).join("")}…`;
	}
	return nombre;
}

/**
 * ¿Es una partida de varios recién abierta, sin que nadie haya decidido su mano?
 * Lo que no lo sea (retomar una a medias, volver a una en marcha) ya tiene
 * decidido quién empezó, y no hay nada que sortear.
 */
export function partidaSinEmpezar(v: Pick<VistaMesa, "jugadores" | "general">): boolean {
	return (
		v.jugadores.length >= 2 &&
		v.general.turno.numero === 0 &&
		(v.general.mulligan?.decididos.length ?? 0) === 0
	);
}
