import type { CardEdits } from "@/bindings/CardEdits";
import type { DeckCard } from "@/bindings/DeckCard";

/**
 * **A qué cartas llega una edición** de «Editar carta»:
 * - `carta`: solo esa.
 * - `copias`: esa y sus copias, las que llevan la misma imagen delante (el
 *   mismo blob). Como la imagen es la misma, se copia todo: giro, encuadre y
 *   retoques.
 * - `todas`: esa entera y, en el resto de cartas con imagen, solo los
 *   retoques de color (el giro y el encuadre de una imagen no valen para otra).
 */
export type Alcance = "carta" | "copias" | "todas";

/** Las otras cartas del deck con la misma imagen delante que `carta`. */
export function copiasDe(cartas: DeckCard[], carta: DeckCard): DeckCard[] {
	const blob = carta.front?.blob;
	if (!blob) return [];
	return cartas.filter((c) => c.id !== carta.id && c.front?.blob === blob);
}

/** Las cartas con la edición aplicada según el alcance. */
export function aplicarEdicion(
	cartas: DeckCard[],
	carta: DeckCard,
	edicion: CardEdits,
	alcance: Alcance,
): DeckCard[] {
	const blob = carta.front?.blob;
	return cartas.map((c) => {
		if (c.id === carta.id) return { ...c, edits: edicion };
		if (alcance === "copias" && blob && c.front?.blob === blob) return { ...c, edits: edicion };
		if (alcance === "todas" && c.front)
			return { ...c, edits: { ...c.edits, adjust: edicion.adjust } };
		return c;
	});
}
