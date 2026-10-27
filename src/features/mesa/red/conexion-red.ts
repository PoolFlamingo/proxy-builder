import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { AvisoRed } from "@/bindings/AvisoRed";
import type { ConfigMesa } from "@/bindings/ConfigMesa";
import type { PeticionUnion } from "@/bindings/PeticionUnion";
import { paraTraducir, traduccionSuelta } from "@/components/language-provider";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { useRed } from "@/stores/red-store";
import { useUi } from "@/stores/ui-store";
import { useVivo } from "@/stores/vivo-store";
import { conProgreso, suscribir, suscrito } from "../conexion";

/**
 * **La partida en red desde la pantalla** (código directo, Fase E): crear
 * una y dar el código, unirse con uno, empezar y salir. Rust avisa por
 * `red://aviso` de la sala, de que empieza y de la conexión; aquí se
 * escucha desde el primer uso hasta que se cierra la app.
 */
let escucha: Promise<UnlistenFn> | null = null;

export function escuchar(): Promise<UnlistenFn> {
	escucha ??= listen<AvisoRed>("red://aviso", (e) => alAviso(e.payload));
	return escucha;
}

function alAviso(a: AvisoRed) {
	const r = useRed.getState();
	switch (a.tipo) {
		case "sala":
			r.poner({ sala: a.jugadores });
			break;
		case "empezada":
			r.poner({ empezada: true, conexion: "conectada", intento: 0 });
			if (!suscrito()) void suscribir();
			break;
		case "reconectando":
			r.poner({ conexion: "reconectando", intento: a.intento });
			break;
		case "conectada":
			r.poner({ conexion: "conectada", intento: 0 });
			break;
		case "perdida":
			r.poner({ conexion: "perdida" });
			break;
		case "enlaces":
			r.poner({ desfaseMs: a.desfaseMs, enlaces: a.filas });
			break;
		case "ausentes":
			r.poner({ ausentes: a.filas.map((f) => ({ ...f, visto: Date.now() })) });
			break;
		case "fantasma":
			if (useUi.getState().mesa.movimientoEnVivo) {
				useVivo
					.getState()
					.fantasma(
						a.jugador,
						{ tablero: a.tablero, carta: a.carta, x: a.x, y: a.y },
						performance.now(),
					);
			}
			break;
		case "senal":
			useVivo.getState().senal(a.jugador, a.carta, a.a, performance.now());
			break;
		case "cerrada":
			toast(traduccionSuelta(paraTraducir("El anfitrión ha cerrado la partida.")));
			void salirDeRed();
			break;
		case "rechazada":
			toast.error(
				mensajeDeError({ code: "Rejected", params: { motivo: a.motivo } }, traduccionSuelta),
			);
			void salirDeRed();
			break;
	}
}

/** Pone la pantalla al día con lo que tiene Rust (al abrir la pestaña). */
export async function refrescarRed() {
	try {
		const e = await call("red_estado");
		useRed.getState().poner({
			papel: e.papel,
			codigo: e.codigo,
			// El invitado sabe de la sala por los avisos (Rust no la guarda).
			...(e.papel === "anfitrion" || e.papel === null ? { sala: e.sala } : {}),
			empezada: e.empezada,
		});
		if (e.papel) await escuchar();
	} catch {
		/* sin red */
	}
}

/** Abre una partida en red y deja el código a la vista. */
export async function crearPartida(apodo: string, deck: string | null): Promise<boolean> {
	await escuchar();
	try {
		await call("red_crear", { apodo, deck });
		await refrescarRed();
		return true;
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
		return false;
	}
}

/** Se une con un código (prepara antes el deck, con su progreso). */
export async function unirsePartida(peticion: PeticionUnion): Promise<boolean> {
	await escuchar();
	const ok = await conProgreso(
		() => call("red_unirse", { peticion }),
		async () => true,
	);
	if (ok) await refrescarRed();
	return ok;
}

/** El anfitrión empieza con los de la sala. */
export async function empezarRed(config: ConfigMesa): Promise<boolean> {
	const ok = await conProgreso(() => call("red_empezar", { config }));
	if (ok) useRed.getState().poner({ empezada: true });
	return ok;
}

/** Sale de la partida en red (en la sala; en la mesa, con `salir`). */
export async function salirDeRed() {
	try {
		await call("red_salir");
	} catch {
		/* ya no estaba */
	}
	useRed.getState().olvidar();
	useVivo.getState().limpiar();
}
