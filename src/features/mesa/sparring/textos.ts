import type { Explicacion } from "@/bindings/Explicacion";
import type { Intensidad } from "@/bindings/Intensidad";
import type { ReaccionSparring } from "@/bindings/ReaccionSparring";
import type { TipoReaccion } from "@/bindings/TipoReaccion";
import { paraTraducir } from "@/components/language-provider";

/**
 * **El sparring en palabras.** Rust explica cada reacción con claves y
 * parámetros (`Explicacion`, `mesa::sparring::reacciones`); aquí se
 * escriben con `t()`. Los números llegan ya como texto («72», «1.4»).
 */

type T = (es: string, vars?: Record<string, string | number>) => string;

export const INTENSIDADES: { valor: Intensidad; nombre: string }[] = [
	{ valor: "apagado", nombre: paraTraducir("Desactivado") },
	{ valor: "suave", nombre: paraTraducir("Ligero") },
	{ valor: "realista", nombre: paraTraducir("Realista") },
	{ valor: "despiadado", nombre: paraTraducir("Despiadado") },
];

export function nombreIntensidad(i: Intensidad, t: T): string {
	return t(INTENSIDADES.find((x) => x.valor === i)?.nombre ?? "Desactivado");
}

/** Lo que hace la reacción, como título («Exilia tu Metallic Mimic»). */
export function titulo(r: ReaccionSparring, objetivo: string, t: T): string {
	const n = r.objetivo.length;
	switch (r.tipo) {
		case "eliminar":
			return r.exilia
				? t("Exilia tu {carta}", { carta: objetivo })
				: t("Destruye tu {carta}", { carta: objetivo });
		case "contrarrestar":
			return t("Contrarresta tu {carta}", { carta: objetivo });
		case "devolver":
			return t("Devuelve {carta} a tu mano", { carta: objetivo });
		case "sacrificio":
			return t("Te obliga a sacrificar una criatura");
		case "barrer":
			return r.exilia
				? t("Exilia tus {n} criaturas", { n })
				: t("Destruye tus {n} criaturas", { n });
	}
}

/** «¿Por qué ahora?». */
export function porQue(e: Explicacion, t: T): string {
	const p = e.params;
	switch (e.clave) {
		case "contrarrestar":
			return t(
				"La ronda {ronda} está en el {ritmo} % del pico de la interacción, y {carta} pesa {peso}× como amenaza.",
				p,
			);
		case "barrer":
			return t(
				"La ronda {ronda} está en el {ritmo} % del pico de los barridos, y tienes {n} criaturas en el campo.",
				{ ...p, n: Number(p.criaturas ?? 0) },
			);
		default:
			return t(
				"La ronda {ronda} está en el {ritmo} % del pico de las eliminaciones, y {carta} pesa {peso}× como amenaza.",
				p,
			);
	}
}

const REPARTO: Record<TipoReaccion, string> = {
	eliminar: paraTraducir("Las eliminaciones son el {pct} % de toda la interacción"),
	contrarrestar: paraTraducir("Los contrahechizos son el {pct} % de toda la interacción"),
	barrer: paraTraducir("Los barridos son el {pct} % de toda la interacción"),
	devolver: paraTraducir("Devolver a la mano es el {pct} % de toda la interacción"),
	sacrificio: paraTraducir("Los sacrificios forzados son el {pct} % de toda la interacción"),
};

/** Una línea de «Ver el porqué». */
export function lineaBase(e: Explicacion, t: T): string {
	const p = e.params;
	switch (e.clave) {
		case "reparto":
			return t(REPARTO[p.tipo as TipoReaccion] ?? REPARTO.eliminar, p);
		case "ritmo":
			return t("La ronda {ronda} está en el {pct} % del pico de este tipo de reacción", p);
		case "amenaza":
			return t(
				"Peso de {carta} como amenaza: {peso}× (por su coste, si es el comandante y lo que hace)",
				p,
			);
		case "criaturas":
			return t("Criaturas en tu campo: {cuantas} (barridos ×{mult})", {
				cuantas: p.n ?? "0",
				mult: p.mult ?? "1",
			});
		case "intensidad":
			return t("Intensidad: {intensidad}", {
				intensidad: nombreIntensidad((p.intensidad ?? "apagado") as Intensidad, t),
			});
		case "probabilidad":
			return t("Saltó con un {pct} % de probabilidad", p);
		case "modelo":
			return t(
				"Las tablas son un punto de partida para afinar jugando: no salen de partidas reales.",
			);
		default:
			return e.clave;
	}
}

/** El resumen de una reacción en el panel («Quitó Metallic Mimic»). */
export function resumen(r: ReaccionSparring, objetivo: string, t: T): string {
	switch (r.tipo) {
		case "eliminar":
			return t("Quitó {carta}", { carta: objetivo });
		case "contrarrestar":
			return t("Contrarrestó {carta}", { carta: objetivo });
		case "devolver":
			return t("Devolvió {carta} a la mano", { carta: objetivo });
		case "sacrificio":
			return t("Obligó a sacrificar una criatura");
		case "barrer":
			return t("Barrió el campo");
	}
}

/** La línea del registro de la partida. */
export function lineaRegistro(
	r: ReaccionSparring,
	objetivo: string,
	t: T,
	carta = r.carta.nombre,
): string {
	const vars = { carta, objetivo };
	switch (r.tipo) {
		case "eliminar":
			return r.exilia
				? t("El fantasma exilia {objetivo} con {carta}", vars)
				: t("El fantasma destruye {objetivo} con {carta}", vars);
		case "contrarrestar":
			return t("El fantasma contrarresta {objetivo} con {carta}", vars);
		case "devolver":
			return t("El fantasma devuelve {objetivo} a la mano con {carta}", vars);
		case "sacrificio":
			return t("El fantasma obliga a sacrificar una criatura con {carta}", vars);
		case "barrer":
			return t("El fantasma barre el campo con {carta}", vars);
	}
}
