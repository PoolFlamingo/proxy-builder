/**
 * El nombre de un deck como se enseña. Un deck nuevo nace con «Deck sin
 * título» en el idioma de la interfaz de ese momento; si luego se cambia de
 * idioma (la app arranca en inglés la primera vez), seguiría diciendo
 * «Untitled deck». Mientras no se renombre, se enseña en el idioma de ahora.
 * Solo es de pantalla: lo guardado no cambia.
 */

/** «Deck sin título» en los 15 idiomas (lo comprueba un test contra los diccionarios). */
export const NOMBRES_SIN_TITULO: ReadonlySet<string> = new Set([
	"Deck sin título",
	"Untitled deck",
	"Deck sans titre",
	"Unbenanntes Deck",
	"Mazzo senza titolo",
	"Deck sem título",
	"Pachet fără titlu",
	"Talia bez nazwy",
	"Deck zonder titel",
	"Колода без названия",
	"Колода без назви",
	"無題のデッキ",
	"제목 없는 덱",
	"未命名套牌",
]);

/** `nombre`, o `sinTitulo` (el de ahora) si es el de fábrica en cualquier idioma. */
export function nombreVisible(nombre: string, sinTitulo: string): string {
	return NOMBRES_SIN_TITULO.has(nombre.trim()) ? sinTitulo : nombre;
}
