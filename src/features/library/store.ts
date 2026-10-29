import { create } from "zustand";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Tag } from "@/bindings/Tag";
import { call } from "@/lib/ipc";
import { guardarAhora } from "@/stores/deck-store";
import type { Detalles } from "./organizar";

/**
 * Los datos de la biblioteca: los decks (lo barato, al momento), sus
 * detalles (colores, precio… llegan después, de Rust) y las etiquetas. Cada
 * acción llama a Rust y vuelve a leer lo que cambia.
 */
type Store = {
	decks: DeckSummary[] | null;
	detalles: Detalles;
	etiquetas: Tag[];
	/** Calculando los detalles. */
	contando: boolean;
	cargar: () => Promise<void>;
	/** Solo la lista y las etiquetas (tras etiquetar, marcar favorito…). */
	refrescar: () => Promise<void>;
	etiquetar: (decks: string[], add: string[], remove: string[]) => Promise<void>;
	crearEtiqueta: (nombre: string, color: string, icono?: string | null) => Promise<Tag>;
	editarEtiqueta: (
		id: string,
		nombre: string,
		color: string,
		/** Sin pasar: el icono no cambia. */
		icono?: string | null,
	) => Promise<void>;
	borrarEtiqueta: (id: string) => Promise<void>;
	favorito: (decks: string[], si: boolean) => Promise<void>;
	/** El idioma elegido a mano (`null`: el que se calcula). */
	idioma: (decks: string[], lang: string | null) => Promise<void>;
	notas: (id: string, texto: string) => Promise<void>;
	portada: (id: string, carta: string | null) => Promise<void>;
};

export const useBiblioteca = create<Store>((set, get) => ({
	decks: null,
	detalles: {},
	etiquetas: [],
	contando: false,
	cargar: async () => {
		await guardarAhora();
		await get().refrescar();
		set({ contando: true });
		try {
			const v = await call("deck_details", { ids: [] });
			set({ detalles: Object.fromEntries(v.map((d) => [d.id, d])) });
		} finally {
			set({ contando: false });
		}
	},
	refrescar: async () => {
		const [decks, etiquetas] = await Promise.all([call("deck_list", {}), call("tag_list", {})]);
		set({ decks, etiquetas });
	},
	etiquetar: async (decks, add, remove) => {
		await call("deck_tags_set", { decks, add, remove });
		await get().refrescar();
	},
	crearEtiqueta: async (name, color, icon) => {
		const t = await call("tag_create", { name, color });
		// Una que ya existía guarda su icono si no se eligió otro.
		if (icon && icon !== t.icon) await call("tag_icon_set", { id: t.id, icon });
		await get().refrescar();
		return t;
	},
	editarEtiqueta: async (id, name, color, icon) => {
		await call("tag_update", { id, name, color });
		if (icon !== undefined) await call("tag_icon_set", { id, icon });
		await get().refrescar();
	},
	borrarEtiqueta: async (id) => {
		await call("tag_delete", { id });
		await get().refrescar();
	},
	favorito: async (decks, pinned) => {
		await call("deck_pin", { decks, pinned });
		await get().refrescar();
	},
	idioma: async (decks, lang) => {
		await call("deck_lang_set", { decks, lang });
		await get().refrescar();
	},
	notas: async (id, notes) => {
		await call("deck_notes_set", { id, notes });
		await get().refrescar();
	},
	portada: async (id, card) => {
		await call("deck_cover_set", { id, card });
		await get().refrescar();
	},
}));
