import type { ReactNode } from "react";
import { toast } from "sonner";
import type { MarcaCarta } from "@/bindings/MarcaCarta";
import { ContenidoContextual } from "@/components/contenido-contextual";
import { Icono } from "@/components/icono";
import { useLang } from "@/components/language-provider";
import {
	ContextMenu,
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
	exportarImagen,
	fijarCartas,
	idiomaDeImagen,
	marcarCartas,
	moverUno,
	objetivo,
	reemplazarImagen,
	reversoDelDeck,
	tieneReverso,
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
			<ContenidoContextual className="w-72 text-xs">
				{carta?.source.kind === "blank" && !varias && (
					<>
						<ContextMenuItem onSelect={() => anadirEnHueco(id)}>
							<Icono icon="lucide:search" />
							{t("Añadir carta de Scryfall…")}
						</ContextMenuItem>
						<ContextMenuSeparator />
					</>
				)}
				<ContextMenuItem onSelect={() => duplicarCartas(ids())}>
					<Icono icon="lucide:copy" />
					{t("Duplicar")}
					<ContextMenuShortcut>Ctrl+D</ContextMenuShortcut>
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !carta?.front}
					onSelect={() => useUi.getState().abrir({ tipo: "editar", id })}
				>
					<Icono icon="lucide:sliders-horizontal" />
					{t("Editar carta")}
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !esScryfall}
					onSelect={() => useUi.getState().abrir({ tipo: "variante", id })}
				>
					<Icono icon="lucide:palette" />
					{t("Cambiar variante")}
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !esScryfall}
					onSelect={() => useUi.getState().abrir({ tipo: "anadir", cambiar: id })}
				>
					<Icono icon="lucide:repeat-2" />
					{t("Cambiar carta…")}
				</ContextMenuItem>
				<ContextMenuItem
					disabled={varias || !disenable}
					onSelect={() => useUi.getState().disenar(id)}
				>
					<Icono icon="lucide:brush" />
					{carta?.design ? t("Editar diseño") : t("Diseñar proxy")}
				</ContextMenuItem>
				{carta?.design && (
					<ContextMenuItem onSelect={quitarDisenos}>
						<Icono icon="lucide:eraser" />
						{varias ? t("Quitar el diseño de las seleccionadas") : t("Quitar diseño")}
					</ContextMenuItem>
				)}
				{varias && (
					<ContextMenuItem
						onSelect={() =>
							useUi.getState().abrir({ tipo: "vincularLote", ids: ids(), soloSeleccion: true })
						}
					>
						<Icono icon="lucide:link-2" />
						{t("Vincular las seleccionadas…")}
					</ContextMenuItem>
				)}
				{carta?.source.kind === "custom" && !varias && (
					<ContextMenuItem
						disabled={!carta.front}
						onSelect={() => useUi.getState().abrir({ tipo: "vincular", id })}
					>
						<Icono icon="lucide:link-2" />
						{carta.source.scryfallId ? t("Cambiar el vínculo…") : t("Vincular con una carta…")}
					</ContextMenuItem>
				)}
				{carta?.source.kind === "custom" && (
					<ContextMenuSub>
						<ContextMenuSubTrigger>
							<Icono icon="lucide:languages" />
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
							<Icono icon="lucide:tag" />
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
					<Icono icon="lucide:image-up" />
					{t("Reemplazar imagen…")}
				</ContextMenuItem>
				<ContextMenuSub>
					<ContextMenuSubTrigger>
						<Icono icon="lucide:square-stack" />
						{t("Cambiar reverso")}
					</ContextMenuSubTrigger>
					<ContextMenuSubContent className="text-xs">
						<ContextMenuItem onSelect={intentar(() => cambiarReverso(ids(), t))}>
							<Icono icon="lucide:replace" />
							{t("Elegir imagen…")}
						</ContextMenuItem>
						<ContextMenuItem onSelect={() => reversoDelDeck(ids())}>
							{t("Usar el reverso del deck")}
						</ContextMenuItem>
					</ContextMenuSubContent>
				</ContextMenuSub>
				{carta?.front && (
					<ContextMenuSub>
						<ContextMenuSubTrigger disabled={varias}>
							<Icono icon="lucide:download" />
							{t("Exportar imagen")}
						</ContextMenuSubTrigger>
						<ContextMenuSubContent className="text-xs">
							<ContextMenuItem onSelect={intentar(() => exportarImagen(id, false, t))}>
								{t("Frente…")}
							</ContextMenuItem>
							<ContextMenuItem
								disabled={!tieneReverso(carta)}
								onSelect={intentar(() => exportarImagen(id, true, t))}
							>
								{t("Reverso…")}
							</ContextMenuItem>
							<p className="px-2 py-1.5 text-[10px] text-muted-foreground">
								{t(
									"PNG, JPEG, WebP, TIFF o BMP, según la extensión del nombre. A los PPP de la calidad del PDF y sin sangrado.",
								)}
							</p>
						</ContextMenuSubContent>
					</ContextMenuSub>
				)}
				<ContextMenuSeparator />
				<ContextMenuItem onSelect={() => moverUno(ids(), -1)}>
					<Icono icon="lucide:arrow-left" />
					{t("Mover antes")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => moverUno(ids(), 1)}>
					<Icono icon="lucide:arrow-right" />
					{t("Mover después")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => useUi.getState().abrir({ tipo: "moverA", ids: ids() })}>
					<Icono icon="lucide:hash" />
					{t("Mover a posición…")}
				</ContextMenuItem>
				<ContextMenuItem onSelect={() => fijarCartas(ids(), !fijada)}>
					{fijada ? <Icono icon="lucide:lock-open" /> : <Icono icon="lucide:lock" />}
					{fijada ? t("Desfijar") : t("Fijar")}
				</ContextMenuItem>
				<ContextMenuSeparator />
				<ContextMenuItem variant="destructive" onSelect={() => eliminarCartas(ids())}>
					<Icono icon="lucide:trash-2" />
					{varias ? t("Eliminar las seleccionadas") : t("Eliminar")}
					<ContextMenuShortcut>Supr</ContextMenuShortcut>
				</ContextMenuItem>
			</ContenidoContextual>
		</ContextMenu>
	);
}
