import { useDraggable, useDroppable } from "@dnd-kit/core";
import { memo, useCallback } from "react";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";
import type { PrintSettings } from "@/bindings/PrintSettings";
import type { Slot } from "@/bindings/Slot";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { insignias } from "@/lib/cartas";
import { type AnchoMiniatura, parametros, urlMiniatura } from "@/lib/image-url";
import { cn } from "@/lib/utils";
import { useUi } from "@/stores/ui-store";
import { anadirEnHueco, seleccionarConClic } from "./acciones-carta";
import { SlotMenu } from "./slot-menu";

const TONOS = {
	info: "bg-sky-600/90 text-white",
	aviso: "bg-amber-500/90 text-black",
	error: "bg-red-600/90 text-white",
	marca: "bg-violet-600/90 text-white",
} as const;

/**
 * Una casilla ocupada de la vista previa. Se coloca en el `trim` del plan y
 * dentro recorta la imagen al rectángulo de sangrado (`slot.bleed`), igual
 * que hace el PDF: la imagen trae el sangrado completo en los cuatro lados y
 * la caja se queda con lo que el papel lleva de verdad.
 */
export const SlotView = memo(function SlotView({
	slot,
	carta,
	escala,
	ajustes,
	dorsoDeck,
	conImagen,
	ancho,
}: {
	slot: Slot;
	carta: DeckCard | undefined;
	escala: number;
	ajustes: PrintSettings;
	dorsoDeck: FaceImage | null;
	conImagen: boolean;
	ancho: AnchoMiniatura;
}) {
	const { t } = useLang();
	const id = slot.cardId;
	const seleccionada = useUi(useCallback((s) => s.seleccion.includes(id), [id]));
	const verReversos = useUi((s) => s.verReversos);
	const info = useUi((s) => s.infoCarta);
	const drag = useDraggable({ id });
	const drop = useDroppable({ id });
	const setRef = useCallback(
		(el: HTMLElement | null) => {
			drag.setNodeRef(el);
			drop.setNodeRef(el);
		},
		[drag.setNodeRef, drop.setNodeRef],
	);

	const { trim, bleed } = slot;
	const full = slot.bleedFullMm;
	const e = escala;

	// Qué imagen se enseña: la de la casilla o, con «Ver reversos», el dorso
	// que llevará la carta (en su sitio y sin espejo; solo vista).
	let imagen: FaceImage | null = slot.image;
	let esDorso = slot.face === "back";
	let ediciones = slot.applyEdits ? (carta?.edits ?? null) : null;
	if (verReversos && slot.face !== "back" && carta) {
		imagen =
			carta.back.kind === "dfc" || carta.back.kind === "custom" ? carta.back.image : dorsoDeck;
		esDorso = true;
		ediciones = null;
	}
	const url = imagen
		? urlMiniatura(
				imagen.blob,
				// A la medida de la casilla (las cartas grandes llevan la suya).
				{
					...parametros(ajustes, ediciones, esDorso, full),
					card: { widthMm: trim.w, heightMm: trim.h },
				},
				ancho,
			)
		: null;
	const marcas = carta && !verReversos ? insignias(carta) : [];
	const hueco = carta?.source.kind === "blank";

	return (
		<SlotMenu id={id}>
			<div
				ref={setRef}
				data-carta={id}
				{...drag.listeners}
				{...drag.attributes}
				role="button"
				aria-label={carta?.meta.name || t("Hueco")}
				aria-pressed={seleccionada}
				onClick={(ev) => {
					seleccionarConClic(id, ev);
					// Un clic suelto en un hueco abre «Añadir carta» para llenarlo.
					if (hueco && !ev.ctrlKey && !ev.metaKey && !ev.shiftKey) anadirEnHueco(id);
				}}
				onDoubleClick={() => carta?.front && useUi.getState().abrir({ tipo: "editar", id })}
				className={cn(
					"group absolute cursor-grab outline-none active:cursor-grabbing",
					drag.isDragging && "opacity-40",
				)}
				style={{ left: trim.x * e, top: trim.y * e, width: trim.w * e, height: trim.h * e }}
			>
				<div
					className={cn(
						"absolute overflow-hidden",
						hueco ? "rounded-sm border border-dashed border-muted-foreground/40" : "bg-muted",
					)}
					style={{
						left: (bleed.x - trim.x) * e,
						top: (bleed.y - trim.y) * e,
						width: bleed.w * e,
						height: bleed.h * e,
					}}
				>
					{url && conImagen ? (
						<img
							src={url}
							alt=""
							draggable={false}
							decoding="async"
							loading="lazy"
							className="absolute max-w-none select-none"
							style={{
								left: (trim.x - full - bleed.x) * e,
								top: (trim.y - full - bleed.y) * e,
								width: (trim.w + 2 * full) * e,
								height: (trim.h + 2 * full) * e,
							}}
						/>
					) : hueco ? (
						<div className="flex size-full items-center justify-center text-muted-foreground/60">
							<Icono icon="lucide:plus" className="size-5" />
						</div>
					) : !imagen ? (
						<div className="flex size-full flex-col items-center justify-center gap-1 p-2 text-center text-[10px] text-muted-foreground">
							<Icono icon="lucide:image-off" className="size-4" />
							{carta?.meta.name}
						</div>
					) : null}
				</div>

				{/* Encima, en el marco del trim: selección, marcas e info. */}
				<div
					className={cn(
						"pointer-events-none absolute inset-0 rounded-[3px] transition-shadow",
						seleccionada
							? "ring-2 ring-primary ring-offset-1 ring-offset-background"
							: "group-hover:ring-1 group-hover:ring-primary/50",
						drop.isOver && !drag.isDragging && "ring-2 ring-sky-500",
					)}
				/>
				{marcas.length > 0 && (
					<div className="absolute top-1 left-1 flex flex-wrap gap-0.5">
						{marcas.map((m) => (
							<WithTooltip key={m.texto} content={t(m.ayuda)}>
								<span
									className={cn(
										"rounded-[3px] px-1 py-px font-mono text-[8px] leading-tight font-bold shadow-sm",
										TONOS[m.tono],
									)}
								>
									{t(m.texto)}
								</span>
							</WithTooltip>
						))}
					</div>
				)}
				{carta?.locked && (
					<span className="absolute top-1 right-1 rounded-full bg-background/85 p-0.5 shadow-sm">
						<Icono icon="lucide:lock" className="size-2.5" />
					</span>
				)}
				{info && carta && !hueco && (
					<div className="absolute inset-x-0 bottom-0 truncate bg-black/70 px-1 py-0.5 text-[9px] leading-tight text-white">
						{carta.meta.name}
						{carta.meta.set && (
							<span className="opacity-70">
								{" "}
								· {carta.meta.set.toUpperCase()} {carta.meta.collectorNumber}
								{carta.meta.lang && carta.meta.lang !== "en" ? ` · ${carta.meta.lang}` : ""}
							</span>
						)}
					</div>
				)}
			</div>
		</SlotMenu>
	);
});
