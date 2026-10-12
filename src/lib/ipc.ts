import { invoke } from "@tauri-apps/api/core";
import type { Deck } from "@/bindings/Deck";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { FaceImage } from "@/bindings/FaceImage";
import type { GcReport } from "@/bindings/GcReport";
import type { PagePlan } from "@/bindings/PagePlan";
import type { ParsedDecklist } from "@/bindings/ParsedDecklist";
import type { SaveResult } from "@/bindings/SaveResult";

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
	parse_decklist: { args: { text: string }; result: ParsedDecklist };
	compute_page_plan: { args: { deck: Deck }; result: PagePlan };
	deck_list: { args: Record<string, never>; result: DeckSummary[] };
	deck_create: { args: { name: string }; result: Deck };
	deck_open: { args: { id: string }; result: Deck };
	deck_save: { args: { deck: Deck }; result: SaveResult };
	deck_rename: { args: { id: string; name: string }; result: null };
	deck_duplicate: { args: { id: string; name: string }; result: Deck };
	deck_delete: { args: { id: string }; result: null };
	blob_import_files: { args: { paths: string[] }; result: FaceImage[] };
	blob_generic_back: { args: Record<string, never>; result: FaceImage };
	blob_gc: { args: Record<string, never>; result: GcReport };
	app_setting_get: { args: { key: string }; result: string | null };
	app_setting_set: { args: { key: string; value: string }; result: null };
}

export function call<C extends keyof Commands>(
	cmd: C,
	args?: Commands[C]["args"],
): Promise<Commands[C]["result"]> {
	return invoke<Commands[C]["result"]>(cmd, args);
}
