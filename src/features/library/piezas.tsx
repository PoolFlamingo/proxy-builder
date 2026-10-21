import { Lock } from "lucide-react";
import type { DeckDetails } from "@/bindings/DeckDetails";
import type { DeckKind } from "@/bindings/DeckKind";
import type { DeckSummary } from "@/bindings/DeckSummary";
import type { Tag } from "@/bindings/Tag";
import { paraTraducir, useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { COLORES } from "@/features/stats/textos";
import { IDIOMAS_SCRYFALL, localeDe } from "@/lib/idiomas";
import { cn } from "@/lib/utils";
import { idiomasDe, WUBRG } from "./organizar";

/** Los colores de etiqueta (los mismos que acepta Rust). */
export const COLORES_ETIQUETA: Record<string, { punto: string; chip: string; nombre: string }> = {
	gray: {
		punto: "bg-zinc-400",
		chip: "bg-zinc-500/15 text-zinc-700 ring-zinc-500/30 dark:text-zinc-300",
		nombre: paraTraducir("Gris"),
	},
	red: {
		punto: "bg-red-500",
		chip: "bg-red-500/15 text-red-700 ring-red-500/30 dark:text-red-300",
		nombre: paraTraducir("Rojo"),
	},
	orange: {
		punto: "bg-orange-500",
		chip: "bg-orange-500/15 text-orange-700 ring-orange-500/30 dark:text-orange-300",
		nombre: paraTraducir("Naranja"),
	},
	amber: {
		punto: "bg-amber-400",
		chip: "bg-amber-400/20 text-amber-800 ring-amber-500/30 dark:text-amber-200",
		nombre: paraTraducir("Ámbar"),
	},
	green: {
		punto: "bg-green-500",
		chip: "bg-green-500/15 text-green-700 ring-green-500/30 dark:text-green-300",
		nombre: paraTraducir("Verde"),
	},
	teal: {
		punto: "bg-teal-500",
		chip: "bg-teal-500/15 text-teal-700 ring-teal-500/30 dark:text-teal-300",
		nombre: paraTraducir("Turquesa"),
	},
	sky: {
		punto: "bg-sky-500",
		chip: "bg-sky-500/15 text-sky-700 ring-sky-500/30 dark:text-sky-300",
		nombre: paraTraducir("Celeste"),
	},
	blue: {
		punto: "bg-blue-600",
		chip: "bg-blue-600/15 text-blue-700 ring-blue-600/30 dark:text-blue-300",
		nombre: paraTraducir("Azul"),
	},
	violet: {
		punto: "bg-violet-500",
		chip: "bg-violet-500/15 text-violet-700 ring-violet-500/30 dark:text-violet-300",
		nombre: paraTraducir("Violeta"),
	},
	pink: {
		punto: "bg-pink-500",
		chip: "bg-pink-500/15 text-pink-700 ring-pink-500/30 dark:text-pink-300",
		nombre: paraTraducir("Rosa"),
	},
};

export const COLORES_DE_ETIQUETA = Object.keys(COLORES_ETIQUETA);

export function PuntoEtiqueta({ color, className }: { color: string; className?: string }) {
	return (
		<span
			className={cn(
				"inline-block size-2 shrink-0 rounded-full",
				(COLORES_ETIQUETA[color] ?? COLORES_ETIQUETA.gray).punto,
				className,
			)}
		/>
	);
}

export function ChipEtiqueta({ tag, className }: { tag: Tag; className?: string }) {
	return (
		<span
			className={cn(
				"inline-flex max-w-full items-center gap-1 truncate rounded-full px-1.5 py-px text-[10px] leading-4 font-medium ring-1 ring-inset",
				(COLORES_ETIQUETA[tag.color] ?? COLORES_ETIQUETA.gray).chip,
				className,
			)}
		>
			<span className="truncate">{tag.name}</span>
		</span>
	);
}

/** Las etiquetas de un deck, hasta `max` (y «+n»). */
export function ChipsEtiquetas({
	ids,
	etiquetas,
	max = 3,
	className,
}: {
	ids: string[];
	etiquetas: Tag[];
	max?: number;
	className?: string;
}) {
	const suyas = etiquetas.filter((t) => ids.includes(t.id));
	if (suyas.length === 0) return null;
	const resto = suyas.length - max;
	return (
		<div className={cn("flex min-w-0 flex-wrap gap-1", className)}>
			{suyas.slice(0, max).map((t) => (
				<ChipEtiqueta key={t.id} tag={t} />
			))}
			{resto > 0 && (
				<WithTooltip
					content={suyas
						.slice(max)
						.map((t) => t.name)
						.join(", ")}
				>
					<span className="rounded-full px-1 text-[10px] leading-4 text-muted-foreground ring-1 ring-border ring-inset">
						+{resto}
					</span>
				</WithTooltip>
			)}
		</div>
	);
}

/** Un círculo de maná con su letra (no son los símbolos del juego). */
export function Pip({ color, className }: { color: string; className?: string }) {
	const { t } = useLang();
	const c = COLORES[color];
	return (
		<WithTooltip content={t(c?.nombre ?? color)}>
			<span
				className={cn(
					"inline-flex size-4 shrink-0 items-center justify-center rounded-full font-mono text-[9px] leading-none font-bold shadow-sm ring-1 ring-black/15 dark:ring-white/30",
					c?.chip ?? "bg-muted",
					className,
				)}
			>
				{color}
			</span>
		</WithTooltip>
	);
}

/** La identidad de color de un deck (incolora: «C»). */
export function Identidad({ colores, className }: { colores: string[]; className?: string }) {
	const orden = [...colores].sort(
		(a, b) =>
			WUBRG.indexOf(a as (typeof WUBRG)[number]) - WUBRG.indexOf(b as (typeof WUBRG)[number]),
	);
	return (
		<span className={cn("inline-flex items-center gap-0.5", className)}>
			{(orden.length > 0 ? orden : ["C"]).map((c) => (
				<Pip key={c} color={c} />
			))}
		</span>
	);
}

export const CLASES: Record<DeckKind, string> = {
	commander: paraTraducir("Commander"),
	constructed: paraTraducir("Construido"),
	limited: paraTraducir("Limitado"),
};

/**
 * Nombre del idioma de las cartas: el suyo propio (no se traduce), salvo el
 * phyrexiano, que no es de la tabla de la app.
 */
export const IDIOMAS: Record<string, string> = {
	...Object.fromEntries(IDIOMAS_SCRYFALL.map((i) => [i.codigo, i.nativo])),
	ph: paraTraducir("Phyrexiano"),
};

/** Los idiomas de un deck por su nombre; con un candado si se eligió a mano. */
export function NombresIdiomas({ deck, det }: { deck: DeckSummary; det?: DeckDetails }) {
	const { t } = useLang();
	const texto = idiomasDe(deck, det)
		.map((l) => t(IDIOMAS[l] ?? l.toUpperCase()))
		.join(", ");
	return (
		<span className="inline-flex min-w-0 items-center justify-end gap-1">
			<span className="truncate">{texto || "—"}</span>
			{deck.langOverride && (
				<WithTooltip content={t("Elegido a mano")}>
					<Lock className="size-3 shrink-0 text-muted-foreground" />
				</WithTooltip>
			)}
		</span>
	);
}

/** Los formatos que se pueden filtrar (los de las estadísticas). */
export const FORMATOS: Record<string, string> = {
	standard: "Standard",
	pioneer: "Pioneer",
	modern: "Modern",
	legacy: "Legacy",
	vintage: "Vintage",
	commander: "Commander",
	pauper: "Pauper",
	historic: "Historic",
	brawl: "Brawl",
};

/** Una fecha corta según el idioma de la interfaz. */
export function useFecha() {
	const { lang } = useLang();
	return (ms: number | null, conHora = true) =>
		ms === null
			? "—"
			: new Date(ms).toLocaleString(localeDe(lang), {
					dateStyle: "medium",
					...(conHora ? { timeStyle: "short" as const } : {}),
				});
}

/** «hace 3 días», «hace 2 h»… */
export function useHace() {
	const { lang } = useLang();
	const rtf = new Intl.RelativeTimeFormat(localeDe(lang), { numeric: "auto" });
	return (ms: number | null) => {
		if (ms === null) return "—";
		const s = (ms - Date.now()) / 1000;
		const a = Math.abs(s);
		if (a < 60) return rtf.format(Math.round(s), "second");
		if (a < 3600) return rtf.format(Math.round(s / 60), "minute");
		if (a < 86400) return rtf.format(Math.round(s / 3600), "hour");
		if (a < 86400 * 30) return rtf.format(Math.round(s / 86400), "day");
		if (a < 86400 * 365) return rtf.format(Math.round(s / (86400 * 30)), "month");
		return rtf.format(Math.round(s / (86400 * 365)), "year");
	};
}

/** «€1.234,56» / «$80.00». */
export function useDinero() {
	const { lang } = useLang();
	return (v: number | null, moneda: "EUR" | "USD" = "EUR") =>
		v === null
			? "—"
			: new Intl.NumberFormat(localeDe(lang), {
					style: "currency",
					currency: moneda,
					maximumFractionDigits: v >= 100 ? 0 : 2,
				}).format(v);
}

const NOMBRES_COLORES: Record<string, string> = {
	"": paraTraducir("Incoloro"),
	W: paraTraducir("Blanco"),
	U: paraTraducir("Azul"),
	B: paraTraducir("Negro"),
	R: paraTraducir("Rojo"),
	G: paraTraducir("Verde"),
	WU: "Azorius",
	UB: "Dimir",
	BR: "Rakdos",
	RG: "Gruul",
	WG: "Selesnya",
	WB: "Orzhov",
	UR: "Izzet",
	BG: "Golgari",
	WR: "Boros",
	UG: "Simic",
	WUB: "Esper",
	UBR: "Grixis",
	BRG: "Jund",
	WRG: "Naya",
	WUG: "Bant",
	WBG: "Abzan",
	WUR: "Jeskai",
	UBG: "Sultai",
	WBR: "Mardu",
	URG: "Temur",
	WUBRG: paraTraducir("Cinco colores"),
};

/** El nombre de una identidad de color («Azorius», «Esper»…), sin traducir. */
export function nombreColores(colores: string[]): string {
	const clave = WUBRG.filter((c) => colores.includes(c)).join("");
	return NOMBRES_COLORES[clave] ?? paraTraducir("Cuatro colores");
}
