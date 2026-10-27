import type { Cambio } from "@/bindings/Cambio";
import type { Evento } from "@/bindings/Evento";

/**
 * Lo poco de Planechase y Archenemy que decide la pantalla. El dado, el
 * Planeswalk y los planes los resuelve `crates/mesa`.
 */

/**
 * ¿Salió Planeswalk en el dado y aún no se ha hecho? Se mira hacia atrás en
 * el registro: lo primero que aparece, un Planeswalk (ya hecho) o una tirada.
 */
export function planeswalkPendiente(registro: Pick<Evento, "cambio">[]): boolean {
	for (let i = registro.length - 1; i >= 0; i--) {
		const c: Cambio | undefined = registro[i]?.cambio;
		if (!c) continue;
		if (c.tipo === "accion" && c.accion.que === "planeswalk") return false;
		if (c.tipo === "dadoPlanar") return c.cara === "planeswalk";
	}
	return false;
}

/** ¿Es este jugador el archienemigo de la partida? */
export function esArchienemigo(
	archenemy: number | null | undefined,
	jugador: number | null,
): boolean {
	return archenemy != null && jugador === archenemy;
}
