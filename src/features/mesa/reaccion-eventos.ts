import type { Evento } from "@/bindings/Evento";
import { useChat } from "@/stores/chat-store";
import { useMesa } from "@/stores/mesa-store";
import { BOCADILLO_MS, mensajeDeEvento } from "./cuentas-chat";
import { usePantalla } from "./pantalla";
import { sonar, sonidoDeEvento } from "./sonidos";

/**
 * **Lo que la pantalla hace cuando llega un evento nuevo** (solo los de
 * ahora: lo que llega con la vista entera o en una repetición no suena ni
 * sale en bocadillos): el bocadillo de un mensaje y la cuenta de los que no
 * se han leído, y los sonidos de la mesa (`sonidos.ts`). Lo llama
 * `conexion.ts` por cada parche, ya aplicado a la vista.
 */
export function reaccionarAEvento(ev: Evento) {
	const mesa = useMesa.getState();
	const chat = useChat.getState();
	const m = mensajeDeEvento(ev);
	const sonido = sonidoDeEvento(ev, mesa.yo, mesa.general?.turno.activo ?? null);
	// Quien está silenciado tampoco suena.
	if (sonido && !(m && chat.silenciados.includes(m.jugador))) sonar(sonido);
	if (!m || chat.silenciados.includes(m.jugador)) return;
	chat.poner(m.jugador, { seq: m.seq, texto: m.texto, aviso: m.aviso });
	window.setTimeout(() => useChat.getState().quitar(m.jugador, m.seq), BOCADILLO_MS);
	const mio = mesa.yo === m.jugador;
	if (!mio && usePantalla.getState().panel !== "chat") chat.sumarSinLeer();
}
