// Los diccionarios de la interfaz. Cada idioma es una carpeta (`src/i18n/<código>/`)
// con un JSON por área de la app, y aquí se funden. El inglés va en el paquete
// principal (es el respaldo de todos); cada uno de los demás es un trozo aparte
// que se carga al elegirlo, para no meter los 14 en el arranque.

import type { Lang } from "@/lib/idiomas";
import type { Diccionario } from "./tipos";

const AREAS_EN = import.meta.glob<Diccionario>("./en/*.json", { eager: true, import: "default" });
// El español (`es/`) no se carga: es la fuente, y `pnpm idiomas` la comprueba.
const AREAS_PEREZOSAS = import.meta.glob<Diccionario>(
	["./*/*.json", "!./en/*.json", "!./es/*.json", "!./*/identicas.json"],
	{ import: "default" },
);

/** Une las áreas de un idioma. Una cadena en dos áreas gana la última; `pnpm idiomas` lo impide. */
function fundir(areas: Diccionario[]): Diccionario {
	return Object.assign({}, ...areas);
}

const EN = fundir(Object.values(AREAS_EN));

const cargados = new Map<Lang, Diccionario>([["en", EN]]);

export { EN };

/** El diccionario de `lang`, si ya está cargado (el español no tiene: es la fuente). */
export function diccionarioCargado(lang: Lang): Diccionario | undefined {
	return cargados.get(lang);
}

/**
 * Carga el diccionario de `lang` (una sola vez). Un idioma sin diccionario, o
 * uno que no se pueda cargar, se queda sin él: `traducir` cae al inglés.
 */
export async function cargarDiccionario(lang: Lang): Promise<Diccionario | undefined> {
	const hecho = cargados.get(lang);
	if (hecho || lang === "es") return hecho;
	const cargar = Object.entries(AREAS_PEREZOSAS)
		.filter(([ruta]) => ruta.startsWith(`./${lang}/`))
		.map(([, area]) => area());
	if (cargar.length === 0) return undefined;
	try {
		const dic = fundir(await Promise.all(cargar));
		cargados.set(lang, dic);
		return dic;
	} catch {
		return undefined;
	}
}
