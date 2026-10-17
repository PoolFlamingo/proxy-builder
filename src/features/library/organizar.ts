import type { DeckDetails } from "@/bindings/DeckDetails";
import type { DeckKind } from "@/bindings/DeckKind";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Tag } from "@/bindings/Tag";

/**
 * Filtrar, ordenar y agrupar los decks de la biblioteca. Funciones puras: la
 * pantalla solo pinta lo que salga de aquí. Los detalles (colores, precio…)
 * pueden no haber llegado aún: un deck sin ellos no pasa un filtro que los
 * necesite y va al final al ordenar por ellos.
 */

export type Orden =
	| "modificado"
	| "creado"
	| "abierto"
	| "nombre"
	| "cartas"
	| "precio"
	| "coste"
	| "colores";
export type Agrupar = "ninguno" | "etiqueta" | "color" | "clase" | "idioma";
/** Con los colores elegidos: que los tenga todos, que sean exactamente esos o
 *  que no use otros (se puede jugar con ellos). */
export type ModoColor = "incluye" | "exacto" | "dentro";

export type Filtros = {
	texto: string;
	etiquetas: string[];
	/** Con varias etiquetas: que tenga todas (y no solo alguna). */
	todasLasEtiquetas: boolean;
	sinEtiqueta: boolean;
	favoritos: boolean;
	/** W U B R G y C (incoloro). */
	colores: string[];
	modoColor: ModoColor;
	clases: DeckKind[];
	/** Legal en este formato (`modern`, `commander`…). */
	formato: string | null;
	idioma: string | null;
	conDisenos: boolean;
	conPropias: boolean;
};

export const FILTROS_VACIOS: Filtros = {
	texto: "",
	etiquetas: [],
	todasLasEtiquetas: false,
	sinEtiqueta: false,
	favoritos: false,
	colores: [],
	modoColor: "incluye",
	clases: [],
	formato: null,
	idioma: null,
	conDisenos: false,
	conPropias: false,
};

/** Los filtros del panel (no cuentan el texto ni la barra lateral). */
export function filtrosDelPanel(f: Filtros): number {
	return (
		(f.colores.length > 0 ? 1 : 0) +
		(f.clases.length > 0 ? 1 : 0) +
		(f.formato ? 1 : 0) +
		(f.idioma ? 1 : 0) +
		(f.conDisenos ? 1 : 0) +
		(f.conPropias ? 1 : 0)
	);
}

export function hayFiltros(f: Filtros): boolean {
	return (
		f.texto.trim() !== "" ||
		f.etiquetas.length > 0 ||
		f.sinEtiqueta ||
		f.favoritos ||
		filtrosDelPanel(f) > 0
	);
}

export type Detalles = Record<string, DeckDetails | undefined>;

export const WUBRG = ["W", "U", "B", "R", "G"] as const;

/** Sin tildes ni mayúsculas, para buscar. */
export function normalizar(s: string): string {
	return s
		.normalize("NFD")
		.replace(/\p{Diacritic}/gu, "")
		.toLowerCase();
}

function pasaColores(id: string[], f: Filtros): boolean {
	const elegidos = f.colores.filter((c) => c !== "C");
	const incoloro = f.colores.includes("C");
	switch (f.modoColor) {
		case "incluye":
			return (!incoloro || id.length === 0) && elegidos.every((c) => id.includes(c));
		case "exacto":
			return (
				id.length === elegidos.length &&
				elegidos.every((c) => id.includes(c)) &&
				(!incoloro || id.length === 0)
			);
		case "dentro":
			return id.every((c) => elegidos.includes(c));
	}
}

/** Lo que se mira al buscar: nombre, portada, notas, etiquetas y cartas. */
function pajar(d: DeckSummary, det: DeckDetails | undefined, nombres: Map<string, string>): string {
	return normalizar(
		[
			d.name,
			d.coverName ?? "",
			d.notes,
			...d.tags.map((t) => nombres.get(t) ?? ""),
			...(det?.cardNames ?? []),
		].join("\n"),
	);
}

export function filtrar(
	decks: DeckSummary[],
	det: Detalles,
	etiquetas: Tag[],
	f: Filtros,
): DeckSummary[] {
	const nombres = new Map(etiquetas.map((t) => [t.id, t.name]));
	const palabras = normalizar(f.texto).split(/\s+/).filter(Boolean);
	return decks.filter((d) => {
		if (f.favoritos && !d.pinned) return false;
		if (f.sinEtiqueta && d.tags.length > 0) return false;
		if (f.etiquetas.length > 0) {
			const tiene = (t: string) => d.tags.includes(t);
			if (f.todasLasEtiquetas ? !f.etiquetas.every(tiene) : !f.etiquetas.some(tiene)) return false;
		}
		const x = det[d.id];
		if (palabras.length > 0) {
			const p = pajar(d, x, nombres);
			if (!palabras.every((w) => p.includes(w))) return false;
		}
		if (filtrosDelPanel(f) > 0) {
			if (!x) return false;
			if (f.colores.length > 0 && !pasaColores(x.colorIdentity, f)) return false;
			if (f.clases.length > 0 && !f.clases.includes(x.kind)) return false;
			if (f.formato && !x.legalFormats.includes(f.formato)) return false;
			if (f.idioma && !x.langs.includes(f.idioma)) return false;
			if (f.conDisenos && x.designed === 0) return false;
			if (f.conPropias && x.custom === 0) return false;
		}
		return true;
	});
}

/** Orden de un grupo de colores: por cuántos y luego WUBRG. */
export function claveColores(id: string[]): string {
	const pos = id.map((c) => WUBRG.indexOf(c as (typeof WUBRG)[number])).sort((a, b) => a - b);
	return `${id.length}${pos.join("")}`;
}

/** Ordena (copia). Los decks sin el dato van al final, sea cual sea el sentido. */
export function ordenar(
	decks: DeckSummary[],
	det: Detalles,
	orden: Orden,
	ascendente: boolean,
	favoritosArriba = false,
): DeckSummary[] {
	const valor = (d: DeckSummary): number | string | null => {
		const x = det[d.id];
		switch (orden) {
			case "modificado":
				return d.updatedAt;
			case "creado":
				return d.createdAt;
			case "abierto":
				return d.openedAt;
			case "nombre":
				return normalizar(d.name);
			case "cartas":
				return x ? x.deckSize : d.cardCount;
			case "precio":
				return x?.priceEur ?? x?.priceUsd ?? null;
			case "coste":
				return x && x.deckSize - x.lands - x.unknown > 0 ? x.averageCmc : null;
			case "colores":
				return x ? claveColores(x.colorIdentity) : null;
		}
	};
	const signo = ascendente ? 1 : -1;
	return [...decks].sort((a, b) => {
		if (favoritosArriba && a.pinned !== b.pinned) return a.pinned ? -1 : 1;
		const va = valor(a);
		const vb = valor(b);
		if (va === null || vb === null) {
			if (va !== vb) return va === null ? 1 : -1;
		} else if (va !== vb) {
			const c =
				typeof va === "string" && typeof vb === "string"
					? va.localeCompare(vb)
					: (va as number) - (vb as number);
			if (c !== 0) return c * signo;
		}
		return normalizar(a.name).localeCompare(normalizar(b.name));
	});
}

/** De qué es un grupo (la pantalla pone el título). */
export type ClaveGrupo =
	| { tipo: "todos" }
	| { tipo: "etiqueta"; tag: Tag }
	| { tipo: "sinEtiqueta" }
	| { tipo: "color"; colores: string[] }
	| { tipo: "clase"; clase: DeckKind }
	| { tipo: "idioma"; idioma: string }
	| { tipo: "sinDatos" };

export type Grupo = { id: string; clave: ClaveGrupo; decks: DeckSummary[] };

const CLASES: DeckKind[] = ["commander", "constructed", "limited"];

/**
 * Reparte los decks (ya filtrados y ordenados) en grupos. Por etiqueta, un
 * deck con varias sale en cada una; los demás, en un solo grupo.
 */
export function agrupar(
	decks: DeckSummary[],
	det: Detalles,
	etiquetas: Tag[],
	por: Agrupar,
): Grupo[] {
	if (por === "ninguno") return [{ id: "todos", clave: { tipo: "todos" }, decks }];
	if (por === "etiqueta") {
		const grupos: Grupo[] = etiquetas
			.map((tag) => ({
				id: `t:${tag.id}`,
				clave: { tipo: "etiqueta" as const, tag },
				decks: decks.filter((d) => d.tags.includes(tag.id)),
			}))
			.filter((g) => g.decks.length > 0);
		const conocidas = new Set(etiquetas.map((t) => t.id));
		const sin = decks.filter((d) => !d.tags.some((t) => conocidas.has(t)));
		if (sin.length > 0)
			grupos.push({ id: "sin-etiqueta", clave: { tipo: "sinEtiqueta" }, decks: sin });
		return grupos;
	}
	const mapa = new Map<string, Grupo>();
	const orden: string[] = [];
	const meter = (id: string, clave: ClaveGrupo, d: DeckSummary) => {
		let g = mapa.get(id);
		if (!g) {
			g = { id, clave, decks: [] };
			mapa.set(id, g);
			orden.push(id);
		}
		g.decks.push(d);
	};
	for (const d of decks) {
		const x = det[d.id];
		if (!x) {
			meter("sin-datos", { tipo: "sinDatos" }, d);
			continue;
		}
		if (por === "color") {
			meter(`c:${claveColores(x.colorIdentity)}`, { tipo: "color", colores: x.colorIdentity }, d);
		} else if (por === "clase") {
			meter(`k:${x.kind}`, { tipo: "clase", clase: x.kind }, d);
		} else {
			const idioma = x.langs[0] ?? "";
			meter(`l:${idioma}`, { tipo: "idioma", idioma }, d);
		}
	}
	// Los grupos sin datos o sin idioma, al final.
	const rango = (g: Grupo): [number, string] => {
		const c = g.clave;
		if (c.tipo === "sinDatos") return [2, ""];
		if (c.tipo === "color") return [0, claveColores(c.colores)];
		if (c.tipo === "clase") return [0, String(CLASES.indexOf(c.clase))];
		if (c.tipo === "idioma") return c.idioma ? [0, c.idioma] : [1, ""];
		return [0, ""];
	};
	return orden
		.map((id) => mapa.get(id) as Grupo)
		.sort((a, b) => {
			const [ra, sa] = rango(a);
			const [rb, sb] = rango(b);
			return ra - rb || (sa < sb ? -1 : sa > sb ? 1 : 0);
		});
}

/** Cuántos decks hay con cada etiqueta (y sin ninguna, y favoritos). */
export function contar(decks: DeckSummary[]): {
	porEtiqueta: Map<string, number>;
	sinEtiqueta: number;
	favoritos: number;
} {
	const porEtiqueta = new Map<string, number>();
	let sinEtiqueta = 0;
	let favoritos = 0;
	for (const d of decks) {
		if (d.pinned) favoritos++;
		if (d.tags.length === 0) sinEtiqueta++;
		for (const t of d.tags) porEtiqueta.set(t, (porEtiqueta.get(t) ?? 0) + 1);
	}
	return { porEtiqueta, sinEtiqueta, favoritos };
}

/**
 * La selección tras pulsar `id` con Ctrl (añade o quita) o Mayús (el tramo
 * desde el ancla, en el orden de `visibles`).
 */
export function seleccionar(
	actual: string[],
	visibles: string[],
	id: string,
	ancla: string | null,
	modo: "alternar" | "tramo",
): string[] {
	if (modo === "alternar")
		return actual.includes(id) ? actual.filter((x) => x !== id) : [...actual, id];
	const a = ancla ? visibles.indexOf(ancla) : -1;
	const b = visibles.indexOf(id);
	if (a < 0 || b < 0) return actual.includes(id) ? actual : [...actual, id];
	const tramo = visibles.slice(Math.min(a, b), Math.max(a, b) + 1);
	return [...new Set([...actual, ...tramo])];
}
