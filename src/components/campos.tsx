import { ChevronRight } from "lucide-react";
import { type ReactNode, useEffect, useId, useState } from "react";
import { Input } from "@/components/ui/input";
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";

/**
 * Los controles pequeños de los paneles (ajustes de exportación, opciones de
 * la lista): una fila con su etiqueta a la izquierda y el control a la
 * derecha, en la densidad de la casa (`text-xs`, `gap-2.5`).
 */

/** Una sección plegable; recuerda si estaba abierta. */
export function Seccion({
	id,
	titulo,
	icono,
	children,
	abiertaAlPrincipio = true,
	extra,
}: {
	id: string;
	titulo: string;
	icono?: ReactNode;
	children: ReactNode;
	abiertaAlPrincipio?: boolean;
	extra?: ReactNode;
}) {
	const clave = `proxybuilder.seccion.${id}`;
	const [abierta, setAbierta] = useState(() => {
		try {
			const v = localStorage.getItem(clave);
			return v === null ? abiertaAlPrincipio : v === "1";
		} catch {
			return abiertaAlPrincipio;
		}
	});
	const cambiar = () => {
		setAbierta(!abierta);
		try {
			localStorage.setItem(clave, abierta ? "0" : "1");
		} catch {
			/* sin almacenamiento */
		}
	};
	return (
		<section className="rounded-lg border bg-card/40">
			<div className="flex items-center gap-1.5 px-2.5 py-2">
				<button
					type="button"
					onClick={cambiar}
					aria-expanded={abierta}
					className="flex flex-1 items-center gap-1.5 text-left text-[11px] font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground"
				>
					<ChevronRight className={cn("size-3.5 transition-transform", abierta && "rotate-90")} />
					{icono}
					{titulo}
				</button>
				{extra}
			</div>
			{abierta && <div className="flex flex-col gap-2.5 px-2.5 pb-2.5">{children}</div>}
		</section>
	);
}

export function Fila({
	etiqueta,
	ayuda,
	children,
	htmlFor,
	className,
}: {
	etiqueta: string;
	ayuda?: string;
	children: ReactNode;
	htmlFor?: string;
	className?: string;
}) {
	const texto = (
		<label htmlFor={htmlFor} className="min-w-0 flex-1 truncate text-xs text-foreground/90">
			{etiqueta}
		</label>
	);
	return (
		<div className={cn("flex min-h-7 items-center gap-2.5", className)}>
			{ayuda ? <WithTooltip content={ayuda}>{texto}</WithTooltip> : texto}
			<div className="flex shrink-0 items-center gap-1.5">{children}</div>
		</div>
	);
}

/**
 * Un número con sufijo («mm», «%»). Se escribe libre y se confirma al salir
 * o con Enter; lo que no es número o se sale del rango vuelve a lo que había.
 */
export function CampoNumero({
	valor,
	onChange,
	min,
	max,
	paso = 0.1,
	sufijo,
	ancho = "w-20",
	id,
	vacioEsNulo,
	placeholder,
	ariaLabel,
}: {
	valor: number | null;
	onChange: (v: number | null) => void;
	min?: number;
	max?: number;
	paso?: number;
	sufijo?: string;
	ancho?: string;
	id?: string;
	/** Dejarlo vacío vale `null` (p. ej. «Partir cada N hojas»). */
	vacioEsNulo?: boolean;
	placeholder?: string;
	ariaLabel?: string;
}) {
	const [texto, setTexto] = useState(valor === null ? "" : String(valor));
	useEffect(() => setTexto(valor === null ? "" : String(valor)), [valor]);

	const confirmar = () => {
		const limpio = texto.trim().replace(",", ".");
		if (limpio === "" && vacioEsNulo) {
			if (valor !== null) onChange(null);
			return;
		}
		const n = Number(limpio);
		if (!Number.isFinite(n) || limpio === "") return setTexto(valor === null ? "" : String(valor));
		const acotado = Math.min(max ?? n, Math.max(min ?? n, n));
		const redondeado = Math.round(acotado / paso) * paso;
		const final = Number(redondeado.toFixed(4));
		setTexto(String(final));
		if (final !== valor) onChange(final);
	};

	return (
		<div className="relative">
			<Input
				id={id}
				inputMode="decimal"
				value={texto}
				placeholder={placeholder}
				aria-label={ariaLabel}
				onChange={(e) => setTexto(e.target.value)}
				onBlur={confirmar}
				onKeyDown={(e) => {
					if (e.key === "Enter") confirmar();
					if (e.key === "ArrowUp" || e.key === "ArrowDown") {
						e.preventDefault();
						const base = valor ?? min ?? 0;
						const n = base + (e.key === "ArrowUp" ? paso : -paso) * (e.shiftKey ? 10 : 1);
						const acotado = Math.min(max ?? n, Math.max(min ?? n, n));
						onChange(Number(acotado.toFixed(4)));
					}
				}}
				className={cn("h-7 px-2 text-right font-mono text-xs", ancho)}
				// Hueco para el sufijo, a la medida de su texto.
				style={sufijo ? { paddingRight: `${1 + sufijo.length * 0.45}rem` } : undefined}
			/>
			{sufijo && (
				<span className="pointer-events-none absolute inset-y-0 right-2 flex items-center text-[10px] text-muted-foreground">
					{sufijo}
				</span>
			)}
		</div>
	);
}

export function FilaInterruptor({
	etiqueta,
	ayuda,
	valor,
	onChange,
	deshabilitado,
}: {
	etiqueta: string;
	ayuda?: string;
	valor: boolean;
	onChange: (v: boolean) => void;
	deshabilitado?: boolean;
}) {
	const id = useId();
	return (
		<Fila etiqueta={etiqueta} ayuda={ayuda} htmlFor={id}>
			<Switch id={id} checked={valor} onCheckedChange={onChange} disabled={deshabilitado} />
		</Fila>
	);
}

export type Opcion<V extends string> = { valor: V; etiqueta: string; grupo?: string };

export function Selector<V extends string>({
	valor,
	opciones,
	onChange,
	ancho = "w-36",
	ariaLabel,
}: {
	valor: V;
	opciones: Opcion<V>[];
	onChange: (v: V) => void;
	ancho?: string;
	ariaLabel?: string;
}) {
	const grupos = new Map<string, Opcion<V>[]>();
	for (const o of opciones) {
		const g = o.grupo ?? "";
		grupos.set(g, [...(grupos.get(g) ?? []), o]);
	}
	return (
		<Select value={valor} onValueChange={(v) => onChange(v as V)}>
			<SelectTrigger size="sm" className={cn("h-7 text-xs", ancho)} aria-label={ariaLabel}>
				<SelectValue />
			</SelectTrigger>
			<SelectContent>
				{[...grupos].map(([g, ops]) => (
					<SelectGroup key={g || "_"}>
						{g && <SelectLabel className="text-[10px] uppercase">{g}</SelectLabel>}
						{ops.map((o) => (
							<SelectItem key={o.valor} value={o.valor} className="text-xs">
								{o.etiqueta}
							</SelectItem>
						))}
					</SelectGroup>
				))}
			</SelectContent>
		</Select>
	);
}

/** Un deslizador con su número al lado. */
export function FilaDeslizador({
	etiqueta,
	valor,
	onChange,
	onSoltar,
	min,
	max,
	paso = 1,
	sufijo = "",
}: {
	etiqueta: string;
	valor: number;
	onChange: (v: number) => void;
	/** Al soltar el deslizador (para lo caro: pedir un render). */
	onSoltar?: (v: number) => void;
	min: number;
	max: number;
	paso?: number;
	sufijo?: string;
}) {
	return (
		<div className="flex flex-col gap-1.5">
			<div className="flex items-center justify-between text-xs">
				<span className="text-foreground/90">{etiqueta}</span>
				<span className="font-mono text-[11px] text-muted-foreground">
					{Number(valor.toFixed(2))}
					{sufijo}
				</span>
			</div>
			<Slider
				value={[valor]}
				min={min}
				max={max}
				step={paso}
				onValueChange={([v]) => onChange(v)}
				onValueCommit={([v]) => onSoltar?.(v)}
				aria-label={etiqueta}
			/>
		</div>
	);
}
