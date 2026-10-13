import { type ReactNode, useMemo } from "react";

import { divisionesEnteras, techoLegible } from "@/components/ui/grafico-barras";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/**
 * Barras apiladas con el mismo lenguaje que `GraficoBarras` (rejilla tenue,
 * escala desde cero con techo legible, barras finas con tope de ancho), para
 * cuando cada tramo se reparte en partes: la curva de maná por tipo.
 *
 * Los colores de las partes los pone quien llama (clases de Tailwind), porque
 * tienen significado (un tipo de carta, un color de maná) y no dependen del
 * tema. La leyenda va aparte (`LeyendaApilado`) para poder colocarla donde
 * convenga.
 */

export interface ParteApilada {
	clave: string;
	valor: number;
	/** Clase del fondo de la parte (`bg-emerald-500`). */
	clase: string;
}

export interface TramoApilado {
	clave: string;
	etiqueta: string;
	partes: ParteApilada[];
	/** Lo que sale al pasar el ratón por el tramo entero. */
	detalle?: ReactNode;
}

export function GraficoApilado({
	datos,
	alto = "h-48",
	formatearValor = (v: number) => String(Math.round(v)),
	className,
}: {
	datos: TramoApilado[];
	alto?: string;
	formatearValor?: (v: number) => string;
	className?: string;
}) {
	const techo = useMemo(
		() => techoLegible(Math.max(0, ...datos.map((d) => d.partes.reduce((s, p) => s + p.valor, 0)))),
		[datos],
	);
	// Son recuentos: divisiones enteras.
	const divisiones = divisionesEnteras(techo);
	if (datos.length === 0) return null;

	return (
		<div className={cn("flex w-full gap-2", className)}>
			<div
				className={cn(
					"flex w-8 shrink-0 flex-col justify-between text-right text-[9px] tabular-nums text-muted-foreground/70",
					alto,
				)}
			>
				{Array.from({ length: divisiones + 1 }, (_, i) => (
					<span key={i} className="-translate-y-1 leading-none">
						{formatearValor(techo - (techo / divisiones) * i)}
					</span>
				))}
			</div>
			<div className="min-w-0 flex-1">
				<div className={cn("relative", alto)}>
					{Array.from({ length: divisiones + 1 }, (_, i) => (
						<div
							key={i}
							className={cn(
								"absolute inset-x-0 border-t",
								i === divisiones ? "border-border/50" : "border-border/25",
							)}
							style={{ top: `${(100 / divisiones) * i}%` }}
						/>
					))}
					<div className="absolute inset-0 flex items-end gap-1">
						{datos.map((d) => {
							const total = d.partes.reduce((s, p) => s + p.valor, 0);
							return (
								<Tooltip key={d.clave}>
									<TooltipTrigger asChild>
										<button
											type="button"
											className="group flex h-full min-w-0 flex-1 cursor-help flex-col items-center justify-end rounded-sm transition-colors hover:bg-muted/40"
											aria-label={`${d.etiqueta}: ${formatearValor(total)}`}
										>
											<span
												className="relative w-full max-w-10"
												style={{ height: techo > 0 ? `${(total / techo) * 100}%` : 0 }}
											>
												{/* El total, encima de la barra (fuera de su alto, para que
												    la barra mida lo que vale). */}
												{total > 0 && (
													<span className="absolute inset-x-0 bottom-full mb-0.5 text-center text-[9px] leading-none tabular-nums text-muted-foreground">
														{formatearValor(total)}
													</span>
												)}
												<span className="flex size-full flex-col-reverse overflow-hidden rounded-t-[3px]">
													{d.partes
														.filter((p) => p.valor > 0)
														.map((p) => (
															<span
																key={p.clave}
																className={cn("w-full opacity-85 group-hover:opacity-100", p.clase)}
																style={{ height: `${(p.valor / total) * 100}%` }}
															/>
														))}
												</span>
											</span>
										</button>
									</TooltipTrigger>
									<TooltipContent className="text-[11px]">{d.detalle ?? d.etiqueta}</TooltipContent>
								</Tooltip>
							);
						})}
					</div>
				</div>
				<div className="mt-1.5 flex gap-1">
					{datos.map((d) => (
						<span
							key={d.clave}
							className="flex-1 text-center text-[9px] leading-none text-muted-foreground"
						>
							{d.etiqueta}
						</span>
					))}
				</div>
			</div>
		</div>
	);
}

export function LeyendaApilado({
	elementos,
}: {
	elementos: { clave: string; etiqueta: string; clase: string }[];
}) {
	return (
		<div className="flex flex-wrap gap-x-3 gap-y-1">
			{elementos.map((e) => (
				<span key={e.clave} className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
					<span className={cn("size-2.5 rounded-[3px]", e.clase)} />
					{e.etiqueta}
				</span>
			))}
		</div>
	);
}
