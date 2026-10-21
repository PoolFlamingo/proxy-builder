import { toast } from "sonner";
import type { DeckCard } from "@/bindings/DeckCard";
import type { DesignLang } from "@/bindings/DesignLang";
import { idioma } from "@/lib/idiomas";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { useDeck } from "@/stores/deck-store";
import { useTrabajo } from "@/stores/trabajo-store";
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
 * En un idioma que no es el español ni el inglés, antes se completa: Rust
 * baja las letras (japonés, coreano, chino) y la impresión de cada carta a
 * la que le falte ese idioma.
 */
export async function ponerIdiomaDiseno(lang: DesignLang, t: T, mensaje: (e: unknown) => string) {
	useDeck.getState().setAjustes((a) => ({ ...a, designLang: lang }));
	if (lang !== "es" && lang !== "en") await completarIdioma(lang, t, mensaje);
	await rehacerMiniaturas();
}

/**
 * El trabajo de Rust que completa `lang` en las cartas diseñadas del deck
 * (o solo en `ids`). Lo hecho entra en el deck sin historial.
 */
export async function completarIdioma(
	lang: DesignLang,
	t: T,
	mensaje: (e: unknown) => string,
	ids?: Set<string>,
) {
	const deck = useDeck.getState().deck;
	if (!deck || useTrabajo.getState().actual) return;
	const antes = deck.cards;
	const disenadas = antes.filter((c) => c.design && (!ids || ids.has(c.id)));
	const nombre = idioma(lang).nativo;
	const fase = t("Preparando las cartas en {idioma}…", { idioma: nombre });
	const aviso = toast.loading(fase);
	const hechas: DeckCard[] = [];
	const trabajo = useTrabajo.getState();
	trabajo.empezar("idioma", fase);
	try {
		const { jobId, fin } = await lanzarTrabajo(
			"designlang://progress",
			"designlang://done",
			() => call("design_complete_lang", { cards: disenadas, lang }),
			(p) => {
				if (p.card) hechas.push(p.card);
				useTrabajo.getState().avanzar(fase, p.done, p.total);
				toast.loading(`${fase} ${p.done}/${p.total}`, { id: aviso });
			},
		);
		useTrabajo.getState().setId(jobId);
		const final = await fin;
		// Sin historial: no es un cambio del usuario, y lo hecho entra aunque
		// se cancelara a medias.
		useDeck.getState().refrescarCartas(antes, hechas);
		if (final.error && final.error.code !== "Cancelled") toast.error(mensaje(final.error));
	} catch (e) {
		toast.error(mensaje(e));
	} finally {
		toast.dismiss(aviso);
		useTrabajo.getState().terminar();
	}
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
