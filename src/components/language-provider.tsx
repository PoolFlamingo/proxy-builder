import {
	createContext,
	type ReactNode,
	useContext,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";

import { cargarDiccionario, diccionarioCargado, EN } from "@/i18n/cargar";
import { traducir, type Vars } from "@/i18n/traducir";
import { mensajeDeError } from "@/lib/errores";
import { esLang, type Lang, localeDe } from "@/lib/idiomas";

/**
 * Idioma de la interfaz (patrón de judlex).
 *
 * El diccionario se indexa por la **cadena fuente en español**: el código se
 * escribe en español y cada idioma es una traducción de esa cadena. Si a un
 * idioma le falta una entrada, sale la inglesa, y si tampoco está, el
 * español tal cual; nunca queda un hueco vacío ni una clave técnica a la
 * vista. `pnpm idiomas` comprueba que no falte ninguna.
 *
 * Un valor puede ser un plural (`{ one, few, many, other }`): la forma la
 * elige la variable `n`, que debe ser un número.
 *
 * Si la misma palabra española significa dos cosas en sitios distintos, se
 * desambigua ALARGANDO la cadena fuente del caso menos genérico, nunca con un
 * segundo diccionario.
 */
export type { Lang };

const LANG_KEY = "proxybuilder.lang";

/**
 * Marca una cadena que se traduce MÁS TARDE, en otro sitio, con `t()`. En
 * ejecución no hace nada; existe para que `scripts/idiomas.mjs` la vea cuando
 * el texto vive en una tabla de datos y se traduce lejos de donde se escribe.
 */
export function paraTraducir<T extends string>(es: T): T {
	return es;
}

/** El idioma elegido; sin elegir (o uno que ya no existe), inglés. */
export function leerIdioma(): Lang {
	try {
		const l = localStorage.getItem(LANG_KEY);
		return esLang(l) ? l : "en";
	} catch {
		return "en";
	}
}

/**
 * `t()` para lo poquísimo que se pinta FUERA del proveedor: hoy solo
 * `LimiteError`, que tiene que envolverlo para que un fallo dentro del propio
 * proveedor no deje la ventana en blanco.
 */
export function traduccionSuelta(es: string, vars?: Vars): string {
	const lang = leerIdioma();
	return traducir(es, lang, vars, diccionarioCargado(lang), EN);
}

type Ctx = {
	lang: Lang;
	setLang: (l: Lang) => void;
	t: (es: string, vars?: Vars) => string;
	/** Un error de Rust (o cualquiera) en el idioma de la interfaz. */
	mensaje: (e: unknown) => string;
};

const LanguageContext = createContext<Ctx | null>(null);

export function LanguageProvider({ children }: { children: ReactNode }) {
	const [lang, setLangState] = useState<Lang>(leerIdioma);
	// El último idioma pedido: si se piden dos seguidos, gana el último aunque
	// su diccionario llegue antes.
	const pedido = useRef(lang);

	useEffect(() => {
		// El locale entero y no el código: así WebKit elige bien las letras
		// japonesas o chinas para los ideogramas que comparten.
		document.documentElement.setAttribute("lang", localeDe(lang));
	}, [lang]);

	const value = useMemo<Ctx>(() => {
		const dic = diccionarioCargado(lang);
		const t = (es: string, vars?: Vars) => traducir(es, lang, vars, dic, EN);
		return {
			lang,
			setLang: (l) => {
				pedido.current = l;
				try {
					localStorage.setItem(LANG_KEY, l);
				} catch {
					/* sin almacenamiento: el idioma vive solo en esta sesión */
				}
				// Primero el diccionario y luego el cambio: mientras carga se
				// queda el idioma de antes, sin parpadeo de textos a medias.
				void cargarDiccionario(l).then(() => {
					if (pedido.current === l) setLangState(l);
				});
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
