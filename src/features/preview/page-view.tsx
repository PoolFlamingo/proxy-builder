import { useDroppable } from "@dnd-kit/core";
import { Plus, Trash2 } from "lucide-react";
import { memo, useState } from "react";
import type { DeckCard } from "@/bindings/DeckCard";
import type { FaceImage } from "@/bindings/FaceImage";
import type { LineMm } from "@/bindings/LineMm";
import type { Page } from "@/bindings/Page";
import type { PrintSettings } from "@/bindings/PrintSettings";
import type { RectMm } from "@/bindings/RectMm";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import type { AnchoMiniatura } from "@/lib/image-url";
import { cn } from "@/lib/utils";
import { useUi } from "@/stores/ui-store";
import { SlotView } from "./slot-view";
import { useVisible } from "./visibilidad";

/** Grosor de las guías en mm (1 px CSS = 0,2646 mm). */
const PX_A_MM = 25.4 / 96;

/**
 * Una hoja de la vista previa, del tamaño del papel a la escala del zoom.
 * Las casillas siempre están (son destinos del arrastre); las imágenes solo
 * mientras la hoja está cerca de la vista.
 */
export const PageView = memo(function PageView({
	pagina,
	numero,
	total,
	escala,
	cartas,
	ajustes,
	dorsoDeck,
	ancho,
	celdas,
	huecosLibres,
	onBorrar,
}: {
	pagina: Page;
	numero: number;
	total: number;
	escala: number;
	cartas: Map<string, DeckCard>;
	ajustes: PrintSettings;
	dorsoDeck: FaceImage | null;
	ancho: AnchoMiniatura;
	/** Casillas vacías de esta hoja (solo la última tiene). */
	celdas: RectMm[];
	huecosLibres: boolean;
	onBorrar: (pagina: Page) => void;
}) {
	const { t } = useLang();
	const [hoja, setHoja] = useState<HTMLDivElement | null>(null);
	const visible = useVisible(hoja);
	const e = escala;
	const guias = ajustes.cutGuides;

	return (
		<div className="flex flex-col items-center gap-1.5">
			<div
				className="flex items-center justify-between gap-2.5 text-[10px] font-semibold tracking-widest text-muted-foreground uppercase"
				style={{ width: pagina.widthMm * e }}
			>
				<span>{t("Página {n} de {total}", { n: numero, total })}</span>
				<WithTooltip content={t("Eliminar las cartas de esta página")}>
					<Button
						size="icon-xs"
						variant="ghost"
						onClick={() => onBorrar(pagina)}
						aria-label={t("Eliminar las cartas de esta página")}
					>
						<Trash2 />
					</Button>
				</WithTooltip>
			</div>
			<div
				ref={setHoja}
				className="relative bg-white shadow-md ring-1 ring-black/10"
				style={{ width: pagina.widthMm * e, height: pagina.heightMm * e }}
			>
				{pagina.slots.map((s) => (
					<SlotView
						key={`${s.cardId}:${s.face}`}
						slot={s}
						carta={cartas.get(s.cardId)}
						escala={e}
						ajustes={ajustes}
						dorsoDeck={dorsoDeck}
						conImagen={visible}
						ancho={ancho}
					/>
				))}
				{celdas.map((c, i) => (
					<CeldaVacia
						key={`${c.x}:${c.y}`}
						celda={c}
						escala={e}
						primera={i === 0 && huecosLibres}
					/>
				))}
				{guias.enabled && pagina.guides.length > 0 && (
					<svg
						aria-hidden
						className="pointer-events-none absolute inset-0"
						width={pagina.widthMm * e}
						height={pagina.heightMm * e}
						viewBox={`0 0 ${pagina.widthMm} ${pagina.heightMm}`}
					>
						{pagina.guides.map((l: LineMm) => (
							<line
								key={`${l.x1}:${l.y1}:${l.x2}:${l.y2}`}
								x1={l.x1}
								y1={l.y1}
								x2={l.x2}
								y2={l.y2}
								stroke={guias.color}
								strokeWidth={guias.widthPx * PX_A_MM}
							/>
						))}
					</svg>
				)}
			</div>
		</div>
	);
});

/** Una casilla sin carta; un clic abre «Añadir carta». La primera lo rotula. */
function CeldaVacia({
	celda,
	escala,
	primera,
}: {
	celda: RectMm;
	escala: number;
	primera: boolean;
}) {
	const { t } = useLang();
	const drop = useDroppable({ id: `vacia:${celda.x}:${celda.y}` });
	const e = escala;
	return (
		<div
			ref={drop.setNodeRef}
			className={cn(
				"absolute rounded-sm border border-dashed border-black/15",
				drop.isOver && "border-sky-500 bg-sky-500/10",
			)}
			style={{ left: celda.x * e, top: celda.y * e, width: celda.w * e, height: celda.h * e }}
		>
			<button
				type="button"
				aria-label={t("Añadir carta")}
				onClick={() => useUi.getState().abrir({ tipo: "anadir" })}
				className="flex size-full flex-col items-center justify-center gap-1 text-[10px] font-semibold tracking-wider text-black/40 uppercase hover:bg-black/5 hover:text-black/70"
			>
				<Plus className="size-5" />
				{primera && t("Añadir carta")}
			</button>
		</div>
	);
}
