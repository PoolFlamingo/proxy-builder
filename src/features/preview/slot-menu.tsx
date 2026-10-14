import {
	ArrowLeft,
	ArrowRight,
	Brush,
	Copy,
	Eraser,
	Hash,
	ImageUp,
	Link2,
	Lock,
	LockOpen,
	Palette,
	Replace,
	SlidersHorizontal,
	SquareStack,
	Trash2,
} from "lucide-react";
import type { ReactNode } from "react";
import { toast } from "sonner";
import { useLang } from "@/components/language-provider";
import {
	ContextMenu,
	ContextMenuContent,
	ContextMenuItem,
	ContextMenuSeparator,
	ContextMenuShortcut,
	ContextMenuSub,
	ContextMenuSubContent,
	ContextMenuSubTrigger,
	ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { quitarDiseno, scryfallDe } from "@/features/disenar/grupos";
import { useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";
import {
	cambiarReverso,
	cartaPorId,
	duplicarCartas,
	eliminarCartas,
	fijarCartas,
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
						{t("Vincular con Scryfall las seleccionadas…")}
					</ContextMenuItem>
				)}
				{carta?.source.kind === "custom" && !varias && (
					<ContextMenuItem
						disabled={!carta.front}
						onSelect={() => useUi.getState().abrir({ tipo: "vincular", id })}
					>
						<Link2 />
						{carta.source.scryfallId
							? t("Cambiar el vínculo con Scryfall…")
							: t("Vincular con Scryfall…")}
					</ContextMenuItem>
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
