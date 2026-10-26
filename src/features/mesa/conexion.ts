import { Channel } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { ConfigMesa } from "@/bindings/ConfigMesa";
import type { MensajeMesa } from "@/bindings/MensajeMesa";
import { traduccionSuelta } from "@/components/language-provider";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useMesa } from "@/stores/mesa-store";
import { useTrabajo } from "@/stores/trabajo-store";

/**
 * **La pantalla y la partida de Rust.** Se suscribe por un canal
 * (`tauri::ipc::Channel`, en orden): llega la vista entera y luego un parche
 * por evento. Si un parche no sigue al anterior, se pide la vista.
 */
let canal: Channel<MensajeMesa> | null = null;

async function pedirVista() {
	try {
		useMesa.getState().recibirVista(await call("mesa_vista"));
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
	}
}

function alMensaje(m: MensajeMesa) {
	const s = useMesa.getState();
	switch (m.tipo) {
		case "vista":
			s.recibirVista(m.vista, m.registro);
			break;
		case "parche":
			if (!s.recibirParche(m.parche, m.evento)) void pedirVista();
			break;
		case "cerrada":
			s.cerrar();
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

/** Prepara los decks y abre la partida (con el progreso en la barra de trabajos). */
async function conProgreso(lanzar: () => Promise<number>): Promise<boolean> {
	const fase = traduccionSuelta("Preparando la mesa…");
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
		return await suscribir();
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
