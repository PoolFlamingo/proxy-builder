import type { Evento } from "@/bindings/Evento";
import { useChat } from "@/stores/chat-store";
import { useMesa } from "@/stores/mesa-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { useUi } from "@/stores/ui-store";
import { BOCADILLO_MS, mensajeDeEvento } from "./cuentas-chat";
import { eventosNuevos } from "./cuentas-repeticion";
import { useDados } from "./dados/cola";
import { usePantalla } from "./pantalla";
import { sonar, sonidoDeEvento } from "./sonidos";

/**
 * **Lo que la pantalla hace cuando llega un evento nuevo** (solo los de
 * ahora: lo que llega con la vista entera o en una repetición no suena ni
 * sale en bocadillos): el bocadillo de un mensaje y la cuenta de los que no
 * se han leído, y los sonidos de la mesa (`sonidos.ts`). Lo llama
 * `conexion.ts` por cada parche, ya aplicado a la vista.
 *
 * `soloEfectos`: la repetición, donde los mensajes ya están en el registro y
 * no hay bocadillos ni cuenta de sin leer, solo suena.
 */
export function reaccionarAEvento(ev: Evento, soloEfectos = false) {
	const mesa = useMesa.getState();
	const chat = useChat.getState();
	const m = mensajeDeEvento(ev);
	const sonido = sonidoDeEvento(ev, mesa.yo, mesa.general?.turno.activo ?? null);
	// Quien está silenciado tampoco suena.
	if (sonido && !(m && chat.silenciados.includes(m.jugador))) sonar(sonido);
	if (soloEfectos || !m || chat.silenciados.includes(m.jugador)) return;
	chat.poner(m.jugador, { seq: m.seq, texto: m.texto, aviso: m.aviso });
	window.setTimeout(() => useChat.getState().quitar(m.jugador, m.seq), BOCADILLO_MS);
	const mio = mesa.yo === m.jugador;
	if (!mio && usePantalla.getState().panel !== "chat") chat.sumarSinLeer();
}

/**
 * **Los efectos de la repetición**: una repetición recibe la partida entera de
 * cada punto, no un parche por evento, así que lo que acaba de pasar se saca de
 * lo que la vista nueva trae de más en el registro (`antes` es el de la vista
 * anterior). Si el paso es corto hacia delante, suenan las cartas y el turno y
 * se tiran los dados, como en la partida. Un salto o ir atrás, no; y se apaga en
 * Ajustes > Mesa.
 */
export function reaccionarEnRepeticion(antes: readonly Evento[]) {
	const rep = useRepeticion.getState();
	if (!rep.activa || rep.animarMs === 0 || !useUi.getState().mesa.efectosRepeticion) return;
	for (const ev of eventosNuevos(antes, useMesa.getState().registro)) {
		reaccionarAEvento(ev, true);
		useDados.getState().encolar(ev);
	}
}
