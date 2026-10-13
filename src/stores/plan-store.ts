import { create } from "zustand";
import type { Deck } from "@/bindings/Deck";
import type { PagePlan } from "@/bindings/PagePlan";
import { call } from "@/lib/ipc";
import { useDeck } from "./deck-store";

/**
 * **El plan de páginas del deck abierto**, calculado en Rust
 * (`compute_page_plan`). La vista previa y el pie lo leen de aquí; nadie
 * recalcula posiciones en TypeScript.
 *
 * Se pide 100 ms después del último cambio de cartas o ajustes. Si llegan
 * respuestas desordenadas, gana la de la última petición.
 */
type PlanStore = {
	plan: PagePlan | null;
	error: unknown;
	/** El deck con el que se calculó (para saber de qué carta es cada casilla). */
	deDeck: Deck | null;
};

export const usePlan = create<PlanStore>(() => ({ plan: null, error: null, deDeck: null }));

export function iniciarPlan(retardoMs = 100): () => void {
	let temporizador: ReturnType<typeof setTimeout> | null = null;
	let peticion = 0;

	const pedir = async () => {
		const deck = useDeck.getState().deck;
		if (!deck) {
			usePlan.setState({ plan: null, error: null, deDeck: null });
			return;
		}
		const mia = ++peticion;
		try {
			const plan = await call("compute_page_plan", { deck });
			if (mia === peticion) usePlan.setState({ plan, error: null, deDeck: deck });
		} catch (error) {
			if (mia === peticion) usePlan.setState({ error });
		}
	};

	const programar = (inmediato: boolean) => {
		if (temporizador) clearTimeout(temporizador);
		temporizador = setTimeout(
			() => {
				temporizador = null;
				void pedir();
			},
			inmediato ? 0 : retardoMs,
		);
	};

	const parar = useDeck.subscribe((s, antes) => {
		const d = s.deck;
		const a = antes.deck;
		if (d === a) return;
		// Un deck distinto se pinta sin esperar.
		if (!d || !a || d.id !== a.id) return programar(true);
		if (d.cards !== a.cards || d.settings !== a.settings || d.defaultBack !== a.defaultBack)
			programar(false);
	});
	programar(true);

	return () => {
		parar();
		if (temporizador) clearTimeout(temporizador);
	};
}
