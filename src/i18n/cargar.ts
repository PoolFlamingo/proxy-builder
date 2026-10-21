// Los diccionarios de la interfaz. El inglés va en el paquete principal (es el
// respaldo de todos); cada uno de los demás es un trozo aparte que se carga al
// elegirlo, para no meter los 14 en el arranque.

import type { Lang } from "@/lib/idiomas";
import { DICCIONARIO as EN } from "./en";
import type { Diccionario } from "./tipos";

type Modulo = { DICCIONARIO: Diccionario };

const PEREZOSOS = import.meta.glob<Modulo>(["./*/index.ts", "!./en/index.ts"]);

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
	const cargar = PEREZOSOS[`./${lang}/index.ts`];
	if (!cargar) return undefined;
	try {
		const { DICCIONARIO } = await cargar();
		cargados.set(lang, DICCIONARIO);
		return DICCIONARIO;
	} catch {
		return undefined;
	}
}
