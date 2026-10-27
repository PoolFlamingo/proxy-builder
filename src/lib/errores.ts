import type { AppError } from "@/bindings/AppError";
import type { ErrorCode } from "@/bindings/ErrorCode";
import { paraTraducir } from "@/components/language-provider";

/**
 * El texto de cada código de error de Rust (`src-tauri/src/error.rs`), en
 * español; `t()` lo pasa al idioma de la interfaz. Los `{datos}` salen de
 * `params`.
 */
const TEXTOS: Record<ErrorCode, string> = {
	Network: paraTraducir("No hay conexión o el servidor no responde."),
	RateLimited: paraTraducir("El servicio pide ir más despacio. Vuelve a intentarlo en un momento."),
	NotFound: paraTraducir("No se ha encontrado lo que se buscaba."),
	Io: paraTraducir("No se ha podido leer o escribir en el disco."),
	Db: paraTraducir("La biblioteca local ha dado un error."),
	InvalidInput: paraTraducir("Los datos no son válidos."),
	Cancelled: paraTraducir("Cancelado."),
	Pdf: paraTraducir("No se ha podido generar el PDF."),
	Image: paraTraducir("No se ha podido procesar la imagen."),
	Unsupported: paraTraducir("Ese formato de fichero no se admite."),
	Busy: paraTraducir("Hay un trabajo en marcha; espera a que termine."),
	Internal: paraTraducir("Error interno inesperado."),
	Rejected: paraTraducir("La mesa no deja hacer eso ahora."),
	Server: paraTraducir("El servidor de salas ha dado un error."),
};

/**
 * Los rechazos de la mesa (`mesa::Rechazo`, en `params.motivo`), la
 * actualización que espera a la partida (`Busy` con `motivo: partida`) y los
 * errores del servidor de salas (`params.code`), con su texto propio. Lo que
 * no esté aquí sale con el texto general de su código.
 */
const MOTIVOS: Partial<Record<ErrorCode, Record<string, string>>> = {
	Busy: {
		partida: paraTraducir("Hay una partida abierta: termínala o sal de la mesa para actualizar."),
	},
	InvalidInput: {
		falta_lista: paraTraducir(
			"Este deck tiene más cartas de las que se juegan: elige una lista para jugar.",
		),
		lista: paraTraducir("La lista para jugar no vale para este formato: revísala."),
		lista_sin_nombre: paraTraducir("Ponle un nombre a la lista."),
		codigo: paraTraducir("Ese código no es válido: cópialo entero, empieza por PB1-."),
		deck: paraTraducir("Elige un deck para jugar."),
		identidad: paraTraducir("Ese fichero no es una identidad de Proxy Builder."),
	},
	Network: {
		sin_servidor: paraTraducir("No hay conexión con el servidor de salas."),
		sin_respuesta: paraTraducir("El anfitrión no responde."),
		desconectado: paraTraducir("Se ha perdido la conexión con la partida."),
	},
	NotFound: {
		sin_sala: paraTraducir("No hay ninguna partida en red abierta."),
	},
	Rejected: {
		NoExiste: paraTraducir("Esa carta o ese jugador ya no está ahí."),
		SinPermiso: paraTraducir("Eso es de otro jugador: no puedes tocarlo."),
		ZonaVacia: paraTraducir("No queda ninguna carta ahí."),
		FueraDeTurno: paraTraducir("Solo el jugador activo puede hacer eso."),
		Limite: paraTraducir("Es demasiado (un número o un texto pasa del límite)."),
		NoSePuede: paraTraducir("Eso no se puede hacer ahora."),
		codigo: paraTraducir("El código no es el de esta partida: pide uno nuevo al anfitrión."),
		version: paraTraducir("Tenéis versiones distintas de la app: actualizad los dos."),
		llena: paraTraducir("La partida está llena."),
		empezada: paraTraducir("La partida ya ha empezado: puedes entrar a mirar."),
		sin_deck: paraTraducir("La partida empezó sin ti: no habías elegido deck."),
		ritmo: paraTraducir("Demasiadas acciones seguidas: espera un momento."),
		espectador: paraTraducir("Estás mirando: los espectadores no pueden jugar."),
		sin_partida: paraTraducir("La partida aún no ha empezado."),
		expulsado: paraTraducir("Quien creó la sala te ha expulsado de ella."),
		no_anfitrion: paraTraducir("Solo el anfitrión puede hacer eso."),
		sin_mayoria: paraTraducir(
			"Sin la mayoría de los jugadores conectados no se juega: espera a que vuelvan.",
		),
		no_activo: paraTraducir("Ese anfitrión aún no tiene la mayoría de los jugadores."),
		partida_abierta: paraTraducir(
			"Hay una partida abierta: sal de la mesa para ver una repetición.",
		),
		sin_registro: paraTraducir("Esta partida no se guardó entera: no se puede repetir."),
		sin_terminar: paraTraducir("La partida aún no ha terminado."),
		sala_cambio: paraTraducir(
			"La sala ha cambiado mientras elegías: revisa los jugadores y vuelve a empezar.",
		),
		ajeno: paraTraducir("En red cada uno cambia solo su propio mazo."),
		sin_mazos: paraTraducir(
			"Tras el traspaso del anfitrión no se puede abrir otra partida del encuentro: ya no están los mazos de todos.",
		),
		cambio: paraTraducir("Ese cambio de banquillo no encaja: revisa las cartas."),
		repeticion: paraTraducir("Estás viendo una repetición: no se puede jugar."),
	},
	Unsupported: {
		version: paraTraducir(
			"Esta partida es de una versión más nueva de la app: actualiza para verla.",
		),
	},
	// Los códigos del servidor de salas (`params.code`; `server/docs/protocolo.md`).
	Server: {
		version: paraTraducir("El servidor de salas usa otra versión: actualiza la app."),
		firma: paraTraducir("El servidor no ha reconocido esta instalación."),
		limite: paraTraducir("Has llegado al límite: espera un momento o deja algo libre."),
		no_existe: paraTraducir("Eso ya no existe en el servidor."),
		caducada: paraTraducir("La invitación ha caducado o ya se usó del todo."),
		llena: paraTraducir("Está llena."),
		sin_permiso: paraTraducir("No tienes permiso para eso."),
		ya_empezada: paraTraducir("La partida ya ha empezado: puedes entrar a mirar."),
		expulsado: paraTraducir("Quien creó la sala te ha expulsado de ella."),
		mensaje: paraTraducir("El servidor no ha entendido la petición."),
	},
};

/** ¿Es un `AppError` de Rust? (Lo que llega del `invoke` es `unknown`.) */
export function esAppError(e: unknown): e is AppError {
	if (typeof e !== "object" || e === null) return false;
	const code = (e as { code?: unknown }).code;
	return typeof code === "string" && code in TEXTOS;
}

/** Cualquier error, en una frase para la persona. */
export function mensajeDeError(
	e: unknown,
	t: (es: string, vars?: Record<string, string | number>) => string,
): string {
	if (esAppError(e)) {
		const clave = e.params.motivo ?? e.params.code;
		const propio = clave ? MOTIVOS[e.code]?.[clave] : undefined;
		return t(propio ?? TEXTOS[e.code], e.params);
	}
	if (e instanceof Error) return e.message;
	return String(e);
}
