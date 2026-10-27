import type { ConfigMesa } from "@/bindings/ConfigMesa";
import type { Equipo } from "@/bindings/Equipo";
import type { Formato } from "@/bindings/Formato";
import type { JugadorMesa } from "@/bindings/JugadorMesa";
import type { Variantes } from "@/bindings/Variantes";
import { COLOR_CONTRA_ARCHIENEMIGO, COLORES_EQUIPO, COLORES_JUGADOR } from "./colores";
import { opcionesDeFabrica } from "./opciones-fabrica";

/**
 * **La configuración de una partida local** a partir de lo que se rellena en
 * el diálogo: los asientos, el formato y cómo se juega por equipos. Es solo
 * armar el `ConfigMesa`; las reglas (quién es oponente de quién, la vida del
 * equipo) las aplica `crates/mesa`.
 */

export type ModoEquiposLocal = "ninguno" | "equipos" | "dosCabezas" | "archienemigo";

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
	/** El asiento del archienemigo (con `modo: "archienemigo"`). */
	archienemigo: number;
	/** Planechase: un mazo común de planos con el dado planar. */
	planechase: boolean;
	/** Partidas del encuentro: 1 (una suelta), 3 o 5. */
	alMejorDe: AlMejorDe;
	/** El asiento que empieza, o `null` para sortearlo. */
	primero: number | null;
};

/** Las duraciones de un encuentro (`mesa::encuentro::DURACIONES`). */
export type AlMejorDe = 1 | 3 | 5;

/** Los formatos con banquillo entre partidas. */
export function conBanquillo(f: Formato): boolean {
	return f === "constructed" || f === "limited";
}

export const AJUSTES_LOCALES: AjustesLocal = {
	formato: "commander",
	modo: "ninguno",
	fuegoAmigo: false,
	vidaCompartida: false,
	archienemigo: 0,
	planechase: false,
	alMejorDe: 1,
	primero: null,
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

/** El asiento del archienemigo, dentro de los que hay. */
export function asientoArchienemigo(a: AjustesLocal, jugadores: number): number {
	return Math.max(0, Math.min(a.archienemigo, jugadores - 1));
}

export function variantesDe(a: AjustesLocal, jugadores = MAX_JUGADORES): Variantes {
	return {
		// Archenemy es «todos contra uno»: los demás van en un equipo sin fuego amigo.
		equipos:
			a.modo === "equipos" || a.modo === "archienemigo"
				? { tipo: "equipos", fuegoAmigo: a.modo === "equipos" && a.fuegoAmigo }
				: a.modo === "dosCabezas"
					? { tipo: "dosCabezas" }
					: { tipo: "ninguno" },
		archenemy: a.modo === "archienemigo" ? asientoArchienemigo(a, jugadores) : null,
		planechase: a.planechase,
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
	heroes = nombre(1),
): Equipo[] {
	if (a.modo === "archienemigo") {
		return [{ id: 0, nombre: heroes, color: COLOR_CONTRA_ARCHIENEMIGO, vidaCompartida: null }];
	}
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
	heroes?: string,
): ConfigMesa {
	const variantes = variantesDe(a, asientos.length);
	const opciones = opcionesDeFabrica(a.formato, asientos.length, variantes.equipos);
	const porEquipos = a.modo === "equipos";
	const archienemigo = variantes.archenemy;
	const jugadores: JugadorMesa[] = asientos.map((s, i) => ({
		deckId: s.deckId,
		nombre: s.nombre.trim(),
		color: COLORES_JUGADOR[i % COLORES_JUGADOR.length],
		lista: s.lista,
		equipo:
			archienemigo !== null
				? i === archienemigo
					? undefined
					: 0
				: porEquipos && s.equipo !== null
					? s.equipo
					: undefined,
	}));
	return {
		jugadores,
		formato: a.formato,
		variantes,
		opciones,
		intensidad: "apagado",
		equipos: equiposDe(asientos, a, opciones.vidaInicial, nombre, heroes),
		primero: a.primero !== null && a.primero < asientos.length ? a.primero : undefined,
		// Un encuentro necesita rival y un formato con banquillo.
		alMejorDe:
			a.alMejorDe > 1 && asientos.length >= 2 && conBanquillo(a.formato) ? a.alMejorDe : undefined,
	};
}

/**
 * **El orden de los asientos.** Se juega por asientos, de uno al siguiente,
 * así que quién se sienta junto a quién importa (y en Dos Cabezas las parejas
 * son los asientos contiguos). Un orden es una lista `orden[nuevo] = viejo`.
 */

/** El orden que resulta de llevar el asiento `de` a la posición `a`. */
export function ordenTrasMover(n: number, de: number, a: number): number[] {
	const orden = Array.from({ length: n }, (_, i) => i);
	if (de < 0 || de >= n) return orden;
	const [x] = orden.splice(de, 1);
	orden.splice(Math.max(0, Math.min(a, n - 1)), 0, x);
	return orden;
}

/** Un orden cualquiera (Fisher–Yates; `azar` da un número de 0 a 1). */
export function ordenAlAzar(n: number, azar: () => number = Math.random): number[] {
	const orden = Array.from({ length: n }, (_, i) => i);
	for (let i = n - 1; i > 0; i--) {
		const j = Math.min(i, Math.floor(azar() * (i + 1)));
		[orden[i], orden[j]] = [orden[j], orden[i]];
	}
	return orden;
}

export function aplicarOrden<T>(lista: T[], orden: number[]): T[] {
	return orden.map((viejo) => lista[viejo]);
}

/**
 * Los ajustes tras reordenar: el asiento que empieza y el del archienemigo
 * siguen siendo los mismos jugadores, ahora en otro sitio.
 */
export function ajustesTrasOrden(a: AjustesLocal, orden: number[]): AjustesLocal {
	const nuevo = (viejo: number) => Math.max(0, orden.indexOf(viejo));
	return {
		...a,
		primero: a.primero === null ? null : nuevo(a.primero),
		archienemigo: nuevo(a.archienemigo),
	};
}

/** Los ajustes tras quitar el asiento `i`. */
export function ajustesTrasQuitar(a: AjustesLocal, i: number): AjustesLocal {
	const baja = (v: number) => (v > i ? v - 1 : v);
	return {
		...a,
		primero: a.primero === null || a.primero === i ? null : baja(a.primero),
		archienemigo: a.archienemigo === i ? 0 : baja(a.archienemigo),
	};
}
