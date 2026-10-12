import type { DeckCard } from "@/bindings/DeckCard";

/**
 * Operaciones puras sobre la lista de cartas de un deck. No tocan nada fuera:
 * reciben la lista y devuelven otra. El store las usa y los tests las prueban
 * sin montar React ni Tauri.
 */

/** Un id nuevo para una copia (uuid v4). */
export function nuevoId(): string {
	return crypto.randomUUID();
}

/** Mueve la carta de `desde` a `hasta` (índices de la lista). */
export function mover(cartas: DeckCard[], desde: number, hasta: number): DeckCard[] {
	if (desde === hasta || desde < 0 || desde >= cartas.length) return cartas;
	const destino = Math.max(0, Math.min(hasta, cartas.length - 1));
	const copia = cartas.slice();
	const [c] = copia.splice(desde, 1);
	copia.splice(destino, 0, c);
	return copia;
}

/** Mueve un grupo de cartas (por id) para que empiecen en `hasta`. */
export function moverGrupo(cartas: DeckCard[], ids: string[], hasta: number): DeckCard[] {
	const set = new Set(ids);
	const grupo = cartas.filter((c) => set.has(c.id));
	if (grupo.length === 0) return cartas;
	const resto = cartas.filter((c) => !set.has(c.id));
	const pos = Math.max(0, Math.min(hasta, resto.length));
	return [...resto.slice(0, pos), ...grupo, ...resto.slice(pos)];
}

/** Duplica las cartas indicadas, cada copia justo detrás de su original. */
export function duplicar(
	cartas: DeckCard[],
	ids: string[],
	id: () => string = nuevoId,
): DeckCard[] {
	const set = new Set(ids);
	const out: DeckCard[] = [];
	for (const c of cartas) {
		out.push(c);
		if (set.has(c.id)) out.push({ ...c, id: id(), locked: false });
	}
	return out;
}

export function quitar(cartas: DeckCard[], ids: string[]): DeckCard[] {
	const set = new Set(ids);
	return cartas.filter((c) => !set.has(c.id));
}

/** Inserta cartas en una posición (al final si no se dice). */
export function insertar(cartas: DeckCard[], nuevas: DeckCard[], en?: number): DeckCard[] {
	const pos = en === undefined ? cartas.length : Math.max(0, Math.min(en, cartas.length));
	return [...cartas.slice(0, pos), ...nuevas, ...cartas.slice(pos)];
}

export function actualizar(
	cartas: DeckCard[],
	ids: string[],
	cambio: (c: DeckCard) => DeckCard,
): DeckCard[] {
	const set = new Set(ids);
	return cartas.map((c) => (set.has(c.id) ? cambio(c) : c));
}

/** `n` huecos en blanco (lo que añade «+ Añadir página»). */
export function huecos(n: number, id: () => string = nuevoId): DeckCard[] {
	return Array.from({ length: n }, () => ({
		id: id(),
		source: { kind: "blank" },
		front: null,
		back: { kind: "deckDefault" },
		edits: {
			rotation: "r0",
			crop: { zoom: 1, offsetX: 0, offsetY: 0 },
			adjust: {
				brightness: 0,
				exposure: 0,
				contrast: 0,
				saturation: 0,
				warmth: 0,
				vibrance: 0,
				sharpness: 0,
				clarity: 0,
				noiseReduction: 0,
				vignette: 0,
				fade: 0,
			},
		},
		locked: false,
		meta: {
			name: "",
			set: null,
			collectorNumber: null,
			lang: null,
			langFallback: false,
			layout: null,
			isDfc: false,
			imageStatus: null,
			highres: false,
			typeLine: null,
		},
	}));
}
