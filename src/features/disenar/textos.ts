import type { Aviso } from "@/bindings/Aviso";
import type { DesignStyle } from "@/bindings/DesignStyle";
import type { Maquetacion } from "@/bindings/Maquetacion";
import type { OriginalName } from "@/bindings/OriginalName";
import type { Palette } from "@/bindings/Palette";
import { paraTraducir } from "@/components/language-provider";
import { idioma } from "@/lib/idiomas";

type T = (es: string, p?: Record<string, string | number>) => string;

export const ESTILOS: { valor: DesignStyle; etiqueta: string }[] = [
	{ valor: "borderless", etiqueta: paraTraducir("Despejado") },
	{ valor: "extended", etiqueta: paraTraducir("Enmarcado extendido") },
	{ valor: "classic", etiqueta: paraTraducir("Enmarcado clásico") },
	{ valor: "fullArt", etiqueta: paraTraducir("Paisaje (tierras básicas)") },
];

export const PALETAS: { valor: Palette; etiqueta: string; muestra: string }[] = [
	{ valor: "W", etiqueta: paraTraducir("Blanco"), muestra: "#e3dcc4" },
	{ valor: "U", etiqueta: paraTraducir("Azul"), muestra: "#1f6fb5" },
	{ valor: "B", etiqueta: paraTraducir("Negro"), muestra: "#4a4446" },
	{ valor: "R", etiqueta: paraTraducir("Rojo"), muestra: "#c0392b" },
	{ valor: "G", etiqueta: paraTraducir("Verde"), muestra: "#2f8a4a" },
	{ valor: "gold", etiqueta: paraTraducir("Oro (multicolor)"), muestra: "#d9ab3c" },
	{ valor: "artifact", etiqueta: paraTraducir("Artefacto"), muestra: "#a9b0b7" },
	{ valor: "land", etiqueta: paraTraducir("Tierra"), muestra: "#9a8461" },
	{ valor: "colorless", etiqueta: paraTraducir("Incoloro"), muestra: "#b5b5b5" },
];

export const NOMBRE_ORIGINAL: { valor: OriginalName; etiqueta: string }[] = [
	{ valor: "auto", etiqueta: paraTraducir("Lo que diga la colección") },
	{ valor: "ribbon", etiqueta: paraTraducir("En una cinta bajo el título") },
	{ valor: "footer", etiqueta: paraTraducir("En el pie") },
	{ valor: "hidden", etiqueta: paraTraducir("No se pone") },
];

export const RAREZAS: Record<string, string> = {
	common: paraTraducir("Común"),
	uncommon: paraTraducir("Infrecuente"),
	rare: paraTraducir("Rara"),
	mythic: paraTraducir("Mítica"),
	special: paraTraducir("Especial"),
	bonus: paraTraducir("Bonus"),
};

/** Un aviso del motor, en palabras. */
export function textoAviso(a: Aviso, t: T): string {
	switch (a.kind) {
		case "sinTextoOficial":
			return t(
				"No hay impresión de esta carta en el idioma de diseño: sus reglas son el texto en inglés. Puedes escribirlas a mano.",
			);
		case "textoNoCabe":
			return t("El texto de reglas no cabe ni con la letra más pequeña: sobran {mm} mm.", {
				mm: Math.round(a.sobra * 10) / 10,
			});
		case "tituloReducido":
			return a.hueco === "tipo"
				? t("La línea de tipo no cabía ni estrechada y se ha reducido la letra.")
				: t("El título no cabía ni estrechado y se ha reducido la letra.");
		case "reglasEditadas":
			// El idioma, por su nombre propio y entre paréntesis: así la frase
			// vale en cualquier idioma de la interfaz.
			return t("Reglas escritas a mano ({idioma}).", {
				idioma: idioma(a.lang).nativo,
			});
		case "faltaNombre":
			return t("Falta el nombre temático ({idioma}): se usa el original.", {
				idioma: idioma(a.lang).nativo,
			});
		case "letraNoEncontrada":
			return t("La letra «{familia}» no está: se usa la de la casa.", { familia: a.familia });
		case "faltanSimbolos":
			return t("Faltan símbolos de maná ({lista}). Descárgalos en «Letras, marcos y símbolos».", {
				lista: a.simbolos.join(", "),
			});
		case "estiloNoEnElPack":
			return t("El pack de marcos no tiene el estilo «{estilo}»: se usa su estilo de serie.", {
				estilo: a.estilo,
			});
		case "packSinLayout":
			return t("El pack de marcos no tiene diseño para «{layout}»: se usa el normal.", {
				layout: a.layout,
			});
		case "layoutDesconocido":
			return t("Tipo de carta que la app aún no sabe maquetar ({layout}): se usa el normal.", {
				layout: a.layout,
			});
		case "coleccionPerdida":
			return t("La colección de esta carta ya no existe: se usan los valores de fábrica.");
		case "sinCinta":
			return t("El marco de este estilo no tiene cinta: el nombre original va en el pie.");
		case "faltanLetras":
			return t("Faltan las letras de {idioma}: sin ellas, sus caracteres salen como cuadrados.", {
				idioma: idioma(a.lang).nativo,
			});
	}
}

/** Los avisos que impiden dar la carta por buena sin mirarla. */
export function esGrave(a: Aviso): boolean {
	return a.kind === "textoNoCabe" || a.kind === "faltanSimbolos" || a.kind === "faltanLetras";
}

/** Cómo se llama cada maquetación en el editor. */
export const MAQUETACIONES: Record<Maquetacion, string> = {
	normal: paraTraducir("Normal"),
	ficha: paraTraducir("Ficha"),
	planeswalker: paraTraducir("Planeswalker"),
	saga: paraTraducir("Saga"),
	clase: paraTraducir("Clase"),
	caso: paraTraducir("Caso"),
	partida: paraTraducir("Partida"),
	fusionar: paraTraducir("Partida con Fusionar"),
	secuela: paraTraducir("Secuela"),
	habitacion: paraTraducir("Habitación"),
	aventura: paraTraducir("Aventura"),
	flip: paraTraducir("Flip (Kamigawa)"),
	transformable: paraTraducir("Transformable (dos caras)"),
	mdfc: paraTraducir("Doble cara modal"),
	batalla: paraTraducir("Batalla (dos caras)"),
	meld: paraTraducir("Meld (dos caras)"),
	reversible: paraTraducir("Reversible (dos caras)"),
	fichaDosCaras: paraTraducir("Ficha de dos caras"),
	leveler: paraTraducir("Niveles (leveler)"),
	prototipo: paraTraducir("Prototipo"),
	mutar: paraTraducir("Mutar"),
	estacion: paraTraducir("Estación"),
	emblema: paraTraducir("Emblema"),
	mazmorra: paraTraducir("Mazmorra"),
	plano: paraTraducir("Plano (carta grande)"),
	plan: paraTraducir("Plan (carta grande)"),
	vanguard: paraTraducir("Vanguard (carta grande)"),
};

/** La letra de rareza que pone el motor si nadie la cambia (`efectivo::letra_rareza`). */
export function letraRareza(rareza: string): string {
	const letras: Record<string, string> = {
		common: "C",
		uncommon: "U",
		rare: "R",
		mythic: "M",
		bonus: "B",
	};
	return letras[rareza] ?? "S";
}
