import { useUi } from "@/stores/ui-store";

/**
 * **El audio de la mesa**: un solo contexto de Web Audio (con compresor y el
 * volumen de Ajustes > Mesa) y un trozo de ruido que comparten los sonidos de
 * los dados (`dados/sonidos.ts`) y los del resto de la mesa (`sonidos.ts`).
 * Suena solo con el interruptor del tipo encendido, con volumen y con la
 * ventana a la vista.
 */

let contexto: AudioContext | null = null;
let salida: GainNode | null = null;
let ruido: AudioBuffer | null = null;
/** Para no saturar: como mucho tantos sonidos por cada 100 ms. */
const MAX_GOLPES = 14;
let ventana = { desde: 0, golpes: 0 };

export type Audio = { ctx: AudioContext; salida: GainNode; ruido: AudioBuffer };

/** El audio, si el tipo de sonido está encendido y se puede sonar (`null`: no suena). */
export function audio(
	encendido: (m: ReturnType<typeof useUi.getState>["mesa"]) => boolean,
): Audio | null {
	const m = useUi.getState().mesa;
	if (!encendido(m) || m.volumen <= 0 || document.hidden) return null;
	if (!contexto) {
		const Ctx =
			window.AudioContext ??
			(window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
		if (!Ctx) return null;
		contexto = new Ctx();
		const compresor = contexto.createDynamicsCompressor();
		compresor.threshold.value = -14;
		compresor.ratio.value = 6;
		salida = contexto.createGain();
		salida.connect(compresor);
		compresor.connect(contexto.destination);
		ruido = contexto.createBuffer(1, Math.floor(contexto.sampleRate * 0.25), contexto.sampleRate);
		const d = ruido.getChannelData(0);
		for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
		// El navegador no deja sonar hasta el primer gesto: se reanuda con él.
		const reanudar = () => void contexto?.resume();
		window.addEventListener("pointerdown", reanudar);
		window.addEventListener("keydown", reanudar);
	}
	if (contexto.state === "suspended") void contexto.resume();
	if (!salida || !ruido) return null;
	salida.gain.value = (m.volumen / 100) ** 1.6;
	return { ctx: contexto, salida, ruido };
}

export function hayHueco(ctx: AudioContext): boolean {
	const ahora = ctx.currentTime;
	if (ahora - ventana.desde > 0.1) ventana = { desde: ahora, golpes: 0 };
	if (ventana.golpes >= MAX_GOLPES) return false;
	ventana.golpes++;
	return true;
}

/** Un número al azar entre `a` y `b` (solo para variar el sonido: no decide nada). */
export const azar = (a: number, b: number) => a + Math.random() * (b - a);

/** Una envolvente de golpe: sube en 2 ms y se apaga en `dura` s. */
export function envolvente(ctx: AudioContext, t: number, pico: number, dura: number): GainNode {
	const g = ctx.createGain();
	g.gain.setValueAtTime(0.0001, t);
	g.gain.linearRampToValueAtTime(pico, t + 0.002);
	g.gain.exponentialRampToValueAtTime(0.0001, t + dura);
	return g;
}
