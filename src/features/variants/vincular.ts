import type { CardSource } from "@/bindings/CardSource";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FuenteCarta } from "@/bindings/FuenteCarta";
import { quitarDiseno, scryfallDe } from "@/features/disenar/grupos";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";

/**
 * **Vincular una carta propia con la carta real de Scryfall.** La imagen
 * sigue siendo la propia; lo que cambia es que la carta «sabe» qué es (nombre,
 * set, tipo…) y cuenta en las estadísticas. Se aplica a todas las copias con
 * la misma imagen.
 */

const esPropia = (c: DeckCard) => c.source.kind === "custom";

/** La carta con la que se vincula: un id de Scryfall o una de otra fuente. */
export type Destino = string | { id: string; fuente?: FuenteCarta | null };

/**
 * ¿Vincular `c` con `destino` la cambia de carta? Entonces su diseño (hecho
 * con los datos de la anterior) ya no vale.
 */
export function cambiaDeCarta(c: DeckCard, destino: Destino): boolean {
	const d = typeof destino === "string" ? { id: destino, fuente: null } : destino;
	return !!d.fuente || scryfallDe(c) !== d.id;
}

/**
 * Vincula la carta `id` (y las copias con su misma imagen) con `destino`: un
 * id de Scryfall o una carta de otra fuente (una ficha de Cockatrice).
 * `idiomaImagen`: el idioma de la imagen si se sabe (el pie leído por el
 * OCR); si no, se queda el que tuviera. Una carta con diseño de otra carta
 * lo pierde (vuelve a como era antes de diseñarla, con el vínculo nuevo): si
 * no, el editor seguiría mostrando el diseño de la carta anterior, y «Quitar
 * diseño» le devolvería el vínculo viejo.
 */
export async function vincular(
	id: string,
	destino: Destino,
	idiomaImagen?: string | null,
): Promise<string> {
	const d = typeof destino === "string" ? { id: destino, fuente: null } : destino;
	const meta = d.fuente
		? await call("card_link_externa", { fuente: d.fuente, id: d.id })
		: await call("card_link", { scryfallId: d.id });
	const source: CardSource = d.fuente
		? { kind: "custom", externa: { fuente: d.fuente, id: d.id } }
		: { kind: "custom", scryfallId: d.id };
	const blob = useDeck.getState().deck?.cards.find((c) => c.id === id)?.front?.blob;
	useDeck.getState().setCartas((cs) =>
		cs.map((c) =>
			esPropia(c) && (c.id === id || (blob && c.front?.blob === blob))
				? {
						...(c.design && cambiaDeCarta(c, destino) ? quitarDiseno([c], new Set([c.id]))[0] : c),
						source,
						// La imagen es propia: su estado y su idioma no son los de Scryfall.
						meta: {
							...meta,
							imageStatus: null,
							highres: c.meta.highres,
							langFallback: false,
							imageLang: idiomaImagen ?? c.meta.imageLang,
						},
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

export const estaVinculada = (c: DeckCard) =>
	c.source.kind === "custom" && (!!c.source.scryfallId || !!c.source.externa);
