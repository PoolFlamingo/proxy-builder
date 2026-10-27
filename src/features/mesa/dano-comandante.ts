import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { VistaJugador } from "@/bindings/VistaJugador";

/**
 * Lo poco que decide la pantalla de la tabla de daño de comandante: qué
 * columnas tiene y qué se pide al motor. Cuánto vale cada cosa lo cuenta
 * `crates/mesa` (`DanoComandante` baja la vida; `Lanzamientos`, el impuesto).
 */

export type Comandante = { id: IdCarta; nombre: string; dueno: IdJugador };

/** Los comandantes que se ven en la mesa, por dueño y luego por carta. */
export function comandantesDe(
	cartas: Record<number, Pick<VistaCarta, "id" | "dueno" | "origen">>,
): Comandante[] {
	const out: Comandante[] = [];
	for (const c of Object.values(cartas)) {
		if (c.origen?.info.esComandante)
			out.push({ id: c.id, nombre: c.origen.nombre, dueno: c.dueno });
	}
	return out.sort((a, b) => a.dueno - b.dueno || a.id - b.id);
}

/**
 * Lo que se le manda al motor al sumar `delta` a un daño de `actual`: nunca
 * más de lo que hay que quitar (el motor devolvería vida de más).
 */
export function deltaSeguro(actual: number, delta: number): number {
	return Math.max(delta, -actual) || 0;
}

/** El daño más alto que ha recibido de un solo comandante. */
export function danoMaximo(j: Pick<VistaJugador, "danoComandante">): number {
	return Math.max(0, ...Object.values(j.danoComandante));
}

/** ¿Ha recibido el daño que mata de un mismo comandante? */
export function esLetal(j: Pick<VistaJugador, "danoComandante">, limite: number): boolean {
	return danoMaximo(j) >= limite;
}

/** El impuesto de lanzar un comandante otra vez: 2 por cada vez, salvo si no lo lleva. */
export function impuestoDe(
	dueno: Pick<VistaJugador, "lanzamientos" | "sinCosteExtra"> | undefined,
	comandante: IdCarta,
): number {
	if (!dueno || dueno.sinCosteExtra.includes(comandante)) return 0;
	return (dueno.lanzamientos[comandante] ?? 0) * 2;
}
