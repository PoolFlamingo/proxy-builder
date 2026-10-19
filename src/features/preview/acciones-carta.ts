import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";
import { elegirImagenes, type T } from "@/features/decklist/acciones";
import { nombreDeFichero } from "@/lib/cartas";
import { actualizar, duplicar, huecos, moverGrupo, quitar } from "@/lib/deck-ops";
import { call } from "@/lib/ipc";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

/**
 * Lo que se puede hacer con las cartas seleccionadas (menú contextual,
 * teclado y arrastre). Todo pasa por el store del deck: se autoguarda y se
 * deshace con Ctrl+Z.
 */

const cartas = () => useDeck.getState().deck?.cards ?? [];

/** Las cartas sobre las que actúa un clic derecho en `id`: la selección si la incluye. */
export function objetivo(id: string): string[] {
	const sel = useUi.getState().seleccion;
	return sel.includes(id) ? sel : [id];
}

export function duplicarCartas(ids: string[]) {
	useDeck.getState().setCartas((c) => duplicar(c, ids));
}

export function eliminarCartas(ids: string[]) {
	useDeck.getState().setCartas((c) => quitar(c, ids));
	useUi.getState().seleccionar([], null);
}

/**
 * El idioma de la imagen de las cartas propias (y de sus copias con la misma
 * imagen); `null` lo quita. Es el que cuenta en la Biblioteca, no el de la
 * carta vinculada.
 */
export function idiomaDeImagen(ids: string[], lang: string | null) {
	const cs = cartas();
	const set = new Set(ids);
	const blobs = new Set(
		cs.filter((c) => set.has(c.id) && c.front).map((c) => c.front?.blob as string),
	);
	useDeck
		.getState()
		.setCartas((c) =>
			c.map((x) =>
				x.source.kind === "custom" && (set.has(x.id) || (x.front && blobs.has(x.front.blob)))
					? { ...x, meta: { ...x.meta, imageLang: lang ?? undefined } }
					: x,
			),
		);
}

export function fijarCartas(ids: string[], fijar: boolean) {
	useDeck.getState().setCartas((c) => actualizar(c, ids, (x) => ({ ...x, locked: fijar })));
}

/** Mueve el grupo una posición antes (-1) o después (+1). */
export function moverUno(ids: string[], dir: -1 | 1) {
	const cs = cartas();
	const set = new Set(ids);
	const indices = cs.flatMap((c, i) => (set.has(c.id) ? [i] : []));
	if (indices.length === 0) return;
	const resto = cs.length - indices.length;
	const inicio = indices[0] + dir;
	useDeck.getState().setCartas((c) => moverGrupo(c, ids, Math.max(0, Math.min(resto, inicio))));
}

/** Mueve el grupo para que empiece en la posición `n` (1 = la primera). */
export function moverA(ids: string[], n: number) {
	useDeck.getState().setCartas((c) => moverGrupo(c, ids, n - 1));
}

/**
 * Soltar `ids` sobre la carta `destino`: si se arrastra hacia delante, quedan
 * detrás de ella; hacia atrás, delante. `null` = al final.
 */
export function soltarSobre(ids: string[], destino: string | null) {
	const cs = cartas();
	if (destino !== null && ids.includes(destino)) return;
	const set = new Set(ids);
	const resto = cs.filter((c) => !set.has(c.id));
	if (destino === null) {
		useDeck.getState().setCartas((c) => moverGrupo(c, ids, resto.length));
		return;
	}
	const origen = cs.findIndex((c) => set.has(c.id));
	const idxDestino = cs.findIndex((c) => c.id === destino);
	const enResto = resto.findIndex((c) => c.id === destino);
	const pos = origen < idxDestino ? enResto + 1 : enResto;
	useDeck.getState().setCartas((c) => moverGrupo(c, ids, pos));
}

/** Pone una imagen como frente: la carta pasa a ser propia. */
export function ponerFrente(id: string, img: FaceImage, nombre?: string) {
	useDeck.getState().setCartas((c) =>
		actualizar(c, [id], (x) => ({
			...x,
			source: { kind: "custom" },
			front: img,
			edits: { ...x.edits, crop: { zoom: 1, offsetX: 0, offsetY: 0 }, rotation: "r0" },
			meta: {
				...x.meta,
				name: x.meta.name || nombre || "",
				imageStatus: null,
				langFallback: false,
			},
		})),
	);
}

export async function reemplazarImagen(id: string, t: T) {
	const r = await elegirImagenes(t, false);
	const img = r?.imagenes[0];
	if (r && img) ponerFrente(id, img, nombreDeFichero(r.rutas[0]));
}

/** Reemplaza el frente con un fichero arrastrado desde el sistema. */
export async function reemplazarConFichero(id: string, ruta: string) {
	const [img] = await call("blob_import_files", { paths: [ruta] });
	if (img) ponerFrente(id, img, nombreDeFichero(ruta));
}

export async function cambiarReverso(ids: string[], t: T) {
	const r = await elegirImagenes(t, false);
	const img = r?.imagenes[0];
	if (!img) return;
	useDeck
		.getState()
		.setCartas((c) => actualizar(c, ids, (x) => ({ ...x, back: { kind: "custom", image: img } })));
}

export function reversoDelDeck(ids: string[]) {
	useDeck
		.getState()
		.setCartas((c) =>
			actualizar(c, ids, (x) =>
				x.back.kind === "custom" ? { ...x, back: { kind: "deckDefault" } } : x,
			),
		);
}

/** «Auto ordenar» (en Rust): agrupa copias, quita huecos y respeta las fijadas. */
export async function autoOrdenar(t: T) {
	const cs = cartas();
	const orden = await call("arrange_cards", { cards: cs });
	const porId = new Map(cs.map((c) => [c.id, c]));
	const nuevas = orden.flatMap((id) => {
		const c = porId.get(id);
		return c ? [c] : [];
	});
	useDeck.getState().setCartas(() => nuevas);
	useUi.getState().anunciar(t("Cartas ordenadas"));
}

/** «+ Añadir página»: una página de huecos. */
export function anadirPagina(porPagina: number) {
	useDeck.getState().setCartas((c) => [...c, ...huecos(porPagina)]);
}

/** Selección con Ctrl (añadir/quitar) y Mayús (tramo desde el ancla). */
export function seleccionarConClic(
	id: string,
	e: { ctrlKey: boolean; metaKey: boolean; shiftKey: boolean },
) {
	const ui = useUi.getState();
	const cs = cartas();
	if (e.shiftKey && ui.ancla) {
		const a = cs.findIndex((c) => c.id === ui.ancla);
		const b = cs.findIndex((c) => c.id === id);
		if (a >= 0 && b >= 0) {
			const [i, j] = a < b ? [a, b] : [b, a];
			const tramo = cs.slice(i, j + 1).map((c) => c.id);
			const base = e.ctrlKey || e.metaKey ? ui.seleccion : [];
			ui.seleccionar([...new Set([...base, ...tramo])]);
			return;
		}
	}
	if (e.ctrlKey || e.metaKey) {
		const ya = ui.seleccion.includes(id);
		ui.seleccionar(ya ? ui.seleccion.filter((x) => x !== id) : [...ui.seleccion, id], id);
		return;
	}
	ui.seleccionar([id], id);
}

export function cartaPorId(id: string): DeckCard | undefined {
	return cartas().find((c) => c.id === id);
}
