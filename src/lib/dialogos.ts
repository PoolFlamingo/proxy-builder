import {
	type OpenDialogOptions,
	open,
	type SaveDialogOptions,
	save,
} from "@tauri-apps/plugin-dialog";
import { call } from "@/lib/ipc";

/**
 * **Diálogos de ficheros que recuerdan la carpeta.** La app guarda (en Rust)
 * la última carpeta desde la que se importó algo y aquella en la que se
 * exportó, por separado, y cada diálogo empieza en la que le toca.
 */

async function inicial(tipo: "importar" | "exportar", nombre?: string) {
	try {
		return (await call("carpeta_inicial", { tipo, nombre: nombre ?? null })) ?? undefined;
	} catch {
		return nombre;
	}
}

/** Recuerda la carpeta de `ruta` (también al arrastrar ficheros a la app). */
export function recordar(tipo: "importar" | "exportar", ruta: string) {
	void call("carpeta_recordar", { tipo, ruta }).catch(() => {});
}

/** Abrir uno o varios ficheros (o una carpeta) para importar. */
export async function abrirParaImportar(
	opciones: Omit<OpenDialogOptions, "defaultPath">,
): Promise<string[] | null> {
	const r = await open({ ...opciones, defaultPath: await inicial("importar") });
	if (!r) return null;
	const rutas = Array.isArray(r) ? r : [r];
	if (rutas[0]) recordar("importar", rutas[0]);
	return rutas;
}

/** Elegir dónde exportar; `nombre` es el fichero propuesto. */
export async function guardarParaExportar(
	opciones: Omit<SaveDialogOptions, "defaultPath">,
	nombre: string,
): Promise<string | null> {
	const r = await save({ ...opciones, defaultPath: await inicial("exportar", nombre) });
	if (r) recordar("exportar", r);
	return r;
}
