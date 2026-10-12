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
	if (esAppError(e)) return t(TEXTOS[e.code], e.params);
	if (e instanceof Error) return e.message;
	return String(e);
}
