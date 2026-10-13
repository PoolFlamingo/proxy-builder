import type { DeckCard } from "@/bindings/DeckCard";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";

/**
 * **Vincular una carta propia con la carta real de Scryfall.** La imagen
 * sigue siendo la propia; lo que cambia es que la carta «sabe» qué es (nombre,
 * set, tipo…) y cuenta en las estadísticas. Se aplica a todas las copias con
 * la misma imagen.
 */

const esPropia = (c: DeckCard) => c.source.kind === "custom";

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

/** Una carta propia por imagen (las copias van juntas), de entre `ids` o de todo el deck. */
export function propiasPorImagen(ids?: string[]): DeckCard[] {
	const filtro = ids ? new Set(ids) : null;
	const porImagen = new Map<string, DeckCard>();
	for (const c of useDeck.getState().deck?.cards ?? []) {
		if (filtro && !filtro.has(c.id)) continue;
		if (esPropia(c) && c.front && !porImagen.has(c.front.blob)) porImagen.set(c.front.blob, c);
	}
	return [...porImagen.values()];
}

export const estaVinculada = (c: DeckCard) => c.source.kind === "custom" && !!c.source.scryfallId;
