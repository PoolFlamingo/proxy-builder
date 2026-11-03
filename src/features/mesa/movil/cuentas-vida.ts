import type { Accion } from "@/bindings/Accion";
import type { IdCarta } from "@/bindings/IdCarta";
import type { IdJugador } from "@/bindings/IdJugador";
import { deltaSeguro } from "../dano-comandante";

/**
 * **Las cuentas de la pantalla de vida del móvil.** Cada objetivo (un jugador
 * o el fantasma del solitario) y cada cosa que se le cuenta (vida, veneno,
 * daño de un comandante) acaba en la misma `Accion` que mandan hoy los
 * marcadores del tablero; aquí solo se juntan los toques (un solo evento por
 * rachas, como `VidaJugador`) y se reparte lo mismo a varios objetivos.
 */

export type Objetivo = { tipo: "jugador"; id: IdJugador } | { tipo: "fantasma" };

/** Qué se está contando. */
export type Cuenta =
	| { que: "vida" }
	| { que: "veneno" }
	| { que: "comandante"; comandante: IdCarta };

export const mismoObjetivo = (a: Objetivo, b: Objetivo): boolean =>
	a.tipo === b.tipo && (a.tipo === "fantasma" || (b.tipo === "jugador" && a.id === b.id));

/** La clave de un objetivo con una cuenta (donde se juntan los toques). */
export function claveDe(o: Objetivo, c: Cuenta): string {
	const quien = o.tipo === "fantasma" ? "f" : `j${o.id}`;
	return c.que === "comandante" ? `${quien}:c${c.comandante}` : `${quien}:${c.que}`;
}

/**
 * La `Accion` de sumar `delta` a `cuenta` de `o`. `actual`: lo que ya tiene (el
 * daño de comandante nunca baja de 0: se manda lo que quita de verdad).
 * `null` si no hay nada que mandar.
 */
export function accionDe(o: Objetivo, c: Cuenta, delta: number, actual = 0): Accion | null {
	if (delta === 0) return null;
	if (c.que === "vida") {
		return o.tipo === "fantasma"
			? { que: "vidaFantasma", delta }
			: { que: "vida", jugador: o.id, delta };
	}
	if (c.que === "veneno") {
		return o.tipo === "fantasma"
			? { que: "venenoFantasma", delta }
			: { que: "veneno", jugador: o.id, delta };
	}
	const d = deltaSeguro(actual, delta);
	if (d === 0) return null;
	return o.tipo === "fantasma"
		? { que: "danoComandanteFantasma", comandante: c.comandante, delta: d }
		: { que: "danoComandante", jugador: o.id, comandante: c.comandante, delta: d };
}

/** Cuánto suma un toque y cuánto una pulsación larga. */
export const PASO_TOQUE = 1;
export const PASO_LARGO = 10;
/** Lo que hay que mantener el botón para que sume de 10 en 10, y cada cuánto repite, en ms. */
export const MS_MANTENER = 500;
export const MS_REPETIR = 350;

export interface Mantener {
	/** El dedo baja: si se mantiene, `alMantener` salta pronto y luego se repite. */
	bajar(): void;
	/** El dedo sube. Devuelve `true` si fue un toque (no se mantuvo lo bastante). */
	subir(): boolean;
	cancelar(): void;
}

/** Mantener pulsado un botón de ±: un toque suma 1 y mantenido suma 10, repetido. */
export function crearMantener(
	alMantener: () => void,
	ms = MS_MANTENER,
	cada = MS_REPETIR,
): Mantener {
	let espera: ReturnType<typeof setTimeout> | null = null;
	let repite: ReturnType<typeof setInterval> | null = null;
	let mantenido = false;
	const parar = () => {
		if (espera !== null) clearTimeout(espera);
		if (repite !== null) clearInterval(repite);
		espera = null;
		repite = null;
	};
	return {
		bajar() {
			parar();
			mantenido = false;
			espera = setTimeout(() => {
				espera = null;
				mantenido = true;
				alMantener();
				repite = setInterval(alMantener, cada);
			}, ms);
		},
		subir() {
			const fueToque = !mantenido && espera !== null;
			parar();
			mantenido = false;
			return fueToque;
		},
		cancelar() {
			parar();
			mantenido = false;
		},
	};
}

/**
 * Junta los toques de cada clave y manda la suma cuando dejan de llegar
 * durante `ms` (un solo evento en el registro por racha, como el marcador del
 * tablero). `pendiente` dice lo que va sin mandar, para enseñarlo.
 */
export interface Acumulador {
	sumar(clave: string, delta: number): void;
	pendiente(clave: string): number;
	/** Manda ya lo que haya (al cerrar la pantalla). */
	vaciar(): void;
}

export function crearAcumulador(
	enviar: (clave: string, delta: number) => void,
	avisar: () => void,
	ms = 700,
): Acumulador {
	const sumas = new Map<string, number>();
	const relojes = new Map<string, ReturnType<typeof setTimeout>>();
	const mandar = (clave: string) => {
		const n = sumas.get(clave) ?? 0;
		sumas.delete(clave);
		const r = relojes.get(clave);
		if (r !== undefined) clearTimeout(r);
		relojes.delete(clave);
		if (n !== 0) enviar(clave, n);
		avisar();
	};
	return {
		sumar(clave, delta) {
			sumas.set(clave, (sumas.get(clave) ?? 0) + delta);
			const r = relojes.get(clave);
			if (r !== undefined) clearTimeout(r);
			relojes.set(
				clave,
				setTimeout(() => mandar(clave), ms),
			);
			avisar();
		},
		pendiente: (clave) => sumas.get(clave) ?? 0,
		vaciar() {
			for (const clave of [...sumas.keys()]) mandar(clave);
		},
	};
}

/** Los objetivos que quedan tras tocar uno: se suma o se quita, y siempre queda al menos uno. */
export function alternarObjetivo(elegidos: Objetivo[], o: Objetivo): Objetivo[] {
	const esta = elegidos.some((x) => mismoObjetivo(x, o));
	if (!esta) return [...elegidos, o];
	return elegidos.length > 1 ? elegidos.filter((x) => !mismoObjetivo(x, o)) : elegidos;
}

/** Lo justo de un jugador y de los equipos para la vida compartida (2HG). */
type JugadorConVida = { id: IdJugador; vida: number; equipo: number | null };
type EquipoConVida = { id: number; vidaCompartida?: number | null };

/** El equipo de `j` si comparte la vida (`null` si no). */
function equipoCompartido(
	j: JugadorConVida,
	equipos: readonly EquipoConVida[] | undefined,
): EquipoConVida | null {
	if (j.equipo === null) return null;
	const e = equipos?.find((x) => x.id === j.equipo);
	return e && e.vidaCompartida != null ? e : null;
}

/** La vida que cuenta de un jugador: la del equipo si la comparten. */
export function vidaDe(j: JugadorConVida, equipos: readonly EquipoConVida[] | undefined): number {
	return equipoCompartido(j, equipos)?.vidaCompartida ?? j.vida;
}

/**
 * Quién recibe de verdad un cambio de `c` sobre `o`. Con vida compartida, el
 * equipo es **un solo objetivo** (el compañero de asiento más bajo): elegir a
 * los dos no la cambia dos veces y los dos enseñan lo mismo sin mandar. Lo
 * demás (veneno, daño de comandante) es de cada jugador.
 */
export function objetivoQueCuenta(
	o: Objetivo,
	c: Cuenta,
	jugadores: Readonly<Record<number, JugadorConVida>>,
	equipos: readonly EquipoConVida[] | undefined,
): Objetivo {
	if (o.tipo !== "jugador" || c.que !== "vida") return o;
	const j = jugadores[o.id];
	if (!j || !equipoCompartido(j, equipos)) return o;
	const ids = Object.values(jugadores)
		.filter((x) => x.equipo === j.equipo)
		.map((x) => x.id);
	return { tipo: "jugador", id: Math.min(...ids) };
}

/** Los objetivos elegidos sin repetir ninguno de verdad (`objetivoQueCuenta`). */
export function objetivosQueCuentan(
	elegidos: readonly Objetivo[],
	c: Cuenta,
	jugadores: Readonly<Record<number, JugadorConVida>>,
	equipos: readonly EquipoConVida[] | undefined,
): Objetivo[] {
	const vistos = new Set<string>();
	const fuera: Objetivo[] = [];
	for (const o of elegidos) {
		const r = objetivoQueCuenta(o, c, jugadores, equipos);
		const clave = claveDe(r, c);
		if (vistos.has(clave)) continue;
		vistos.add(clave);
		fuera.push(r);
	}
	return fuera;
}
