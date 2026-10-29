import { forwardRef, type ReactNode } from "react";
import type { DeckKind } from "@/bindings/DeckKind";
import { Icono } from "@/components/icono";
import { paraTraducir, useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuLabel,
	DropdownMenuRadioGroup,
	DropdownMenuRadioItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Switch } from "@/components/ui/switch";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { COLORES } from "@/features/stats/textos";
import { cn } from "@/lib/utils";
import { type Agrupar, filtrosDelPanel, idiomasDe, type ModoColor, type Orden } from "./organizar";
import { ChipEtiqueta, CLASES, FORMATOS, IDIOMAS, Pip } from "./piezas";
import { ascendentePorDefecto, useAncla, useFiltros, usePrefs } from "./preferencias";
import { useBiblioteca } from "./store";

const ORDENES: [Orden, string][] = [
	["modificado", paraTraducir("Modificado")],
	["abierto", paraTraducir("Abierto")],
	["creado", paraTraducir("Creado")],
	["nombre", paraTraducir("Nombre")],
	["cartas", paraTraducir("Cartas")],
	["precio", paraTraducir("Precio")],
	["coste", paraTraducir("Coste medio")],
	["colores", paraTraducir("Colores")],
];

const AGRUPAR: [Agrupar, string][] = [
	["ninguno", paraTraducir("Sin agrupar")],
	["etiqueta", paraTraducir("Por etiqueta")],
	["color", paraTraducir("Por colores")],
	["clase", paraTraducir("Por clase")],
	["idioma", paraTraducir("Por idioma")],
];

/** La búsqueda, los filtros, el orden, los grupos y la vista. */
export const BarraHerramientas = forwardRef<
	HTMLInputElement,
	{ acciones: ReactNode; titulo: string }
>(function BarraHerramientas({ acciones, titulo }, buscar) {
	const { t } = useLang();
	const anclaBarra = useAncla("barra");
	const anclaAcciones = useAncla("acciones");
	const p = usePrefs();
	const { filtros: f, ponerFiltros } = useFiltros();
	return (
		<div
			className="flex flex-wrap items-center gap-2 border-b px-3 py-2"
			data-tutorial={anclaBarra}
		>
			<WithTooltip content={p.lateral ? t("Ocultar etiquetas") : t("Ver etiquetas")}>
				<Button
					size="icon-sm"
					variant="ghost"
					onClick={() => p.poner({ lateral: !p.lateral })}
					aria-label={t("Etiquetas")}
					className={cn(p.lateral && "text-primary")}
				>
					<Icono icon="lucide:panel-left" />
				</Button>
			</WithTooltip>
			<h2 className="text-sm font-semibold">{titulo}</h2>
			<div className="relative w-72">
				<Icono
					icon="lucide:search"
					className="pointer-events-none absolute top-2 left-2 size-3.5 text-muted-foreground"
				/>
				<Input
					ref={buscar}
					value={f.texto}
					onChange={(e) => ponerFiltros({ texto: e.target.value })}
					placeholder={t("Buscar decks, cartas, notas…")}
					className="h-7 pl-7 text-xs"
					type="search"
				/>
			</div>
			<PanelFiltros />

			<div className="mx-1 h-5 w-px bg-border" />

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button size="sm" variant="ghost" className="h-7 text-xs">
						{p.ascendente ? (
							<Icono icon="lucide:arrow-up-narrow-wide" />
						) : (
							<Icono icon="lucide:arrow-down-wide-narrow" />
						)}
						{t(ORDENES.find(([o]) => o === p.orden)?.[1] ?? "")}
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="start" className="text-xs">
					<DropdownMenuLabel className="text-[10px] text-muted-foreground uppercase">
						{t("Ordenar por")}
					</DropdownMenuLabel>
					<DropdownMenuRadioGroup
						value={p.orden}
						onValueChange={(v) => {
							const o = v as Orden;
							p.poner({ orden: o, ascendente: ascendentePorDefecto(o) });
						}}
					>
						{ORDENES.map(([o, nombre]) => (
							<DropdownMenuRadioItem key={o} value={o}>
								{t(nombre)}
							</DropdownMenuRadioItem>
						))}
					</DropdownMenuRadioGroup>
					<DropdownMenuSeparator />
					<DropdownMenuCheckboxItem
						checked={p.ascendente}
						onCheckedChange={(v) => p.poner({ ascendente: v === true })}
					>
						{t("Ascendente")}
					</DropdownMenuCheckboxItem>
					<DropdownMenuCheckboxItem
						checked={p.favoritosArriba}
						onCheckedChange={(v) => p.poner({ favoritosArriba: v === true })}
					>
						{t("Favoritos primero")}
					</DropdownMenuCheckboxItem>
				</DropdownMenuContent>
			</DropdownMenu>

			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button
						size="sm"
						variant="ghost"
						className={cn("h-7 text-xs", p.agrupar !== "ninguno" && "text-primary")}
					>
						<Icono icon="lucide:group" />
						{t(AGRUPAR.find(([a]) => a === p.agrupar)?.[1] ?? "")}
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="start" className="text-xs">
					<DropdownMenuRadioGroup
						value={p.agrupar}
						onValueChange={(v) => p.poner({ agrupar: v as Agrupar })}
					>
						{AGRUPAR.map(([a, nombre]) => (
							<DropdownMenuRadioItem key={a} value={a}>
								{t(nombre)}
							</DropdownMenuRadioItem>
						))}
					</DropdownMenuRadioGroup>
					{p.agrupar === "etiqueta" && (
						<p className="max-w-56 px-2 py-1.5 text-[10px] leading-snug text-muted-foreground">
							{t("Un deck con varias etiquetas sale en cada una.")}
						</p>
					)}
				</DropdownMenuContent>
			</DropdownMenu>

			<ToggleGroup
				type="single"
				size="sm"
				variant="outline"
				value={p.vista}
				onValueChange={(v) => v && p.poner({ vista: v as "cuadricula" | "lista" })}
			>
				<WithTooltip content={t("Cuadrícula")}>
					<ToggleGroupItem value="cuadricula" aria-label={t("Cuadrícula")} className="h-7">
						<Icono icon="lucide:layout-grid" />
					</ToggleGroupItem>
				</WithTooltip>
				<WithTooltip content={t("Lista")}>
					<ToggleGroupItem value="lista" aria-label={t("Lista")} className="h-7">
						<Icono icon="lucide:list" />
					</ToggleGroupItem>
				</WithTooltip>
			</ToggleGroup>
			{p.vista === "cuadricula" && (
				<ToggleGroup
					type="single"
					size="sm"
					value={p.tamano}
					onValueChange={(v) => v && p.poner({ tamano: v as "s" | "m" | "l" })}
					aria-label={t("Tamaño")}
				>
					{(["s", "m", "l"] as const).map((x) => (
						<WithTooltip
							key={x}
							content={x === "s" ? t("Pequeñas") : x === "m" ? t("Medianas") : t("Grandes")}
						>
							<ToggleGroupItem value={x} className="h-7 w-7 font-mono text-[10px] uppercase">
								{x}
							</ToggleGroupItem>
						</WithTooltip>
					))}
				</ToggleGroup>
			)}
			<div className="ml-auto flex items-center gap-1.5" data-tutorial={anclaAcciones}>
				{acciones}
			</div>
		</div>
	);
});

/** El desplegable de filtros: colores, clase, formato, idioma… */
function PanelFiltros() {
	const { t } = useLang();
	const { filtros: f, ponerFiltros } = useFiltros();
	const detalles = useBiblioteca((s) => s.detalles);
	const n = filtrosDelPanel(f);
	// Los idiomas que hay en la biblioteca.
	const decks = useBiblioteca((s) => s.decks);
	const idiomas = [...new Set((decks ?? []).flatMap((d) => idiomasDe(d, detalles[d.id])))].sort();
	const alternar = <T,>(lista: T[], x: T) =>
		lista.includes(x) ? lista.filter((y) => y !== x) : [...lista, x];
	return (
		<Popover>
			<PopoverTrigger asChild>
				<Button size="sm" variant={n > 0 ? "secondary" : "ghost"} className="h-7 text-xs">
					<Icono icon="lucide:filter" />
					{t("Filtros")}
					{n > 0 && (
						<span className="rounded-full bg-primary px-1.5 text-[10px] text-primary-foreground">
							{n}
						</span>
					)}
				</Button>
			</PopoverTrigger>
			<PopoverContent align="start" className="flex w-80 flex-col gap-3 text-xs">
				<Bloque titulo={t("Colores")}>
					<div className="flex items-center gap-1.5">
						{["W", "U", "B", "R", "G", "C"].map((c) => (
							<button
								type="button"
								key={c}
								onClick={() => ponerFiltros({ colores: alternar(f.colores, c) })}
								className={cn(
									"rounded-full transition-opacity",
									f.colores.includes(c)
										? "ring-2 ring-primary ring-offset-1 ring-offset-background"
										: "opacity-40 hover:opacity-80",
								)}
								aria-pressed={f.colores.includes(c)}
								aria-label={t(COLORES[c]?.nombre ?? c)}
							>
								<Pip color={c} className="size-6 text-[11px]" />
							</button>
						))}
					</div>
					<ToggleGroup
						type="single"
						size="sm"
						variant="outline"
						value={f.modoColor}
						onValueChange={(v) => v && ponerFiltros({ modoColor: v as ModoColor })}
						className="w-full"
					>
						<ToggleGroupItem value="incluye" className="flex-1 text-[11px]">
							{t("Los incluye")}
						</ToggleGroupItem>
						<ToggleGroupItem value="exacto" className="flex-1 text-[11px]">
							{t("Exactos")}
						</ToggleGroupItem>
						<ToggleGroupItem value="dentro" className="flex-1 text-[11px]">
							{t("Como mucho")}
						</ToggleGroupItem>
					</ToggleGroup>
					<p className="text-[10px] leading-snug text-muted-foreground">
						{f.modoColor === "incluye"
							? t("Decks que usan al menos esos colores.")
							: f.modoColor === "exacto"
								? t("Decks con justo esa identidad de color.")
								: t("Decks que no usan otros colores (caben en esa identidad).")}
					</p>
				</Bloque>
				<Bloque titulo={t("Clase")}>
					<div className="flex flex-wrap gap-1.5">
						{(Object.keys(CLASES) as DeckKind[]).map((k) => (
							<Button
								key={k}
								size="xs"
								variant={f.clases.includes(k) ? "default" : "outline"}
								onClick={() => ponerFiltros({ clases: alternar(f.clases, k) })}
							>
								{t(CLASES[k])}
							</Button>
						))}
					</div>
				</Bloque>
				<Bloque titulo={t("Legal en")}>
					<div className="flex flex-wrap gap-1">
						{Object.entries(FORMATOS).map(([k, nombre]) => (
							<Button
								key={k}
								size="xs"
								variant={f.formato === k ? "default" : "outline"}
								onClick={() => ponerFiltros({ formato: f.formato === k ? null : k })}
							>
								{nombre}
							</Button>
						))}
					</div>
				</Bloque>
				{idiomas.length > 0 && (
					<Bloque titulo={t("Idioma de las cartas")}>
						<div className="flex flex-wrap gap-1">
							{idiomas.map((l) => (
								<Button
									key={l}
									size="xs"
									variant={f.idioma === l ? "default" : "outline"}
									onClick={() => ponerFiltros({ idioma: f.idioma === l ? null : l })}
								>
									{t(IDIOMAS[l] ?? l.toUpperCase())}
								</Button>
							))}
						</div>
					</Bloque>
				)}
				<label className="flex items-center justify-between gap-2">
					{t("Con cartas diseñadas en la app")}
					<Switch checked={f.conDisenos} onCheckedChange={(v) => ponerFiltros({ conDisenos: v })} />
				</label>
				<label className="flex items-center justify-between gap-2">
					{t("Con imágenes propias")}
					<Switch checked={f.conPropias} onCheckedChange={(v) => ponerFiltros({ conPropias: v })} />
				</label>
				{n > 0 && (
					<Button
						size="sm"
						variant="ghost"
						className="self-start"
						onClick={() =>
							ponerFiltros({
								colores: [],
								clases: [],
								formato: null,
								idioma: null,
								conDisenos: false,
								conPropias: false,
							})
						}
					>
						<Icono icon="lucide:x" />
						{t("Quitar estos filtros")}
					</Button>
				)}
			</PopoverContent>
		</Popover>
	);
}

function Bloque({ titulo, children }: { titulo: string; children: ReactNode }) {
	return (
		<div className="flex flex-col gap-1.5">
			<span className="text-[10px] font-semibold tracking-widest text-muted-foreground uppercase">
				{titulo}
			</span>
			{children}
		</div>
	);
}

/** Los filtros puestos, como fichas que se quitan con la ×. */
export function FiltrosActivos() {
	const { t } = useLang();
	const { filtros: f, ponerFiltros, limpiar } = useFiltros();
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const fichas: { clave: string; contenido: ReactNode; quitar: () => void }[] = [];
	if (f.favoritos)
		fichas.push({
			clave: "fav",
			contenido: t("Favoritos"),
			quitar: () => ponerFiltros({ favoritos: false }),
		});
	if (f.sinEtiqueta)
		fichas.push({
			clave: "sin",
			contenido: t("Sin etiqueta"),
			quitar: () => ponerFiltros({ sinEtiqueta: false }),
		});
	for (const id of f.etiquetas) {
		const tag = etiquetas.find((x) => x.id === id);
		if (tag)
			fichas.push({
				clave: `t${id}`,
				contenido: <ChipEtiqueta tag={tag} />,
				quitar: () => ponerFiltros({ etiquetas: f.etiquetas.filter((x) => x !== id) }),
			});
	}
	if (f.colores.length > 0)
		fichas.push({
			clave: "col",
			contenido: (
				<span className="inline-flex items-center gap-1">
					{f.modoColor === "exacto"
						? t("Exactos")
						: f.modoColor === "dentro"
							? t("Como mucho")
							: t("Incluye")}
					{f.colores.map((c) => (
						<Pip key={c} color={c} />
					))}
				</span>
			),
			quitar: () => ponerFiltros({ colores: [] }),
		});
	for (const k of f.clases)
		fichas.push({
			clave: `k${k}`,
			contenido: t(CLASES[k]),
			quitar: () => ponerFiltros({ clases: f.clases.filter((x) => x !== k) }),
		});
	if (f.formato)
		fichas.push({
			clave: "fmt",
			contenido: t("Legal en {f}", { f: FORMATOS[f.formato] ?? f.formato }),
			quitar: () => ponerFiltros({ formato: null }),
		});
	if (f.idioma) {
		const idioma = f.idioma;
		fichas.push({
			clave: "idi",
			contenido: t("Idioma: {i}", { i: t(IDIOMAS[idioma] ?? idioma.toUpperCase()) }),
			quitar: () => ponerFiltros({ idioma: null }),
		});
	}
	if (f.conDisenos)
		fichas.push({
			clave: "dis",
			contenido: t("Con diseñadas"),
			quitar: () => ponerFiltros({ conDisenos: false }),
		});
	if (f.conPropias)
		fichas.push({
			clave: "pro",
			contenido: t("Con propias"),
			quitar: () => ponerFiltros({ conPropias: false }),
		});
	if (fichas.length === 0) return null;
	return (
		<div className="flex flex-wrap items-center gap-1.5 border-b bg-muted/30 px-3 py-1.5 text-[11px]">
			{fichas.map((x) => (
				<span
					key={x.clave}
					className="inline-flex items-center gap-1 rounded-full border bg-background py-0.5 pr-0.5 pl-2"
				>
					{x.contenido}
					<button
						type="button"
						onClick={x.quitar}
						className="rounded-full p-0.5 text-muted-foreground hover:bg-accent hover:text-foreground"
						aria-label={t("Quitar")}
					>
						<Icono icon="lucide:x" className="size-3" />
					</button>
				</span>
			))}
			{fichas.length > 1 && (
				<button
					type="button"
					onClick={() => limpiar()}
					className="text-muted-foreground underline-offset-2 hover:text-foreground hover:underline"
				>
					{t("Quitar todos")}
				</button>
			)}
		</div>
	);
}
