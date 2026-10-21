/**
 * Un plural: una forma por cada categoría de `Intl.PluralRules` del idioma
 * (`pnpm idiomas` comprueba que estén todas). Se elige con la variable `n`.
 */
export type Plural = {
	zero?: string;
	one?: string;
	two?: string;
	few?: string;
	many?: string;
	other: string;
};

export type Traduccion = string | Plural;

/** Un diccionario, indexado por la cadena fuente en español. */
export type Diccionario = Record<string, Traduccion>;
