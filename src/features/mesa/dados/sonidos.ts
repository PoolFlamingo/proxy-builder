import type { Audio } from "../audio";
import { audio as audioMesa, azar, envolvente, hayHueco } from "../audio";

/**
 * **Los sonidos de los dados y la moneda**, sintetizados con la Web Audio
 * API (sin ficheros): el golpe de un dado es un chasquido de ruido filtrado
 * con un «toc» del cuerpo, y la moneda, unos parciales metálicos que se
 * apagan. Cada golpe es un poco distinto, y en el 3D suenan cuando y con la
 * fuerza con la que chocan los dados en la física. El azar del sonido no es
 * el de la partida (`Math.random`): no decide nada.
 *
 * Suenan con «Sonido de los dados y las monedas» encendido y el volumen de
 * Ajustes > Mesa, y no con la ventana oculta.
 */

const audio = (): Audio | null => audioMesa((m) => m.sonidoDados);

/** El golpe de un dado contra la mesa u otro dado; `fuerza` de 0 a 1. */
export function golpeDado(fuerza: number, retraso = 0) {
	const a = audio();
	if (!a || !hayHueco(a.ctx)) return;
	const { ctx, salida, ruido } = a;
	const t = ctx.currentTime + Math.max(0, retraso);
	const f = Math.min(1, Math.max(0, fuerza));
	const pico = 0.12 + 0.75 * f;
	// El chasquido de la resina.
	const fuente = ctx.createBufferSource();
	fuente.buffer = ruido;
	fuente.playbackRate.value = azar(0.85, 1.2);
	const banda = ctx.createBiquadFilter();
	banda.type = "bandpass";
	banda.frequency.value = azar(2400, 4200);
	banda.Q.value = azar(1.2, 2.2);
	const e1 = envolvente(ctx, t, pico, azar(0.05, 0.09));
	fuente.connect(banda).connect(e1).connect(salida);
	fuente.start(t);
	fuente.stop(t + 0.12);
	// El «toc» del cuerpo del dado.
	const tono = ctx.createOscillator();
	tono.type = "triangle";
	tono.frequency.setValueAtTime(azar(780, 1250), t);
	tono.frequency.exponentialRampToValueAtTime(azar(520, 700), t + 0.04);
	const e2 = envolvente(ctx, t, pico * 0.45, 0.045);
	tono.connect(e2).connect(salida);
	tono.start(t);
	tono.stop(t + 0.06);
	// Y algo de la mesa, grave y corto.
	const mesa = ctx.createBufferSource();
	mesa.buffer = ruido;
	const grave = ctx.createBiquadFilter();
	grave.type = "lowpass";
	grave.frequency.value = 320;
	const e3 = envolvente(ctx, t, pico * 0.7 * f, 0.07);
	mesa.connect(grave).connect(e3).connect(salida);
	mesa.start(t);
	mesa.stop(t + 0.1);
}

/** Una moneda que choca y suena; `fuerza` de 0 a 1. */
export function golpeMoneda(fuerza: number, retraso = 0) {
	const a = audio();
	if (!a || !hayHueco(a.ctx)) return;
	const { ctx, salida, ruido } = a;
	const t = ctx.currentTime + Math.max(0, retraso);
	const f = Math.min(1, Math.max(0, fuerza));
	const base = azar(1900, 2500);
	const pico = 0.06 + 0.3 * f;
	for (const [mult, amp, dura] of [
		[1, 1, 0.9],
		[2.76, 0.55, 0.6],
		[5.4, 0.35, 0.38],
		[8.93, 0.2, 0.22],
	] as const) {
		const o = ctx.createOscillator();
		o.frequency.value = base * mult * azar(0.995, 1.005);
		const e = envolvente(ctx, t, pico * amp, dura * (0.35 + 0.65 * f));
		o.connect(e).connect(salida);
		o.start(t);
		o.stop(t + dura + 0.05);
	}
	const clic = ctx.createBufferSource();
	clic.buffer = ruido;
	const agudo = ctx.createBiquadFilter();
	agudo.type = "highpass";
	agudo.frequency.value = 5000;
	const e = envolvente(ctx, t, pico * 0.8, 0.02);
	clic.connect(agudo).connect(e).connect(salida);
	clic.start(t);
	clic.stop(t + 0.04);
}

/** El pulgar que lanza la moneda: un «ting» corto. */
export function lanzarMoneda(retraso = 0) {
	golpeMoneda(0.45, retraso);
}

/** Los dados en la mano antes de soltarlos (para los niveles sin física). */
export function agitarDados(cuantos: number, dura = 0.45) {
	const n = Math.min(10, 3 + cuantos * 2);
	for (let i = 0; i < n; i++) golpeDado(azar(0.08, 0.25), (i / n) * dura + azar(0, 0.03));
}
