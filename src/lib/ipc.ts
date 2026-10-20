import { invoke } from "@tauri-apps/api/core";
import type { ActualizacionEstado } from "@/bindings/ActualizacionEstado";
import type { ActualizacionInfo } from "@/bindings/ActualizacionInfo";
import type { ArchidektImport } from "@/bindings/ArchidektImport";
import type { ArtRef } from "@/bindings/ArtRef";
import type { Aviso } from "@/bindings/Aviso";
import type { CacheStats } from "@/bindings/CacheStats";
import type { CalibrationTextsInput } from "@/bindings/CalibrationTextsInput";
import type { CapaInfo } from "@/bindings/CapaInfo";
import type { CardDesign } from "@/bindings/CardDesign";
import type { CardIdentify } from "@/bindings/CardIdentify";
import type { CardMeta } from "@/bindings/CardMeta";
import type { CardSize } from "@/bindings/CardSize";
import type { CardSummary } from "@/bindings/CardSummary";
import type { Collection } from "@/bindings/Collection";
import type { Deck } from "@/bindings/Deck";
import type { DeckCard } from "@/bindings/DeckCard";
import type { DeckDetails } from "@/bindings/DeckDetails";
import type { DeckKind } from "@/bindings/DeckKind";
import type { DeckStats } from "@/bindings/DeckStats";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { DescargableInfo } from "@/bindings/DescargableInfo";
import type { DesignDraft } from "@/bindings/DesignDraft";
import type { DesignLang } from "@/bindings/DesignLang";
import type { ExportEstimate } from "@/bindings/ExportEstimate";
import type { FaceImage } from "@/bindings/FaceImage";
import type { FichaPropuesta } from "@/bindings/FichaPropuesta";
import type { FormatoLista } from "@/bindings/FormatoLista";
import type { FuenteCarta } from "@/bindings/FuenteCarta";
import type { GcReport } from "@/bindings/GcReport";
import type { IdentifyRequest } from "@/bindings/IdentifyRequest";
import type { LetraInfo } from "@/bindings/LetraInfo";
import type { LetraLibre } from "@/bindings/LetraLibre";
import type { NombresIdiomasInfo } from "@/bindings/NombresIdiomasInfo";
import type { OriginalInfo } from "@/bindings/OriginalInfo";
import type { PackInfo } from "@/bindings/PackInfo";
import type { PagePlan } from "@/bindings/PagePlan";
import type { ParsedDecklist } from "@/bindings/ParsedDecklist";
import type { Preset } from "@/bindings/Preset";
import type { PresetKind } from "@/bindings/PresetKind";
import type { PrintSettings } from "@/bindings/PrintSettings";
import type { ResolveOptions } from "@/bindings/ResolveOptions";
import type { Resumen } from "@/bindings/Resumen";
import type { SaveResult } from "@/bindings/SaveResult";
import type { SetInfo } from "@/bindings/SetInfo";
import type { SimbolosInfo } from "@/bindings/SimbolosInfo";
import type { Sugerencia } from "@/bindings/Sugerencia";
import type { Tag } from "@/bindings/Tag";
import type { TemaCatalogo } from "@/bindings/TemaCatalogo";
import type { TemaInstalado } from "@/bindings/TemaInstalado";
import type { TipoCarpeta } from "@/bindings/TipoCarpeta";

/**
 * **La única puerta a Rust.** Cada comando de `src-tauri/src/commands/` tiene
 * aquí su firma: el nombre, lo que recibe y lo que devuelve. Una pantalla
 * nunca llama a `invoke` directamente; así el compilador vigila que el nombre
 * y los tipos casen con lo que Rust registra.
 *
 * Los errores llegan tal cual (un `AppError` `{ code, params }`) y se
 * traducen con `useLang().mensaje(e)`.
 */
export interface Commands {
	ping: { args: Record<string, never>; result: string };
	ping_error: { args: Record<string, never>; result: null };
	update_status: { args: Record<string, never>; result: ActualizacionEstado };
	update_check: { args: Record<string, never>; result: ActualizacionInfo | null };
	update_download: { args: { version: string }; result: null };
	update_install: { args: { version: string }; result: null };
	parse_decklist: { args: { text: string }; result: ParsedDecklist };
	compute_page_plan: { args: { deck: Deck }; result: PagePlan };
	deck_list: { args: Record<string, never>; result: DeckSummary[] };
	deck_create: { args: { name: string }; result: Deck };
	deck_open: { args: { id: string }; result: Deck };
	deck_save: { args: { deck: Deck }; result: SaveResult };
	deck_rename: { args: { id: string; name: string }; result: null };
	deck_duplicate: { args: { id: string; name: string }; result: Deck };
	deck_delete: { args: { id: string }; result: null };
	deck_peek: { args: { id: string }; result: Deck };
	deck_details: { args: { ids: string[] }; result: DeckDetails[] };
	tag_list: { args: Record<string, never>; result: Tag[] };
	tag_create: { args: { name: string; color: string }; result: Tag };
	tag_update: { args: { id: string; name: string; color: string }; result: Tag };
	tag_delete: { args: { id: string }; result: null };
	deck_tags_set: { args: { decks: string[]; add: string[]; remove: string[] }; result: null };
	deck_pin: { args: { decks: string[]; pinned: boolean }; result: null };
	deck_notes_set: { args: { id: string; notes: string }; result: null };
	deck_cover_set: { args: { id: string; card: string | null }; result: null };
	deck_lang_set: { args: { decks: string[]; lang: string | null }; result: null };
	blob_import_files: { args: { paths: string[] }; result: FaceImage[] };
	blob_generic_back: { args: Record<string, never>; result: FaceImage };
	blob_gc: { args: Record<string, never>; result: GcReport };
	app_setting_get: { args: { key: string }; result: string | null };
	app_setting_set: { args: { key: string; value: string }; result: null };
	resolve_decklist: { args: { list: ParsedDecklist; options: ResolveOptions }; result: number };
	job_cancel: { args: { jobId: number }; result: boolean };
	card_autocomplete: { args: { q: string }; result: string[] };
	card_search: {
		args: { q: string; lang: string | null; highresOnly: boolean };
		result: CardSummary[];
	};
	card_prints: {
		args: { name: string; lang: string | null; highresOnly: boolean };
		result: CardSummary[];
	};
	card_fetch: { args: { scryfallId: string }; result: DeckCard };
	card_fetch_externa: { args: { fuente: FuenteCarta; id: string }; result: DeckCard };
	deck_tokens_detect: { args: { cards: DeckCard[] }; result: FichaPropuesta[] };
	sets_list: { args: Record<string, never>; result: SetInfo[] };
	set_cards: { args: { code: string; typeFilter: string | null }; result: CardSummary[] };
	import_archidekt: { args: { url: string; includeSideboard: boolean }; result: ArchidektImport };
	arrange_cards: { args: { cards: DeckCard[] }; result: string[] };
	carpeta_recordar: { args: { tipo: TipoCarpeta; ruta: string }; result: null };
	carpeta_inicial: { args: { tipo: TipoCarpeta; nombre: string | null }; result: string | null };
	deck_stats: { args: { cards: DeckCard[]; kind: DeckKind | null }; result: DeckStats };
	ocr_ready: { args: Record<string, never>; result: boolean };
	card_identify: {
		args: { blob: string; hint: string | null; ocr: boolean };
		result: CardIdentify;
	};
	card_link: { args: { scryfallId: string }; result: CardMeta };
	card_link_externa: { args: { fuente: FuenteCarta; id: string }; result: CardMeta };
	identify_batch: { args: { items: IdentifyRequest[]; ocr: boolean }; result: number };
	design_draft: {
		args: { card: DeckCard; usados: string[]; collection: string | null };
		result: DesignDraft;
	};
	collection_list: { args: Record<string, never>; result: Collection[] };
	collection_save: { args: { collection: Collection }; result: Collection };
	collection_delete: { args: { id: string }; result: null };
	fonts_list: { args: Record<string, never>; result: LetraInfo[] };
	font_import: { args: { path: string }; result: LetraInfo };
	font_remove: { args: { id: string }; result: null };
	packs_list: { args: Record<string, never>; result: PackInfo[] };
	pack_import: { args: { path: string }; result: PackInfo };
	pack_remove: { args: { id: string }; result: null };
	fonts_free_list: { args: Record<string, never>; result: LetraLibre[] };
	font_free_download: { args: { id: string }; result: LetraLibre };
	original_info: { args: Record<string, never>; result: OriginalInfo };
	original_download: { args: Record<string, never>; result: PackInfo };
	packs_downloadable_list: { args: Record<string, never>; result: DescargableInfo[] };
	pack_downloadable_download: { args: { id: string }; result: PackInfo };
	pack_extra_use: { args: { id: string; ruta: string }; result: FaceImage };
	names_index_info: { args: Record<string, never>; result: NombresIdiomasInfo };
	names_index_download: { args: Record<string, never>; result: NombresIdiomasInfo };
	names_index_remove: { args: Record<string, never>; result: null };
	themes_catalog: { args: Record<string, never>; result: TemaCatalogo[] };
	theme_fetch: { args: { id: string }; result: TemaCatalogo };
	theme_install: { args: { id: string }; result: TemaInstalado };
	themes_installed: { args: Record<string, never>; result: TemaInstalado[] };
	theme_remove: { args: { id: string }; result: null };
	themes_open_community: { args: Record<string, never>; result: null };
	symbols_status: { args: { refresh: boolean }; result: SimbolosInfo };
	design_register: {
		args: { design: CardDesign; cara: number; lang: DesignLang };
		result: string;
	};
	design_overlay: {
		args: { design: CardDesign; cara: number; lang: DesignLang; ancho: number };
		result: CapaInfo | null;
	};
	design_fit_report: { args: { design: CardDesign; lang: DesignLang }; result: Aviso[] };
	design_drafts_for_deck: {
		args: { cards: DeckCard[]; usados: string[]; collection: string | null; lang: DesignLang };
		result: number;
	};
	design_export_png: {
		args: {
			cards: DeckCard[];
			folder: string;
			cardSize: CardSize;
			dpi: number;
			langs: DesignLang[];
		};
		result: number;
	};
	design_suggest_translation: {
		args: { design: CardDesign; cara: number };
		result: Sugerencia;
	};
	design_structure: {
		args: { design: CardDesign; cara: number; lang: DesignLang };
		result: Resumen;
	};
	design_save: {
		args: { card: DeckCard; design: CardDesign; lang: DesignLang };
		result: DeckCard;
	};
	design_remove: { args: { card: DeckCard }; result: DeckCard };
	design_rerender: { args: { cards: DeckCard[]; lang: DesignLang }; result: DeckCard[] };
	design_renumber: { args: { cards: DeckCard[]; lang: DesignLang }; result: DeckCard[] };
	art_from_clipboard: { args: Record<string, never>; result: ArtRef | null };
	art_from_blob: { args: { blob: string }; result: ArtRef };
	art_from_scryfall: { args: { scryfallId: string; cara: number }; result: ArtRef };
	art_from_card: {
		args: { blob: string; x: number; y: number; w: number; h: number };
		result: ArtRef;
	};
	export_pdf: { args: { deck: Deck; path: string }; result: number };
	export_estimate: { args: { deck: Deck }; result: ExportEstimate };
	export_calibration: {
		args: { settings: PrintSettings; path: string; texts: CalibrationTextsInput };
		result: string;
	};
	export_cancel: { args: { jobId: number }; result: boolean };
	export_open: { args: { path: string }; result: null };
	export_reveal: { args: { path: string }; result: null };
	cache_stats: { args: Record<string, never>; result: CacheStats };
	cache_clear: { args: Record<string, never>; result: CacheStats };
	deck_export_package: { args: { id: string; path: string }; result: number };
	deck_list_text: { args: { deck: Deck; format: FormatoLista }; result: string };
	deck_list_save: { args: { deck: Deck; format: FormatoLista; path: string }; result: null };
	deck_import_package: { args: { path: string }; result: Deck };
	preset_list: { args: { kind: PresetKind }; result: Preset[] };
	preset_save: { args: { kind: PresetKind; name: string; json: string }; result: Preset };
	preset_delete: { args: { id: string }; result: null };
}

export function call<C extends keyof Commands>(
	cmd: C,
	args?: Commands[C]["args"],
): Promise<Commands[C]["result"]> {
	return invoke<Commands[C]["result"]>(cmd, args);
}

/** Los eventos que emite Rust, con su carga. */
export interface Events {
	"resolve://progress": import("@/bindings/ResolveProgressEvent").ResolveProgressEvent;
	"resolve://done": import("@/bindings/ResolveDoneEvent").ResolveDoneEvent;
	"export://progress": import("@/bindings/ExportProgressEvent").ExportProgressEvent;
	"export://done": import("@/bindings/ExportDoneEvent").ExportDoneEvent;
	"identify://progress": import("@/bindings/IdentifyProgressEvent").IdentifyProgressEvent;
	"identify://done": import("@/bindings/IdentifyDoneEvent").IdentifyDoneEvent;
	"design://progress": import("@/bindings/DesignProgressEvent").DesignProgressEvent;
	"design://done": import("@/bindings/DesignDoneEvent").DesignDoneEvent;
	"designpng://progress": import("@/bindings/DesignPngProgressEvent").DesignPngProgressEvent;
	"designpng://done": import("@/bindings/DesignPngDoneEvent").DesignPngDoneEvent;
	"original://progress": import("@/bindings/OriginalProgressEvent").OriginalProgressEvent;
	"packs://progress": import("@/bindings/DescargaProgressEvent").DescargaProgressEvent;
	"names://progress": import("@/bindings/NombresProgressEvent").NombresProgressEvent;
	"update://progress": import("@/bindings/ActualizacionProgressEvent").ActualizacionProgressEvent;
}

/** Escucha un evento de Rust. Devuelve la función que deja de escuchar. */
export async function on<E extends keyof Events>(
	evento: E,
	f: (carga: Events[E]) => void,
): Promise<() => void> {
	const { listen } = await import("@tauri-apps/api/event");
	return listen<Events[E]>(evento, (e) => f(e.payload));
}
