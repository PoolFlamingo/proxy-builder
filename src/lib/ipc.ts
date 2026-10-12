import { invoke } from "@tauri-apps/api/core";

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
}

export function call<C extends keyof Commands>(
	cmd: C,
	args?: Commands[C]["args"],
): Promise<Commands[C]["result"]> {
	return invoke<Commands[C]["result"]>(cmd, args);
}
