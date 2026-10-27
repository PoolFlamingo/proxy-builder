import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { Mesa } from "@/stores/mesa-store";

/**
 * La carta de arriba de la biblioteca de `jugador`, **si se pinta boca arriba**:
 * cuando se juega con la de arriba revelada, o cuando se acaba de revelar
 * («Revelar la de arriba», que la deja vista por todos hasta que se mueva o
 * se baraje). Mirarla (mirar, escrutar) no la enseña en la mesa.
 */
export function cartaDeArribaRevelada(
	m: Pick<Mesa, "jugadores" | "zonas" | "cartas">,
	jugador: IdJugador,
): VistaCarta | null {
	const id = m.zonas[`${jugador}:biblioteca`]?.visibles?.find(([i]) => i === 0)?.[1];
	const c = id === undefined ? undefined : m.cartas[id];
	if (!c) return null;
	return m.jugadores[jugador]?.deArribaRevelada || c.revelada ? c : null;
}
