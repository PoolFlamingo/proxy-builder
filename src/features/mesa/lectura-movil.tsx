import { useLayoutEffect, useRef, useState } from "react";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { ANCHO_PANEL_LECTURA, lecturaMovil, MARGEN_LECTURA_MOVIL } from "./leer-en-grande";

/** El tamaño del hueco de la mesa (la capa que lo cubre), al día al girar el teléfono. */
function useHueco(ref: React.RefObject<HTMLDivElement | null>): { w: number; h: number } {
	const [v, setV] = useState(() => ({ w: window.innerWidth, h: window.innerHeight }));
	useLayoutEffect(() => {
		const el = ref.current;
		if (!el) return;
		const medir = () => {
			const r = el.getBoundingClientRect();
			if (r.width > 0 && r.height > 0) setV({ w: r.width, h: r.height });
		};
		medir();
		const ro = new ResizeObserver(medir);
		ro.observe(el);
		return () => ro.disconnect();
	}, [ref]);
	return v;
}

/**
 * **La carta en grande en un móvil** (la vista grande y la del fantasma): la carta
 * entera dentro de la pantalla, a su proporción y sin recortar (`lecturaMovil`), y
 * la X **fuera** de la carta, para que no tape el título. De lado, la carta va de
 * arriba abajo en el lado contrario a donde se tocó (`aLaIzquierda`) y la X y el
 * panel de estado, a su lado, hacia el centro; de pie, la carta va arriba a lo
 * ancho y la X y el panel, debajo.
 *
 * `cara(ancho, alto)` pinta cada cara al tamaño dado; `panel`, el panel de estado.
 */
export function LecturaMovil({
	caras,
	cara,
	panel,
	aLaIzquierda,
	onCerrar,
}: {
	caras: number;
	cara: (i: number, ancho: number, alto: number) => React.ReactNode;
	panel?: React.ReactNode;
	aLaIzquierda: boolean;
	onCerrar: () => void;
}) {
	const { t } = useLang();
	const hueco = useRef<HTMLDivElement>(null);
	const v = useHueco(hueco);
	const horizontal = v.w >= v.h;
	const { ancho, alto } = lecturaMovil(v.w, v.h, horizontal, caras, panel != null);
	const m = MARGEN_LECTURA_MOVIL;
	const cerrar = (
		<Button
			size="icon"
			variant="secondary"
			onPointerDown={(e) => e.stopPropagation()}
			onClick={onCerrar}
			aria-label={t("Cerrar")}
			data-lee-cierra=""
			className="pointer-events-auto shrink-0 rounded-full shadow-lg"
		>
			<Icono icon="lucide:x" className="size-5" />
		</Button>
	);
	return (
		<div ref={hueco} className="pointer-events-none absolute inset-0 z-[8000]">
			<div
				className={cn(
					"pointer-events-none absolute flex gap-2",
					horizontal ? (aLaIzquierda ? "flex-row" : "flex-row-reverse") : "flex-col",
				)}
				style={
					horizontal
						? { top: m, ...(aLaIzquierda ? { left: m } : { right: m }) }
						: { top: m, left: m, right: m }
				}
			>
				<div className={cn("flex gap-2", !horizontal && "justify-center")}>
					{Array.from({ length: caras }, (_, i) => (
						<div
							key={i}
							className="shrink-0 overflow-hidden rounded-[4.5%] shadow-2xl"
							style={{ width: ancho, height: alto }}
						>
							{cara(i, ancho, alto)}
						</div>
					))}
				</div>
				<div
					className={cn(
						"flex min-w-0 gap-2",
						horizontal ? "flex-col" : "flex-row items-start",
						horizontal && !aLaIzquierda && "items-end",
					)}
					style={horizontal && panel ? { width: ANCHO_PANEL_LECTURA } : undefined}
				>
					{cerrar}
					{panel && (
						<div
							className="min-w-0 flex-1 overflow-hidden"
							style={{ maxHeight: horizontal ? v.h - 2 * m - 52 : 120 }}
						>
							{panel}
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
