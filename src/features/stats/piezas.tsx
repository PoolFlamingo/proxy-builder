import { useState } from "react";
import type { CartaCuenta } from "@/bindings/CartaCuenta";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { useUi } from "@/stores/ui-store";

/** Piezas comunes de las pestañas de estadísticas. */

/** Selecciona esas copias en la vista previa y cierra las estadísticas. */
export function seleccionarEnDeck(ids: string[], texto: string) {
	if (ids.length === 0) return;
	const ui = useUi.getState();
	ui.seleccionar(ids, ids[0]);
	ui.abrir(null);
	ui.anunciar(texto);
}

export function porcentaje(p: number, decimales = 1) {
	return `${(p * 100).toFixed(decimales)} %`;
}

export function Cifra({
	titulo,
	valor,
	nota,
	ayuda,
	className,
}: {
	titulo: string;
	valor: string;
	nota?: string;
	ayuda?: string;
	className?: string;
}) {
	const tarjeta = (
		<div className={cn("rounded-lg border bg-card p-2.5", ayuda && "cursor-help", className)}>
			<div className="truncate text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
				{titulo}
			</div>
			<div className="text-lg leading-tight font-semibold tabular-nums">{valor}</div>
			{nota && <div className="line-clamp-2 text-[10px] text-muted-foreground">{nota}</div>}
		</div>
	);
	return ayuda ? <WithTooltip content={ayuda}>{tarjeta}</WithTooltip> : tarjeta;
}

export function Bloque({
	titulo,
	extra,
	children,
	className,
}: {
	titulo: string;
	extra?: React.ReactNode;
	children: React.ReactNode;
	className?: string;
}) {
	return (
		<section className={cn("flex min-w-0 flex-col gap-2 rounded-lg border p-2.5", className)}>
			<div className="flex min-h-5 items-center justify-between gap-2">
				<h3 className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
					{titulo}
				</h3>
				{extra}
			</div>
			{children}
		</section>
	);
}

export type FilaLista = { k: string; n: number; txt: string; clase?: string; detalle?: string };

/** Barras horizontales: una fila por clave, proporcionales a la mayor. */
export function Lista({
	filas,
	ancho = "7rem",
	formato = (n: number) => String(n),
	vacio,
}: {
	filas: FilaLista[];
	ancho?: string;
	formato?: (n: number) => string;
	vacio?: string;
}) {
	const max = Math.max(1, ...filas.map((f) => f.n));
	if (filas.length === 0 && vacio) {
		return <p className="text-[11px] text-muted-foreground">{vacio}</p>;
	}
	return (
		<div className="flex flex-col gap-1">
			{filas.map((f) => {
				const fila = (
					<div
						key={f.k}
						className="grid items-center gap-2 text-xs"
						style={{ gridTemplateColumns: `${ancho} 1fr 3.5rem` }}
					>
						<span className="truncate">{f.txt}</span>
						<div className="h-2 overflow-hidden rounded-full bg-muted">
							<div
								className={cn("h-full rounded-full", f.clase ?? "bg-primary")}
								style={{ width: `${(f.n / max) * 100}%` }}
							/>
						</div>
						<span className="text-right font-mono text-[11px] tabular-nums">{formato(f.n)}</span>
					</div>
				);
				return f.detalle ? (
					<WithTooltip key={f.k} content={f.detalle}>
						{fila}
					</WithTooltip>
				) : (
					fila
				);
			})}
		</div>
	);
}

/** Lista de cartas con sus copias, plegable, y un botón para seleccionarlas. */
export function ListaCartas({
	titulo,
	cartas,
	ayuda,
	abiertaAlPrincipio = false,
	clase,
}: {
	titulo: string;
	cartas: CartaCuenta[];
	ayuda?: string;
	abiertaAlPrincipio?: boolean;
	clase?: string;
}) {
	const { t } = useLang();
	const [abierta, setAbierta] = useState(abiertaAlPrincipio);
	const total = cartas.reduce((s, c) => s + c.count, 0);
	const ids = cartas.flatMap((c) => c.ids);
	const cabecera = (
		<button
			type="button"
			onClick={() => setAbierta((a) => !a)}
			disabled={cartas.length === 0}
			className="flex min-w-0 flex-1 items-center gap-1.5 text-left text-xs disabled:opacity-50"
		>
			<Icono
				icon="lucide:chevron-right"
				className={cn("size-3.5 shrink-0 transition-transform", abierta && "rotate-90")}
			/>
			{clase && <span className={cn("size-2.5 shrink-0 rounded-[3px]", clase)} />}
			<span className="truncate font-medium">{titulo}</span>
			<span className="ml-auto font-mono text-[11px] tabular-nums text-muted-foreground">
				{total}
			</span>
		</button>
	);
	return (
		<div className="flex flex-col">
			<div className="flex items-center gap-1">
				{ayuda ? <WithTooltip content={ayuda}>{cabecera}</WithTooltip> : cabecera}
				<WithTooltip content={t("Seleccionar estas cartas en el deck")}>
					<Button
						size="icon-xs"
						variant="ghost"
						disabled={ids.length === 0}
						onClick={() =>
							seleccionarEnDeck(
								ids,
								t("{n} cartas seleccionadas: {f}", { n: ids.length, f: titulo }),
							)
						}
					>
						<Icono icon="lucide:mouse-pointer-click" />
					</Button>
				</WithTooltip>
			</div>
			{abierta && cartas.length > 0 && (
				<ul className="mt-1 mb-1.5 ml-5 columns-1 gap-4 text-[11px] sm:columns-2">
					{cartas.map((c) => (
						<li key={c.name} className="flex break-inside-avoid gap-1.5 py-px">
							<span className="w-5 shrink-0 text-right font-mono text-muted-foreground tabular-nums">
								{c.count}
							</span>
							<span className="truncate">{c.name}</span>
						</li>
					))}
				</ul>
			)}
		</div>
	);
}
