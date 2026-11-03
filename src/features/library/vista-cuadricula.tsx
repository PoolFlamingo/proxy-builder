import { createContext, useContext, useState } from "react";
import type { DeckSummary } from "@/bindings/DeckSummary";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useVisible } from "@/features/preview/visibilidad";
import { useFormato } from "@/lib/formato";
import { EDICION_NEUTRA, urlMiniatura } from "@/lib/image-url";
import { usePulsacionLarga } from "@/lib/pulsacion-larga";
import { useNombreDeck } from "@/lib/use-nombre-deck";
import { cn } from "@/lib/utils";
import { clasesCuadricula } from "./disposicion";
import { BotonMenuDeck, MenuContextualDeck } from "./menu-deck";
import { ChipsEtiquetas, CLASES, Identidad, useHace } from "./piezas";
import { usePrefsSel } from "./preferencias";
import { useBiblioteca } from "./store";

/** Parámetros neutros para la miniatura de la portada de un deck. */
export const PORTADA = {
	edits: EDICION_NEUTRA,
	edge: {
		enabled: false,
		mode: "contrastEdges" as const,
		amount: 0,
		widthPct: 0,
		autoDetect: false,
		applyScryfall: false,
		applyUploads: false,
		applyBacks: false,
	},
	back: false,
	bleedMm: 0,
	bleedMode: "clamp" as const,
	fillCorners: false,
	card: { widthMm: 63, heightMm: 88 },
};

/** La selección y qué hacer al pulsar un deck: lo pone la pantalla. */
export type Seleccion = {
	seleccion: string[];
	/** El deck abierto ahora mismo en la vista Deck. */
	abierto: string | null;
	/** Pulsar: con Ctrl o Mayús selecciona; si no, abre. */
	pulsar: (id: string, e: React.MouseEvent) => void;
	/** La casilla: quita o pone (con Mayús, el tramo desde la última). */
	alternar: (id: string, e?: React.MouseEvent) => void;
	/** Marca (o desmarca) varios a la vez. */
	marcar: (ids: string[], si: boolean) => void;
	abrir: (id: string) => void;
	/**
	 * Con el dedo, mantener pulsado un deck entra en el modo selección con él
	 * elegido (los toques siguientes suman o quitan). Sin esto, no hay.
	 */
	entrarSeleccion?: (id: string) => void;
	/** Con casillas se pueden elegir varios; sin ellas (Jugar) solo se elige uno. */
	casillas: boolean;
	/** El doble clic, si hace algo distinto de `abrir` (en la lista, `abrir`). */
	alDobleClic?: (id: string) => void;
};

export const SeleccionDecks = createContext<Seleccion | null>(null);

export function useSeleccion(): Seleccion {
	const s = useContext(SeleccionDecks);
	if (!s) throw new Error("SeleccionDecks sin proveedor");
	return s;
}

export function VistaCuadricula({ decks }: { decks: DeckSummary[] }) {
	const tamano = usePrefsSel((s) => s.tamano);
	const { tamano: formato } = useFormato();
	return (
		<div className={cn("grid", clasesCuadricula(formato, tamano))}>
			{decks.map((d) => (
				<TarjetaDeck key={d.id} deck={d} />
			))}
		</div>
	);
}

function TarjetaDeck({ deck: d }: { deck: DeckSummary }) {
	const { t } = useLang();
	const nombreDeck = useNombreDeck();
	const hace = useHace();
	const sel = useSeleccion();
	const det = useBiblioteca((s) => s.detalles[d.id]);
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const tamano = usePrefsSel((s) => s.tamano);
	const [ref, setRef] = useState<HTMLDivElement | null>(null);
	const visible = useVisible(ref);
	const marcado = sel.seleccion.includes(d.id);
	const conSeleccion = sel.seleccion.length > 0;
	const { tactil } = useFormato();
	// Con el dedo y casillas: mantener pulsado elige (un gesto por elemento; el
	// menú es el «⋯»). Sin casillas (Jugar) sigue el menú del clic derecho.
	const eligeAlMantener = tactil && sel.casillas && !!sel.entrarSeleccion;
	const larga = usePulsacionLarga(() => sel.entrarSeleccion?.(d.id));
	return (
		<MenuContextualDeck deck={d} deshabilitado={eligeAlMantener}>
			<div
				ref={setRef}
				{...(eligeAlMantener ? larga : {})}
				data-pulsacion-larga={eligeAlMantener ? "" : undefined}
				className={cn(
					"group relative flex flex-col gap-2 rounded-xl border bg-card p-2.5 transition-shadow hover:shadow-md",
					d.id === sel.abierto && "ring-2 ring-primary",
					marcado && "bg-primary/10 ring-2 ring-primary/70",
				)}
			>
				<button
					type="button"
					onClick={(e) => sel.pulsar(d.id, e)}
					onDoubleClick={sel.alDobleClic && (() => sel.alDobleClic?.(d.id))}
					className="relative aspect-[63/88] w-full overflow-hidden rounded-[4.5%] bg-muted"
				>
					{d.thumbnail && visible ? (
						<img
							src={urlMiniatura(d.thumbnail, PORTADA, tamano === "l" ? 488 : 244)}
							alt=""
							loading="lazy"
							draggable={false}
							className="size-full object-cover transition-transform group-hover:scale-[1.02]"
						/>
					) : (
						<div className="flex size-full items-center justify-center text-muted-foreground">
							<Icono icon="lucide:layers" className="size-8" />
						</div>
					)}
				</button>
				{/* Encima de la portada: casilla y favorito. */}
				{sel.casillas && (
					<div
						className={cn(
							"absolute top-4 left-4 transition-opacity",
							conSeleccion || marcado ? "opacity-100" : "opacity-0 group-hover:opacity-100",
						)}
					>
						<Checkbox
							checked={marcado}
							onClick={(e) => {
								e.preventDefault();
								sel.alternar(d.id, e);
							}}
							aria-label={t("Seleccionar")}
							className="bg-background/90 shadow"
						/>
					</div>
				)}
				<WithTooltip content={d.pinned ? t("Quitar de favoritos") : t("Añadir a favoritos")}>
					<button
						type="button"
						onClick={() => void useBiblioteca.getState().favorito([d.id], !d.pinned)}
						className={cn(
							"absolute top-3.5 right-3.5 rounded-full bg-background/80 p-1 shadow transition-opacity",
							d.pinned
								? "opacity-100"
								: "opacity-0 group-hover:opacity-100 [[data-tactil]_&]:opacity-100",
						)}
						aria-label={d.pinned ? t("Quitar de favoritos") : t("Añadir a favoritos")}
					>
						<Icono
							icon="lucide:star"
							className={cn(
								"size-3.5",
								d.pinned ? "fill-amber-400 text-amber-500" : "text-muted-foreground",
							)}
						/>
					</button>
				</WithTooltip>
				<div className="flex items-start gap-1">
					<div className="min-w-0 flex-1">
						<div className="flex items-center gap-1">
							<button
								type="button"
								onClick={(e) => sel.pulsar(d.id, e)}
								className="truncate text-left text-xs font-semibold hover:underline"
							>
								{nombreDeck(d.name)}
							</button>
							{d.notes && (
								<WithTooltip content={<span className="whitespace-pre-line">{d.notes}</span>}>
									<Icono
										icon="lucide:sticky-note"
										className="size-3 shrink-0 text-muted-foreground"
									/>
								</WithTooltip>
							)}
						</div>
						<div className="truncate text-[10px] text-muted-foreground">
							{t("{n} cartas", { n: d.cardCount })} · {hace(d.updatedAt)}
						</div>
					</div>
					<BotonMenuDeck deck={d}>
						<Button size="icon-xs" variant="ghost" aria-label={t("Más acciones")}>
							<Icono icon="lucide:more-horizontal" />
						</Button>
					</BotonMenuDeck>
				</div>
				{((det && det.deckSize > 0) || d.tags.length > 0) && (
					<div className="flex min-w-0 flex-wrap items-center gap-1.5">
						{det && det.deckSize > 0 && (
							<>
								<Identidad colores={det.colorIdentity} />
								{tamano !== "s" && (
									<span className="text-[10px] text-muted-foreground">{t(CLASES[det.kind])}</span>
								)}
							</>
						)}
						<ChipsEtiquetas ids={d.tags} etiquetas={etiquetas} max={tamano === "s" ? 1 : 3} />
					</div>
				)}
			</div>
		</MenuContextualDeck>
	);
}
