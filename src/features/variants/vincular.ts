import type { CardSummary } from "@/bindings/CardSummary";
import type { DeckCard } from "@/bindings/DeckCard";
import { paraTraducir } from "@/components/language-provider";
import type { T } from "@/features/decklist/acciones";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
import { useUi } from "@/stores/ui-store";

/**
 * **Vincular una carta propia con la carta real de Scryfall.** La imagen
 * sigue siendo la propia; lo que cambia es que la carta «sabe» qué es (nombre,
 * set, tipo…) y cuenta en las estadísticas. Se aplica a todas las copias con
 * la misma imagen.
 */

const esPropia = (c: DeckCard) => c.source.kind === "custom";

/** Para comparar nombres: sin mayúsculas, tildes ni signos. */
export function normalizar(s: string | null | undefined): string {
	return (s ?? "")
		.normalize("NFD")
		.replace(/\p{Diacritic}/gu, "")
		.toLowerCase()
		.replace(/[^\p{L}\p{N}]+/gu, " ")
		.trim();
}

/** Vincula la carta `id` (y las copias con su misma imagen) con `scryfallId`. */
export async function vincular(id: string, scryfallId: string): Promise<string> {
	const meta = await call("card_link", { scryfallId });
	const blob = useDeck.getState().deck?.cards.find((c) => c.id === id)?.front?.blob;
	useDeck.getState().setCartas((cs) =>
		cs.map((c) =>
			esPropia(c) && (c.id === id || (blob && c.front?.blob === blob))
				? {
						...c,
						source: { kind: "custom", scryfallId },
						// La imagen es propia: su estado no es el de Scryfall.
						meta: { ...meta, imageStatus: null, highres: c.meta.highres, langFallback: false },
					}
				: c,
		),
	);
	return meta.name;
}

/** Quita el vínculo (de esta carta y de sus copias con la misma imagen). */
export function desvincular(id: string) {
	const blob = useDeck.getState().deck?.cards.find((c) => c.id === id)?.front?.blob;
	useDeck
		.getState()
		.setCartas((cs) =>
			cs.map((c) =>
				esPropia(c) && (c.id === id || (blob && c.front?.blob === blob))
					? { ...c, source: { kind: "custom" } }
					: c,
			),
		);
}

/** La candidata que coincide de verdad con alguno de los textos (o ninguna). */
export function coincidencia(candidatas: CardSummary[], textos: string[]): CardSummary | null {
	const buscados = new Set(textos.map(normalizar).filter((t) => t.length >= 3));
	const nombres = (c: CardSummary) => [
		c.name,
		c.printedName,
		...c.name.split("//"), // cada cara de una DFC
	];
	return candidatas.find((c) => nombres(c).some((n) => buscados.has(normalizar(n)))) ?? null;
}

const FASE = paraTraducir("Identificando cartas propias…");

/**
 * Vincula solas las cartas propias sin vincular cuyo nombre (el del fichero o
 * el leído por OCR) coincide exactamente con una carta de Scryfall. Las
 * dudosas se dejan para hacerlas a mano.
 */
export async function autoVincular(t: T, ocr: boolean) {
	const trabajo = useTrabajo.getState();
	if (trabajo.actual) throw { code: "Busy", params: {} };
	const cartas = useDeck.getState().deck?.cards ?? [];
	const porImagen = new Map<string, DeckCard>();
	for (const c of cartas) {
		if (c.source.kind === "custom" && !c.source.scryfallId && c.front)
			porImagen.set(c.front.blob, c);
	}
	const lista = [...porImagen.values()];
	if (lista.length === 0) {
		useUi.getState().anunciar(t("No hay cartas propias sin vincular"));
		return;
	}
	trabajo.empezar("vincular", t(FASE));
	let hechas = 0;
	let vinculadas = 0;
	try {
		for (const c of lista) {
			if (useTrabajo.getState().actual?.parar) break;
			useTrabajo.getState().avanzar(t(FASE), hechas, lista.length);
			const blob = c.front?.blob;
			if (blob) {
				const r = await call("card_identify", { blob, hint: c.meta.name || null, ocr });
				const elegida = coincidencia(r.candidates, [c.meta.name, ...r.lines]);
				if (elegida) {
					await vincular(c.id, elegida.id);
					vinculadas++;
				}
			}
			hechas++;
		}
	} finally {
		useTrabajo.getState().terminar();
	}
	useUi.getState().anunciar(
		t("{v} de {n} cartas propias vinculadas; el resto, a mano (clic derecho → Vincular)", {
			v: vinculadas,
			n: lista.length,
		}),
	);
}
