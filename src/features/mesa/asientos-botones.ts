import { ordenTrasPasar } from "./config-local";

/**
 * Los botones de subir y bajar de la fila de un asiento (`FilaOrdenable`), para
 * quien no arrastra bien con el dedo. `null` en uno si ya está en el borde (o,
 * en una sala, si lo pondría antes del anfitrión, que se queda el primero:
 * `primero` = 1).
 */
export function botonesDeAsiento(
	n: number,
	i: number,
	reordenar: (orden: number[]) => void,
	primero = 0,
): { subir: (() => void) | null; bajar: (() => void) | null } {
	const subir = ordenTrasPasar(n, i, -1, primero);
	const bajar = ordenTrasPasar(n, i, 1, primero);
	return {
		subir: subir ? () => reordenar(subir) : null,
		bajar: bajar ? () => reordenar(bajar) : null,
	};
}
