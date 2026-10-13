import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import type { ExportPhase } from "@/bindings/ExportPhase";
import type { ExportResult } from "@/bindings/ExportResult";
import { paraTraducir } from "@/components/language-provider";
import type { T } from "@/features/decklist/acciones";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";

const FASES: Record<ExportPhase, string> = {
	prepare: paraTraducir("Preparando imágenes…"),
	compose: paraTraducir("Componiendo páginas…"),
	write: paraTraducir("Escribiendo el PDF…"),
};

/** Por encima de esto se avisa antes de exportar. */
export const AVISO_BYTES = 500 * 1024 * 1024;

export function megas(b: number) {
	return `${Math.round(b / 1024 / 1024)} MB`;
}

/** Un nombre de fichero sin caracteres que los sistemas no admiten. */
export function nombreSeguro(nombre: string, ext: string) {
	const base = nombre.replace(/[\\/:*?"<>|]+/g, " ").trim() || "deck";
	return `${base}.${ext}`;
}

/** Pide dónde guardar el PDF. `null` si se cancela. */
export async function pedirDestino(t: T): Promise<string | null> {
	const deck = useDeck.getState().deck;
	if (!deck) return null;
	return save({
		title: t("Exportar PDF"),
		defaultPath: nombreSeguro(deck.name, "pdf"),
		filters: [{ name: "PDF", extensions: ["pdf"] }],
	});
}

/**
 * Exporta el deck tal como está en pantalla a `destino`. 🛑 Se manda la
 * instantánea del store, no lo que haya en la base: el autoguardado puede ir
 * por detrás.
 */
export async function exportarA(destino: string, t: T): Promise<ExportResult | null> {
	const deck = useDeck.getState().deck;
	if (!deck) return null;
	const trabajo = useTrabajo.getState();
	if (trabajo.actual) throw { code: "Busy", params: {} };
	trabajo.empezar("exportar", t(FASES.prepare));
	try {
		const { jobId, fin } = await lanzarTrabajo(
			"export://progress",
			"export://done",
			() => call("export_pdf", { deck, path: destino }),
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
		if (r) avisarHecho(r, t);
		return r;
	} finally {
		useTrabajo.getState().terminar();
	}
}

function avisarHecho(r: ExportResult, t: T) {
	const texto =
		r.paths.length > 1
			? t("PDF guardado en {f} ficheros ({n} páginas)", { f: r.paths.length, n: r.pages })
			: t("PDF guardado ({n} páginas)", { n: r.pages });
	useUi.getState().anunciar(texto);
	const ruta = r.paths[0];
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
	const destino = await save({
		title: t("Hoja de calibración"),
		defaultPath: nombreSeguro(t("calibracion-duplex"), "pdf"),
		filters: [{ name: "PDF", extensions: ["pdf"] }],
	});
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
		action: { label: t("Abrir"), onClick: () => void call("export_open", { path: ruta }) },
	});
}
