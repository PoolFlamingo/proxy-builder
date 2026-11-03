import type { CambioEncuadre } from "./encuadre";

/**
 * Los dedos (o el ratón) sobre la carta, sin React: con un puntero se arrastra
 * y con dos se arrastra y se acerca o se aleja (pellizco). Cada `tomar` da lo
 * que ha cambiado desde el anterior, así que quien lo usa suma pasos pequeños,
 * igual que la rueda del ratón. Al entrar o salir un dedo se vuelve a medir
 * desde cero: la ilustración no pega un salto.
 */
export type Punto = { x: number; y: number };

/** Lo que ha cambiado desde el último paso: el centro de los dedos (px) y la
 *  razón entre la distancia de ahora y la de antes (1 = sin pellizco). */
export type Gesto = { dx: number; dy: number; escala: number };

type Medida = { centro: Punto; distancia: number };

export function crearGestos() {
	const dedos = new Map<number, Punto>();
	let anterior: Medida | null = null;

	// Solo cuentan los dos primeros: un tercer dedo no cambia nada.
	const medir = (): Medida | null => {
		const [a, b] = [...dedos.values()];
		if (!a) return null;
		if (!b) return { centro: a, distancia: 0 };
		return {
			centro: { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 },
			distancia: Math.hypot(a.x - b.x, a.y - b.y),
		};
	};

	return {
		/** Un dedo (o el ratón) toca la carta. */
		bajar(id: number, x: number, y: number) {
			dedos.set(id, { x, y });
			anterior = medir();
		},
		/** Un dedo se mueve. Solo apunta dónde está: el cambio se pide con `tomar`. */
		mover(id: number, x: number, y: number) {
			if (dedos.has(id)) dedos.set(id, { x, y });
		},
		/**
		 * Lo que ha cambiado desde la última vez (`null` si nada). Se pide una vez
		 * por fotograma y no a cada movimiento: con dos dedos el navegador manda un
		 * movimiento por dedo, y contarlos por separado hacía que el centro
		 * «bailase» y la ilustración se corriese en un pellizco recto.
		 */
		tomar(): Gesto | null {
			const ahora = medir();
			const antes = anterior;
			anterior = ahora;
			if (!ahora || !antes) return null;
			const escala =
				antes.distancia > 0 && ahora.distancia > 0 ? ahora.distancia / antes.distancia : 1;
			return { dx: ahora.centro.x - antes.centro.x, dy: ahora.centro.y - antes.centro.y, escala };
		},
		/** Un dedo se levanta (o se cancela). */
		subir(id: number) {
			dedos.delete(id);
			anterior = medir();
		},
		/** Cuántos punteros hay tocando. */
		cantidad: () => dedos.size,
	};
}

/**
 * El gesto como cambio del encuadre: el arrastre en fracciones de la carta
 * (`marco`, en px) y el pellizco como suma al zoom de ahora (`zoom` ×
 * `escala`), que es lo que espera `moverEncuadre`.
 */
export function cambioDeGesto(
	g: Gesto,
	marco: { w: number; h: number },
	zoomActual: number,
): CambioEncuadre {
	const c: CambioEncuadre = { dx: g.dx / Math.max(1, marco.w), dy: g.dy / Math.max(1, marco.h) };
	if (g.escala !== 1) c.zoom = zoomActual * (g.escala - 1);
	return c;
}
