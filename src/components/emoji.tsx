import { cn } from "@/lib/utils";

/**
 * Emoji de **Noto** (SVG o, si pesaban mucho, PNG) en vez de la letra de emoji
 * del sistema: así salen iguales en Linux, Windows (Segoe UI Emoji no tiene
 * banderas) y macOS. Solo están los que usa la app: los copia
 * `scripts/emoji-noto.mjs` a `src/assets/emoji/`.
 */
const IMAGENES = import.meta.glob<string>("@/assets/emoji/*.{svg,png}", {
	eager: true,
	query: "?url",
	import: "default",
});

const POR_NOMBRE = new Map(
	Object.entries(IMAGENES).map(([ruta, url]) => [
		ruta.slice(ruta.lastIndexOf("/") + 1).replace(/\.(svg|png)$/, ""),
		url,
	]),
);

/** Los puntos de código de Noto (`emoji_u1f1ea_1f1f8`), sin el selector de variante. */
export function nombreNoto(emoji: string): string {
	return `emoji_u${[...emoji]
		.map((c) => c.codePointAt(0) ?? 0)
		.filter((p) => p !== 0xfe0f)
		.map((p) => p.toString(16).padStart(4, "0"))
		.join("_")}`;
}

/** La URL de la imagen de un emoji, si la app la trae. */
export function urlEmoji(emoji: string): string | undefined {
	return POR_NOMBRE.get(nombreNoto(emoji));
}

/** Dos letras de país (ISO 3166) → su bandera, como texto. */
export function banderaDe(pais: string): string {
	return [...pais.toUpperCase()]
		.map((c) => String.fromCodePoint(0x1f1e6 + c.charCodeAt(0) - 65))
		.join("");
}

/** Un emoji del tamaño del texto. Si no hay imagen para él, el carácter tal cual. */
export function Emoji({
	children,
	alt,
	className,
}: {
	children: string;
	alt?: string;
	className?: string;
}) {
	const url = urlEmoji(children);
	if (!url) return <span className={className}>{children}</span>;
	return (
		<img
			src={url}
			alt={alt ?? children}
			draggable={false}
			className={cn("inline-block size-[1.15em] shrink-0 align-[-0.2em]", className)}
		/>
	);
}

/** La bandera de un país (`pais="ES"`). */
export function Bandera({
	pais,
	alt,
	className,
}: {
	pais: string;
	alt?: string;
	className?: string;
}) {
	return (
		<Emoji alt={alt ?? pais} className={className}>
			{banderaDe(pais)}
		</Emoji>
	);
}
