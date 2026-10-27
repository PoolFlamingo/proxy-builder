import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";
import type { MarcaCarta } from "@/bindings/MarcaCarta";
import type { ParsedDecklist } from "@/bindings/ParsedDecklist";
import type { ParsedLine } from "@/bindings/ParsedLine";
import { paraTraducir } from "@/components/language-provider";
import { huecos, nuevoId } from "./deck-ops";

/** Lo que va al dorso de una carta al imprimir a doble cara (para «Ver reversos»). */
export function imagenDorso(c: DeckCard, deck: Deck): FaceImage | null {
	if (c.back.kind === "dfc" || c.back.kind === "custom") return c.back.image;
	return deck.defaultBack;
}

/** Una carta propia a partir de una imagen subida. */
export function cartaPropia(img: FaceImage, nombre: string): DeckCard {
	const [base] = huecos(1);
	return {
		...base,
		id: nuevoId(),
		source: { kind: "custom" },
		front: img,
		meta: { ...base.meta, name: nombre },
	};
}

/**
 * El nombre de carta que da un fichero: sin carpeta ni extensión y sin la
 * numeración que suelen llevar las carpetas de proxies («0001 - », «T003 - »,
 * «12. ») ni las marcas de cara o de repetido del final («(1)», «~2»), como
 * los que saca «Exportar imágenes». Si no queda nada, el nombre tal cual.
 */
export function nombreDeFichero(ruta: string): string {
	const f = (ruta.split(/[\\/]/).pop() ?? ruta).replace(/\.[^.]+$/, "");
	const limpio = f
		.replace(/^[A-Za-z]?\d{1,4}\s*[-–._]\s*/, "")
		.replace(/\s+(\(\d{1,2}\)|~\d+)$/, "")
		.trim();
	return limpio || f.trim();
}

export type Insignia = {
	texto: string;
	tono: "info" | "aviso" | "error" | "marca";
	ayuda: string;
};

/** Las marcas fijas de una carta del deck (`DeckCard.marca`), con su texto. */
export const MARCAS: Record<MarcaCarta, { nombre: string; insignia: string; ayuda: string }> = {
	variante: {
		nombre: paraTraducir("Variante de diseño"),
		insignia: paraTraducir("VARIANTE"),
		ayuda: paraTraducir(
			"Otra versión de una carta que ya está: se escogerá una. De sus copias solo cuenta una para el mazo.",
		),
	},
	opcional: {
		nombre: paraTraducir("Opcional"),
		insignia: paraTraducir("OPCIONAL"),
		ayuda: paraTraducir("Puede ir o no: no cuenta para el mazo."),
	},
	banquillo: {
		nombre: paraTraducir("Banquillo"),
		insignia: paraTraducir("BANQUILLO"),
		ayuda: paraTraducir("Del banquillo (sideboard): no cuenta para el mazo."),
	},
	copia: {
		nombre: paraTraducir("Copia (hace de ficha)"),
		insignia: paraTraducir("COPIA"),
		ayuda: paraTraducir(
			"Una carta que hace de ficha (la copia de un Clone, un cambiaformas…): no cuenta para el mazo y en la mesa sale entre las fichas.",
		),
	},
};

/** Las marcas que se pintan sobre una carta (CUSTOM, ↩ EN, LOWRES…). */
export function insignias(c: DeckCard): Insignia[] {
	const marca: Insignia[] = c.marca
		? [{ texto: MARCAS[c.marca].insignia, tono: "marca", ayuda: MARCAS[c.marca].ayuda }]
		: [];
	return [...marca, ...insigniasDeLaCarta(c)];
}

function insigniasDeLaCarta(c: DeckCard): Insignia[] {
	// Una carta diseñada se pinta aquí: lo de la imagen de Scryfall no aplica.
	if (c.design)
		return [
			c.design.status === "ready"
				? {
						texto: "PROXY",
						tono: "info",
						ayuda: paraTraducir("Diseñada en la app, lista para imprimir"),
					}
				: {
						texto: paraTraducir("BORRADOR"),
						tono: "aviso",
						ayuda: paraTraducir("Diseñada en la app, todavía en borrador"),
					},
		];
	const out: Insignia[] = [];
	if (c.source.kind === "custom")
		out.push(
			c.source.scryfallId
				? {
						texto: "CUSTOM ↔",
						tono: "info",
						ayuda: paraTraducir("Imagen propia vinculada a su carta de Scryfall"),
					}
				: { texto: "CUSTOM", tono: "info", ayuda: paraTraducir("Imagen propia") },
		);
	if (c.meta.langFallback)
		out.push({
			texto: "↩ EN",
			tono: "aviso",
			ayuda: paraTraducir("No existe en el idioma pedido: se ha quedado en inglés"),
		});
	const st = c.meta.imageStatus;
	if (st === "lowres")
		out.push({
			texto: "LOWRES",
			tono: "aviso",
			ayuda: paraTraducir("Scryfall solo tiene un escaneo de baja resolución"),
		});
	if (st === "placeholder")
		out.push({
			texto: "PLACEHOLDER",
			tono: "error",
			ayuda: paraTraducir("Scryfall aún no tiene la imagen real"),
		});
	if (st === "missing")
		out.push({ texto: "MISSING", tono: "error", ayuda: paraTraducir("Scryfall no tiene imagen") });
	return out;
}

/**
 * La lista que vuelve a pedir a Scryfall las cartas no fijadas, una línea por
 * copia, para cambiarles el idioma («Reaplicar idioma»). Devuelve también qué
 * carta va con cada línea.
 */
export function listaParaReaplicar(cartas: DeckCard[]): {
	lista: ParsedDecklist;
	ids: string[];
} {
	const lines: ParsedLine[] = [];
	const ids: string[] = [];
	for (const c of cartas) {
		if (c.locked || c.source.kind !== "scryfall" || !c.meta.name) continue;
		ids.push(c.id);
		lines.push({
			lineNo: lines.length + 1,
			raw: c.meta.name,
			kind: "card",
			qty: 1,
			name: c.meta.name,
			set: c.meta.set,
			collector: c.meta.collectorNumber,
			foil: false,
			section: "main",
			included: true,
			error: null,
		});
	}
	return { lista: { lines, cardCount: lines.length, invalidCount: 0 }, ids };
}

/**
 * Aplica el resultado de «Reaplicar idioma»: cada línea resuelta trae la
 * carta nueva en orden; las no resueltas (por número de línea) se saltan y la
 * carta se queda como estaba. Se conservan id, retoques y fijado.
 */
export function aplicarReaplicado(
	cartas: DeckCard[],
	ids: string[],
	nuevas: DeckCard[],
	noResueltas: number[],
): DeckCard[] {
	const fuera = new Set(noResueltas);
	const porId = new Map<string, DeckCard>();
	let k = 0;
	ids.forEach((id, i) => {
		if (fuera.has(i + 1)) return;
		const n = nuevas[k++];
		if (n) porId.set(id, n);
	});
	return cartas.map((c) => {
		const n = porId.get(c.id);
		if (!n) return c;
		const dorso = c.back.kind === "custom" ? c.back : n.back;
		return { ...n, id: c.id, edits: c.edits, locked: c.locked, back: dorso };
	});
}
