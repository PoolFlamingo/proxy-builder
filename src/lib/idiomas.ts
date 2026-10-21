/**
 * Los idiomas de la app: los de la interfaz y los de las cartas diseñadas son
 * los mismos, con los códigos de Scryfall (`pt`, `zhs`, `zht`) para que el
 * idioma de la pantalla, el de diseño y el de un deck usen la misma clave. El
 * BCP-47 de cada uno va en `locale` (para `Intl` y `<html lang>`).
 *
 * El nombre de cada idioma va en su propio idioma y no se traduce: así se
 * reconoce en un selector.
 */

export const LANGS = [
	"es",
	"en",
	"fr",
	"de",
	"it",
	"pt",
	"ro",
	"pl",
	"nl",
	"ru",
	"uk",
	"ja",
	"ko",
	"zhs",
	"zht",
] as const;

export type Lang = (typeof LANGS)[number];

export type Idioma = {
	codigo: Lang;
	/** El nombre en su idioma. */
	nativo: string;
	/** Código ISO del país de su bandera. */
	pais: string;
	/** BCP-47, para `Intl` y `<html lang>`. */
	locale: string;
	/** Si Scryfall tiene cartas impresas en este idioma. */
	impresas: boolean;
};

export const IDIOMAS: readonly Idioma[] = [
	{ codigo: "es", nativo: "Español", pais: "ES", locale: "es-ES", impresas: true },
	{ codigo: "en", nativo: "English", pais: "GB", locale: "en-GB", impresas: true },
	{ codigo: "fr", nativo: "Français", pais: "FR", locale: "fr-FR", impresas: true },
	{ codigo: "de", nativo: "Deutsch", pais: "DE", locale: "de-DE", impresas: true },
	{ codigo: "it", nativo: "Italiano", pais: "IT", locale: "it-IT", impresas: true },
	{ codigo: "pt", nativo: "Português (Brasil)", pais: "BR", locale: "pt-BR", impresas: true },
	{ codigo: "ro", nativo: "Română", pais: "RO", locale: "ro-RO", impresas: false },
	{ codigo: "pl", nativo: "Polski", pais: "PL", locale: "pl-PL", impresas: false },
	{ codigo: "nl", nativo: "Nederlands", pais: "NL", locale: "nl-NL", impresas: false },
	{ codigo: "ru", nativo: "Русский", pais: "RU", locale: "ru-RU", impresas: true },
	{ codigo: "uk", nativo: "Українська", pais: "UA", locale: "uk-UA", impresas: false },
	{ codigo: "ja", nativo: "日本語", pais: "JP", locale: "ja-JP", impresas: true },
	{ codigo: "ko", nativo: "한국어", pais: "KR", locale: "ko-KR", impresas: true },
	{ codigo: "zhs", nativo: "简体中文", pais: "CN", locale: "zh-Hans-CN", impresas: true },
	{ codigo: "zht", nativo: "繁體中文", pais: "TW", locale: "zh-Hant-TW", impresas: true },
];

const POR_CODIGO = new Map<string, Idioma>(IDIOMAS.map((i) => [i.codigo, i]));

export function esLang(x: unknown): x is Lang {
	return typeof x === "string" && POR_CODIGO.has(x);
}

/** El idioma de un código; uno desconocido, el inglés. */
export function idioma(codigo: string): Idioma {
	return POR_CODIGO.get(codigo) ?? (POR_CODIGO.get("en") as Idioma);
}

export function localeDe(lang: Lang): string {
	return idioma(lang).locale;
}

/**
 * Los idiomas con cartas en Scryfall, en el orden de sus listas (el inglés
 * primero). El nombre de la variante portuguesa es el corto: en las cartas es
 * «Português».
 */
export const IDIOMAS_SCRYFALL: readonly Idioma[] = (
	["en", "es", "fr", "de", "it", "pt", "ja", "ko", "ru", "zhs", "zht"] as const
).map((c) => (c === "pt" ? { ...idioma(c), nativo: "Português" } : idioma(c)));
