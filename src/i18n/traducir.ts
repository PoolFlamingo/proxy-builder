// Traducir una cadena: funciones puras, sin React, para poder probarlas solas.

import { type Lang, localeDe } from "@/lib/idiomas";
import type { Diccionario, Plural, Traduccion } from "./tipos";

export type Vars = Record<string, string | number>;

const reglas = new Map<string, Intl.PluralRules>();
const numeros = new Map<string, Intl.NumberFormat>();

function pluralDe(locale: string) {
	let r = reglas.get(locale);
	if (!r) {
		r = new Intl.PluralRules(locale);
		reglas.set(locale, r);
	}
	return r;
}

function numeroDe(locale: string) {
	let f = numeros.get(locale);
	if (!f) {
		f = new Intl.NumberFormat(locale);
		numeros.set(locale, f);
	}
	return f;
}

/** La forma de un plural para `n`; sin la suya, `other`. */
function forma(p: Plural, n: unknown, locale: string): string {
	if (typeof n !== "number" || !Number.isFinite(n)) return p.other;
	return p[pluralDe(locale).select(n)] ?? p.other;
}

/** Cambia cada `{x}` por su valor; los números, con el formato del idioma. */
export function interpolar(s: string, vars: Vars | undefined, locale: string): string {
	if (!vars) return s;
	return s.replace(/\{(\w+)\}/g, (_, k) => {
		const v = vars[k];
		if (v == null) return `{${k}}`;
		return typeof v === "number" ? numeroDe(locale).format(v) : v;
	});
}

/**
 * La cadena `es` en `lang`: del diccionario de ese idioma; si le falta, del
 * inglés (quien usa la app en polaco entiende mejor el inglés que el
 * español); si también falta, el español tal cual.
 */
export function traducir(
	es: string,
	lang: Lang,
	vars: Vars | undefined,
	dic: Diccionario | undefined,
	en: Diccionario,
): string {
	let valor: Traduccion | undefined;
	let locale = localeDe(lang);
	if (lang === "es") valor = es;
	else {
		valor = dic?.[es];
		if (valor === undefined) {
			valor = en[es];
			locale = localeDe("en");
		}
		if (valor === undefined) {
			valor = es;
			locale = localeDe("es");
		}
	}
	const texto = typeof valor === "string" ? valor : forma(valor, vars?.n, locale);
	// Los números siempre con el formato del idioma de la interfaz.
	return interpolar(texto, vars, localeDe(lang));
}
