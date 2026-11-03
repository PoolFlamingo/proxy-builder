import { useCallback, useRef, useState } from "react";
import { useBiblioteca } from "@/features/library/store";
import { call } from "@/lib/ipc";
import { lanzarTrabajo } from "@/lib/trabajos";
import { type Devolver, prestarDeck } from "@/stores/deck-store";

/** Cómo va la bajada de las imágenes que faltan tras recibir «solo originales». */
export type AvanceImagenes = {
	deckId: string;
	hechas: number;
	total: number;
	/** Los decks que quedan por hacer, este incluido. */
	quedan: number;
};

/**
 * **Bajar las imágenes que le faltan a los decks recibidos** como «solo cartas
 * originales» (Rust: `pasar_completar`, un trabajo con progreso). Un deck
 * detrás de otro; el que ya se ha pedido no se vuelve a pedir.
 */
export function useBajarImagenes() {
	const [avance, setAvance] = useState<AvanceImagenes | null>(null);
	const [fallidas, setFallidas] = useState(0);
	const pedidos = useRef(new Set<string>());
	const cola = useRef<Promise<void>>(Promise.resolve());
	const pendientes = useRef(0);

	const pedir = useCallback((deckIds: string[]) => {
		for (const deckId of deckIds) {
			if (pedidos.current.has(deckId)) continue;
			pedidos.current.add(deckId);
			pendientes.current += 1;
			cola.current = cola.current.then(async () => {
				// Si es el deck abierto, Rust lo va guardando: la pantalla no lo
				// pisa y al acabar mezcla lo bajado con lo que se cambió entretanto.
				let devolver: Devolver | null = null;
				try {
					devolver = await prestarDeck(deckId, { conservar: true });
					const trabajo = await lanzarTrabajo(
						"pasar://imagenes-progress",
						"pasar://imagenes-done",
						() => call("pasar_completar", { deckId }),
						(p) =>
							setAvance({
								deckId,
								hechas: p.hechas,
								total: p.total,
								quedan: pendientes.current,
							}),
					);
					const fin = await trabajo.fin;
					setFallidas((n) => n + fin.fallidas);
				} catch {
					setFallidas((n) => n + 1);
				} finally {
					await devolver?.();
					pendientes.current -= 1;
					if (pendientes.current <= 0) setAvance(null);
					void useBiblioteca.getState().refrescar();
				}
			});
		}
	}, []);

	return { avance, fallidas, pedir };
}
