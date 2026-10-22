import type { BeforeDesign } from "@/bindings/BeforeDesign";
import type { CardDesign } from "@/bindings/CardDesign";
import type { DeckCard } from "@/bindings/DeckCard";
import { duplicar } from "@/lib/deck-ops";

/**
 * Las cartas del deck agrupadas como se diseñan: todas las copias de la misma
 * carta de Scryfall juntas (un diseño se aplica a todas, salvo «solo esta
 * copia»). Las propias sin vincular van sueltas y no se pueden diseñar.
 */
export type Grupo = {
	clave: string;
	/** La carta de Scryfall (`null` = propia sin vincular). */
	scryfallId: string | null;
	cartas: DeckCard[];
};

export function scryfallDe(c: DeckCard): string | null {
	switch (c.source.kind) {
		case "scryfall":
		case "token":
			return c.source.scryfallId;
		case "custom":
			return c.source.scryfallId ?? null;
		default:
			return null;
	}
}

export function agrupar(cartas: DeckCard[]): Grupo[] {
	const grupos = new Map<string, Grupo>();
	for (const c of cartas) {
		if (c.source.kind === "blank") continue;
		const id = scryfallDe(c);
		const clave = id ? `s:${id}` : `c:${c.id}`;
		const g = grupos.get(clave);
		if (g) g.cartas.push(c);
		else grupos.set(clave, { clave, scryfallId: id, cartas: [c] });
	}
	return [...grupos.values()];
}

/**
 * La carta a la que pasan las flechas desde `actual` (la copia que se está
 * diseñando): la primera de la carta siguiente o, con `porCopias`, la copia
 * siguiente aunque sea de la misma carta. Da la vuelta al llegar al final.
 */
export function vecina(
	elegibles: Grupo[],
	actual: string | null,
	dir: 1 | -1,
	porCopias: boolean,
): string | null {
	const ids = porCopias
		? elegibles.flatMap((g) => g.cartas.map((c) => c.id))
		: elegibles.flatMap((g) => (g.cartas[0] ? [g.cartas[0].id] : []));
	if (ids.length === 0) return null;
	const i = porCopias
		? ids.indexOf(actual ?? "")
		: elegibles.findIndex((g) => g.cartas.some((c) => c.id === actual));
	if (i < 0) return (dir > 0 ? ids[0] : ids[ids.length - 1]) ?? null;
	return ids[(i + dir + ids.length) % ids.length] ?? null;
}

/** El estado de diseño de un grupo (el de su primera copia). */
export type EstadoDiseno = "sinVincular" | "sinDiseno" | "borrador" | "listo";

export function estadoDe(g: Grupo): EstadoDiseno {
	if (!g.scryfallId) return "sinVincular";
	const d = g.cartas[0]?.design;
	if (!d) return "sinDiseno";
	return d.status === "ready" ? "listo" : "borrador";
}

/** Cómo era una carta antes de diseñarla (para «Quitar diseño»). */
export function antesDe(c: DeckCard): BeforeDesign {
	return (
		c.design?.before ?? {
			source: c.source,
			front: c.front ?? undefined,
			back: c.back,
			edits: c.edits,
		}
	);
}

/** Los números de colección que ya hay en el deck (para numerar el siguiente). */
export function numerosUsados(cartas: DeckCard[]): string[] {
	const v: string[] = [];
	for (const c of cartas) if (c.design?.footer.number) v.push(c.design.footer.number);
	return v;
}

/**
 * Pone la carta guardada (`nueva`, que devuelve Rust con su miniatura) en el
 * deck: en `principal` y, si `todas`, en las demás copias del grupo, cada una
 * con su propio «antes».
 */
export function aplicarAlDeck(
	cartas: DeckCard[],
	grupo: Grupo,
	principal: string,
	nueva: DeckCard,
	todas: boolean,
): DeckCard[] {
	const ids = new Set(todas ? grupo.cartas.map((c) => c.id) : [principal]);
	return cartas.map((c) => {
		if (!ids.has(c.id)) return c;
		if (c.id === principal) return nueva;
		const design: CardDesign | undefined = nueva.design
			? { ...nueva.design, before: antesDe(c) }
			: undefined;
		return { ...nueva, id: c.id, locked: c.locked, design };
	});
}

/**
 * Otra copia de la carta `id`, justo detrás de ella y con una copia de su
 * diseño, marcada como variante de diseño (se imprime, pero no cuenta para el
 * mazo). Si `id` no está, las cartas no cambian.
 */
export function conVariante(cartas: DeckCard[], id: string, nuevo: string): DeckCard[] {
	return duplicar(cartas, [id], () => nuevo).map((c) =>
		c.id === nuevo ? { ...c, marca: "variante" as const } : c,
	);
}

/** Deja las cartas como estaban antes del diseño. */
export function quitarDiseno(cartas: DeckCard[], ids: Set<string>): DeckCard[] {
	return cartas.map((c) => {
		if (!ids.has(c.id) || !c.design) return c;
		const a = c.design.before;
		const { design: _fuera, ...resto } = c;
		return {
			...resto,
			source: a.source,
			front: a.front ?? null,
			back: a.back,
			edits: a.edits,
		};
	});
}
