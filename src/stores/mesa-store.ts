import { create } from "zustand";
import type { Evento } from "@/bindings/Evento";
import type { Formato } from "@/bindings/Formato";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import type { Opciones } from "@/bindings/Opciones";
import type { ParcheVista } from "@/bindings/ParcheVista";
import type { RefZona } from "@/bindings/RefZona";
import type { Variantes } from "@/bindings/Variantes";
import type { VistaCarta } from "@/bindings/VistaCarta";
import type { VistaGeneral } from "@/bindings/VistaGeneral";
import type { VistaJugador } from "@/bindings/VistaJugador";
import type { VistaMesa } from "@/bindings/VistaMesa";
import type { VistaZona } from "@/bindings/VistaZona";
import { conOriginales, useOriginales } from "./originales-store";

/**
 * **La mesa en pantalla.** Guarda la vista que manda Rust (`mesa::vista`)
 * normalizada —cartas, zonas y jugadores por clave— y le aplica los parches
 * tal cual llegan, sin recalcular nada: lo que se ve lo decide Rust. Si un
 * parche no sigue al anterior (`version`), se pide la vista entera.
 *
 * Aparte, lo que es solo de la pantalla (selección, carta bajo el ratón,
 * registro de lo que va pasando), que nunca viaja a Rust.
 */

/** La clave de una zona en el store. */
export function claveZona(z: RefZona): string {
	return z.tipo === "de" ? `${z.jugador}:${z.zona}` : `c:${z.zona}`;
}

export type Mesa = {
	version: number;
	yo: IdJugador | null;
	formato: Formato | null;
	variantes: Variantes | null;
	opciones: Opciones | null;
	cartas: Record<number, VistaCarta>;
	zonas: Record<string, VistaZona>;
	jugadores: Record<number, VistaJugador>;
	/** El orden de los asientos. */
	asientos: IdJugador[];
	general: VistaGeneral | null;
};

export const MESA_VACIA: Mesa = {
	version: -1,
	yo: null,
	formato: null,
	variantes: null,
	opciones: null,
	cartas: {},
	zonas: {},
	jugadores: {},
	asientos: [],
	general: null,
};

/** La mesa entera desde una vista. */
export function desdeVista(v: VistaMesa): Mesa {
	const cartas: Record<number, VistaCarta> = {};
	const orig = useOriginales.getState();
	for (const c of v.cartas) cartas[c.id] = conOriginales(c, orig);
	const zonas: Record<string, VistaZona> = {};
	for (const z of v.zonas) zonas[claveZona(z.zona)] = z;
	const jugadores: Record<number, VistaJugador> = {};
	for (const j of v.jugadores) jugadores[j.id] = j;
	return {
		version: v.version,
		yo: v.yo,
		formato: v.formato,
		variantes: v.variantes,
		opciones: v.opciones,
		cartas,
		zonas,
		jugadores,
		asientos: v.jugadores.map((j) => j.id),
		general: v.general,
	};
}

/**
 * Aplica un parche. Solo se copia lo que cambia (así una carta que no se
 * tocó sigue siendo el mismo objeto y su componente no se repinta). Si el
 * parche no sigue a la versión que hay, `null`: hay que pedir la vista.
 */
export function conParche(m: Mesa, p: ParcheVista): Mesa | null {
	if (p.version !== m.version + 1) return null;
	let cartas = m.cartas;
	if (p.cartas.length > 0 || p.quitadas.length > 0) {
		cartas = { ...m.cartas };
		for (const id of p.quitadas) delete cartas[id];
		const orig = useOriginales.getState();
		for (const c of p.cartas) cartas[c.id] = conOriginales(c, orig);
	}
	let zonas = m.zonas;
	if (p.zonas.length > 0) {
		zonas = { ...m.zonas };
		for (const z of p.zonas) zonas[claveZona(z.zona)] = z;
	}
	let jugadores = m.jugadores;
	if (p.jugadores.length > 0) {
		jugadores = { ...m.jugadores };
		for (const j of p.jugadores) jugadores[j.id] = j;
	}
	return {
		...m,
		version: p.version,
		cartas,
		zonas,
		jugadores,
		general: p.general ?? m.general,
	};
}

/** Lo que se apunta de cada evento para el registro (los últimos). */
export const MAX_REGISTRO = 500;

type MesaStore = Mesa & {
	/** Hay una partida abierta y la pantalla está suscrita. */
	activa: boolean;
	/** Los últimos eventos, para el registro. */
	registro: Evento[];
	/** Selección de cartas en la pantalla. */
	seleccion: IdCarta[];
	/** La carta bajo el ratón (vista grande). */
	sobre: IdCarta | null;
	/** La carta que se inspecciona (detalle con su texto). */
	inspeccion: IdCarta | null;
	/**
	 * De quién es el tablero que se ve (`null`: el propio). En red, para mirar
	 * el de un rival; los tableros a la vez llegan con la Fase G.
	 */
	mirando: IdJugador | null;
	/**
	 * La vista entera. Con `registro` (al suscribirse, deshacer o reiniciar)
	 * también cambia el registro: lo deshecho y lo de antes ya no sale.
	 */
	recibirVista: (v: VistaMesa, registro?: Evento[]) => void;
	/** Aplica un parche; devuelve `false` si hay que pedir la vista. */
	recibirParche: (p: ParcheVista, ev: Evento) => boolean;
	cerrar: () => void;
	seleccionar: (ids: IdCarta[]) => void;
	alternar: (id: IdCarta) => void;
	setSobre: (id: IdCarta | null) => void;
	inspeccionar: (id: IdCarta | null) => void;
	mirar: (j: IdJugador | null) => void;
	/** Pone al día las caras de las cartas con el modo «originales» (E9). */
	reaplicarOriginales: () => void;
};

export const useMesa = create<MesaStore>((set, get) => ({
	...MESA_VACIA,
	activa: false,
	registro: [],
	seleccion: [],
	sobre: null,
	inspeccion: null,
	mirando: null,
	recibirVista: (v, registro) => {
		const m = desdeVista(v);
		set({
			...(registro ? { registro: registro.slice(-MAX_REGISTRO) } : {}),
			...m,
			activa: true,
			seleccion: get().seleccion.filter((id) => id in m.cartas),
			// Otra partida (o deshacer) reutiliza ids: lo de debajo del ratón no vale.
			sobre: !registro && m.cartas[get().sobre ?? -1] ? get().sobre : null,
		});
	},
	recibirParche: (p, ev) => {
		const m = conParche(get(), p);
		if (!m) return false;
		const registro = [...get().registro, ev];
		if (registro.length > MAX_REGISTRO) registro.splice(0, registro.length - MAX_REGISTRO);
		const quitadas = new Set(p.quitadas);
		set({
			...m,
			registro,
			seleccion: get().seleccion.filter((id) => !quitadas.has(id)),
			sobre: get().sobre !== null && quitadas.has(get().sobre as number) ? null : get().sobre,
		});
		return true;
	},
	cerrar: () => {
		useOriginales.getState().limpiar();
		set({
			...MESA_VACIA,
			activa: false,
			registro: [],
			seleccion: [],
			sobre: null,
			inspeccion: null,
			mirando: null,
		});
	},
	seleccionar: (seleccion) => set({ seleccion }),
	alternar: (id) => {
		const s = get().seleccion;
		set({ seleccion: s.includes(id) ? s.filter((x) => x !== id) : [...s, id] });
	},
	setSobre: (sobre) => set({ sobre }),
	inspeccionar: (inspeccion) => set({ inspeccion }),
	mirar: (j) => set({ mirando: j === get().yo ? null : j, seleccion: [] }),
	reaplicarOriginales: () => {
		const orig = useOriginales.getState();
		const cartas = get().cartas;
		let nuevas: Record<number, VistaCarta> | null = null;
		for (const c of Object.values(cartas)) {
			const n = conOriginales(c, orig);
			if (n !== c) {
				nuevas ??= { ...cartas };
				nuevas[c.id] = n;
			}
		}
		if (nuevas) set({ cartas: nuevas });
	},
}));

/** De quién es el tablero que se ve: el que se mira o el propio. */
export function tableroDe(m: Pick<MesaStore, "mirando" | "yo">): IdJugador {
	return m.mirando ?? m.yo ?? 0;
}

/** Un array vacío que es siempre el mismo (los selectores no deben crear uno nuevo). */
export const NINGUNA: never[] = [];

/** Las cartas de una zona, en orden (las que se ven). */
export function cartasDe(m: Pick<Mesa, "zonas">, z: RefZona): IdCarta[] {
	return m.zonas[claveZona(z)]?.cartas ?? NINGUNA;
}

/** Cuántas cartas hay en una zona (se vean o no). */
export function cuantasEn(m: Pick<Mesa, "zonas">, z: RefZona): number {
	return m.zonas[claveZona(z)]?.cuantas ?? 0;
}
