import type { IdCarta } from "@/bindings/IdCarta";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { type Disposicion, posAPx } from "./disposicion";

/** Las cartas adjuntas asoman por arriba a la izquierda de su carta. */
export const ASOMA = 0.22;

export type Puesto = { x: number; y: number; z?: number };

/**
 * **Dónde se pinta cada carta del campo**, en píxeles (el centro): las que
 * tienen posición propia, donde dice el motor, y las adjuntas o amontonadas
 * (`adjuntaA`, `agrupadaBajo`) junto a su carta, asomando por arriba a la
 * izquierda. Lo usan el campo y el arrastre, para que lo adjunto se mueva
 * con su carta con la misma geometría. `hijas`: las de cada carta, en orden.
 */
export function puestosDelCampo(
	ids: readonly IdCarta[],
	cartas: Record<number, VistaCarta | undefined>,
	disp: Pick<Disposicion, "campo" | "carta">,
): { puestos: Map<IdCarta, Puesto>; hijas: Map<IdCarta, IdCarta[]> } {
	const { w, h } = disp.carta;
	const puestos = new Map<IdCarta, Puesto>();
	const hijas = new Map<IdCarta, IdCarta[]>();
	for (const id of ids) {
		const c = cartas[id];
		const madre = c?.adjuntaA ?? c?.agrupadaBajo ?? null;
		if (madre !== null && cartas[madre]?.pos) hijas.set(madre, [...(hijas.get(madre) ?? []), id]);
	}
	for (const id of ids) {
		const c = cartas[id];
		if (!c?.pos) continue;
		const madre = c.adjuntaA ?? c.agrupadaBajo;
		if (madre !== null && cartas[madre]?.pos) continue;
		const p = posAPx(disp.campo, c.pos);
		puestos.set(id, p);
		(hijas.get(id) ?? []).forEach((hija, i) => {
			const amontonada = cartas[hija]?.agrupadaBajo === id;
			const paso = amontonada ? 4 : ASOMA * w;
			puestos.set(hija, {
				x: p.x - paso * (i + 1),
				y: p.y - (amontonada ? 4 : ASOMA * h) * (i + 1),
				z: Math.max(0, c.z - 1 - i),
			});
		});
	}
	return { puestos, hijas };
}

/**
 * Lo que acompaña a `llevadas` al arrastrarlas: sus adjuntas y amontonadas
 * (las que ya van entre las llevadas no cuentan).
 */
export function acompanantes(
	llevadas: readonly IdCarta[],
	hijas: Map<IdCarta, IdCarta[]>,
): IdCarta[] {
	return llevadas.flatMap((id) => hijas.get(id) ?? []).filter((h) => !llevadas.includes(h));
}
