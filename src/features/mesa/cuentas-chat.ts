import type { AvisoRapido } from "@/bindings/AvisoRapido";
import type { Evento } from "@/bindings/Evento";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Senal } from "@/bindings/Senal";
import { paraTraducir } from "@/components/language-provider";

/**
 * **Cuentas del chat y los avisos** (H5): qué mensajes salen en el chat, los
 * avisos de un clic con su tecla y las señales de prioridad. Quién manda qué
 * lo decide el motor (`Accion::Chat`, `Accion::Aviso`, `Accion::Senal`).
 */

/** El tope del texto de un mensaje (el de `mesa::aplicar::MAX_CHAT`). */
export const MAX_CHAT = 500;

/** Cuánto se enseña el bocadillo de un mensaje, en ms. */
export const BOCADILLO_MS = 6_000;

/** Los avisos de un clic, con su texto y su tecla (Alt+1…6). */
export const AVISOS: readonly { tipo: AvisoRapido; texto: string; tecla: string }[] = [
	{ tipo: "espera", texto: paraTraducir("Espera"), tecla: "Alt+1" },
	{ tipo: "respondo", texto: paraTraducir("Respondo"), tecla: "Alt+2" },
	{ tipo: "ok", texto: paraTraducir("OK"), tecla: "Alt+3" },
	{ tipo: "paso", texto: paraTraducir("Paso"), tecla: "Alt+4" },
	{ tipo: "tuTurno", texto: paraTraducir("Tu turno"), tecla: "Alt+5" },
	{ tipo: "pienso", texto: paraTraducir("Pienso"), tecla: "Alt+6" },
];

/** Las señales de prioridad con su texto y su tecla (Ctrl+Mayús+1…4). */
export const SENALES: readonly { senal: Senal; texto: string; atajo: string; tecla: string }[] = [
	{
		senal: "sinRespuesta",
		texto: paraTraducir("Sin respuesta"),
		atajo: paraTraducir("Señal: sin respuesta"),
		tecla: "Ctrl+Mayús+1",
	},
	{
		senal: "espera",
		texto: paraTraducir("Espera"),
		atajo: paraTraducir("Señal: espera"),
		tecla: "Ctrl+Mayús+2",
	},
	{
		senal: "pensando",
		texto: paraTraducir("Pensando"),
		atajo: paraTraducir("Señal: pensando"),
		tecla: "Ctrl+Mayús+3",
	},
	{
		senal: "ausente",
		texto: paraTraducir("Ausente"),
		atajo: paraTraducir("Señal: ausente"),
		tecla: "Ctrl+Mayús+4",
	},
];

export function textoDeAviso(tipo: AvisoRapido): string {
	return AVISOS.find((a) => a.tipo === tipo)?.texto ?? "";
}

/** Un mensaje del chat o un aviso, tal como sale en la lista. */
export type MensajeChat = {
	seq: number;
	tMs: number;
	jugador: IdJugador;
	/** Texto escrito (`null` en un aviso). */
	texto: string | null;
	aviso: AvisoRapido | null;
};

/** El mensaje que lleva un evento, si es de chat o de aviso y de un jugador. */
export function mensajeDeEvento(ev: Evento): MensajeChat | null {
	if (ev.cambio.tipo !== "accion" || ev.actor.tipo !== "jugador") return null;
	const a = ev.cambio.accion;
	const base = { seq: ev.seq, tMs: ev.tMs, jugador: ev.actor.id };
	if (a.que === "chat") return { ...base, texto: a.texto, aviso: null };
	if (a.que === "aviso") return { ...base, texto: null, aviso: a.tipo };
	return null;
}

/** Los mensajes del registro, sin los de los jugadores silenciados (solo para quien silencia). */
export function mensajesDeChat(
	registro: readonly Evento[],
	silenciados: readonly IdJugador[],
): MensajeChat[] {
	const fuera = new Set(silenciados);
	const salida: MensajeChat[] = [];
	for (const ev of registro) {
		const m = mensajeDeEvento(ev);
		if (m && !fuera.has(m.jugador)) salida.push(m);
	}
	return salida;
}

type TeclaEvento = Pick<KeyboardEvent, "code" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">;

/**
 * Cómo se escribe la tecla de un aviso o una señal: por el código físico
 * (con Mayús o Alt, `key` cambia de símbolo). `null` si no es un dígito con
 * Alt o con Ctrl+Mayús.
 */
export function teclaDeComunicacion(e: TeclaEvento): string | null {
	const d = /^(?:Digit|Numpad)([0-9])$/.exec(e.code)?.[1];
	if (!d) return null;
	const ctrl = e.ctrlKey || e.metaKey;
	if (ctrl && e.shiftKey && !e.altKey) return `Ctrl+Mayús+${d}`;
	if (e.altKey && !ctrl && !e.shiftKey) return `Alt+${d}`;
	return null;
}

/** La señal que queda al elegir `elegida` estando `actual`: la misma otra vez la quita. */
export function senalTrasElegir(actual: Senal | null, elegida: Senal): Senal | null {
	return actual === elegida ? null : elegida;
}
