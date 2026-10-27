import {
	Layers,
	MoreHorizontal,
	Palette,
	Pencil,
	Plus,
	Star,
	TagIcon,
	Tags,
	Trash2,
} from "lucide-react";
import { type ReactNode, useState } from "react";
import type { Tag } from "@/bindings/Tag";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { WithTooltip } from "@/components/ui/with-tooltip";
import { cn } from "@/lib/utils";
import { contar } from "./organizar";
import { COLORES_DE_ETIQUETA, COLORES_ETIQUETA, PuntoEtiqueta } from "./piezas";
import { useAncla, useFiltros } from "./preferencias";
import { useBiblioteca } from "./store";

/**
 * La columna de la izquierda: todos, favoritos y cada etiqueta con cuántos
 * decks tiene. Pulsar filtra; con Ctrl se eligen varias etiquetas.
 */
export function BarraLateral({
	onNuevaEtiqueta,
	onEditarEtiqueta,
	onBorrarEtiqueta,
}: {
	onNuevaEtiqueta: () => void;
	onEditarEtiqueta: (t: Tag) => void;
	onBorrarEtiqueta: (t: Tag) => void;
}) {
	const { t } = useLang();
	const decks = useBiblioteca((s) => s.decks) ?? [];
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const { filtros: f, ponerFiltros } = useFiltros();
	const ancla = useAncla("lateral");
	const n = contar(decks);
	const ninguno = !f.favoritos && !f.sinEtiqueta && f.etiquetas.length === 0;
	const elegirEtiqueta = (id: string, e: React.MouseEvent) => {
		const varias = e.ctrlKey || e.metaKey;
		const ya = f.etiquetas.includes(id);
		ponerFiltros({
			sinEtiqueta: false,
			etiquetas: varias
				? ya
					? f.etiquetas.filter((x) => x !== id)
					: [...f.etiquetas, id]
				: ya && f.etiquetas.length === 1
					? []
					: [id],
		});
	};
	return (
		<aside
			className="flex h-full w-56 shrink-0 flex-col border-r bg-sidebar/40"
			data-tutorial={ancla}
		>
			<ScrollArea className="min-h-0 flex-1">
				<div className="flex flex-col gap-0.5 p-2">
					<Entrada
						activa={ninguno}
						icono={<Layers className="size-3.5" />}
						texto={t("Todos los decks")}
						cuenta={decks.length}
						onClick={() => ponerFiltros({ favoritos: false, sinEtiqueta: false, etiquetas: [] })}
					/>
					<Entrada
						activa={f.favoritos}
						icono={<Star className="size-3.5" />}
						texto={t("Favoritos")}
						cuenta={n.favoritos}
						onClick={() => ponerFiltros({ favoritos: !f.favoritos })}
					/>

					<div className="mt-3 flex items-center gap-1 px-2 pb-1">
						<span className="flex-1 text-[10px] font-semibold tracking-widest text-muted-foreground uppercase">
							{t("Etiquetas")}
						</span>
						<WithTooltip content={t("Nueva etiqueta")}>
							<Button
								size="icon-xs"
								variant="ghost"
								onClick={onNuevaEtiqueta}
								aria-label={t("Nueva etiqueta")}
							>
								<Plus />
							</Button>
						</WithTooltip>
					</div>
					{etiquetas.length === 0 ? (
						<button
							type="button"
							onClick={onNuevaEtiqueta}
							className="mx-1 flex flex-col items-center gap-1.5 rounded-lg border border-dashed p-3 text-center text-[11px] text-muted-foreground hover:bg-accent/50"
						>
							<Tags className="size-4" />
							{t("Crea etiquetas para ordenar tus decks por formato, idioma, estado…")}
						</button>
					) : (
						etiquetas.map((tag) => (
							<FilaEtiqueta
								key={tag.id}
								tag={tag}
								activa={f.etiquetas.includes(tag.id)}
								cuenta={n.porEtiqueta.get(tag.id) ?? 0}
								onClick={(e) => elegirEtiqueta(tag.id, e)}
								onEditar={() => onEditarEtiqueta(tag)}
								onBorrar={() => onBorrarEtiqueta(tag)}
							/>
						))
					)}
					{etiquetas.length > 0 && (
						<Entrada
							activa={f.sinEtiqueta}
							icono={<TagIcon className="size-3.5 opacity-50" />}
							texto={t("Sin etiqueta")}
							cuenta={n.sinEtiqueta}
							onClick={() => ponerFiltros({ sinEtiqueta: !f.sinEtiqueta, etiquetas: [] })}
						/>
					)}
					{f.etiquetas.length > 1 && (
						<div className="mt-2 flex flex-col gap-1.5 rounded-lg border bg-card p-2">
							<span className="text-[10px] text-muted-foreground">
								{t("Con {n} etiquetas elegidas, enseñar los que tengan:", {
									n: f.etiquetas.length,
								})}
							</span>
							<ToggleGroup
								type="single"
								size="sm"
								variant="outline"
								value={f.todasLasEtiquetas ? "todas" : "alguna"}
								onValueChange={(v) => v && ponerFiltros({ todasLasEtiquetas: v === "todas" })}
								className="w-full"
							>
								<ToggleGroupItem value="alguna" className="flex-1 text-[11px]">
									{t("Alguna")}
								</ToggleGroupItem>
								<ToggleGroupItem value="todas" className="flex-1 text-[11px]">
									{t("Todas")}
								</ToggleGroupItem>
							</ToggleGroup>
						</div>
					)}
					{etiquetas.length > 0 && f.etiquetas.length <= 1 && (
						<p className="px-2 pt-2 text-[10px] leading-snug text-muted-foreground">
							{t("Ctrl + clic para elegir varias.")}
						</p>
					)}
				</div>
			</ScrollArea>
		</aside>
	);
}

function Entrada({
	activa,
	icono,
	texto,
	cuenta,
	onClick,
}: {
	activa: boolean;
	icono: ReactNode;
	texto: string;
	cuenta: number;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"flex items-center gap-2 rounded-md px-2 py-1.5 text-left text-xs hover:bg-accent",
				activa && "bg-accent font-medium text-accent-foreground",
			)}
		>
			<span className="text-muted-foreground">{icono}</span>
			<span className="min-w-0 flex-1 truncate">{texto}</span>
			<span className="font-mono text-[10px] text-muted-foreground tabular-nums">{cuenta}</span>
		</button>
	);
}

function FilaEtiqueta({
	tag,
	activa,
	cuenta,
	onClick,
	onEditar,
	onBorrar,
}: {
	tag: Tag;
	activa: boolean;
	cuenta: number;
	onClick: (e: React.MouseEvent) => void;
	onEditar: () => void;
	onBorrar: () => void;
}) {
	const { t } = useLang();
	const editar = useBiblioteca((s) => s.editarEtiqueta);
	const [menu, setMenu] = useState(false);
	return (
		<div
			className={cn(
				"group flex items-center rounded-md text-xs hover:bg-accent",
				activa && "bg-accent font-medium text-accent-foreground",
			)}
		>
			<button
				type="button"
				onClick={onClick}
				className="flex min-w-0 flex-1 items-center gap-2 py-1.5 pl-2 text-left"
			>
				<PuntoEtiqueta color={tag.color} className="mx-[3px]" />
				<span className="min-w-0 flex-1 truncate">{tag.name}</span>
			</button>
			<span
				className={cn(
					"px-1 font-mono text-[10px] text-muted-foreground tabular-nums group-hover:hidden",
					menu && "hidden",
				)}
			>
				{cuenta}
			</span>
			<DropdownMenu onOpenChange={setMenu}>
				<DropdownMenuTrigger asChild>
					<Button
						size="icon-xs"
						variant="ghost"
						className="mr-0.5 hidden group-hover:inline-flex data-[state=open]:inline-flex"
						aria-label={t("Más acciones")}
					>
						<MoreHorizontal />
					</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="start" className="text-xs">
					<DropdownMenuItem onSelect={onEditar}>
						<Pencil />
						{t("Renombrar")}
					</DropdownMenuItem>
					<DropdownMenuSub>
						<DropdownMenuSubTrigger>
							<Palette />
							{t("Color")}
						</DropdownMenuSubTrigger>
						<DropdownMenuSubContent className="text-xs">
							{COLORES_DE_ETIQUETA.map((c) => (
								<DropdownMenuItem key={c} onSelect={() => void editar(tag.id, tag.name, c)}>
									<PuntoEtiqueta color={c} className="mx-1" />
									{t(COLORES_ETIQUETA[c].nombre)}
								</DropdownMenuItem>
							))}
						</DropdownMenuSubContent>
					</DropdownMenuSub>
					<DropdownMenuSeparator />
					<DropdownMenuItem variant="destructive" onSelect={onBorrar}>
						<Trash2 />
						{t("Eliminar etiqueta")}
					</DropdownMenuItem>
				</DropdownMenuContent>
			</DropdownMenu>
		</div>
	);
}
