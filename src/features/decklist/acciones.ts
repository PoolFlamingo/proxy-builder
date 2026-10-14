import type { DeckCard } from "@/bindings/DeckCard";
import type { ParsedDecklist } from "@/bindings/ParsedDecklist";
import type { ResolvePhase } from "@/bindings/ResolvePhase";
import type { ResolveResult } from "@/bindings/ResolveResult";
import { paraTraducir } from "@/components/language-provider";
import { aplicarReaplicado, cartaPropia, listaParaReaplicar, nombreDeFichero } from "@/lib/cartas";
import { insertar } from "@/lib/deck-ops";
import { abrirParaImportar, recordar } from "@/lib/dialogos";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";

/**
 * Lo que hacen los botones del panel de la lista. Viven fuera de los
 * componentes para que el menú de las cartas y el arrastre de ficheros usen
 * las mismas.
 */
export type T = (es: string, vars?: Record<string, string | number>) => string;

const FASES: Record<ResolvePhase, string> = {
	lookup: paraTraducir("Buscando cartas…"),
	language: paraTraducir("Buscando el idioma…"),
	tokens: paraTraducir("Buscando tokens…"),
	images: paraTraducir("Descargando imágenes…"),
};

export const EXTENSIONES_IMAGEN = ["png", "jpg", "jpeg", "webp"];

/** Resuelve una lista con las preferencias del panel. `null` si se canceló. */
export async function resolver(
	lista: ParsedDecklist,
	t: T,
	opciones?: { addTokens?: boolean },
): Promise<ResolveResult | null> {
	const ui = useUi.getState();
	const trabajo = useTrabajo.getState();
	if (trabajo.actual) throw { code: "Busy", params: {} };
	trabajo.empezar("resolver", t(FASES.lookup));
	try {
		const { jobId, fin } = await lanzarTrabajo(
			"resolve://progress",
			"resolve://done",
			() =>
				call("resolve_decklist", {
					list: lista,
					options: {
						lang: ui.lang,
						imagePolicy: ui.imagePolicy,
						addTokens: opciones?.addTokens ?? ui.addTokens,
					},
				}),
			(p) =>
				useTrabajo
					.getState()
					.avanzar(t(FASES[p.progress.phase]), p.progress.done, p.progress.total),
		);
		useTrabajo.getState().setId(jobId);
		const d = await fin;
		if (d.error) {
			if (d.error.code === "Cancelled") return null;
			throw d.error;
		}
		return d.result;
	} finally {
		useTrabajo.getState().terminar();
	}
}

export function cancelarTrabajo() {
	const a = useTrabajo.getState().actual;
	if (!a?.jobId) return;
	void call(a.tipo === "exportar" ? "export_cancel" : "job_cancel", { jobId: a.jobId });
}

function avisarNoResueltas(r: ResolveResult, t: T): string | null {
	if (r.unresolved.length === 0) return null;
	const nombres = r.unresolved
		.slice(0, 5)
		.map((u) => u.name)
		.join(", ");
	const mas = r.unresolved.length > 5 ? ` (+${r.unresolved.length - 5})` : "";
	return t("Sin encontrar: {nombres}", { nombres: nombres + mas });
}

/**
 * «Cargar lista» (reemplaza las cartas) o «+ Añadir» (al final). Devuelve el
 * aviso de las que no se encontraron, si hay.
 */
export async function cargarLista(
	texto: string,
	modo: "reemplazar" | "anadir",
	t: T,
): Promise<string | null> {
	const lista = await call("parse_decklist", { text: texto });
	if (lista.cardCount === 0) {
		useUi.getState().anunciar(t("La lista no tiene cartas"));
		return null;
	}
	const r = await resolver(lista, t);
	if (!r) {
		useUi.getState().anunciar(t("Cancelado: no se ha cambiado nada"));
		return null;
	}
	const deck = useDeck.getState();
	if (modo === "reemplazar") {
		deck.aplicar(() => ({ cards: r.cards }));
		useUi.getState().seleccionar([], null);
	} else {
		deck.setCartas((c) => insertar(c, r.cards));
	}
	useUi
		.getState()
		.anunciar(
			modo === "reemplazar"
				? t("{n} cartas cargadas", { n: r.cards.length })
				: t("{n} cartas añadidas", { n: r.cards.length }),
		);
	return avisarNoResueltas(r, t);
}

/** «Reaplicar idioma»: vuelve a pedir las cartas no fijadas en el idioma elegido. */
export async function reaplicarIdioma(t: T): Promise<string | null> {
	const deck = useDeck.getState().deck;
	if (!deck) return null;
	const { lista, ids } = listaParaReaplicar(deck.cards);
	if (ids.length === 0) {
		useUi.getState().anunciar(t("No hay cartas de Scryfall sin fijar"));
		return null;
	}
	const r = await resolver(lista, t, { addTokens: false });
	if (!r) return null;
	useDeck.getState().setCartas((c) =>
		aplicarReaplicado(
			c,
			ids,
			r.cards,
			r.unresolved.map((u) => u.lineNo),
		),
	);
	useUi.getState().anunciar(t("Idioma reaplicado a {n} cartas", { n: r.cards.length }));
	return avisarNoResueltas(r, t);
}

/** Pide imágenes con el diálogo del sistema y las mete en el almacén. */
export async function elegirImagenes(t: T, varias = true) {
	const rutas = await abrirParaImportar({
		multiple: varias,
		directory: false,
		title: t("Elegir imágenes"),
		filters: [{ name: t("Imágenes"), extensions: EXTENSIONES_IMAGEN }],
	});
	if (!rutas || rutas.length === 0) return null;
	const imagenes = await call("blob_import_files", { paths: rutas });
	return { rutas, imagenes };
}

/** Crea cartas propias a partir de rutas del disco (diálogo o arrastre). */
export async function anadirPropias(rutas: string[], t: T, en?: number) {
	if (rutas[0]) recordar("importar", rutas[0]);
	const imagenes = await call("blob_import_files", { paths: rutas });
	const nuevas: DeckCard[] = imagenes.map((img, i) => cartaPropia(img, nombreDeFichero(rutas[i])));
	useDeck.getState().setCartas((c) => insertar(c, nuevas, en));
	useUi.getState().anunciar(t("{n} imágenes propias añadidas", { n: nuevas.length }));
}

export async function subirCartasPropias(t: T) {
	const r = await elegirImagenes(t);
	if (!r) return;
	const nuevas = r.imagenes.map((img, i) => cartaPropia(img, nombreDeFichero(r.rutas[i])));
	useDeck.getState().setCartas((c) => insertar(c, nuevas));
	useUi.getState().anunciar(t("{n} imágenes propias añadidas", { n: nuevas.length }));
}

/** «Subir reversos»: el primero pasa a ser el reverso por defecto del deck. */
export async function subirReverso(t: T) {
	const r = await elegirImagenes(t, false);
	const img = r?.imagenes[0];
	if (!img) return;
	useDeck.getState().aplicar(() => ({ defaultBack: img }));
	useUi.getState().anunciar(t("Reverso del deck cambiado"));
}

export async function reversoGenerico(t: T) {
	const img = await call("blob_generic_back", {});
	useDeck.getState().aplicar(() => ({ defaultBack: img }));
	useUi.getState().anunciar(t("Reverso genérico restablecido"));
}
