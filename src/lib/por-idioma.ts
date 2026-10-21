import type { ByLang } from "@/bindings/ByLang";
import type { CardData } from "@/bindings/CardData";
import type { DesignLang } from "@/bindings/DesignLang";
import type { FaceTexts } from "@/bindings/FaceTexts";

/**
 * Los valores por idioma de un diseño (`ByLang` de Rust). `ts-rs` solo ve
 * `es` y `en` (los que siempre están); los demás idiomas van en la misma
 * forma, `{ es, en, fr?, … }`, y se leen y escriben con estas funciones.
 */
type PorIdioma<T> = ByLang<T> & Partial<Record<DesignLang, T>>;

/** Si `lang` tiene valor propio (`es` y `en` siempre lo tienen). */
export function tieneIdioma<T>(v: ByLang<T>, lang: DesignLang): boolean {
	return (v as PorIdioma<T>)[lang] !== undefined;
}

/** El valor de `lang`; si no tiene, el inglés (como `ByLang::get` en Rust). */
export function porIdioma<T>(v: ByLang<T>, lang: DesignLang): T {
	return (v as PorIdioma<T>)[lang] ?? v.en;
}

/** El mismo `ByLang` con `lang` cambiado. */
export function conIdioma<T>(v: ByLang<T>, lang: DesignLang, valor: T): ByLang<T> {
	return { ...v, [lang]: valor } as ByLang<T>;
}

const SIN_TEXTOS: FaceTexts = { name: "", flavor: "" };

/**
 * Los textos de una cara en `lang`. Como `FaceDesign::textos` en Rust: un
 * idioma que no es el español ni el inglés no toma nada del inglés.
 */
export function textosDe(v: ByLang<FaceTexts>, lang: DesignLang): FaceTexts {
	return (v as PorIdioma<FaceTexts>)[lang] ?? SIN_TEXTOS;
}

/** Si el texto de `lang` es el Oracle en inglés (como `CardData::sin_texto_en`). */
export function sinTextoEn(c: CardData, lang: DesignLang): boolean {
	if (lang === "es") return c.sinTextoEs;
	if (lang === "en") return false;
	return (c.sinTexto ?? []).includes(lang) || c.faces.some((f) => !tieneIdioma(f.rules, lang));
}
