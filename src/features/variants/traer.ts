import type { CardSummary } from "@/bindings/CardSummary";
import type { DeckCard } from "@/bindings/DeckCard";
import { call } from "@/lib/ipc";

/**
 * Trae una carta de una búsqueda como carta del deck: de Scryfall o, si es de
 * otra fuente (una ficha de Cockatrice), de allí (una propia con su imagen y
 * el vínculo).
 */
export function traerCarta(c: Pick<CardSummary, "id" | "fuente">): Promise<DeckCard> {
	return c.fuente
		? call("card_fetch_externa", { fuente: c.fuente, id: c.id })
		: call("card_fetch", { scryfallId: c.id });
}
