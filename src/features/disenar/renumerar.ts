import { toast } from "sonner";
import type { DeckCard } from "@/bindings/DeckCard";
import type { DesignLang } from "@/bindings/DesignLang";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

type T = (es: string, p?: Record<string, string | number>) => string;

/** Pone las cartas que devuelve Rust en el deck, de una vez. */
export function aplicarCambiadas(nuevas: DeckCard[]) {
	if (nuevas.length === 0) return;
	const porId = new Map(nuevas.map((c) => [c.id, c]));
	useDeck.getState().setCartas((cs) => cs.map((c) => porId.get(c.id) ?? c));
}

/** «Renumerar»: los números de colección por el orden del deck. */
export async function renumerar(t: T, mensaje: (e: unknown) => string) {
	const deck = useDeck.getState().deck;
	if (!deck) return;
	try {
		const nuevas = await call("design_renumber", {
			cards: deck.cards,
			lang: deck.settings.designLang,
		});
		aplicarCambiadas(nuevas);
		useUi
			.getState()
			.anunciar(
				nuevas.length
					? t("{n} cartas renumeradas", { n: nuevas.length })
					: t("La numeración ya estaba al día"),
			);
	} catch (e) {
		toast.error(mensaje(e));
	}
}

/**
 * Cambia el idioma de las cartas diseñadas del deck y rehace sus miniaturas
 * (las de la hoja se pintan al guardar el diseño, con el idioma de entonces).
 */
export async function ponerIdiomaDiseno(lang: DesignLang) {
	useDeck.getState().setAjustes((a) => ({ ...a, designLang: lang }));
	await rehacerMiniaturas();
}

/**
 * Rehace las miniaturas de los diseños que se han quedado viejos (cambió una
 * colección, una letra, el pack…). Se llama al abrir un deck y al cerrar los
 * diálogos de colecciones y recursos.
 */
export async function rehacerMiniaturas() {
	const deck = useDeck.getState().deck;
	if (!deck?.cards.some((c) => c.design)) return;
	try {
		const nuevas = await call("design_rerender", {
			cards: deck.cards,
			lang: deck.settings.designLang,
		});
		// Sin historial: no es un cambio del usuario.
		useDeck.getState().refrescarCartas(deck.cards, nuevas);
	} catch {
		/* se rehará la próxima vez */
	}
}
