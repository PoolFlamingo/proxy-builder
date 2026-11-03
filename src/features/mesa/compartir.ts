import { call } from "@/lib/ipc";
import { useEsMovil } from "@/lib/plataforma";

/**
 * **Copiar y compartir un texto** (el código o el enlace de una invitación).
 * Copiar vale en todas partes; «Compartir» (el selector de apps de Android,
 * `app_compartir_texto`) solo en el móvil (`lib/plataforma.ts`).
 */

/** Copia al portapapeles; devuelve si pudo (algunas vistas web lo niegan sin un gesto). */
export async function copiarTexto(texto: string): Promise<boolean> {
	try {
		await navigator.clipboard.writeText(texto);
		return true;
	} catch {
		return false;
	}
}

/** Manda el texto al selector de apps del sistema. Devuelve si pudo. */
export async function compartirTexto(texto: string): Promise<boolean> {
	try {
		await call("app_compartir_texto", { texto });
		return true;
	} catch {
		return false;
	}
}

/** ¿Se puede compartir un texto con otra app? Solo en el móvil. */
export function usePuedeCompartir(): boolean {
	return useEsMovil();
}
