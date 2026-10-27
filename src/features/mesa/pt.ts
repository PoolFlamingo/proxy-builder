import type { VistaCarta } from "@/bindings/VistaCarta";

/** Un número de fuerza o resistencia impreso («2», «*», «1+*»). */
function numero(s: string | null | undefined): number | null {
	if (s == null) return null;
	const n = Number.parseInt(s, 10);
	return Number.isNaN(n) ? null : n;
}

const CONTADOR_PT = /^~?([+-])(\d+)\/([+-])(\d+)$/;

/**
 * Lo que suman los contadores de fuerza y resistencia de una carta: cada
 * `+X/+Y` o `-X/-Y` (+1/+1, +2/+2, -0/-1…), también temporal (`~`), por su cantidad. Los demás tipos
 * no cuentan.
 */
export function bonoDeContadores(contadores: Record<string, number>): [number, number] {
	let f = 0;
	let r = 0;
	for (const [tipo, n] of Object.entries(contadores)) {
		const m = CONTADOR_PT.exec(tipo);
		if (!m) continue;
		f += (m[1] === "-" ? -1 : 1) * Number(m[2]) * n;
		r += (m[3] === "-" ? -1 : 1) * Number(m[4]) * n;
	}
	return [f + 0, r + 0];
}

/**
 * La fuerza y resistencia que enseña la carta en el campo: la impresa con
 * los contadores de fuerza y resistencia (+1/+1, −1/−1, +2/+2…), las filas «otros» y «hasta fin de turno»; la fija lo
 * tapa todo. `null` si la carta no tiene F/R ni nada que la cambie.
 * `cambiada`: si no es la impresa (se pinta resaltada).
 */
export function ptDe(c: VistaCarta): { f: number; r: number; cambiada: boolean } | null {
	const base = { f: numero(c.origen?.info.fuerza), r: numero(c.origen?.info.resistencia) };
	if (c.pt.fijo) return { f: c.pt.fijo[0], r: c.pt.fijo[1], cambiada: true };
	const [masF, masR] = bonoDeContadores(c.contadores);
	const df = masF + c.pt.otros[0] + c.pt.hastaFinDeTurno[0];
	const dr = masR + c.pt.otros[1] + c.pt.hastaFinDeTurno[1];
	if (base.f === null && base.r === null && df === 0 && dr === 0) return null;
	return { f: (base.f ?? 0) + df, r: (base.r ?? 0) + dr, cambiada: df !== 0 || dr !== 0 };
}
