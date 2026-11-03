import { toast } from "sonner";
import type { Deck } from "@/bindings/Deck";
import type { ExportEstimate } from "@/bindings/ExportEstimate";
import type { ExportPhase } from "@/bindings/ExportPhase";
import type { ExportResult } from "@/bindings/ExportResult";
import { paraTraducir } from "@/components/language-provider";
import type { T } from "@/features/decklist/acciones";
import { guardarParaExportar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { esMovil } from "@/lib/plataforma";
import { lanzarTrabajo } from "@/lib/trabajos";
import { guardarAhora, useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";

const FASES: Record<ExportPhase, string> = {
	prepare: paraTraducir("Preparando imágenes…"),
	compose: paraTraducir("Componiendo páginas…"),
	write: paraTraducir("Escribiendo el PDF…"),
};

/** Por encima de esto se avisa antes de exportar. */
export const AVISO_BYTES = 500 * 1024 * 1024;

/**
 * ¿Se avisa antes de exportar? Si pesa mucho o si el equipo no tiene memoria
 * libre para componerlo (`export_estimate` la mira: en un móvil con poca RAM el
 * sistema cierra la app).
 */
export function pideAviso(e: Pick<ExportEstimate, "bytes" | "lowMemory">): boolean {
	return e.bytes > AVISO_BYTES || e.lowMemory;
}

export function megas(b: number) {
	return `${Math.round(b / 1024 / 1024)} MB`;
}

/** Un nombre de fichero sin caracteres que los sistemas no admiten. */
export function nombreSeguro(nombre: string, ext: string) {
	const base = nombre.replace(/[\\/:*?"<>|]+/g, " ").trim() || "deck";
	return `${base}.${ext}`;
}

/**
 * ¿Saldrá el PDF en varios ficheros (se parte cada N hojas)? En Android no se
 * puede escribir un fichero junto a otro, y los trozos van en un ZIP.
 */
export async function saldraPartido(deck: Deck): Promise<boolean> {
	try {
		return (await call("export_estimate", { deck })).files > 1;
	} catch {
		return false;
	}
}

/** Pide dónde guardar el PDF. `null` si se cancela. */
export async function pedirDestino(t: T): Promise<string | null> {
	const deck = useDeck.getState().deck;
	if (!deck) return null;
	if ((await esMovil()) && (await saldraPartido(deck))) {
		return guardarParaExportar(
			{ title: t("Exportar PDF"), filters: [{ name: "ZIP", extensions: ["zip"] }] },
			nombreSeguro(deck.name, "zip"),
		);
	}
	return guardarParaExportar(
		{ title: t("Exportar PDF"), filters: [{ name: "PDF", extensions: ["pdf"] }] },
		nombreSeguro(deck.name, "pdf"),
	);
}

/** Qué hacer además de exportar. */
export interface OpcionesExportar {
	/**
	 * Es un camino de la caché (`fichero_temporal`) para compartir, no para
	 * guardar: no se saca el aviso de «guardado» y se abre el selector de apps.
	 */
	compartir?: boolean;
	/** Los trozos van en un ZIP aunque el destino sea un camino. */
	zip?: boolean;
}

/**
 * Exporta el deck tal como está en pantalla a `destino`. 🛑 Se manda la
 * instantánea del store, no lo que haya en la base: el autoguardado puede ir
 * por detrás.
 */
export async function exportarA(
	destino: string,
	t: T,
	opciones: OpcionesExportar = {},
): Promise<ExportResult | null> {
	const deck = useDeck.getState().deck;
	if (!deck) return null;
	const trabajo = useTrabajo.getState();
	if (trabajo.actual) throw { code: "Busy", params: {} };
	trabajo.empezar("exportar", t(FASES.prepare));
	try {
		const { jobId, fin } = await lanzarTrabajo(
			"export://progress",
			"export://done",
			() =>
				call(
					"export_pdf",
					opciones.zip ? { deck, path: destino, zip: true } : { deck, path: destino },
				),
			(p) => useTrabajo.getState().avanzar(t(FASES[p.phase]), p.done, p.total),
		);
		useTrabajo.getState().setId(jobId);
		const d = await fin;
		if (d.error) {
			if (d.error.code === "Cancelled") {
				useUi.getState().anunciar(t("Exportación cancelada"));
				return null;
			}
			throw d.error;
		}
		const r = d.result;
		if (r && !opciones.compartir) await avisarHecho(r, t);
		return r;
	} finally {
		useTrabajo.getState().terminar();
	}
}

/** Escribe en la caché lo que haga `escribir` con un camino y lo comparte. */
async function compartirDesdeCache(
	nombre: string,
	escribir: (ruta: string) => Promise<unknown>,
): Promise<void> {
	const ruta = await call("fichero_temporal", { nombre });
	await escribir(ruta);
	await call("compartir_fichero", { ruta });
}

/** «Compartir» la lista del deck como texto plano (solo móvil). */
export async function compartirLista(): Promise<void> {
	const deck = useDeck.getState().deck;
	if (!deck) return;
	await compartirDesdeCache(nombreSeguro(deck.name, "txt"), (path) =>
		call("deck_list_save", { deck, format: "texto", path }),
	);
}

/** «Compartir» el deck entero con sus imágenes, un `.pbdeck` (solo móvil). */
export async function compartirPaquete(): Promise<void> {
	const deck = useDeck.getState().deck;
	if (!deck) return;
	await guardarAhora();
	await compartirDesdeCache(nombreSeguro(deck.name, "pbdeck"), (path) =>
		call("deck_export_package", { id: deck.id, path }),
	);
}

/**
 * «Compartir» el PDF (solo móvil): se exporta a la caché y se abre el
 * selector de apps del sistema.
 */
export async function compartirPdf(t: T): Promise<void> {
	const deck = useDeck.getState().deck;
	if (!deck) return;
	// Con poca memoria, como al exportar: mejor avisar que dejar que el sistema cierre la app.
	const estimacion = await call("export_estimate", { deck });
	if (estimacion.lowMemory) {
		toast.warning(t("A tu equipo le queda poca memoria libre para esto: la app podría cerrarse."));
		return;
	}
	const partido = estimacion.files > 1;
	const ruta = await call("fichero_temporal", {
		nombre: nombreSeguro(deck.name, partido ? "zip" : "pdf"),
	});
	const r = await exportarA(ruta, t, { compartir: true, zip: partido });
	if (r) await call("compartir_fichero", { ruta });
}

async function avisarHecho(r: ExportResult, t: T) {
	const texto =
		r.paths.length > 1
			? t("PDF guardado en {f} ficheros ({n} páginas)", { f: r.paths.length, n: r.pages })
			: t("PDF guardado ({n} páginas)", { n: r.pages });
	useUi.getState().anunciar(texto);
	const ruta = r.paths[0];
	if (await esMovil()) {
		// Sin ruta que enseñar (es un `content://`), sin carpeta a la que ir y sin
		// «abrir» (Android abre con el visor que se elija al compartir).
		toast.success(texto, {
			description: megas(r.bytes),
			duration: 12000,
			action: { label: t("Compartir"), onClick: () => void call("compartir_fichero", { ruta }) },
		});
		return;
	}
	toast.success(texto, {
		description: `${megas(r.bytes)} · ${ruta}`,
		duration: 12000,
		action: { label: t("Abrir"), onClick: () => void call("export_open", { path: ruta }) },
		cancel: {
			label: t("Mostrar en carpeta"),
			onClick: () => void call("export_reveal", { path: ruta }),
		},
	});
}

/** La hoja de calibración dúplex, con los ajustes actuales. */
export async function exportarCalibracion(t: T) {
	const deck = useDeck.getState().deck;
	if (!deck) return;
	const destino = await guardarParaExportar(
		{ title: t("Hoja de calibración"), filters: [{ name: "PDF", extensions: ["pdf"] }] },
		nombreSeguro(t("calibracion-duplex"), "pdf"),
	);
	if (!destino) return;
	const ruta = await call("export_calibration", {
		settings: deck.settings,
		path: destino,
		texts: {
			title: t("Hoja de calibración dúplex"),
			instructions: [
				t("1. Imprime esta hoja a doble cara con los mismos ajustes que usarás para el deck."),
				t(
					"2. Mira la hoja a contraluz: cada cruz roja del reverso debería caer en el centro de su casilla.",
				),
				t("3. Mide con la regla cuánto se desvía cada fila (derecha y abajo, en positivo)."),
				t("4. En «Alinear filas», pulsa «Corregir con lo medido» y teclea esas medidas."),
			],
		},
	});
	useUi.getState().anunciar(t("Hoja de calibración guardada"));
	toast.success(t("Hoja de calibración guardada"), {
		action: (await esMovil())
			? { label: t("Compartir"), onClick: () => void call("compartir_fichero", { ruta }) }
			: { label: t("Abrir"), onClick: () => void call("export_open", { path: ruta }) },
	});
}
