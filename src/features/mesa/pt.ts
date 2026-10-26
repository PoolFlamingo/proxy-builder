import type { VistaCarta } from "@/bindings/VistaCarta";

/** Un número de fuerza o resistencia impreso («2», «*», «1+*»). */
function numero(s: string | null | undefined): number | null {
	if (s == null) return null;
	const n = Number.parseInt(s, 10);
	return Number.isNaN(n) ? null : n;
}

/**
 * La fuerza y resistencia que enseña la carta en el campo: la impresa con
 * los +1/+1 y −1/−1, las filas «otros» y «hasta fin de turno»; la fija lo
 * tapa todo. `null` si la carta no tiene F/R ni nada que la cambie.
 * `cambiada`: si no es la impresa (se pinta resaltada).
 */
export function ptDe(c: VistaCarta): { f: number; r: number; cambiada: boolean } | null {
	const base = { f: numero(c.origen?.info.fuerza), r: numero(c.origen?.info.resistencia) };
	if (c.pt.fijo) return { f: c.pt.fijo[0], r: c.pt.fijo[1], cambiada: true };
	const mas = (c.contadores["+1/+1"] ?? 0) - (c.contadores["-1/-1"] ?? 0);
	const df = mas + c.pt.otros[0] + c.pt.hastaFinDeTurno[0];
	const dr = mas + c.pt.otros[1] + c.pt.hastaFinDeTurno[1];
	if (base.f === null && base.r === null && df === 0 && dr === 0) return null;
	return { f: (base.f ?? 0) + df, r: (base.r ?? 0) + dr, cambiada: df !== 0 || dr !== 0 };
}
