import {
	ArrowLeft,
	ArrowRight,
	Brush,
	Copy,
	Eraser,
	Hash,
	ImageUp,
	Languages,
	Link2,
	Lock,
	LockOpen,
	Palette,
	Replace,
	Search,
	SlidersHorizontal,
	SquareStack,
	Tag,
	Trash2,
} from "lucide-react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import type { MarcaCarta } from "@/bindings/MarcaCarta";
import { useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuRadioGroup,
	ContextMenuRadioItem,
	ContextMenuSeparator,
	ContextMenuShortcut,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { quitarDiseno, scryfallDe } from "@/features/disenar/grupos";
import { IDIOMAS } from "@/features/library/piezas";
import { MARCAS } from "@/lib/cartas";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import {
	anadirEnHueco,
	cambiarReverso,
	cartaPorId,
	duplicarCartas,
	eliminarCartas,
	fijarCartas,
	idiomaDeImagen,
	marcarCartas,
	moverUno,
	objetivo,
	reemplazarImagen,
	reversoDelDeck,
} from "./acciones-carta";

/**
 * El menú del clic derecho de una carta (captura 2 de fabricard). Actúa sobre
 * la selección si la carta está en ella; si no, solo sobre esa carta (y la
 * selecciona).
 */
export function SlotMenu({ id, children }: { id: string; children: ReactNode }) {
	const { t, mensaje } = useLang();
	const intentar = (f: () => Promise<void>) => () => {
		f().catch((e) => toast.error(mensaje(e)));
	};
	const abrirMenu = (abierto: boolean) => {
		if (!abierto) return;
		const ui = useUi.getState();
		if (!ui.seleccion.includes(id)) ui.seleccionar([id], id);
	};
	const ids = () => objetivo(id);
	const carta = cartaPorId(id);
	const varias = useUi((s) => s.seleccion.length > 1 && s.seleccion.includes(id));
	const esScryfall = carta?.source.kind === "scryfall" || carta?.source.kind === "token";
	const fijada = carta?.locked ?? false;
	const disenable = !!carta && scryfallDe(carta) !== null;
	const quitarDisenos = () => {
		const quitar = new Set(ids());
		useDeck.getState().setCartas((cs) => quitarDiseno(cs, quitar));
	};

	return (
		<ContextMenu onOpenChange={abrirMenu}>
			<ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
			<ContextMenuContent className="w-72 text-xs">
				{carta?.source.kind === "blank" && !varias && (
					<>
						<ContextMenuItem onSelect={() => anadirEnHueco(id)}>
							<Search />
							{t("Añadir carta de Scryfall…")}
						</ContextMenuItem>
						<ContextMenuSeparator />
					</>
				)}
				<ContextMenuItem onSelect={() => duplicarCartas(ids())}>
					<Copy />
					{t("Duplicar")}
					<ContextMenuShortcut>Ctrl+D</ContextMenuShortcut>
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !carta?.front}
					onSelect={() => useUi.getState().abrir({ tipo: "editar", id })}
				>
					<SlidersHorizontal />
					{t("Editar carta")}
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !esScryfall}
					onSelect={() => useUi.getState().abrir({ tipo: "variante", id })}
				>
					<Palette />
					{t("Cambiar variante")}
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !disenable}
					onSelect={() => useUi.getState().disenar(id)}
				>
					<Brush />
					{carta?.design ? t("Editar diseño") : t("Diseñar proxy")}
				</ContextMenuItem>
				{carta?.design && (
					<ContextMenuItem onSelect={quitarDisenos}>
						<Eraser />
						{varias ? t("Quitar el diseño de las seleccionadas") : t("Quitar diseño")}
					</ContextMenuItem>
				)}
				{varias && (
					<ContextMenuItem
						onSelect={() =>
							useUi.getState().abrir({ tipo: "vincularLote", ids: ids(), soloSeleccion: true })
						}
					>
						<Link2 />
						{t("Vincular las seleccionadas…")}
					</ContextMenuItem>
				)}
				{carta?.source.kind === "custom" && !varias && (
					<ContextMenuItem
						disabled={!carta.front}
						onSelect={() => useUi.getState().abrir({ tipo: "vincular", id })}
					>
						<Link2 />
						{carta.source.scryfallId ? t("Cambiar el vínculo…") : t("Vincular con una carta…")}
					</ContextMenuItem>
				)}
				{carta?.source.kind === "custom" && (
					<ContextMenuSub>
						<ContextMenuSubTrigger>
							<Languages />
							{t("Idioma de la imagen")}
						</ContextMenuSubTrigger>
						<ContextMenuSubContent className="text-xs">
							<ContextMenuRadioGroup
								value={carta.meta.imageLang ?? ""}
								onValueChange={(v) => idiomaDeImagen(ids(), v || null)}
							>
								<ContextMenuRadioItem value="">{t("Sin indicar")}</ContextMenuRadioItem>
								{Object.entries(IDIOMAS).map(([codigo, nombre]) => (
									<ContextMenuRadioItem key={codigo} value={codigo}>
										{t(nombre)}
									</ContextMenuRadioItem>
								))}
							</ContextMenuRadioGroup>
						</ContextMenuSubContent>
					</ContextMenuSub>
				)}
				{carta && carta.source.kind !== "blank" && (
					<ContextMenuSub>
						<ContextMenuSubTrigger>
							<Tag />
							{t("Marca")}
						</ContextMenuSubTrigger>
						<ContextMenuSubContent className="max-w-64 text-xs">
							<ContextMenuRadioGroup
								value={carta.marca ?? ""}
								onValueChange={(v) => marcarCartas(ids(), (v || null) as MarcaCarta | null)}
							>
								<ContextMenuRadioItem value="">{t("Ninguna")}</ContextMenuRadioItem>
								{(Object.keys(MARCAS) as MarcaCarta[]).map((m) => (
									<ContextMenuRadioItem key={m} value={m}>
										{t(MARCAS[m].nombre)}
									</ContextMenuRadioItem>
								))}
							</ContextMenuRadioGroup>
							<p className="px-2 py-1.5 text-[10px] text-muted-foreground">
								{t(
									"Se imprimen igual; no cuentan para el tamaño del mazo, las copias ni las estadísticas.",
								)}
							</p>
						</ContextMenuSubContent>
					</ContextMenuSub>
				)}
				<ContextMenuItem disabled={varias} onSelect={intentar(() => reemplazarImagen(id, t))}>
					<ImageUp />
					{t("Reemplazar imagen…")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>
						<SquareStack />
						{t("Cambiar reverso")}
					</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						<ContextMenuItem onSelect={intentar(() => cambiarReverso(ids(), t))}>
							<Replace />
							{t("Elegir imagen…")}
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => reversoDelDeck(ids())}>
							{t("Usar el reverso del deck")}
						</ContextMenuItem>
					</ContextMenuSubContent>
				</ContextMenuSub>
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => moverUno(ids(), -1)}>
					<ArrowLeft />
					{t("Mover antes")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => moverUno(ids(), 1)}>
					<ArrowRight />
					{t("Mover después")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => useUi.getState().abrir({ tipo: "moverA", ids: ids() })}>
					<Hash />
					{t("Mover a posición…")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => fijarCartas(ids(), !fijada)}>
					{fijada ? <LockOpen /> : <Lock />}
					{fijada ? t("Desfijar") : t("Fijar")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem variant="destructive" onSelect={() => eliminarCartas(ids())}>
					<Trash2 />
					{varias ? t("Eliminar las seleccionadas") : t("Eliminar")}
					<ContextMenuShortcut>Supr</ContextMenuShortcut>
				</ContextMenuItem>
			</ContextMenuContent>
		</ContextMenu>
	);
}
