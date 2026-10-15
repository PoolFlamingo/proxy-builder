import { createContext, type ReactNode, useContext, useEffect, useMemo, useState } from "react";

import { EN } from "@/i18n/en";
import { mensajeDeError } from "@/lib/errores";

/**
 * Idioma de la interfaz (patrón de judlex).
 *
 * El diccionario se indexa por la **cadena fuente en español**: el código se
 * escribe en español y el inglés es una traducción de esa cadena. Si falta una
 * entrada, se devuelve el español tal cual; nunca queda un hueco vacío ni una
 * clave técnica a la vista. `pnpm idiomas` comprueba que no falte ninguna.
 *
 * Si la misma palabra española significa dos cosas en sitios distintos, se
 * desambigua ALARGANDO la cadena fuente del caso menos genérico, nunca con un
 * segundo diccionario.
 */
export type Lang = "es" | "en";

const LANG_KEY = "proxybuilder.lang";

/**
 * Marca una cadena que se traduce MÁS TARDE, en otro sitio, con `t()`. En
 * ejecución no hace nada; existe para que `scripts/idiomas.mjs` la vea cuando
 * el texto vive en una tabla de datos y se traduce lejos de donde se escribe.
 */
export function paraTraducir<T extends string>(es: T): T {
	return es;
}

/** El idioma elegido; sin elegir, inglés. */
function leerIdioma(): Lang {
	try {
		return localStorage.getItem(LANG_KEY) === "es" ? "es" : "en";
	} catch {
		return "en";
	}
}

/**
 * `t()` para lo poquísimo que se pinta FUERA del proveedor: hoy solo
 * `LimiteError`, que tiene que envolverlo para que un fallo dentro del propio
 * proveedor no deje la ventana en blanco.
 */
export function traduccionSuelta(es: string): string {
	return traducir(es, leerIdioma());
}

function traducir(es: string, lang: Lang): string {
	return lang === "en" ? (EN[es] ?? es) : es;
}

function interpolar(s: string, vars?: Record<string, string | number>) {
	if (!vars) return s;
	return s.replace(/\{(\w+)\}/g, (_, k) => (vars[k] != null ? String(vars[k]) : `{${k}}`));
}

type Ctx = {
	lang: Lang;
	setLang: (l: Lang) => void;
	t: (es: string, vars?: Record<string, string | number>) => string;
	/** Un error de Rust (o cualquiera) en el idioma de la interfaz. */
	mensaje: (e: unknown) => string;
};

const LanguageContext = createContext<Ctx | null>(null);

export function LanguageProvider({ children }: { children: ReactNode }) {
	const [lang, setLangState] = useState<Lang>(leerIdioma);

	useEffect(() => {
		document.documentElement.setAttribute("lang", lang);
	}, [lang]);

	const value = useMemo<Ctx>(() => {
		const t = (es: string, vars?: Record<string, string | number>) =>
			interpolar(traducir(es, lang), vars);
		return {
			lang,
			setLang: (l) => {
				setLangState(l);
				try {
					localStorage.setItem(LANG_KEY, l);
				} catch {
					/* sin almacenamiento: el idioma vive solo en esta sesión */
				}
			},
			t,
			mensaje: (e) => mensajeDeError(e, t),
		};
	}, [lang]);

	return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLang(): Ctx {
	const ctx = useContext(LanguageContext);
	if (!ctx) throw new Error("useLang debe usarse dentro de <LanguageProvider>");
	return ctx;
}
