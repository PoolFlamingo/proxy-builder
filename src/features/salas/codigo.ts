/**
 * **Los códigos que se pegan.** Una invitación llega como código suelto o como
 * enlace entero (`proxybuilder://unirse/<código>`, lo que manda «Compartir»); los
 * campos de «Unirse con un código» aceptan los dos. Rust sigue validando el código
 * (`enlace.rs`): esto solo quita lo que sobra para que el campo no lo rechace.
 */

/** El código de partida directa tal como se pega: sin espacios ni el principio del enlace. */
export function limpiarCodigoDirecto(texto: string): string {
	const sin = texto.replace(/\s+/g, "").replace(/^proxybuilder:\/\/unirse\//i, "");
	// El prefijo va en mayúsculas; el resto es base64 y distingue mayúsculas: no se toca.
	return sin.replace(/^pb1-/i, "PB1-");
}

/**
 * El código de sala o de playgroup (10 caracteres, con o sin guion) de lo que
 * se pega: si es un enlace `proxybuilder://unirse/…`, el código que lleva; si no,
 * el mismo texto.
 */
export function codigoDeSala(texto: string): string {
	const t = texto.trim();
	const m = /^proxybuilder:\/\/unirse\/([^/?#\s]+)/i.exec(t);
	return m?.[1] ?? t;
}

/** Un código de 10 caracteres en el formato que se enseña: `XXXXX-XXXXX`. */
export function codigoBonito(codigo: string): string {
	const limpio = codigo.replace(/[\s-]/g, "");
	return limpio.length === 10 ? `${limpio.slice(0, 5)}-${limpio.slice(5)}` : codigo;
}

/** ¿Hay 10 caracteres de código (sin contar espacios ni guiones)? */
export function codigoDeSalaCompleto(texto: string): boolean {
	return codigoDeSala(texto).replace(/[\s-]/g, "").length === 10;
}

/** Lo que se manda al compartir una invitación: el enlace y, debajo, el código por si el enlace no se abre. */
export function textoParaCompartir(enlace: string | null, codigo: string): string {
	return enlace ? `${enlace}\n${codigo}` : codigo;
}
