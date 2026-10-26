import { toast } from "sonner";
import type { Accion } from "@/bindings/Accion";
import type { Destino } from "@/bindings/Destino";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Pos } from "@/bindings/Pos";
import type { RefZona } from "@/bindings/RefZona";
import type { Sitio } from "@/bindings/Sitio";
import type { Zona } from "@/bindings/Zona";
import type { ZonaComun } from "@/bindings/ZonaComun";
import { traduccionSuelta } from "@/components/language-provider";
import { mensajeDeError } from "@/lib/errores";
import { call } from "@/lib/ipc";
import { useMesa } from "@/stores/mesa-store";

/**
 * **Mandar una acción a la mesa.** Rust la resuelve y la aplica; lo que
 * cambia llega después por el canal (`mesa-store`). Un rechazo sale como
 * aviso con su motivo.
 */
export async function hacer(accion: Accion): Promise<boolean> {
	try {
		await call("mesa_accion", { accion, como: null });
		return true;
	} catch (e) {
		toast.error(mensajeDeError(e, traduccionSuelta));
		return false;
	}
}

export const sitio = {
	arriba: { tipo: "arriba" } as Sitio,
	abajo: { tipo: "abajo" } as Sitio,
	alAzar: { tipo: "alAzar" } as Sitio,
	barajar: { tipo: "barajarDentro" } as Sitio,
	indice: (n: number): Sitio => ({ tipo: "indice", valor: n }),
	xDesdeArriba: (n: number): Sitio => ({ tipo: "xDesdeArriba", valor: n }),
	campo: (p: Pos): Sitio => ({ tipo: "enCampo", valor: p }),
};

export function destino(
	zona: Zona,
	s: Sitio = sitio.arriba,
	jugador: IdJugador | null = null,
): Destino {
	return { zona: { tipo: "de", zona }, jugador, sitio: s };
}

export function destinoComun(zona: ZonaComun, s: Sitio = sitio.arriba): Destino {
	return { zona: { tipo: "comun", zona }, jugador: null, sitio: s };
}

export function mover(cartas: IdCarta[], a: Destino, como: "lanzar" | "poner" | null = null) {
	return hacer({ que: "mover", cartas, a, como });
}

export const zonaDe = (jugador: IdJugador, zona: Zona): RefZona => ({ tipo: "de", jugador, zona });
export const zonaComun = (zona: ZonaComun): RefZona => ({ tipo: "comun", zona });

/**
 * Las cartas sobre las que actúa un menú o un atajo: la selección si la
 * carta está en ella; si no, solo esa carta.
 */
export function objetivo(id: IdCarta): IdCarta[] {
	const sel = useMesa.getState().seleccion;
	return sel.includes(id) ? sel : [id];
}
