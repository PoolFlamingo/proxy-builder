import type { Destino } from "@/bindings/Destino";
import { destino, sitio } from "./acciones";
import { type Disposicion, dentro, pxAPos, type ZonaPantalla } from "./disposicion";

/**
 * **A dónde va lo que se suelta.** Con la geometría de la mesa y el punto
 * donde se suelta, el destino de la acción `Mover`:
 * - en el campo, a esa posición (0–10.000);
 * - en la biblioteca, arriba (abajo solo con `alFondo`, que es Mayús al soltar:
 *   soltar una carta robada por error en la mitad de abajo no la esconde);
 * - en la mano, al final;
 * - en cementerio, exilio, mando o banquillo, arriba.
 *
 * `x`, `y`: el punto donde se suelta, relativo al tablero.
 */
export function destinoDeSoltar(
	d: Disposicion,
	x: number,
	y: number,
	alFondo = false,
): Destino | null {
	const zonas: [ZonaPantalla, Destino][] = [
		["cementerio", destino("cementerio")],
		["exilio", destino("exilio")],
		["mando", destino("mando")],
		["banquillo", destino("banquillo")],
	];
	const lib = d.zonas.biblioteca;
	if (dentro(lib, x, y)) {
		return destino("biblioteca", alFondo ? sitio.abajo : sitio.arriba);
	}
	for (const [n, dest] of zonas) if (dentro(d.zonas[n], x, y)) return dest;
	if (dentro(d.mano, x, y)) return destino("mano");
	if (dentro(d.campo, x, y)) return destino("campo", sitio.campo(pxAPos(d.campo, x, y)));
	return null;
}

/** Un arrastre se empieza a partir de esta distancia, en px. */
export const UMBRAL_ARRASTRE = 5;

export function pasaUmbral(dx: number, dy: number): boolean {
	return Math.hypot(dx, dy) >= UMBRAL_ARRASTRE;
}
