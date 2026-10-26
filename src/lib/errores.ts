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
	Rejected: {
		NoExiste: paraTraducir("Esa carta o ese jugador ya no está ahí."),
		SinPermiso: paraTraducir("Eso es de otro jugador: no puedes tocarlo."),
		ZonaVacia: paraTraducir("No queda ninguna carta ahí."),
		FueraDeTurno: paraTraducir("Solo el jugador activo puede hacer eso."),
		Limite: paraTraducir("Es demasiado (un número o un texto pasa del límite)."),
		NoSePuede: paraTraducir("Eso no se puede hacer ahora."),
	},
	Server: {},
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
