import type { Evento } from "@/bindings/Evento";
import type { IdJugador } from "@/bindings/IdJugador";
import { type Audio, audio as audioMesa, azar, envolvente, hayHueco } from "./audio";
import { MUESTRAS, tocar } from "./muestras";

/**
 * **Los sonidos de la mesa** (H6): barajar, robar, girar, jugar al campo,
 * mover (muestras de `muestras/`, con el sonido sintetizado de respaldo) y
 * pasar el turno, «es tu turno», mensajes y el fantasma del sparring
 * (sintetizados con Web Audio, sin ficheros). Qué suena con
 * cada evento lo decide `sonidoDeEvento` (puro, con test); cada tipo tiene su
 * interruptor en Ajustes > Mesa. Los sonidos de los dados, en `dados/`.
 */
export type SonidoMesa =
	| "barajar"
	| "robar"
	| "girar"
	| "jugar"
	| "mover"
	| "turno"
	| "tuTurno"
	| "chat"
	| "sparring";

/** A qué interruptor de Ajustes > Mesa pertenece cada sonido. */
export const GRUPO_DE_SONIDO: Record<SonidoMesa, "cartas" | "turno" | "chat"> = {
	barajar: "cartas",
	robar: "cartas",
	girar: "cartas",
	jugar: "cartas",
	mover: "cartas",
	turno: "turno",
	tuTurno: "turno",
	chat: "chat",
	sparring: "chat",
};

/**
 * El sonido de un evento nuevo, si lo tiene. `yo` es quien juega aquí y
 * `activo` el jugador al que le toca ya tras el evento: pasar el turno suena
 * distinto si llega a mí.
 */
export function sonidoDeEvento(
	ev: Evento,
	yo: IdJugador | null,
	activo: IdJugador | null,
): SonidoMesa | null {
	const c = ev.cambio;
	if (c.tipo === "sparring") return "sparring";
	if (c.tipo === "restaurar") return c.deshecho === null ? "barajar" : null;
	if (c.tipo !== "accion") return null;
	const a = c.accion;
	switch (a.que) {
		case "robar":
			return "robar";
		case "barajar":
		case "mulligan":
		case "barajarTodoDentro":
		case "todoAlFondoAlAzar":
			return "barajar";
		case "girar":
		case "enderezarTodo":
			return "girar";
		case "mover":
			return a.a.zona.tipo === "de" && a.a.zona.zona === "campo" ? "jugar" : "mover";
		case "crearFicha":
		case "copiaFicha":
		case "jugarBocaAbajo":
			return "jugar";
		case "pasarTurno":
		case "saltarTurno":
			return yo !== null && activo === yo ? "tuTurno" : "turno";
		case "chat":
		case "aviso":
			// Lo que escribo yo no me avisa a mí.
			return ev.actor.tipo === "jugador" && ev.actor.id === yo ? null : "chat";
		default:
			return null;
	}
}

const audio = (grupo: "cartas" | "turno" | "chat"): Audio | null =>
	audioMesa((m) =>
		grupo === "cartas" ? m.sonidoCartas : grupo === "turno" ? m.sonidoTurno : m.sonidoChat,
	);

/** Una nota con envolvente de campana: sube rápido y se apaga. */
function nota(a: Audio, t: number, hz: number, pico: number, dura: number, tipo: OscillatorType) {
	const o = a.ctx.createOscillator();
	o.type = tipo;
	o.frequency.value = hz;
	o.connect(envolvente(a.ctx, t, pico, dura)).connect(a.salida);
	o.start(t);
	o.stop(t + dura + 0.05);
}

/** Un chasquido de papel: ruido filtrado en banda. */
function papel(a: Audio, t: number, hz: number, q: number, pico: number, dura: number) {
	const f = a.ctx.createBufferSource();
	f.buffer = a.ruido;
	f.playbackRate.value = azar(0.8, 1.25);
	const b = a.ctx.createBiquadFilter();
	b.type = "bandpass";
	b.frequency.value = hz * azar(0.9, 1.1);
	b.Q.value = q;
	f.connect(b)
		.connect(envolvente(a.ctx, t, pico, dura))
		.connect(a.salida);
	f.start(t);
	f.stop(t + dura + 0.03);
}

/** Un golpe grave y corto (una carta sobre la mesa). */
function golpe(a: Audio, t: number, hz: number, pico: number) {
	const o = a.ctx.createOscillator();
	o.type = "sine";
	o.frequency.setValueAtTime(hz, t);
	o.frequency.exponentialRampToValueAtTime(hz * 0.55, t + 0.06);
	o.connect(envolvente(a.ctx, t, pico, 0.08)).connect(a.salida);
	o.start(t);
	o.stop(t + 0.1);
}

/** Suena un sonido de la mesa (si su interruptor está encendido y hay hueco). */
export function sonar(s: SonidoMesa) {
	try {
		sonarSinRed(s);
	} catch {
		// Sin dispositivo de audio no suena y ya está: nunca rompe la partida.
	}
}

function sonarSinRed(s: SonidoMesa) {
	const a = audio(GRUPO_DE_SONIDO[s]);
	if (!a || !hayHueco(a.ctx)) return;
	const t = a.ctx.currentTime;
	// Las de cartas, con muestras reales; si aún no están listas, suena la sintetizada.
	if (s === "barajar" && tocar(a, MUESTRAS.barajar, { t, volumen: 0.8 })) return;
	if (s === "robar" && tocar(a, MUESTRAS.robar, { t, volumen: 0.7 })) return;
	if (s === "girar" && tocar(a, MUESTRAS.girar, { t, volumen: 0.6 })) return;
	if (s === "jugar" && tocar(a, MUESTRAS.jugar, { t, volumen: 0.9 })) return;
	if (s === "mover" && tocar(a, MUESTRAS.mover, { t, volumen: 0.45 })) return;
	switch (s) {
		case "barajar":
			// Un abanico de chasquidos que se aceleran, como las cartas al juntarse.
			for (let i = 0; i < 14; i++) {
				papel(a, t + (i / 14) ** 0.8 * 0.5, azar(2600, 4200), 1.4, azar(0.12, 0.22), 0.04);
			}
			break;
		case "robar":
			papel(a, t, 2200, 0.8, 0.28, 0.1);
			papel(a, t + 0.05, 4200, 1.5, 0.12, 0.05);
			break;
		case "girar":
			papel(a, t, 1500, 1.1, 0.2, 0.05);
			golpe(a, t + 0.03, 240, 0.12);
			break;
		case "jugar":
			papel(a, t, 3000, 1.2, 0.3, 0.04);
			golpe(a, t, 170, 0.4);
			break;
		case "mover":
			papel(a, t, 2800, 1.2, 0.16, 0.035);
			break;
		case "turno":
			nota(a, t, 523.25, 0.16, 0.35, "sine");
			nota(a, t + 0.09, 392, 0.12, 0.4, "sine");
			break;
		case "tuTurno":
			nota(a, t, 659.25, 0.2, 0.4, "triangle");
			nota(a, t + 0.11, 880, 0.2, 0.45, "triangle");
			nota(a, t + 0.22, 1318.5, 0.16, 0.6, "sine");
			break;
		case "chat":
			nota(a, t, 987.77, 0.14, 0.12, "sine");
			nota(a, t + 0.07, 1318.5, 0.12, 0.16, "sine");
			break;
		case "sparring":
			nota(a, t, 196, 0.22, 0.5, "sawtooth");
			nota(a, t, 207.65, 0.12, 0.5, "sawtooth");
			break;
	}
}
