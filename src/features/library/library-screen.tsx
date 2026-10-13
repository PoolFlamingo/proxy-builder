import { open, save } from "@tauri-apps/plugin-dialog";
import {
	Copy,
	FolderOpen,
	HardDriveDownload,
	Import,
	Layers,
	MoreHorizontal,
	PackageOpen,
	Pencil,
	Plus,
	Search,
	Trash2,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { DeckSummary } from "@/bindings/DeckSummary";
import { Confirmar } from "@/components/confirmar";
import { useLang } from "@/components/language-provider";
import { Button } from "@/components/ui/button";
import {
	Dialog,
	DialogContent,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
	Empty,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { megas, nombreSeguro } from "@/features/export/acciones";
import { RaizScroll, useVisible } from "@/features/preview/visibilidad";
import { EDICION_NEUTRA, urlMiniatura } from "@/lib/image-url";
import { call } from "@/lib/ipc";
import { cn } from "@/lib/utils";
import { abrirDeck, guardarAhora, useDeck } from "@/stores/deck-store";
import { useUi } from "@/stores/ui-store";

/** Parámetros neutros para la miniatura de la portada de un deck. */
const PORTADA = {
	edits: EDICION_NEUTRA,
	edge: {
		enabled: false,
		mode: "contrastEdges" as const,
		amount: 0,
		widthPct: 0,
		autoDetect: false,
		applyScryfall: false,
		applyUploads: false,
		applyBacks: false,
	},
	back: false,
	bleedMm: 0,
	bleedMode: "clamp" as const,
	card: { widthMm: 63, heightMm: 88 },
};

/**
 * **Biblioteca**: los decks guardados en este equipo. Abrir, renombrar,
 * duplicar, borrar, llevarse un deck con sus imágenes (`.pbdeck`) y liberar
 * el espacio de las imágenes que ya no usa nadie.
 */
export function LibraryScreen() {
	const { t, mensaje } = useLang();
	const actual = useDeck((s) => s.deck?.id);
	const [decks, setDecks] = useState<DeckSummary[] | null>(null);
	const [filtro, setFiltro] = useState("");
	const [renombrar, setRenombrar] = useState<DeckSummary | null>(null);
	const [borrar, setBorrar] = useState<DeckSummary | null>(null);
	const [viewport, setViewport] = useState<HTMLDivElement | null>(null);

	const intentar =
		<A extends unknown[]>(f: (...a: A) => Promise<unknown>) =>
		(...a: A) => {
			f(...a).catch((e) => toast.error(mensaje(e)));
		};

	const refrescar = useCallback(async () => {
		await guardarAhora();
		setDecks(await call("deck_list", {}));
	}, []);
	useEffect(() => {
		refrescar().catch((e) => toast.error(mensaje(e)));
	}, [refrescar, mensaje]);

	const visibles = useMemo(() => {
		const f = filtro.trim().toLowerCase();
		return (decks ?? []).filter((d) => !f || d.name.toLowerCase().includes(f));
	}, [decks, filtro]);

	const abrir = intentar(async (id: string) => {
		await abrirDeck(id);
		useUi.getState().setVista("deck");
	});

	const nuevo = intentar(async () => {
		await guardarAhora();
		const d = await call("deck_create", { name: t("Deck sin título") });
		await abrirDeck(d.id);
		useUi.getState().setVista("deck");
	});

	const duplicar = intentar(async (d: DeckSummary) => {
		await call("deck_duplicate", { id: d.id, name: t("{n} (copia)", { n: d.name }) });
		await refrescar();
	});

	const exportarPaquete = intentar(async (d: DeckSummary) => {
		await guardarAhora();
		const destino = await save({
			title: t("Exportar deck con sus imágenes"),
			defaultPath: nombreSeguro(d.name, "pbdeck"),
			filters: [{ name: "Proxy Builder", extensions: ["pbdeck"] }],
		});
		if (!destino) return;
		const bytes = await call("deck_export_package", { id: d.id, path: destino });
		toast.success(t("Deck exportado ({mb})", { mb: megas(bytes) }), {
			action: {
				label: t("Mostrar en carpeta"),
				onClick: () => void call("export_reveal", { path: destino }),
			},
		});
	});

	const importarPaquete = intentar(async () => {
		const ruta = await open({
			title: t("Importar deck"),
			multiple: false,
			directory: false,
			filters: [{ name: "Proxy Builder", extensions: ["pbdeck"] }],
		});
		if (!ruta || Array.isArray(ruta)) return;
		const d = await call("deck_import_package", { path: ruta });
		toast.success(t("«{n}» importado", { n: d.name }));
		await refrescar();
	});

	const liberar = intentar(async () => {
		const r = await call("blob_gc", {});
		toast.success(
			r.removed > 0
				? t("{n} imágenes sin usar borradas ({mb})", { n: r.removed, mb: megas(r.freedBytes) })
				: t("No había imágenes sin usar (se guardan 24 h por si acaso)."),
		);
	});

	return (
		<section className="flex h-full min-h-0 flex-col">
			<div className="flex flex-wrap items-center gap-2.5 border-b px-4 py-2.5">
				<h2 className="text-sm font-semibold">{t("Biblioteca")}</h2>
				<div className="relative w-64">
					<Search className="pointer-events-none absolute top-2 left-2 size-3.5 text-muted-foreground" />
					<Input
						value={filtro}
						onChange={(e) => setFiltro(e.target.value)}
						placeholder={t("Buscar por nombre…")}
						className="h-7 pl-7 text-xs"
						type="search"
					/>
				</div>
				<div className="ml-auto flex items-center gap-1.5">
					<Button size="sm" variant="ghost" onClick={liberar}>
						<HardDriveDownload />
						{t("Liberar espacio")}
					</Button>
					<Button size="sm" variant="outline" onClick={importarPaquete}>
						<Import />
						{t("Importar .pbdeck")}
					</Button>
					<Button size="sm" onClick={nuevo}>
						<Plus />
						{t("Nuevo deck")}
					</Button>
				</div>
			</div>
			<ScrollArea className="min-h-0 flex-1" onViewportChange={setViewport}>
				<RaizScroll.Provider value={viewport}>
					{decks !== null && visibles.length === 0 ? (
						<Empty className="mt-16">
							<EmptyHeader>
								<EmptyMedia variant="icon">
									<Layers />
								</EmptyMedia>
								<EmptyTitle>
									{filtro ? t("Ningún deck con ese nombre") : t("No hay decks guardados")}
								</EmptyTitle>
								<EmptyDescription>
									{t("Los decks se guardan solos mientras trabajas.")}
								</EmptyDescription>
							</EmptyHeader>
						</Empty>
					) : (
						<div className="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-4 p-4">
							{visibles.map((d) => (
								<TarjetaDeck
									key={d.id}
									deck={d}
									abierto={d.id === actual}
									onAbrir={() => abrir(d.id)}
									onRenombrar={() => setRenombrar(d)}
									onDuplicar={() => duplicar(d)}
									onExportar={() => exportarPaquete(d)}
									onBorrar={() => setBorrar(d)}
								/>
							))}
						</div>
					)}
				</RaizScroll.Provider>
			</ScrollArea>

			<DialogoRenombrar
				deck={renombrar}
				onCerrar={() => setRenombrar(null)}
				onHecho={intentar(async (id: string, nombre: string) => {
					await call("deck_rename", { id, name: nombre });
					if (id === useDeck.getState().deck?.id)
						useDeck.getState().setSinHistorial({ name: nombre });
					await refrescar();
				})}
			/>
			<Confirmar
				abierto={borrar !== null}
				onCerrar={() => setBorrar(null)}
				titulo={t("¿Eliminar «{n}»?", { n: borrar?.name ?? "" })}
				descripcion={t("El deck se borra de la biblioteca. Esto no se puede deshacer.")}
				accion={t("Eliminar")}
				destructiva
				onConfirmar={intentar(async () => {
					if (!borrar) return;
					const eraElAbierto = borrar.id === useDeck.getState().deck?.id;
					await call("deck_delete", { id: borrar.id });
					if (eraElAbierto) {
						const d = await call("deck_create", { name: t("Deck sin título") });
						await abrirDeck(d.id);
					}
					await refrescar();
				})}
			/>
		</section>
	);
}

function TarjetaDeck({
	deck: d,
	abierto,
	onAbrir,
	onRenombrar,
	onDuplicar,
	onExportar,
	onBorrar,
}: {
	deck: DeckSummary;
	abierto: boolean;
	onAbrir: () => void;
	onRenombrar: () => void;
	onDuplicar: () => void;
	onExportar: () => void;
	onBorrar: () => void;
}) {
	const { t, lang } = useLang();
	const [ref, setRef] = useState<HTMLDivElement | null>(null);
	const visible = useVisible(ref);
	const fecha = new Date(d.updatedAt).toLocaleString(lang === "en" ? "en-GB" : "es-ES", {
		dateStyle: "medium",
		timeStyle: "short",
	});
	return (
		<div
			ref={setRef}
			className={cn(
				"group flex flex-col gap-2 rounded-xl border bg-card p-2.5 transition-shadow hover:shadow-md",
				abierto && "ring-2 ring-primary",
			)}
		>
			<button
				type="button"
				onClick={onAbrir}
				className="relative aspect-[63/88] w-full overflow-hidden rounded-[4.5%] bg-muted"
			>
				{d.thumbnail && visible ? (
					<img
						src={urlMiniatura(d.thumbnail, PORTADA, 244)}
						alt=""
						loading="lazy"
						className="size-full object-cover transition-transform group-hover:scale-[1.02]"
					/>
				) : (
					<div className="flex size-full items-center justify-center text-muted-foreground">
						<Layers className="size-8" />
					</div>
				)}
			</button>
			<div className="flex items-start gap-1">
				<div className="min-w-0 flex-1">
					<div className="truncate text-xs font-semibold">{d.name}</div>
					<div className="text-[10px] text-muted-foreground">
						{t("{n} cartas", { n: d.cardCount })} · {fecha}
					</div>
				</div>
				<DropdownMenu>
					<DropdownMenuTrigger asChild>
						<Button size="icon-xs" variant="ghost" aria-label={t("Más acciones")}>
							<MoreHorizontal />
						</Button>
					</DropdownMenuTrigger>
					<DropdownMenuContent align="end" className="text-xs">
						<DropdownMenuItem onSelect={onAbrir}>
							<FolderOpen />
							{t("Abrir")}
						</DropdownMenuItem>
						<DropdownMenuItem onSelect={onRenombrar}>
							<Pencil />
							{t("Renombrar")}
						</DropdownMenuItem>
						<DropdownMenuItem onSelect={onDuplicar}>
							<Copy />
							{t("Duplicar")}
						</DropdownMenuItem>
						<DropdownMenuItem onSelect={onExportar}>
							<PackageOpen />
							{t("Exportar .pbdeck")}
						</DropdownMenuItem>
						<DropdownMenuSeparator />
						<DropdownMenuItem variant="destructive" onSelect={onBorrar}>
							<Trash2 />
							{t("Eliminar")}
						</DropdownMenuItem>
					</DropdownMenuContent>
				</DropdownMenu>
			</div>
		</div>
	);
}

function DialogoRenombrar({
	deck,
	onCerrar,
	onHecho,
}: {
	deck: DeckSummary | null;
	onCerrar: () => void;
	onHecho: (id: string, nombre: string) => void;
}) {
	const { t } = useLang();
	const [nombre, setNombre] = useState("");
	useEffect(() => setNombre(deck?.name ?? ""), [deck]);
	const hecho = () => {
		if (!deck || !nombre.trim()) return;
		onHecho(deck.id, nombre.trim());
		onCerrar();
	};
	return (
		<Dialog open={deck !== null} onOpenChange={(o) => !o && onCerrar()}>
			<DialogContent className="sm:max-w-sm" cambiosSinGuardar={false}>
				<DialogHeader>
					<DialogTitle>{t("Renombrar deck")}</DialogTitle>
				</DialogHeader>
				<Input
					autoFocus
					value={nombre}
					onChange={(e) => setNombre(e.target.value)}
					onKeyDown={(e) => e.key === "Enter" && hecho()}
					className="text-sm"
				/>
				<DialogFooter>
					<Button size="sm" variant="outline" onClick={onCerrar}>
						{t("Cancelar")}
					</Button>
					<Button size="sm" onClick={hecho} disabled={!nombre.trim()}>
						{t("Guardar")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
