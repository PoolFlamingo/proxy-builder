import type { RefImagen } from "@/bindings/RefImagen";
import type { VistaMesa } from "@/bindings/VistaMesa";
import { type AnchoMesa, urlMesa } from "@/lib/image-url";
import { useUi } from "@/stores/ui-store";
import { caraVisible } from "./imagen-carta";

/** Lo más que se espera a las imágenes antes de enseñar la mesa, en ms. */
export const ESPERA_PRECARGA_MS = 1500;
/** El ancho de la mano y las zonas, y el de las miniaturas. */
const ANCHOS_FIJOS: AnchoMesa[] = [244, 146];

/**
 * Las URLs de lo que se ve nada más abrir la mesa: las caras de las cartas
 * a la vista (campo, mano, lo de arriba de cada zona), los reversos y los
 * tapetes, a los anchos que pide el tablero. Pura, para probarla.
 */
export function urlsAPrecargar(v: VistaMesa, calidad: AnchoMesa): string[] {
	const refs: RefImagen[] = [];
	for (const j of v.jugadores) {
		refs.push(j.dorso);
	}
	for (const c of v.cartas) refs.push(caraVisible(c));
	const urls = new Set<string>();
	for (const r of refs) {
		for (const a of [calidad, ...ANCHOS_FIJOS]) {
			const u = urlMesa(r, a);
			if (u) urls.add(u);
		}
	}
	for (const j of v.jugadores) {
		const u = j.tapete ? urlMesa(j.tapete, 1920) : null;
		if (u) urls.add(u);
	}
	return [...urls];
}

/**
 * Carga y decodifica las imágenes de la primera vista antes de enseñar la
 * mesa (así no aparecen carta a carta), como mucho `ESPERA_PRECARGA_MS`: lo
 * que no llegue a tiempo sigue cargando y entra luego.
 */
export function precargar(v: VistaMesa): Promise<void> {
	const urls = urlsAPrecargar(v, useUi.getState().mesa.calidad);
	if (urls.length === 0) return Promise.resolve();
	const todas = Promise.allSettled(
		urls.map((u) => {
			const img = new Image();
			img.decoding = "async";
			img.src = u;
			return img.decode();
		}),
	).then(() => undefined);
	const tope = new Promise<void>((r) => window.setTimeout(r, ESPERA_PRECARGA_MS));
	return Promise.race([todas, tope]);
}
