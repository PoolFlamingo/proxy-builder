import { convertFileSrc } from "@tauri-apps/api/core";
import type { CardEdits } from "@/bindings/CardEdits";
import type { PreviewParams } from "@/bindings/PreviewParams";
import type { PrintSettings } from "@/bindings/PrintSettings";

/**
 * Las URL del protocolo de imágenes de Rust (`pbimg://`, `src-tauri/src/protocol.rs`).
 * `convertFileSrc("", esquema)` da la base correcta en cada plataforma
 * (`pbimg://localhost/` en Linux y macOS, `http://pbimg.localhost/` en Windows),
 * el mismo truco que `media-url.ts` de judlex.
 */
const ESQUEMA = "pbimg";

let base: string | null = null;
function raiz(): string {
	if (base === null) {
		try {
			base = convertFileSrc("", ESQUEMA);
		} catch {
			base = `${ESQUEMA}://localhost/`;
		}
		if (!base.endsWith("/")) base += "/";
	}
	return base;
}

/** La raíz del protocolo (para las `@font-face` de los temas instalados). */
export function raizProtocolo(): string {
	return raiz();
}

/** base64url sin relleno de un texto ASCII (el JSON de los parámetros). */
function b64url(texto: string): string {
	const bytes = new TextEncoder().encode(texto);
	let bin = "";
	for (const b of bytes) bin += String.fromCharCode(b);
	return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** Anchos que acepta el protocolo (el del trim, sin sangrado). */
export const ANCHOS = [146, 244, 488, 672, 745, 1000] as const;
export type AnchoMiniatura = (typeof ANCHOS)[number];

export const EDICION_NEUTRA: CardEdits = {
	rotation: "r0",
	crop: { zoom: 1, offsetX: 0, offsetY: 0 },
	adjust: {
		brightness: 0,
		exposure: 0,
		contrast: 0,
		saturation: 0,
		warmth: 0,
		vibrance: 0,
		sharpness: 0,
		clarity: 0,
		noiseReduction: 0,
		vignette: 0,
		fade: 0,
	},
};

/** Parámetros de una miniatura a partir de los ajustes del deck. */
export function parametros(
	ajustes: PrintSettings,
	edicion: CardEdits | null,
	dorso: boolean,
	sangradoMm: number,
): PreviewParams {
	return {
		edits: edicion ?? EDICION_NEUTRA,
		edge: ajustes.edgeDarkening,
		back: dorso,
		bleedMm: sangradoMm,
		bleedMode: ajustes.bleedMode,
		fillCorners: ajustes.fillCorners,
		card: ajustes.cardSize,
	};
}

/** URL de la miniatura procesada de un blob. */
export function urlMiniatura(blob: string, p: PreviewParams, ancho: AnchoMiniatura): string {
	return `${raiz()}r/${blob}/${b64url(JSON.stringify(p))}/${ancho}`;
}

/** URL de la miniatura de un resultado de búsqueda (la descarga Rust). */
export function urlBusqueda(huella: string): string {
	return `${raiz()}t/${huella}`;
}

/** La vista previa de un diseño registrado en Rust (`design_register`). */
export function urlDiseno(huella: string, ancho: AnchoMiniatura): string {
	return `${raiz()}d/${huella}/${ancho}`;
}

/** Ese diseño sin la ilustración (`design_overlay`). */
export function urlCapa(huella: string, ancho: AnchoMiniatura): string {
	return `${raiz()}c/${huella}/${ancho}`;
}

/** Lados de la ilustración reducida que acepta el protocolo. */
export type LadoArte = 1000 | 2000;

/** Una ilustración del almacén reducida a `lado` px (el lado más largo). */
export function urlArte(blob: string, lado: LadoArte): string {
	return `${raiz()}b/${blob}/${lado}`;
}

/** Una imagen extra de un pack de marcos descargable (reverso, carta hecha…). */
export function urlExtra(pack: string, indice: number): string {
	return `${raiz()}x/${encodeURIComponent(pack)}/${indice}`;
}
