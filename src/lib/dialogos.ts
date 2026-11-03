import {
	type DialogFilter,
	type OpenDialogOptions,
	open,
	type SaveDialogOptions,
	save,
} from "@tauri-apps/plugin-dialog";
import { call } from "@/lib/ipc";
import { esMovil } from "@/lib/plataforma";

/**
 * **Diálogos de ficheros que recuerdan la carpeta.** La app guarda (en Rust)
 * la última carpeta desde la que se importó algo y aquella en la que se
 * exportó, por separado, y cada diálogo empieza en la que le toca.
 *
 * En **Android** los selectores del sistema devuelven un `content://` (sin
 * carpeta ni extensión): no se recuerda nada, `defaultPath` es solo el nombre
 * propuesto, los filtros pasan a tipos MIME (Android no conoce `.pbdeck` ni
 * `.ttf`) y no hay selector de carpetas (`elegirCarpetaParaExportar` falla;
 * los PNG salen en un ZIP). Rust (`fichero.rs`) sabe leer y escribir esas URI.
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

// ── Filtros en Android ───────────────────────────────────────────────────

/** Los tipos MIME que Android da a estas extensiones (varios si hay variantes). */
const MIME: Record<string, string[]> = {
	png: ["image/png"],
	jpg: ["image/jpeg"],
	jpeg: ["image/jpeg"],
	webp: ["image/webp"],
	tif: ["image/tiff"],
	tiff: ["image/tiff"],
	bmp: ["image/bmp"],
	pdf: ["application/pdf"],
	zip: ["application/zip", "application/x-zip-compressed"],
	txt: ["text/plain"],
};

/** Lo que Android pone a un fichero que no conoce (`.pbdeck`, `.ttf`…). */
const DESCONOCIDO = "application/octet-stream";

/**
 * Los filtros de un selector para abrir, en Android. Si todas las extensiones
 * tienen tipo conocido se filtra por tipo; si alguna no (`.pbdeck`, `.ttf`,
 * `.otf`), el fichero puede venir de cualquier proveedor con cualquier tipo y
 * filtrar lo escondería: se deja elegir entre todos.
 */
export function filtrosParaAbrirEnMovil(filtros: DialogFilter[] | undefined): DialogFilter[] {
	const todas = (filtros ?? []).flatMap((f) => f.extensions.map((e) => e.toLowerCase()));
	if (todas.length === 0 || todas.some((e) => !MIME[e])) return [];
	return (filtros ?? []).map((f) => ({
		name: f.name,
		extensions: [...new Set(f.extensions.flatMap((e) => MIME[e.toLowerCase()] ?? []))],
	}));
}

/**
 * El filtro de un selector para guardar, en Android: un solo tipo (el del
 * primer formato) y, si no lo conoce, el binario genérico, con el que Android
 * respeta el nombre tal cual (sin pegarle una extensión).
 */
export function filtrosParaGuardarEnMovil(filtros: DialogFilter[] | undefined): DialogFilter[] {
	const f = filtros?.[0];
	const ext = f?.extensions[0]?.toLowerCase();
	const mime = (ext ? MIME[ext]?.[0] : undefined) ?? DESCONOCIDO;
	return [{ name: f?.name ?? "", extensions: [mime] }];
}

// ── Los diálogos ─────────────────────────────────────────────────────────

/** Abrir uno o varios ficheros (o una carpeta) para importar. */
export async function abrirParaImportar(
	opciones: Omit<OpenDialogOptions, "defaultPath">,
): Promise<string[] | null> {
	if (await esMovil()) {
		const r = await open({ ...opciones, filters: filtrosParaAbrirEnMovil(opciones.filters) });
		if (!r) return null;
		return Array.isArray(r) ? r : [r];
	}
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
	if (await esMovil()) {
		return save({
			...opciones,
			filters: filtrosParaGuardarEnMovil(opciones.filters),
			defaultPath: nombre,
		});
	}
	const r = await save({ ...opciones, defaultPath: await inicial("exportar", nombre) });
	if (r) recordar("exportar", r);
	return r;
}

/**
 * Elegir la carpeta donde exportar (imágenes sueltas); recuerda la carpeta.
 * No existe en Android (`FolderPickerNotImplemented`): ahí se exporta un ZIP
 * con `guardarParaExportar`.
 */
export async function elegirCarpetaParaExportar(): Promise<string | null> {
	if (await esMovil()) throw { code: "Unsupported", params: { detalle: "carpeta" } };
	const r = await open({
		directory: true,
		multiple: false,
		defaultPath: await inicial("exportar"),
	});
	const ruta = Array.isArray(r) ? r[0] : r;
	if (!ruta) return null;
	recordar("exportar", ruta);
	return ruta;
}

/**
 * Dónde exportar un montón de imágenes con sus carpetas por idioma
 * (`es/`, `en/`…): en escritorio, una carpeta; en Android, que no tiene
 * selector de carpetas, un ZIP con esas mismas carpetas. Lo que devuelve va
 * tal cual a `design_export_png`.
 */
export async function elegirDestinoDeImagenes(zip: {
	titulo: string;
	nombre: string;
}): Promise<string | null> {
	if (await esMovil()) {
		return guardarParaExportar(
			{ title: zip.titulo, filters: [{ name: "ZIP", extensions: ["zip"] }] },
			zip.nombre,
		);
	}
	return elegirCarpetaParaExportar();
}
