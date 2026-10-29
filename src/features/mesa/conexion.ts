import { Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { ConfigMesa } from "@/bindings/ConfigMesa";
import type { MensajeMesa } from "@/bindings/MensajeMesa";
import { paraTraducir, traduccionSuelta } from "@/components/language-provider";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useChat } from "@/stores/chat-store";
import { useMesa } from "@/stores/mesa-store";
import { useRed } from "@/stores/red-store";
import { useRepeticion } from "@/stores/repeticion-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useVivo } from "@/stores/vivo-store";
import { useDados } from "./dados/cola";
import { usePantalla } from "./pantalla";
import { precargar } from "./precarga";
import { reaccionarAEvento, reaccionarEnRepeticion } from "./reaccion-eventos";

/**
 * **La pantalla y la partida de Rust.** Se suscribe por un canal
 * (`tauri::ipc::Channel`, en orden): llega la vista entera y luego un parche
 * por evento. Si un parche no sigue al anterior, se pide la vista.
 */
let canal: Channel<MensajeMesa> | null = null;

export async function pedirVista() {
	try {
		useMesa.getState().recibirVista(await call("mesa_vista"));
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
	}
}

/**
 * Mientras se precargan las imágenes de la primera vista, lo que llega espera
 * aquí y luego se aplica en orden (un parche nunca se adelanta a su vista).
 */
let enEspera: MensajeMesa[] | null = null;

function alMensaje(m: MensajeMesa) {
	if (enEspera) {
		enEspera.push(m);
		return;
	}
	// La primera vista (abrir la mesa): antes de enseñarla, sus imágenes.
	if (m.tipo === "vista" && !useMesa.getState().activa) {
		enEspera = [];
		void precargar(m.vista).finally(() => {
			const pendientes = enEspera ?? [];
			enEspera = null;
			aplicar(m);
			for (const x of pendientes) alMensaje(x);
		});
		return;
	}
	aplicar(m);
}

function aplicar(m: MensajeMesa) {
	const s = useMesa.getState();
	switch (m.tipo) {
		case "vista": {
			const antes = s.registro;
			s.recibirVista(m.vista, m.registro);
			reaccionarEnRepeticion(antes);
			break;
		}
		case "parche":
			if (!s.recibirParche(m.parche, m.evento)) void pedirVista();
			// Las tiradas, a la capa de los dados.
			useDados.getState().encolar(m.evento);
			reaccionarAEvento(m.evento);
			break;
		case "cerrada":
			canal = null;
			enEspera = null;
			s.cerrar();
			useRepeticion.getState().poner(null);
			useChat.getState().limpiar();
			break;
		// En red: el anfitrión no hizo algo que se pidió desde aquí.
		case "rechazo":
			useVivo.getState().olvidarSoltadas();
			usePantalla.getState().setArrastre(null);
			if (m.motivo === "SinPermiso") break; // tocar lo de otro no hace nada, sin aviso
			toast.error(
				mensajeDeError({ code: "Rejected", params: { motivo: m.motivo } }, traduccionSuelta),
			);
			break;
	}
}

/** Se suscribe a la partida abierta (si la hay). */
export async function suscribir(): Promise<boolean> {
	const c = new Channel<MensajeMesa>();
	c.onmessage = alMensaje;
	canal = c;
	try {
		await call("mesa_suscribir", { canal: c });
		return true;
	} catch {
		canal = null;
		return false;
	}
}

/** ¿Hay un canal abierto? (para no suscribirse dos veces). */
export function suscrito(): boolean {
	return canal !== null;
}

/** El texto de la barra de trabajos al preparar los decks. */
const PREPARANDO = paraTraducir("Preparando la mesa…");

/**
 * Lanza un trabajo de la mesa (preparar los decks) con el progreso en la
 * barra de trabajos y, al acabar bien, `despues` (de fábrica, suscribirse a
 * la partida abierta).
 */
export async function conProgreso(
	lanzar: () => Promise<number>,
	despues: () => Promise<boolean> = suscribir,
): Promise<boolean> {
	const fase = traduccionSuelta(PREPARANDO);
	useTrabajo.getState().empezar("mesa", fase);
	try {
		const trabajo = await lanzarTrabajo("mesa://progress", "mesa://done", lanzar, (p) => {
			// Todo el progreso en una escala de 0 a 1000 (varios decks).
			const hechas = p.total > 0 ? (p.mazo + p.hechas / p.total) / Math.max(1, p.mazos) : 0;
			useTrabajo.getState().avanzar(fase, Math.round(hechas * 1000), 1000);
		});
		useTrabajo.getState().setId(trabajo.jobId);
		const fin = await trabajo.fin;
		if (fin.error) {
			toast.error(mensajeDeError(fin.error, traduccionSuelta));
			return false;
		}
		return await despues();
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
		return false;
	} finally {
		useTrabajo.getState().terminar();
	}
}

export function empezar(config: ConfigMesa): Promise<boolean> {
	return conProgreso(() => call("mesa_empezar", { config }));
}

export function retomar(): Promise<boolean> {
	return conProgreso(() => call("mesa_retomar"));
}

/** Sale de la mesa: guardando la partida para seguirla luego, o terminándola. */
export async function salir(guardar = false) {
	await call("mesa_salir", { guardar });
	canal = null;
	useMesa.getState().cerrar();
	useRepeticion.getState().poner(null);
	useChat.getState().limpiar();
	// En red, salir de la mesa es salir de la partida (Rust ya la dejó).
	useRed.getState().olvidar();
}

/** Guarda la partida en su fichero (al cambiar de pantalla o cerrar). */
export async function guardar() {
	if (!useMesa.getState().activa) return;
	try {
		await call("mesa_guardar");
	} catch {
		/* se vuelve a intentar al cerrar */
	}
}
