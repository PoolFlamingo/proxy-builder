import {
	Copy,
	FolderOpen,
	Languages,
	PackageOpen,
	Pencil,
	Plus,
	Settings2,
	Star,
	StarOff,
	Tag as TagIcon,
	Trash2,
} from "lucide-react";
import { type ComponentType, createContext, type ReactNode, useContext } from "react";
import type { DeckSummary } from "@/bindings/DeckSummary";
import { useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuCheckboxItem,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
} from "@/components/ui/dropdown-menu";
import { IDIOMAS, PuntoEtiqueta } from "./piezas";
import { useBiblioteca } from "./store";

/** Lo que se puede hacer con los decks: lo pone la pantalla. */
export type Acciones = {
	abrir: (id: string) => void;
	propiedades: (id: string) => void;
	renombrar: (d: DeckSummary) => void;
	duplicar: (d: DeckSummary) => void;
	exportar: (d: DeckSummary) => void;
	borrar: (ids: string[]) => void;
	/** Crear una etiqueta y ponérsela a estos decks. */
	nuevaEtiqueta: (ids: string[]) => void;
	/** Los decks seleccionados (si el pulsado está entre ellos, la acción va a todos). */
	seleccion: string[];
};

export const AccionesDeck = createContext<Acciones | null>(null);

function useAcciones(): Acciones {
	const a = useContext(AccionesDeck);
	if (!a) throw new Error("AccionesDeck sin proveedor");
	return a;
}

type Primitivas = {
	Item: ComponentType<{
		onSelect?: () => void;
		variant?: "default" | "destructive";
		disabled?: boolean;
		children: ReactNode;
	}>;
	Separator: ComponentType;
	Sub: ComponentType<{ children: ReactNode }>;
	SubTrigger: ComponentType<{ children: ReactNode }>;
	SubContent: ComponentType<{ className?: string; children: ReactNode }>;
	CheckboxItem: ComponentType<{
		checked?: boolean | "indeterminate";
		onSelect?: (e: Event) => void;
		children: ReactNode;
	}>;
};

const DESPLEGABLE: Primitivas = {
	Item: DropdownMenuItem,
	Separator: DropdownMenuSeparator,
	Sub: DropdownMenuSub,
	SubTrigger: DropdownMenuSubTrigger,
	SubContent: DropdownMenuSubContent,
	CheckboxItem: DropdownMenuCheckboxItem,
};

const CONTEXTUAL: Primitivas = {
	Item: ContextMenuItem,
	Separator: ContextMenuSeparator,
	Sub: ContextMenuSub,
	SubTrigger: ContextMenuSubTrigger,
	SubContent: ContextMenuSubContent,
	CheckboxItem: ContextMenuCheckboxItem,
};

/** Lo que hay en el menú de un deck (o de la selección, si es parte de ella). */
function Items({ deck, M }: { deck: DeckSummary; M: Primitivas }) {
	const { t } = useLang();
	const a = useAcciones();
	const decks = useBiblioteca((s) => s.decks) ?? [];
	const etiquetas = useBiblioteca((s) => s.etiquetas);
	const ids = a.seleccion.includes(deck.id) && a.seleccion.length > 1 ? a.seleccion : [deck.id];
	const varios = ids.length > 1;
	const elegidos = decks.filter((d) => ids.includes(d.id));
	const todosFavoritos = elegidos.length > 0 && elegidos.every((d) => d.pinned);
	const marca = (tag: string): boolean | "indeterminate" => {
		const n = elegidos.filter((d) => d.tags.includes(tag)).length;
		return n === 0 ? false : n === elegidos.length ? true : "indeterminate";
	};
	const lib = useBiblioteca.getState();
	return (
		<>
			{!varios && (
				<M.Item onSelect={() => a.abrir(deck.id)}>
					<FolderOpen />
					{t("Abrir")}
				</M.Item>
			)}
			<M.Item onSelect={() => void lib.favorito(ids, !todosFavoritos)}>
				{todosFavoritos ? <StarOff /> : <Star />}
				{todosFavoritos ? t("Quitar de favoritos") : t("Añadir a favoritos")}
			</M.Item>
			<M.Sub>
				<M.SubTrigger>
					<TagIcon />
					{varios ? t("Etiquetas de los {n}", { n: ids.length }) : t("Etiquetas")}
				</M.SubTrigger>
				<M.SubContent className="max-h-80 min-w-44 text-xs">
					{etiquetas.map((tag) => {
						const m = marca(tag.id);
						return (
							<M.CheckboxItem
								key={tag.id}
								checked={m}
								onSelect={(e) => {
									// El menú sigue abierto para poner varias.
									e.preventDefault();
									void lib.etiquetar(ids, m === true ? [] : [tag.id], m === true ? [tag.id] : []);
								}}
							>
								<PuntoEtiqueta color={tag.color} />
								<span className="truncate">{tag.name}</span>
							</M.CheckboxItem>
						);
					})}
					{etiquetas.length > 0 && <M.Separator />}
					<M.Item onSelect={() => a.nuevaEtiqueta(ids)}>
						<Plus />
						{t("Nueva etiqueta…")}
					</M.Item>
				</M.SubContent>
			</M.Sub>
			<M.Sub>
				<M.SubTrigger>
					<Languages />
					{varios ? t("Idioma de los {n}", { n: ids.length }) : t("Idioma")}
				</M.SubTrigger>
				<M.SubContent className="max-h-80 min-w-44 text-xs">
					<M.CheckboxItem
						checked={elegidos.every((d) => !d.langOverride)}
						onSelect={() => void lib.idioma(ids, null)}
					>
						{t("Automático (según las cartas)")}
					</M.CheckboxItem>
					<M.Separator />
					{Object.entries(IDIOMAS).map(([codigo, nombre]) => (
						<M.CheckboxItem
							key={codigo}
							checked={
								elegidos.length > 0 && elegidos.every((d) => d.langOverride === codigo)
									? true
									: elegidos.some((d) => d.langOverride === codigo)
										? "indeterminate"
										: false
							}
							onSelect={() => void lib.idioma(ids, codigo)}
						>
							{t(nombre)}
						</M.CheckboxItem>
					))}
				</M.SubContent>
			</M.Sub>
			{!varios && (
				<>
					<M.Item onSelect={() => a.propiedades(deck.id)}>
						<Settings2 />
						{t("Propiedades…")}
					</M.Item>
					<M.Item onSelect={() => a.renombrar(deck)}>
						<Pencil />
						{t("Renombrar")}
					</M.Item>
					<M.Item onSelect={() => a.duplicar(deck)}>
						<Copy />
						{t("Duplicar")}
					</M.Item>
					<M.Item onSelect={() => a.exportar(deck)}>
						<PackageOpen />
						{t("Exportar .pbdeck")}
					</M.Item>
				</>
			)}
			<M.Separator />
			<M.Item variant="destructive" onSelect={() => a.borrar(ids)}>
				<Trash2 />
				{varios ? t("Eliminar los {n}", { n: ids.length }) : t("Eliminar")}
			</M.Item>
		</>
	);
}

/** El contenido del desplegable «⋯» de un deck. */
export function MenuDeck({ deck }: { deck: DeckSummary }) {
	return (
		<DropdownMenuContent align="end" className="text-xs">
			<Items deck={deck} M={DESPLEGABLE} />
		</DropdownMenuContent>
	);
}

/** El menú del botón derecho alrededor de un deck. */
export function MenuContextualDeck({ deck, children }: { deck: DeckSummary; children: ReactNode }) {
	return (
		<ContextMenu>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContextMenuContent className="text-xs">
				<Items deck={deck} M={CONTEXTUAL} />
			</ContextMenuContent>
		</ContextMenu>
	);
}
