import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { ComoEnSala } from "@/bindings/ComoEnSala";
import type { EnlaceSala } from "@/bindings/EnlaceSala";
import type { EstadoSala } from "@/bindings/EstadoSala";
import type { EstadoSalas } from "@/bindings/EstadoSalas";
import type { Formato } from "@/bindings/Formato";
import type { HostSala } from "@/bindings/HostSala";
import type { MotivoFuera } from "@/bindings/MotivoFuera";
import type { Playgroup } from "@/bindings/Playgroup";
import type { RolPlaygroup } from "@/bindings/RolPlaygroup";
import type { SalaNueva } from "@/bindings/SalaNueva";
import { paraTraducir, traduccionSuelta } from "@/components/language-provider";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { useRed } from "@/stores/red-store";
import { useSalas } from "@/stores/salas-store";
import { conProgreso } from "../mesa/conexion";
import { escuchar as escucharRed } from "../mesa/red/conexion-red";

/**
 * **Playgroups y salas desde la pantalla.** Habla con Rust por `salas_*` y
 * escucha sus avisos (`salas://…`) desde el primer uso. Cuando la sala
 * empieza, la partida llega como en la red por código (`red://aviso`).
 */
let escuchando = false;

function escuchar() {
	if (escuchando) return;
	escuchando = true;
	void escucharRed();
	const s = () => useSalas.getState();
	void listen<boolean>("salas://conexion", (e) => s().poner({ conectado: e.payload }));
	void listen<Playgroup[]>("salas://playgroups", (e) => s().poner({ playgroups: e.payload }));
	void listen<EstadoSala>("salas://sala", (e) => {
		if (s().sala && s().sala?.sala.id !== e.payload.sala.id) return;
		s().poner({ sala: e.payload });
	});
	void listen<EnlaceSala[]>("salas://enlaces", (e) =>
		s().poner({ enlaces: Object.fromEntries(e.payload.map((x) => [x.clave, x])) }),
	);
	void listen<HostSala>("salas://host", (e) => {
		const { yo, como } = s();
		useRed.getState().poner({
			papel: e.payload.host === yo ? "anfitrion" : como === "espectador" ? "espectador" : "jugador",
			empezada: true,
		});
		if (e.payload.epoch > 1) {
			toast(traduccionSuelta(paraTraducir("El host se ha ido: la partida sigue con otro.")));
		}
	});
	void listen<MotivoFuera>("salas://fuera", (e) => {
		if (e.payload === "terminada") {
			toast(traduccionSuelta(paraTraducir("La sala ha terminado.")));
		} else if (e.payload === "expulsado") {
			toast.error(traduccionSuelta(paraTraducir("Quien creó la sala te ha expulsado de ella.")));
		}
		s().sinSala();
	});
}

function aviso(e: unknown) {
	toast.error(mensajeDeError(e, traduccionSuelta));
}

function ponerEstado(e: EstadoSalas) {
	useSalas.getState().poner({
		hayServidor: e.hayServidor,
		conectado: e.conectado,
		yo: e.yo,
		sala: e.sala,
		como: e.como,
		deck: e.deck,
	});
}

/** Conecta con el servidor (si hay) y trae los playgroups. */
export async function conectar(apodo: string) {
	escuchar();
	try {
		const e = await call("salas_conectar", { apodo });
		ponerEstado(e);
		if (e.conectado) useSalas.getState().poner({ playgroups: await call("salas_playgroups") });
	} catch (e) {
		aviso(e);
	}
}

/** Lo que haga falta con una respuesta de playgroups. */
async function conPlaygroups(f: () => Promise<Playgroup[]>): Promise<boolean> {
	try {
		useSalas.getState().poner({ playgroups: await f() });
		return true;
	} catch (e) {
		aviso(e);
		return false;
	}
}

export const crearPlaygroup = (nombre: string) =>
	conPlaygroups(() => call("salas_pg_crear", { nombre }));
export const salirDePlaygroup = (playgroup: string) =>
	conPlaygroups(() => call("salas_pg_salir", { playgroup }));
export const expulsar = (playgroup: string, clave: string) =>
	conPlaygroups(() => call("salas_pg_expulsar", { playgroup, clave }));
export const cambiarRol = (playgroup: string, clave: string, rol: RolPlaygroup) =>
	conPlaygroups(() => call("salas_pg_rol", { playgroup, clave, rol }));

/** Una invitación (a un playgroup o, sin él, a la sala en la que se está). */
export async function invitar(playgroup: string | null): Promise<string | null> {
	try {
		return (await call("salas_invitar", { playgroup })).codigo;
	} catch (e) {
		aviso(e);
		return null;
	}
}

function entrarEnSala(e: EstadoSala, como: ComoEnSala) {
	useSalas.getState().poner({ sala: e, como, deck: null, enlaces: {} });
}

/** Usa un código: un playgroup (se añade a la lista) o una sala (se entra). */
export async function usarCodigo(
	codigo: string,
	como: ComoEnSala,
	apodo: string,
): Promise<boolean> {
	escuchar();
	try {
		const r = await call("salas_usar", { codigo, como, apodo });
		if (r.tipo === "playgroups") useSalas.getState().poner({ playgroups: r.playgroups });
		else entrarEnSala(r.estado, como);
		return true;
	} catch (e) {
		aviso(e);
		return false;
	}
}

export async function crearSala(nueva: SalaNueva, apodo: string): Promise<boolean> {
	try {
		entrarEnSala(await call("salas_sala_crear", { nueva, apodo }), "jugador");
		return true;
	} catch (e) {
		aviso(e);
		return false;
	}
}

export async function unirseASala(sala: string, como: ComoEnSala, apodo: string): Promise<boolean> {
	try {
		entrarEnSala(await call("salas_sala_unirse", { sala, como, apodo }), como);
		return true;
	} catch (e) {
		aviso(e);
		return false;
	}
}

export async function salirDeSala() {
	try {
		await call("salas_sala_salir");
	} catch (e) {
		aviso(e);
	}
	useSalas.getState().sinSala();
	useRed.getState().olvidar();
}

/** Prepara el deck para la sala (con su progreso) y lo dice al servidor. */
export async function prepararDeck(
	deckId: string,
	lista: string | null,
	formato: Formato,
	nombre: string,
) {
	const ok = await conProgreso(
		() => call("salas_deck", { deckId, lista, formato }),
		async () => true,
	);
	if (ok) useSalas.getState().poner({ deck: nombre });
}

export async function empezarSala() {
	try {
		await call("salas_empezar");
	} catch (e) {
		aviso(e);
	}
}
