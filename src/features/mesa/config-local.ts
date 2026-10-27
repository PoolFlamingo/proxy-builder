import type { ConfigMesa } from "@/bindings/ConfigMesa";
import type { Equipo } from "@/bindings/Equipo";
import type { Formato } from "@/bindings/Formato";
import type { JugadorMesa } from "@/bindings/JugadorMesa";
import type { Variantes } from "@/bindings/Variantes";
import { COLORES_EQUIPO, COLORES_JUGADOR } from "./colores";
import { opcionesDeFabrica } from "./opciones-fabrica";

/**
 * **La configuración de una partida local** a partir de lo que se rellena en
 * el diálogo: los asientos, el formato y cómo se juega por equipos. Es solo
 * armar el `ConfigMesa`; las reglas (quién es oponente de quién, la vida del
 * equipo) las aplica `crates/mesa`.
 */

export type ModoEquiposLocal = "ninguno" | "equipos" | "dosCabezas";

export type AsientoLocal = {
	nombre: string;
	deckId: string;
	/** El equipo (0 a `MAX_EQUIPOS - 1`), o `null` si va por su cuenta. */
	equipo: number | null;
	/** La lista para jugar del deck, si se juega solo una parte. */
	lista?: string;
};

export type AjustesLocal = {
	formato: Formato;
	modo: ModoEquiposLocal;
	fuegoAmigo: boolean;
	vidaCompartida: boolean;
};

export const AJUSTES_LOCALES: AjustesLocal = {
	formato: "commander",
	modo: "ninguno",
	fuegoAmigo: false,
	vidaCompartida: false,
};

/** Los jugadores que caben en una mesa. */
export const MAX_JUGADORES = 10;
/** Los equipos que se pueden hacer (con 10 jugadores, parejas). */
export const MAX_EQUIPOS = COLORES_EQUIPO.length;

/** Lo que impide empezar con estos equipos, si algo. */
export type ProblemaDeEquipos = "sinCompaneros" | "dosCabezasPares";

export function problemaDeEquipos(
	asientos: Pick<AsientoLocal, "equipo">[],
	modo: ModoEquiposLocal,
): ProblemaDeEquipos | null {
	if (modo === "dosCabezas") {
		return asientos.length >= 4 && asientos.length % 2 === 0 ? null : "dosCabezasPares";
	}
	if (modo === "equipos") {
		const cuenta = new Map<number, number>();
		for (const a of asientos) {
			if (a.equipo !== null) cuenta.set(a.equipo, (cuenta.get(a.equipo) ?? 0) + 1);
		}
		return [...cuenta.values()].some((n) => n >= 2) ? null : "sinCompaneros";
	}
	return null;
}

export function variantesDe(a: AjustesLocal): Variantes {
	return {
		equipos:
			a.modo === "equipos"
				? { tipo: "equipos", fuegoAmigo: a.fuegoAmigo }
				: a.modo === "dosCabezas"
					? { tipo: "dosCabezas" }
					: { tipo: "ninguno" },
		archenemy: null,
		planechase: false,
	};
}

/** El nombre de un equipo; lo pone la pantalla traducido (`Equipo {n}`). */
export type NombreDeEquipo = (n: number) => string;

/**
 * Los equipos que hay: los que tienen a alguien, con su color y, si se pide,
 * la vida compartida. En «Dos Cabezas» los monta el motor (parejas por asiento).
 */
export function equiposDe(
	asientos: Pick<AsientoLocal, "equipo">[],
	a: AjustesLocal,
	vidaInicial: number,
	nombre: NombreDeEquipo,
): Equipo[] {
	if (a.modo !== "equipos") return [];
	const usados = [...new Set(asientos.map((x) => x.equipo).filter((e): e is number => e !== null))];
	return usados
		.sort((x, y) => x - y)
		.map((id) => ({
			id,
			nombre: nombre(id + 1),
			color: COLORES_EQUIPO[id % COLORES_EQUIPO.length],
			vidaCompartida: a.vidaCompartida ? vidaInicial : null,
		}));
}

/** El `ConfigMesa` de la partida local. */
export function construirConfig(
	asientos: AsientoLocal[],
	a: AjustesLocal,
	nombre: NombreDeEquipo,
): ConfigMesa {
	const variantes = variantesDe(a);
	const opciones = opcionesDeFabrica(a.formato, asientos.length, variantes.equipos);
	const porEquipos = a.modo === "equipos";
	const jugadores: JugadorMesa[] = asientos.map((s, i) => ({
		deckId: s.deckId,
		nombre: s.nombre.trim(),
		color: COLORES_JUGADOR[i % COLORES_JUGADOR.length],
		lista: s.lista,
		equipo: porEquipos && s.equipo !== null ? s.equipo : undefined,
	}));
	return {
		jugadores,
		formato: a.formato,
		variantes,
		opciones,
		intensidad: "apagado",
		equipos: equiposDe(asientos, a, opciones.vidaInicial, nombre),
	};
}
