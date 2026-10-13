import { Check, ImageOff } from "lucide-react";
import { memo, useState } from "react";
import type { CardSummary } from "@/bindings/CardSummary";
import { useLang } from "@/components/language-provider";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { useVisible } from "@/features/preview/visibilidad";
import { urlBusqueda } from "@/lib/image-url";
import { cn } from "@/lib/utils";

/**
 * Rejilla de resultados de Scryfall (añadir carta, variantes, sets). Las
 * miniaturas las descarga Rust (`pbimg://…/t/…`) y, como en la vista previa,
 * solo existen mientras la tarjeta está cerca de la vista (el `ScrollArea`
 * que la envuelve tiene que dar su viewport por `RaizScroll`).
 */
export function RejillaCartas({
	cartas,
	elegidas,
	onElegir,
	onDoble,
	columnas = "grid-cols-[repeat(auto-fill,minmax(120px,1fr))]",
}: {
	cartas: CardSummary[];
	elegidas: Set<string>;
	onElegir: (c: CardSummary, e: React.MouseEvent) => void;
	onDoble?: (c: CardSummary) => void;
	columnas?: string;
}) {
	return (
		<div className={cn("grid gap-2.5 p-1.5", columnas)}>
			{cartas.map((c) => (
				<Tarjeta
					key={c.id}
					carta={c}
					elegida={elegidas.has(c.id)}
					onElegir={onElegir}
					onDoble={onDoble}
				/>
			))}
		</div>
	);
}

const Tarjeta = memo(function Tarjeta({
	carta: c,
	elegida,
	onElegir,
	onDoble,
}: {
	carta: CardSummary;
	elegida: boolean;
	onElegir: (c: CardSummary, e: React.MouseEvent) => void;
	onDoble?: (c: CardSummary) => void;
}) {
	const { t } = useLang();
	const [el, setEl] = useState<HTMLButtonElement | null>(null);
	const visible = useVisible(el);
	return (
		<button
			ref={setEl}
			type="button"
			onClick={(e) => onElegir(c, e)}
			onDoubleClick={() => onDoble?.(c)}
			className={cn(
				"group relative flex flex-col gap-1 rounded-lg p-1 text-left outline-none transition-colors hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring",
				elegida && "bg-primary/15 ring-2 ring-primary",
			)}
		>
			<div className="relative aspect-[63/88] w-full overflow-hidden rounded-[4.5%] bg-muted">
				{c.thumb && visible ? (
					<img
						src={urlBusqueda(c.thumb)}
						alt=""
						loading="lazy"
						decoding="async"
						draggable={false}
						className="size-full object-cover"
					/>
				) : !c.thumb ? (
					<div className="flex size-full items-center justify-center text-muted-foreground">
						<ImageOff className="size-5" />
					</div>
				) : null}
				{elegida && (
					<span className="absolute top-1 right-1 rounded-full bg-primary p-0.5 text-primary-foreground">
						<Check className="size-3" />
					</span>
				)}
				<div className="absolute bottom-1 left-1 flex gap-0.5">
					<span className="rounded-[3px] bg-black/75 px-1 font-mono text-[8px] font-bold text-white uppercase">
						{c.lang}
					</span>
					{!c.highres && (
						<WithTooltip content={t("Escaneo de baja resolución")}>
							<span className="rounded-[3px] bg-amber-500/90 px-1 font-mono text-[8px] font-bold text-black">
								LOWRES
							</span>
						</WithTooltip>
					)}
				</div>
			</div>
			<span className="truncate text-[11px] leading-tight font-medium">
				{c.printedName ?? c.name}
			</span>
			<span className="truncate text-[10px] leading-tight text-muted-foreground">
				{c.set.toUpperCase()} · {c.collectorNumber} · {c.setName}
			</span>
		</button>
	);
});
