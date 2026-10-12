import { type ReactNode, useMemo } from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

/**
 * Gráfico de barras del sistema de diseño.
 *
 * ## Por qué no es Recharts
 *
 * El lenguaje visual es el de los gráficos de shadcn/ui —rejilla horizontal
 * tenue, barras del acento del tema con la punta redondeada, eje de fechas
 * espaciado, tooltip en tarjeta—, pero **sin la dependencia**. Recharts pesa
 * cerca de medio mega y trae un motor de escalas, ejes y animaciones que aquí
 * no se usa: esto es una serie única, sin escalas logarítmicas, sin ejes
 * dobles ni zoom. La alternativa costaba un `npm install` en un proyecto que
 * ya se rompió una vez por instalar (ver CLAUDE.md) y sumaba superficie a una
 * aplicación cuyo argumento es que los datos no salen del ordenador.
 *
 * Si algún día hacen falta líneas, áreas o series apiladas, la conversación
 * es otra y entonces sí toca traer la librería.
 *
 * ## Decisiones que en un gráfico de datos no son de gusto
 *
 * - **Las barras salen de cero.** Nada de escalas recortadas que conviertan
 *   una diferencia mínima en un desastre visual.
 * - **El techo de la escala se redondea a una cifra legible** (1, 2 o 5 por
 *   la potencia de diez que toque). Una rejilla en 0,0983 no dice nada; una
 *   en 0,10 permite estimar cualquier barra de un vistazo.
 * - 🛑 **Un cero se dibuja como cero**: hueco vacío sobre la línea base, sin
 *   mínimos «para que se vea algo». Un dibujo no puede afirmar un valor que
 *   no existió.
 * - **Las etiquetas del eje se saltan** las que no caben en vez de encogerse
 *   o girarse: un eje ilegible es peor que un eje con menos marcas.
 */

export interface BarraDato {
	/** Identificador estable del tramo. */
	clave: string;
	/** Lo que va bajo el eje. Puede omitirse en los tramos intermedios. */
	etiqueta: string;
	valor: number;
	/** Lo que se enseña al pasar el ratón. Si falta, se enseña la etiqueta. */
	detalle?: ReactNode;
}

/**
 * Ancho máximo de una barra.
 *
 * 🛑 Sin tope, la barra ocupa **todo su hueco**, y el hueco depende de cuántos
 * tramos haya: con 30 días salían finas, pero con 12 meses el mismo gráfico
 * las dibujaba de cien píxeles y parecía otra cosa (Claudio, 11/08/2026). El
 * grosor de una barra no significa nada —lo que significa es su altura—, así
 * que no puede cambiar según el periodo elegido. Con tope, la barra se centra
 * en su hueco y el gráfico se ve igual de fino en día, semana y mes.
 */
const ANCHO_MAXIMO_BARRA = "max-w-6";

/** A cuántas etiquetas como mucho se aspira en el eje de abajo. */
const ETIQUETAS_MAXIMAS = 12;

export function GraficoBarras({
	datos,
	alto = "h-56",
	/** Cuántas divisiones tiene la rejilla, sin contar la línea base. */
	divisiones = 4,
	/** Cómo se escribe un valor del eje vertical. */
	formatearValor = (v: number) => v.toFixed(2),
	/** Destaca el último tramo: en una serie temporal es «ahora». */
	destacarUltimo = true,
	/** Al pulsar un tramo: lo que lo forma (el panel económico, Fase 4). Sin
	 *  esto, las barras solo se consultan con el ratón. */
	onPulsar,
	className,
}: {
	datos: BarraDato[];
	alto?: string;
	divisiones?: number;
	formatearValor?: (v: number) => string;
	destacarUltimo?: boolean;
	onPulsar?: (clave: string) => void;
	className?: string;
}) {
	const techo = useMemo(() => techoLegible(Math.max(0, ...datos.map((d) => d.valor))), [datos]);

	// Una etiqueta de cada N, contando DESDE EL FINAL para que la última —la
	// que más se mira— tenga siempre la suya. Con doce tramos o menos se
	// etiquetan todos; con treinta días salen diez marcas.
	const salto = Math.max(1, Math.ceil(datos.length / ETIQUETAS_MAXIMAS));

	if (datos.length === 0) return null;

	return (
		<div className={cn("flex w-full gap-2", className)}>
			{/* Eje vertical. Va fuera del área de barras para que la rejilla y las
          barras compartan exactamente el mismo alto y nada se descuadre. */}
			<div
				className={cn(
					"flex w-12 shrink-0 flex-col justify-between text-right text-[9px] tabular-nums text-muted-foreground/70",
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
					{/* Rejilla: tenue, solo para poder estimar una altura. La línea de
              abajo es la base y va un punto más marcada, porque es el cero y
              el cero no es una división más. */}
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

					<div className="absolute inset-0 flex items-end gap-[2px]">
						{datos.map((d, i) => {
							const ultimo = i === datos.length - 1;
							const alto = techo > 0 && d.valor > 0 ? (d.valor / techo) * 100 : 0;
							return (
								<Tooltip key={d.clave}>
									<TooltipTrigger asChild>
										{/* El botón ocupa TODA la altura aunque la barra sea de
                        cero: así el ratón encuentra el tramo y se puede
                        consultar igual que los demás. */}
										<button
											type="button"
											className={cn(
												"group flex h-full min-w-0 flex-1 flex-col justify-end rounded-sm transition-colors hover:bg-muted/40",
												onPulsar ? "cursor-pointer" : "cursor-help",
											)}
											onClick={onPulsar ? () => onPulsar(d.clave) : undefined}
											aria-label={`${d.etiqueta}: ${formatearValor(d.valor)}`}
										>
											<span
												className={cn(
													"mx-auto w-full rounded-t-[3px] transition-colors",
													ANCHO_MAXIMO_BARRA,
													destacarUltimo && ultimo
														? "bg-primary"
														: "bg-primary/55 group-hover:bg-primary/85",
												)}
												style={{ height: `${alto}%` }}
											/>
										</button>
									</TooltipTrigger>
									<TooltipContent className="text-[11px]">{d.detalle ?? d.etiqueta}</TooltipContent>
								</Tooltip>
							);
						})}
					</div>
				</div>

				{/* Eje de abajo.
            🛑 Cada etiqueta va POSICIONADA sobre el centro de su hueco, no
            metida dentro de él. La primera versión le daba a cada una una celda
            del ancho de su barra y, con treinta días, «14 ago» no cabía en
            cuarenta píxeles: el eje salía «14…», «18…», ilegible. Como solo se
            pinta una de cada N, la etiqueta puede volar por encima de los
            huecos vacíos de al lado sin pisar a nadie. */}
				<div className="relative mt-1.5 h-3">
					{datos.map((d, i) =>
						(datos.length - 1 - i) % salto === 0 ? (
							<span
								key={d.clave}
								className="absolute -translate-x-1/2 whitespace-nowrap text-[9px] leading-none text-muted-foreground"
								style={{ left: `${((i + 0.5) / datos.length) * 100}%` }}
							>
								{d.etiqueta}
							</span>
						) : null,
					)}
				</div>
			</div>
		</div>
	);
}

/**
 * El techo de la escala, redondeado hacia arriba a 1, 2 o 5 por su potencia
 * de diez.
 *
 * Es lo que hace que la rejilla se pueda leer: con un máximo de 0,0983 las
 * divisiones caen en 0,0246 y no significan nada; redondeado a 0,10 caen en
 * 0,025 y cualquiera estima una barra sin pensarlo.
 */
export function techoLegible(maximo: number): number {
	if (!Number.isFinite(maximo) || maximo <= 0) return 1;
	const potencia = 10 ** Math.floor(Math.log10(maximo));
	const normalizado = maximo / potencia;
	const paso = normalizado <= 1 ? 1 : normalizado <= 2 ? 2 : normalizado <= 5 ? 5 : 10;
	return paso * potencia;
}
