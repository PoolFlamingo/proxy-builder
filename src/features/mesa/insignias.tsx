import { useShallow } from "zustand/react/shallow";
import type { VistaCarta } from "@/bindings/VistaCarta";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { NINGUNA, useMesa } from "@/stores/mesa-store";
import { COLORES_ETIQUETA } from "./colores";
import { COLOR_DE_MANA, esTemporal, manaDe, nombreContador, sinTemporal } from "./contadores";
import { ptDe } from "./pt";

/** Abreviatura de un contador para la insignia (los de fuerza y resistencia, enteros). */
function corto(tipo: string): string {
	if (tipo.includes("/")) return tipo;
	return tipo.slice(0, 5).toUpperCase();
}

/**
 * Lo que se pinta encima de una carta del campo: contadores, fuerza y
 * resistencia cambiadas, etiquetas, palabras clave, cantidad del montón e
 * insignias «Robada» y «Copia» (ficha que copia otra carta).
 */
export function Insignias({ c, yo }: { c: VistaCarta; yo: number | null }) {
	const { t } = useLang();
	const pt = ptDe(c);
	const jugadores = useMesa((s) => s.jugadores);
	const contadores = Object.entries(c.contadores);
	const robada = c.controlador !== c.dueno;
	const copia = c.esFicha && c.copiaDe !== null;
	const notas = useMesa(
		useShallow((s) => (s.general?.notas ?? NINGUNA).filter((n) => n.carta === c.id)),
	);
	return (
		<>
			{c.etiquetas.length > 0 && (
				<div className="pointer-events-none absolute inset-x-0.5 top-[14%] flex flex-col items-center gap-0.5">
					{c.etiquetas.map((e) => (
						<span
							key={e.texto}
							className="max-w-full truncate rounded px-1 text-[9px] font-semibold text-white shadow"
							style={{ background: COLORES_ETIQUETA[e.color] ?? "#78716c" }}
						>
							{e.texto}
						</span>
					))}
				</div>
			)}
			{contadores.length > 0 && (
				<div className="pointer-events-none absolute top-1/2 left-0.5 flex -translate-y-1/2 flex-col gap-0.5">
					{contadores.map(([tipo, n]) => {
						const mana = manaDe(tipo);
						return (
							<span
								key={tipo}
								className={cn(
									"flex items-center gap-0.5 rounded-full bg-black/80 px-1.5 text-[10px] leading-4 font-bold text-white ring-1",
									esTemporal(tipo) ? "ring-amber-300/80" : "ring-white/30",
								)}
							>
								{esTemporal(tipo) && (
									<Icono icon="lucide:clock" className="size-2.5 text-amber-300" />
								)}
								{mana ? (
									<>
										<span
											className="size-2.5 rounded-full ring-1 ring-white/50"
											style={{ background: COLOR_DE_MANA[mana] }}
										/>
										{n}
									</>
								) : (
									<>
										{corto(sinTemporal(tipo))}
										{n > 1 || tipo.includes("/") ? ` ×${n}` : ""}
									</>
								)}
							</span>
						);
					})}
				</div>
			)}
			{(robada ||
				copia ||
				c.devolverA !== null ||
				c.noEndereza ||
				c.faseFuera ||
				notas.length > 0) && (
				// Las dos esquinas de arriba, en una capa que mide lo que la carta: `@max-…`
				// quita el texto de una carta pequeña y `cqw` reparte el ancho para que las
				// de un lado nunca pisen las del otro.
				<div className="pointer-events-none absolute inset-0 @container">
					{(robada || copia) && (
						// A la izquierda y apiladas, sin pisarse entre ellas. Abajo taparían la
						// fuerza y resistencia y el texto de una carta pequeña.
						<div className="absolute top-0.5 left-0.5 flex max-w-[50cqw] flex-col items-start gap-0.5">
							{robada && (
								<EtiquetaEstado
									icono="lucide:hand"
									texto={c.controlador === yo ? t("Robada") : t("Prestada")}
									className="bg-rose-600/90 text-white"
								/>
							)}
							{copia && (
								<EtiquetaEstado
									icono="lucide:copy"
									texto={t("Copia")}
									className="bg-violet-600/90 text-white"
								/>
							)}
						</div>
					)}
					{(c.devolverA !== null || c.noEndereza || c.faseFuera || notas.length > 0) && (
						// A la derecha, lo que dura sobre la carta: el control prestado, que no se
						// endereza, que está fuera de fase y sus notas.
						<div className="absolute top-0.5 right-0.5 flex max-w-[45cqw] flex-col items-end gap-0.5">
							{c.devolverA !== null && (
								<EtiquetaEstado
									icono="lucide:clock"
									texto={jugadores[c.devolverA]?.nombre ?? ""}
									className="bg-black/80 text-amber-300 ring-1 ring-amber-300/80"
								/>
							)}
							<div className="flex flex-wrap justify-end gap-0.5">
								{c.noEndereza && <Estado icono="game-icons:padlock" ayuda={t("No se endereza")} />}
								{c.faseFuera && <Estado icono="game-icons:portal" ayuda={t("Fuera de fase")} />}
								{notas.length > 0 && (
									<Tooltip>
										<TooltipTrigger asChild>
											<span className="pointer-events-auto flex h-4 items-center gap-0.5 rounded-full bg-amber-300 px-1 text-[9px] font-bold text-black shadow ring-1 ring-black/30">
												<Icono icon="lucide:sticky-note" className="size-2.5" />
												{notas.length > 1 && notas.length}
											</span>
										</TooltipTrigger>
										<TooltipContent className="max-w-60 text-xs">
											{notas.map((n) => (
												<p key={n.id} className="break-words whitespace-pre-wrap">
													<span
														className="mr-1 inline-block size-2 rounded-full align-middle"
														style={{ background: jugadores[n.autor]?.color }}
													/>
													{n.texto}
												</p>
											))}
										</TooltipContent>
									</Tooltip>
								)}
							</div>
						</div>
					)}
				</div>
			)}
			{c.palabrasClave.length > 0 && (
				<div className="pointer-events-none absolute right-0.5 bottom-[22%] flex max-w-[70%] flex-wrap justify-end gap-0.5">
					{c.palabrasClave.map((p) => (
						<span
							key={p}
							className={cn(
								"flex items-center gap-0.5 rounded bg-sky-700/90 px-1 text-[8px] leading-3 text-white",
								esTemporal(p) && "ring-1 ring-amber-300/80",
							)}
						>
							{esTemporal(p) && <Icono icon="lucide:clock" className="size-2 text-amber-300" />}
							{nombreContador(p, t)}
						</span>
					))}
				</div>
			)}
			{pt && (
				<span
					className={cn(
						"pointer-events-none absolute right-0.5 bottom-0.5 rounded px-1 text-[11px] font-bold tabular-nums",
						pt.cambiada ? "bg-amber-400 text-black" : "bg-black/75 text-white",
					)}
				>
					{pt.f}/{pt.r}
				</span>
			)}
			{c.cantidad > 1 && (
				<span className="pointer-events-none absolute -top-2 -right-2 rounded-full bg-primary px-1.5 text-[11px] font-bold text-primary-foreground shadow">
					×{c.cantidad}
				</span>
			)}
			{c.habilidad && (
				<span className="pointer-events-none absolute inset-x-1 bottom-1 line-clamp-3 rounded bg-black/80 px-1 text-[9px] text-white">
					{c.habilidad}
				</span>
			)}
		</>
	);
}

/** Una etiqueta de símbolo y texto; con la carta pequeña, solo el símbolo. */
function EtiquetaEstado({
	icono,
	texto,
	className,
}: {
	icono: string;
	texto: string;
	className: string;
}) {
	return (
		<span
			role="img"
			aria-label={texto}
			className={cn(
				"flex max-w-full items-center gap-0.5 rounded px-1 py-px text-[8px] font-semibold",
				className,
			)}
		>
			<Icono icon={icono} className="size-2.5 shrink-0" />
			<span className="min-w-0 truncate @max-[96px]:hidden">{texto}</span>
		</span>
	);
}

/** Un estado de la carta que dura (no se endereza, fuera de fase), con su explicación. */
function Estado({ icono, ayuda }: { icono: string; ayuda: string }) {
	return (
		<WithTooltip content={ayuda}>
			<span className="pointer-events-auto flex size-4 items-center justify-center rounded-full bg-black/80 text-amber-300 shadow ring-1 ring-amber-300/80">
				<Icono icon={icono} className="size-2.5" />
			</span>
		</WithTooltip>
	);
}
