import { CONTADORES_CARTA, CONTADORES_JUGADOR, CONTADORES_OTROS, PALABRAS_CLAVE } from "./colores";

/**
 * Los contadores **temporales**: un tipo que empieza por `~` se pierde al
 * terminar el turno (lo borra el motor, `mesa::modelo::PREFIJO_TEMPORAL`).
 * Vale para los contadores y para los de palabra clave, así que la pantalla
 * siempre pinta el nombre sin el prefijo y con el reloj.
 */
export const PREFIJO_TEMPORAL = "~";

export const esTemporal = (tipo: string) => tipo.startsWith(PREFIJO_TEMPORAL);

/** El tipo sin el prefijo de temporal. */
export const sinTemporal = (tipo: string) => (esTemporal(tipo) ? tipo.slice(1) : tipo);

/** El tipo como temporal (`si`) o como permanente. */
export const conTemporal = (tipo: string, si: boolean) =>
	si ? `${PREFIJO_TEMPORAL}${sinTemporal(tipo)}` : sinTemporal(tipo);

/** Los contadores de maná de una carta (`mana-W`…): los mismos tipos que los de jugador. */
export const CONTADORES_MANA = CONTADORES_JUGADOR.filter((c) => c.tipo.startsWith("mana-"));

/** Los colores con los que se pintan las bolitas de maná. */
export const COLOR_DE_MANA: Record<string, string> = {
	W: "#fef3c7",
	U: "#38bdf8",
	B: "#57534e",
	R: "#ef4444",
	G: "#22c55e",
	C: "#a8a29e",
};

/** La letra del maná de un tipo (`mana-R` → `R`), o `null` si no es de maná. */
export function manaDe(tipo: string): string | null {
	const m = /^mana-([WUBRGC])$/.exec(sinTemporal(tipo));
	return m?.[1] ?? null;
}

const CONOCIDOS = new Map(
	[...CONTADORES_CARTA, ...CONTADORES_MANA, ...CONTADORES_OTROS, ...PALABRAS_CLAVE].map((c) => [
		c.tipo,
		c.nombre,
	]),
);

/**
 * El nombre que se enseña de un tipo de contador o de palabra clave (sin el
 * prefijo de temporal): el traducido si está en el catálogo y, si lo escribió
 * el jugador, tal cual.
 */
export function nombreContador(tipo: string, t: (s: string) => string): string {
	const base = sinTemporal(tipo);
	const nombre = CONOCIDOS.get(base);
	return nombre ? t(nombre) : base;
}

/** El nombre de un tipo para el registro de la partida: el temporal lo dice. */
export function nombreEnRegistro(tipo: string, t: (s: string) => string): string {
	const nombre = nombreContador(tipo, t);
	return esTemporal(tipo) ? `${nombre} (${t("Temporal").toLowerCase()})` : nombre;
}
